// Diagnostic messages are compiled out of optimised release binaries.
macro_rules! debug_trace {
    ($($arg:tt)*) => {{
        #[cfg(debug_assertions)]
        { eprintln!($($arg)*); }
        #[cfg(not(debug_assertions))]
        { let _ = (); }
    }};
}

mod biometric;
mod commands;
mod contact_commands;
mod contacts;
mod file_crypto;
mod identity_store;
mod import_scan;
mod transfer;
mod transfer_codec;
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
                        let _ = (&paths, &position);
                        debug_trace!("[vault-drop][rust] enter count={} x={:.0} y={:.0}", paths.len(), position.x, position.y);
                    }
                    DragDropEvent::Over { .. } => {
                        // Avoid spamming the terminal for every pointer movement.
                    }
                    DragDropEvent::Drop { paths, position } => {
                        let _ = &position;
                        debug_trace!("[vault-drop][rust] drop count={} x={:.0} y={:.0}", paths.len(), position.x, position.y);
                        if paths.is_empty() {
                            debug_trace!("[vault-drop][rust] empty native drop: possible in-memory Example File; frontend handles it");
                            return;
                        }
                        let event_name = match handle.state::<commands::DroppedFileState>().0.lock() {
                            Ok(mut pending) => {
                                if paths.len() == 1 {
                                    *pending = Some(paths[0].clone());
                                    debug_trace!("[vault-drop][rust] native selection staged (path redacted)");
                                    "black-vault-file-staged"
                                } else {
                                    *pending = None;
                                    debug_trace!("[vault-drop][rust] rejected native drop count={}", paths.len());
                                    "black-vault-file-drop-multiple"
                                }
                            }
                            Err(_) => {
                                debug_trace!("[vault-drop][rust][error] pending file state mutex poisoned");
                                return;
                            }
                        };
                        // Emit after releasing the mutex: frontend may immediately
                        // invoke describe_dropped_file, which needs the same lock.
                        match webview.emit(event_name, ()) {
                            Ok(()) => debug_trace!("[vault-drop][rust] emitted {}", event_name),
                            Err(_) => debug_trace!("[vault-drop][rust][error] event emission failed (details withheld)"),
                        }
                    }
                    DragDropEvent::Leave => {
                        debug_trace!("[vault-drop][rust] leave (preserving any completed selection)");
                    }
                    _ => {}
                }
            }
        })
        .setup(|app| {
            use tauri::Manager;
            let app_data = app.path().app_local_data_dir()?;
            app.manage(vault::VaultStore::new(app_data.join("vaults")));
            let contacts = contact_commands::ContactState::load(app_data.join("contacts"))
                .map_err(|_| std::io::Error::other("Unable to load contact security state"))?;
            app.manage(contacts);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            biometric::get_biometric_status,
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
            commands::create_nested_text_file,
            commands::manage_nested_file,
            commands::transfer_parent_file_to_nested,
            commands::read_nested_text_file,
            commands::create_vault_text_file,
            commands::read_vault_text_file,
            commands::update_vault_text_file,
            commands::delete_vault_file,
            commands::decrypt_vault_file,
            commands::export_vault_file,
            contact_commands::get_public_identity,
            contact_commands::list_contacts,
            contact_commands::import_contact_identity,
            contact_commands::verify_contact_fingerprint,
            contact_commands::revoke_contact
        ])
        .run(context())
        .expect("Unable to launch Black Vault");
}
