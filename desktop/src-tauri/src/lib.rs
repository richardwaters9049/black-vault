mod commands;
mod vault;

pub(crate) fn context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::PickerState::default())
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
            commands::lock_vault
        ])
        .run(context())
        .expect("Unable to launch Black Vault");
}
