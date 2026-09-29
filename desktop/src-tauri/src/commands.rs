use serde::Serialize;
use std::{fs, path::Path, sync::Mutex};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
pub struct PickerState(Mutex<()>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopInfo {
    app_name: &'static str,
    version: &'static str,
    platform: &'static str,
    architecture: &'static str,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SelectedFile {
    name: String,
    size_bytes: u64,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct NativeError {
    code: &'static str,
    message: &'static str,
}

impl NativeError {
    fn unavailable() -> Self {
        Self {
            code: "file_unavailable",
            message: "The selected file is no longer available or its details cannot be accessed.",
        }
    }
}

#[tauri::command]
pub fn get_desktop_info() -> DesktopInfo {
    DesktopInfo {
        app_name: "Black Vault",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
    }
}

// Only the native picker supplies a path. JavaScript cannot request arbitrary paths.
// This reads filesystem metadata, never file contents, and never persists the selection.
fn describe_selection(path: &Path) -> Result<SelectedFile, NativeError> {
    let metadata = fs::metadata(path).map_err(|_| NativeError::unavailable())?;
    if !metadata.is_file() {
        return Err(NativeError {
            code: "not_a_file",
            message: "Select a regular file rather than a folder or device.",
        });
    }
    if metadata.len() > 9_007_199_254_740_991 {
        return Err(NativeError {
            code: "file_too_large",
            message: "The selected file size cannot be represented accurately.",
        });
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or(NativeError {
            code: "invalid_filename",
            message: "The filename cannot be displayed as Unicode.",
        })?;
    Ok(SelectedFile {
        name: name.to_owned(),
        size_bytes: metadata.len(),
    })
}

#[tauri::command]
pub async fn select_file(app: tauri::AppHandle) -> Result<Option<SelectedFile>, NativeError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<PickerState>();
        let _guard = state.0.try_lock().map_err(|_| NativeError {
            code: "picker_busy",
            message: "A file picker is already open.",
        })?;
        let selection = app
            .dialog()
            .file()
            .set_title("Select a file — Black Vault")
            .blocking_pick_file();
        let Some(selection) = selection else {
            eprintln!("black_vault event=file_selection_cancelled");
            return Ok(None);
        };
        let path = selection
            .into_path()
            .map_err(|_| NativeError::unavailable())?;
        let result = describe_selection(&path);
        // Deliberately exclude paths, filenames, sizes and file contents from audit events.
        eprintln!(
            "black_vault event={}",
            if result.is_ok() {
                "file_selected"
            } else {
                "file_selection_rejected"
            }
        );
        result.map(Some)
    })
    .await
    .map_err(|_| NativeError {
        code: "picker_failed",
        message: "The native file picker could not complete. Please try again.",
    })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/round-trip.txt")
    }
    #[test]
    fn metadata_contract_contains_only_a_name_and_exact_size() {
        let path = fixture();
        assert_eq!(
            serde_json::to_value(describe_selection(&path).unwrap()).unwrap(),
            json!({
                "name": "round-trip.txt", "sizeBytes": fs::metadata(path).unwrap().len(),
            })
        );
    }
    #[test]
    fn missing_files_and_directories_are_rejected_without_leaking_paths() {
        let error = describe_selection(&fixture().join("missing")).unwrap_err();
        assert_eq!(error.code, "file_unavailable");
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("round-trip")
        );
        assert_eq!(
            describe_selection(fixture().parent().unwrap())
                .unwrap_err()
                .code,
            "not_a_file"
        );
    }
    #[test]
    fn desktop_info_contract_matches_the_typescript_field_names() {
        assert_eq!(
            serde_json::to_value(get_desktop_info()).unwrap(),
            json!({
                "appName": "Black Vault", "version": env!("CARGO_PKG_VERSION"),
                "platform": std::env::consts::OS, "architecture": std::env::consts::ARCH,
            })
        );
    }
    #[test]
    fn ipc_allows_only_the_main_local_window() {
        let app = tauri::test::mock_builder()
            .invoke_handler(tauri::generate_handler![get_desktop_info])
            .build(crate::context())
            .unwrap();
        for (label, origin, allowed) in [
            ("main", "tauri://localhost", true),
            ("other", "tauri://localhost", false),
            ("main", "https://example.com", false),
        ] {
            let window = app.get_webview_window(label).unwrap_or_else(|| {
                tauri::WebviewWindowBuilder::new(&app, label, Default::default())
                    .build()
                    .unwrap()
            });
            let result = tauri::test::get_ipc_response(
                &window,
                tauri::webview::InvokeRequest {
                    cmd: "get_desktop_info".into(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: origin.parse().unwrap(),
                    body: tauri::ipc::InvokeBody::default(),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.to_string(),
                },
            );
            assert_eq!(result.is_ok(), allowed, "Unexpected IPC access for {label}");
            if allowed {
                assert_eq!(
                    result.unwrap().deserialize::<serde_json::Value>().unwrap()["appName"],
                    "Black Vault"
                );
            }
        }
    }
}
