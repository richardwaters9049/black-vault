fn main() {
    let attributes = tauri_build::Attributes::new();

    #[cfg(windows)]
    let attributes =
        attributes.windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());

    tauri_build::try_build(
        attributes.app_manifest(tauri_build::AppManifest::new().commands(&[
            "get_biometric_status",
            "authenticate_biometric",
            "get_desktop_info",
            "select_file",
            "list_vaults",
            "create_vault",
            "unlock_vault",
            "lock_vault",
            "import_vault_file",
            "describe_dropped_file",
            "import_dropped_file",
            "import_example_file",
            "delete_locked_vault",
            "list_vault_files",
            "list_nested_vaults",
            "lock_selected_files",
            "unlock_nested_vault",
            "restore_nested_vault",
            "create_nested_text_file",
            "manage_nested_file",
            "transfer_parent_file_to_nested",
            "read_nested_text_file",
            "create_vault_text_file",
            "read_vault_text_file",
            "update_vault_text_file",
            "delete_vault_file",
            "decrypt_vault_file",
            "export_vault_file",
            "get_public_identity",
            "list_contacts",
            "import_contact_identity",
            "verify_contact_fingerprint",
            "revoke_contact",
        ])),
    )
    .expect("Unable to build the desktop command permissions");

    // Tauri's Windows test binaries need the Common Controls v6 manifest.
    // Without it, cargo test can terminate before the Rust harness starts
    // with STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139).
    #[cfg(windows)]
    embed_windows_manifest();
}

#[cfg(windows)]
fn embed_windows_manifest() {
    let manifest = std::path::Path::new("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
}
