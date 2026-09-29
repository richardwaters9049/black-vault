mod commands;
mod file_crypto;
mod import_scan;
mod vault;

pub(crate) fn context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::PickerState::default())
        .manage(commands::DroppedFileState::default())
        .on_webview_event(|webview, event| {
            use tauri::{DragDropEvent, Emitter, Manager, WebviewEvent};
            if webview.label() != "main" { return; }
            if let WebviewEvent::DragDrop(drag) = event {
                let handle = webview.app_handle();
                match drag {
                    DragDropEvent::Enter { paths, position } => {
                        eprintln!("[vault-drop][rust] enter count={} x={:.0} y={:.0}", paths.len(), position.x, position.y);
                    }
                    DragDropEvent::Over { .. } => {
                        // Avoid spamming the terminal for every pointer movement.
                    }
                    DragDropEvent::Drop { paths, position } => {
                        eprintln!("[vault-drop][rust] drop count={} x={:.0} y={:.0}", paths.len(), position.x, position.y);
                        if paths.is_empty() {
                            eprintln!("[vault-drop][rust] empty native drop: possible in-memory Example File; frontend handles it");
                            return;
                        }
                        let event_name = match handle.state::<commands::DroppedFileState>().0.lock() {
                            Ok(mut pending) => {
                                if paths.len() == 1 {
                                    *pending = Some(paths[0].clone());
                                    eprintln!("[vault-drop][rust] native selection staged (path redacted)");
                                    "black-vault-file-staged"
                                } else {
                                    *pending = None;
                                    eprintln!("[vault-drop][rust] rejected native drop count={}", paths.len());
                                    "black-vault-file-drop-multiple"
                                }
                            }
                            Err(_) => {
                                eprintln!("[vault-drop][rust][error] pending file state mutex poisoned");
                                return;
                            }
                        };
                        // Emit after releasing the mutex: frontend may immediately
                        // invoke describe_dropped_file, which needs the same lock.
                        match webview.emit(event_name, ()) {
                            Ok(()) => eprintln!("[vault-drop][rust] emitted {}", event_name),
                            Err(error) => eprintln!("[vault-drop][rust][error] emit failed: {}", error),
                        }
                    }
                    DragDropEvent::Leave => {
                        eprintln!("[vault-drop][rust] leave (preserving any completed selection)");
                    }
                    _ => {}
                }
            }
        })
        .setup(|app| {
            use tauri::Manager;
            let root = app.path().app_local_data_dir()?.join("vaults");
            app.manage(vault::VaultStore::new(root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_desktop_info,
            commands::select_file,
            commands::list_vaults,
            commands::create_vault,
            commands::unlock_vault,
            commands::lock_vault,
            commands::import_vault_file,
            commands::describe_dropped_file,
            commands::import_dropped_file,
            commands::import_example_file,
            commands::delete_locked_vault,
            commands::list_vault_files,
            commands::list_nested_vaults,
            commands::lock_selected_files,
            commands::unlock_nested_vault,
            commands::restore_nested_vault,
            commands::create_vault_text_file,
            commands::read_vault_text_file,
            commands::decrypt_vault_file,
            commands::export_vault_file
        ])
        .run(context())
        .expect("Unable to launch Black Vault");
}
