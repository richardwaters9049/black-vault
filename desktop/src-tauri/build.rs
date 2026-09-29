fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_desktop_info",
            "select_file",
            "list_vaults",
            "create_vault",
            "unlock_vault",
            "lock_vault",
            "import_vault_file",
            "import_example_file",
            "delete_locked_vault",
            "list_vault_files",
            "export_vault_file",
        ]),
    ))
    .expect("Unable to build the desktop command permissions");
}
