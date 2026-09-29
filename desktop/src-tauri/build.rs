fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&["get_desktop_info", "select_file"]),
    ))
    .expect("Unable to build the desktop command permissions");
}
