use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct VaultError {
    pub code: &'static str,
    pub message: &'static str,
}

impl VaultError {
    pub(crate) fn picker_busy() -> Self {
        Self::new("picker_busy", "A native file picker is already open.")
    }

    pub(crate) fn storage_error() -> Self {
        Self::storage()
    }

    pub(crate) fn locked() -> Self {
        Self::new("vault_locked", "Unlock a vault to access files.")
    }

    pub(crate) fn file_invalid() -> Self {
        Self::new(
            "invalid_encrypted_file",
            "The encrypted file could not be authenticated.",
        )
    }

    pub(crate) fn file_unavailable() -> Self {
        Self::new(
            "file_unavailable",
            "The selected file is unavailable or is not a regular file.",
        )
    }

    pub(crate) fn file_large() -> Self {
        Self::new("file_too_large", "Files must not exceed 5 MiB.")
    }

    pub(crate) fn scan_blocked(finding: crate::import_scan::ScanFinding) -> Self {
        Self::new("import_scan_blocked", finding.message())
    }

    pub(crate) fn file_name() -> Self {
        Self::new("invalid_filename", "The filename is unsupported.")
    }

    pub(crate) fn export_exists() -> Self {
        Self::new(
            "export_destination_exists",
            "Choose a new destination; existing files will not be overwritten.",
        )
    }

    pub(super) fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }

    pub(super) fn storage() -> Self {
        Self::new("storage_unavailable", "Local vault storage is unavailable.")
    }

    pub(super) fn invalid() -> Self {
        Self::new(
            "invalid_credentials",
            "The password is incorrect or the vault cannot be authenticated.",
        )
    }

    pub(super) fn busy() -> Self {
        Self::new(
            "vault_already_unlocked",
            "Lock the currently open vault first.",
        )
    }
}
