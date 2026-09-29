fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_desktop_info",
            "select_file",
            "list_vaults",
            "create_vault",
            "unlock_vault",
            "lock_vault",
        ]),
    ))
    .expect("Unable to build the desktop command permissions");
}
