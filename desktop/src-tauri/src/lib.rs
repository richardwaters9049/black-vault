mod commands;

pub(crate) fn context<R: tauri::Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::PickerState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_desktop_info,
            commands::select_file
        ])
        .run(context())
        .expect("Unable to launch Black Vault");
}
