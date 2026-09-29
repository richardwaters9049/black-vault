//! Sprint 02 local vault foundation. No secret material crosses the Tauri IPC boundary.
//! A separate random master key is wrapped for each vault with a password-derived KEK.
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
};
use zeroize::{Zeroize, Zeroizing};

const KDF_MEMORY_KIB: u32 = 19_456;
const KDF_ITERATIONS: u32 = 2;
const KDF_PARALLELISM: u32 = 1;
const MASTER_KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const RECORD_MAX_BYTES: u64 = 8_192;

// Test-only, thread-local deterministic failure points for encrypted transfer and restore
// commit sequence. Never enabled in production; no secret material is logged.
#[cfg(test)]
thread_local! {
    static TRANSFER_FAILURE: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}
#[cfg(test)]
fn transfer_checkpoint(stage: &'static str) -> std::io::Result<()> {
    // Opt-in integration child exits immediately at a real persistence boundary.
    // Compiled only for tests; the parent verifies the exit and reopens storage.
    if std::env::var("BV_TEST_EXIT_STAGE").ok().as_deref() == Some(stage) {
        if let Ok(marker) = std::env::var("BV_TEST_EXIT_MARKER") {
            std::fs::write(&marker, stage)?;
            File::open(&marker)?.sync_all()?;
        }
        std::process::exit(86);
    }
    if TRANSFER_FAILURE.with(|slot| slot.get() == Some(stage)) {
        return Err(std::io::Error::other("injected transfer interruption"));
    }
    Ok(())
}
#[cfg(not(test))]
fn transfer_checkpoint(_stage: &'static str) -> std::io::Result<()> {
    Ok(())
}

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
    fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
    fn storage() -> Self {
        Self::new("storage_unavailable", "Local vault storage is unavailable.")
    }
    fn invalid() -> Self {
        Self::new(
            "invalid_credentials",
            "The password is incorrect or the vault cannot be authenticated.",
        )
    }
    fn busy() -> Self {
        Self::new(
            "vault_already_unlocked",
            "Lock the currently open vault first.",
        )
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultSummary {
    pub id: String,
    // Names are encrypted on disk. Only the unlocked vault exposes its name.
    pub name: Option<String>,
    pub is_locked: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KdfRecord {
    algorithm: String,
    salt: String,
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
    output_len: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WrappedSecret {
    nonce: String,
    ciphertext: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VaultRecord {
    version: u8,
    id: String,
    kdf: KdfRecord,
    wrapped: WrappedSecret,
}
struct OpenVault {
    id: String,
    name: Zeroizing<String>,
    // Zeroising is best-effort; host compromise while unlocked is out of scope.
    #[allow(dead_code)]
    // Retained for subsequent encrypted-file operations; never exposed to JavaScript.
    master_key: Zeroizing<[u8; MASTER_KEY_LEN]>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NestedVaultRecord {
    version: u8,
    parent_id: String,
    id: String,
    kdf: KdfRecord,
    // Double wrapping: parent master key AND independent child password required.
    parent_wrapped: WrappedSecret,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NestedVaultPayload {
    name: String,
    // Existing encrypted envelopes, never plaintext.
    files: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NestedVaultSummary {
    pub id: String,
    pub file_count: usize,
}

pub struct VaultStore {
    root: PathBuf,
    active: Mutex<Option<OpenVault>>,
    nested_attempts: Mutex<std::collections::HashMap<String, (u8, std::time::Instant)>>,
}
impl VaultStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            active: Mutex::new(None),
            nested_attempts: Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn lock_active(&self) -> Result<std::sync::MutexGuard<'_, Option<OpenVault>>, VaultError> {
        self.active.lock().map_err(|_| {
            VaultError::new(
                "vault_state_unavailable",
                "The vault session is unavailable.",
            )
        })
    }
    fn check_nested_attempt(&self, id: &str) -> Result<(), VaultError> {
        let mut attempts = self
            .nested_attempts
            .lock()
            .map_err(|_| VaultError::storage())?;
        if let Some((count, since)) = attempts.get(id).copied()
            && count >= 5
        {
            if since.elapsed() < std::time::Duration::from_secs(60) {
                return Err(VaultError::new(
                    "nested_pin_cooldown",
                    "Too many incorrect PINs. Wait 60 seconds before trying again.",
                ));
            }
            attempts.remove(id);
        }
        Ok(())
    }

    fn record_nested_attempt(&self, id: &str, success: bool) -> Result<(), VaultError> {
        let mut attempts = self
            .nested_attempts
            .lock()
            .map_err(|_| VaultError::storage())?;
        if success {
            attempts.remove(id);
        } else {
            let entry = attempts
                .entry(id.to_owned())
                .or_insert((0, std::time::Instant::now()));
            entry.0 = entry.0.saturating_add(1);
            if entry.0 == 5 {
                entry.1 = std::time::Instant::now();
            }
        }
        Ok(())
    }

    fn ensure_root(&self) -> Result<(), VaultError> {
        fs::create_dir_all(&self.root).map_err(|_| VaultError::storage())?;
        let meta = fs::symlink_metadata(&self.root).map_err(|_| VaultError::storage())?;
        if !meta.file_type().is_dir() {
            return Err(VaultError::storage());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))
                .map_err(|_| VaultError::storage())?;
        }
        Ok(())
    }
    fn file_path(&self, id: &str) -> Result<PathBuf, VaultError> {
        if !valid_id(id) {
            return Err(VaultError::new(
                "invalid_vault_id",
                "The vault identifier is invalid.",
            ));
        }
        Ok(self.root.join(format!("{id}.vault")))
    }

    pub fn list(&self) -> Result<Vec<VaultSummary>, VaultError> {
        let guard = self.lock_active()?;
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut result = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|_| VaultError::storage())? {
            let entry = entry.map_err(|_| VaultError::storage())?;
            if !entry
                .file_type()
                .map_err(|_| VaultError::storage())?
                .is_file()
            {
                continue;
            }
            let filename = entry.file_name();
            let Some(id) = filename
                .to_str()
                .and_then(|name| name.strip_suffix(".vault"))
            else {
                continue;
            };
            if !valid_id(id) {
                continue;
            }
            let open = guard.as_ref().filter(|vault| vault.id == id);
            result.push(VaultSummary {
                id: id.to_owned(),
                name: open.map(|vault| vault.name.to_string()),
                is_locked: open.is_none(),
            });
        }
        result.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(result)
    }

    pub fn create(&self, name: String, password: String) -> Result<VaultSummary, VaultError> {
        let password = Zeroizing::new(password);
        let name = Zeroizing::new(name);
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(VaultError::new(
                "invalid_vault_name",
                "Use a vault name of 1–80 visible characters.",
            ));
        }
        if !(12..=1024).contains(&password.len()) {
            return Err(VaultError::new(
                "invalid_password",
                "Use a password of 12–1024 UTF-8 bytes.",
            ));
        }
        let mut guard = self.lock_active()?;
        if guard.is_some() {
            return Err(VaultError::busy());
        }
        self.ensure_root()?;
        let id = loop {
            let mut bytes = [0u8; 16];
            OsRng.fill_bytes(&mut bytes);
            let id: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            if !self.file_path(&id)?.exists() {
                break id;
            }
        };
        let mut salt = [0u8; SALT_LEN];
        OsRng.fill_bytes(&mut salt);
        let kdf = KdfRecord {
            algorithm: "argon2id-v19".to_owned(),
            salt: STANDARD.encode(salt),
            m_cost: KDF_MEMORY_KIB,
            t_cost: KDF_ITERATIONS,
            p_cost: KDF_PARALLELISM,
            output_len: MASTER_KEY_LEN,
        };
        let kek = derive(&password, &kdf)?;
        salt.zeroize();
        let mut master_key = Zeroizing::new([0u8; MASTER_KEY_LEN]);
        OsRng.fill_bytes(master_key.as_mut());
        // The encrypted payload is [u16 name byte length][name UTF-8][32-byte master key].
        let name_bytes = name.as_bytes();
        let mut plaintext =
            Zeroizing::new(Vec::with_capacity(2 + name_bytes.len() + MASTER_KEY_LEN));
        plaintext.extend_from_slice(&(name_bytes.len() as u16).to_be_bytes());
        plaintext.extend_from_slice(name_bytes);
        plaintext.extend_from_slice(master_key.as_ref());
        let wrapped = seal(&kek, &id, &plaintext)?;
        let record = VaultRecord {
            version: 1,
            id: id.clone(),
            kdf,
            wrapped,
        };
        let bytes = serde_json::to_vec(&record).map_err(|_| VaultError::storage())?;
        self.persist_new(&id, &bytes)?;
        *guard = Some(OpenVault {
            id: id.clone(),
            name: Zeroizing::new(name.to_owned()),
            master_key,
        });
        Ok(VaultSummary {
            id,
            name: Some(name.to_owned()),
            is_locked: false,
        })
    }

    pub fn unlock(&self, id: String, password: String) -> Result<VaultSummary, VaultError> {
        let password = Zeroizing::new(password);
        let mut guard = self.lock_active()?;
        if guard.is_some() {
            return Err(VaultError::busy());
        }
        let path = self.file_path(&id)?;
        let meta = fs::symlink_metadata(&path)
            .map_err(|_| VaultError::new("vault_not_found", "This local vault does not exist."))?;
        if !meta.file_type().is_file() || meta.len() > RECORD_MAX_BYTES {
            return Err(VaultError::new(
                "invalid_vault_record",
                "The vault header is invalid.",
            ));
        }
        let bytes = fs::read(path).map_err(|_| VaultError::storage())?;
        let record: VaultRecord = serde_json::from_slice(&bytes)
            .map_err(|_| VaultError::new("invalid_vault_record", "The vault header is invalid."))?;
        if record.version != 1 || record.id != id {
            return Err(VaultError::new(
                "unsupported_vault",
                "This vault header version or identifier is unsupported.",
            ));
        }
        let kek = derive(&password, &record.kdf)?;
        let plaintext = open(&kek, &id, &record.wrapped)?;
        if plaintext.len() < 2 + MASTER_KEY_LEN {
            return Err(VaultError::invalid());
        }
        let name_len = usize::from(u16::from_be_bytes([plaintext[0], plaintext[1]]));
        if name_len == 0 || plaintext.len() != 2 + name_len + MASTER_KEY_LEN {
            return Err(VaultError::invalid());
        }
        let name = Zeroizing::new(
            std::str::from_utf8(&plaintext[2..2 + name_len])
                .map_err(|_| VaultError::invalid())?
                .to_owned(),
        );
        if name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(VaultError::invalid());
        }
        let mut master_key = Zeroizing::new([0u8; MASTER_KEY_LEN]);
        master_key.copy_from_slice(&plaintext[2 + name_len..]);
        *guard = Some(OpenVault {
            id: id.clone(),
            name: Zeroizing::new(name.to_string()),
            master_key,
        });
        Ok(VaultSummary {
            id,
            name: Some(name.to_string()),
            is_locked: false,
        })
    }

    pub fn lock(&self) -> Result<(), VaultError> {
        let mut guard = self.lock_active()?;
        // Drop zeroises the master key. There is no active session token in JavaScript.
        *guard = None;
        self.nested_attempts
            .lock()
            .map_err(|_| VaultError::storage())?
            .clear();
        Ok(())
    }

    fn active_file_dir(&self, vault: &OpenVault) -> Result<PathBuf, VaultError> {
        let dir = self.root.join(format!("{}.files", vault.id));
        if dir.exists() {
            let meta = fs::symlink_metadata(&dir).map_err(|_| VaultError::storage())?;
            if !meta.file_type().is_dir() {
                return Err(VaultError::storage());
            }
        } else {
            fs::create_dir(&dir).map_err(|_| VaultError::storage())?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
                .map_err(|_| VaultError::storage())?;
        }
        Ok(dir)
    }

    pub fn import_path(
        &self,
        path: &std::path::Path,
    ) -> Result<crate::file_crypto::VaultFile, VaultError> {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(VaultError::file_name)?;
        let bytes = crate::file_crypto::read_regular(path, crate::file_crypto::MAX_FILE_BYTES)?;
        self.import_bytes(name, &bytes)
    }

    pub fn create_text_file(
        &self,
        name: &str,
        content: &str,
    ) -> Result<crate::file_crypto::VaultFile, VaultError> {
        const ALLOWED: &[&str] = &["txt", "md", "json", "csv", "log"];
        let extension = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if !ALLOWED.contains(&extension.as_str()) {
            return Err(VaultError::file_name());
        }
        self.import_bytes(name, content.as_bytes())
    }

    pub fn read_text_file(&self, file_id: &str) -> Result<String, VaultError> {
        if !crate::file_crypto::valid_id(file_id) {
            return Err(VaultError::file_invalid());
        }
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        let dir = self.root.join(format!("{}.files", vault.id));
        let sealed = crate::file_crypto::read_regular(
            &dir.join(format!("{file_id}.bvfile")),
            8 * 1024 * 1024,
        )?;
        let (info, plaintext) =
            crate::file_crypto::open(&vault.id, file_id, &vault.master_key, &sealed)?;
        let extension = info
            .name
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !["txt", "md", "json", "csv", "log"].contains(&extension.as_str()) {
            return Err(VaultError::file_invalid());
        }
        String::from_utf8(plaintext.to_vec()).map_err(|_| VaultError::file_invalid())
    }

    pub fn update_text_file(
        &self,
        file_id: &str,
        name: &str,
        content: &str,
    ) -> Result<crate::file_crypto::VaultFile, VaultError> {
        if !valid_id(file_id) {
            return Err(VaultError::file_invalid());
        }
        let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if !["txt", "md", "json", "csv", "log"].contains(&ext.as_str()) {
            return Err(VaultError::file_name());
        }
        if content.len() as u64 > crate::file_crypto::MAX_FILE_BYTES {
            return Err(VaultError::file_large());
        }
        crate::import_scan::scan(name, content.as_bytes()).map_err(VaultError::scan_blocked)?;
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        let dir = self.active_file_dir(vault)?;
        let path = dir.join(format!("{file_id}.bvfile"));
        let existing = crate::file_crypto::read_regular(&path, 8 * 1024 * 1024)?;
        let (previous, _) =
            crate::file_crypto::open(&vault.id, file_id, &vault.master_key, &existing)?;
        if !["txt", "md", "json", "csv", "log"].contains(
            &previous
                .name
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str(),
        ) {
            return Err(VaultError::file_invalid());
        }
        for entry in fs::read_dir(&dir).map_err(|_| VaultError::storage())? {
            let entry = entry.map_err(|_| VaultError::storage())?;
            let filename = entry.file_name();
            let Some(other_id) = filename.to_str().and_then(|v| v.strip_suffix(".bvfile")) else {
                continue;
            };
            if other_id == file_id || !valid_id(other_id) {
                continue;
            }
            let bytes = crate::file_crypto::read_regular(&entry.path(), 8 * 1024 * 1024)?;
            let (info, _) =
                crate::file_crypto::open(&vault.id, other_id, &vault.master_key, &bytes)?;
            if info.name.eq_ignore_ascii_case(name) {
                return Err(VaultError::new(
                    "duplicate_filename",
                    "A file with this name already exists.",
                ));
            }
        }
        let sealed = crate::file_crypto::seal(
            &vault.id,
            file_id,
            &vault.master_key,
            name,
            content.as_bytes(),
        )?;
        let temp = dir.join(format!(
            ".{file_id}.{}.update-tmp",
            crate::file_crypto::random_id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| -> std::io::Result<()> {
            let mut output = options.open(&temp)?;
            output.write_all(&sealed)?;
            output.sync_all()?;
            fs::rename(&temp, &path)?;
            File::open(&dir)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
            return Err(VaultError::storage());
        }
        Ok(crate::file_crypto::VaultFile {
            id: file_id.into(),
            name: name.into(),
            size_bytes: content.len() as u64,
        })
    }

    pub fn delete_vault_file(&self, file_id: &str) -> Result<(), VaultError> {
        if !valid_id(file_id) {
            return Err(VaultError::file_invalid());
        }
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        let dir = self.active_file_dir(vault)?;
        let path = dir.join(format!("{file_id}.bvfile"));
        let sealed = crate::file_crypto::read_regular(&path, 8 * 1024 * 1024)?;
        crate::file_crypto::open(&vault.id, file_id, &vault.master_key, &sealed)?;
        fs::remove_file(path).map_err(|_| VaultError::storage())?;
        File::open(dir)
            .map_err(|_| VaultError::storage())?
            .sync_all()
            .map_err(|_| VaultError::storage())
    }

    pub fn import_example(
        &self,
        example: &str,
    ) -> Result<crate::file_crypto::VaultFile, VaultError> {
        let (name, content): (&str, &[u8]) = match example {
            "notes" => ("example-notes.txt", b"Black Vault example notes.\nThese are harmless, local sample bytes.\n"),
            "csv" => ("example-records.csv", b"id,label,status\n1,Alpha,active\n2,Beta,pending\n"),
            "sql" => ("example-query.sql", b"-- Inert SQL example, stored as text.\nSELECT id, label FROM records WHERE id = ?;\n"),
            "json" => ("example-settings.json", br#"{"theme":"dark","offline":true,"version":1}"#),
            "markdown" => ("example-readme.md", b"# Black Vault sample\nAn offline Markdown document.\n"),
            "log" => ("example-audit.log", b"2026-09-29 INFO Sample event\n2026-09-29 INFO No network activity\n"),
            "blocked" => ("example-script.sh", b"#!/bin/sh\necho This example is deliberately rejected.\n"),
            _ => return Err(VaultError::file_unavailable()),
        };
        self.import_bytes(name, content)
    }

    fn import_bytes(
        &self,
        name: &str,
        bytes: &[u8],
    ) -> Result<crate::file_crypto::VaultFile, VaultError> {
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        if bytes.len() as u64 > crate::file_crypto::MAX_FILE_BYTES {
            return Err(VaultError::file_large());
        }
        // Screen exactly the in-memory bytes that will be encrypted; never execute examples.
        crate::import_scan::scan(name, bytes).map_err(VaultError::scan_blocked)?;
        let dir = self.active_file_dir(vault)?;
        let id = loop {
            let candidate = crate::file_crypto::random_id();
            if !dir.join(format!("{candidate}.bvfile")).exists()
                && !dir.join(format!(".{candidate}.tmp")).exists()
            {
                break candidate;
            }
        };
        let sealed = crate::file_crypto::seal(&vault.id, &id, &vault.master_key, name, bytes)?;
        crate::file_crypto::persist_new(&dir, &id, &sealed)?;
        Ok(crate::file_crypto::VaultFile {
            id,
            name: name.into(),
            size_bytes: bytes.len() as u64,
        })
    }

    pub fn delete_locked(&self, id: &str, confirmation: &str) -> Result<(), VaultError> {
        if !valid_id(id) || confirmation != format!("DELETE {id}") {
            return Err(VaultError::new(
                "delete_confirmation_required",
                "Enter DELETE followed by the full vault ID to confirm permanent deletion.",
            ));
        }
        let guard = self.lock_active()?;
        if guard.is_some() {
            return Err(VaultError::new(
                "lock_before_delete",
                "Lock the active vault before deleting a vault.",
            ));
        }
        let header = self.file_path(id)?;
        if !fs::symlink_metadata(&header)
            .map_err(|_| VaultError::file_unavailable())?
            .file_type()
            .is_file()
        {
            return Err(VaultError::storage());
        }
        let dir = self.root.join(format!("{id}.files"));
        if dir.exists() {
            if !fs::symlink_metadata(&dir)
                .map_err(|_| VaultError::storage())?
                .file_type()
                .is_dir()
            {
                return Err(VaultError::storage());
            }
            // Refuse deletion when unknown entries are present; never traverse symlinks.
            let mut entries = Vec::new();
            for entry in fs::read_dir(&dir).map_err(|_| VaultError::storage())? {
                let entry = entry.map_err(|_| VaultError::storage())?;
                let filename = entry.file_name();
                let name = filename.to_str().ok_or_else(VaultError::storage)?;
                let valid = name.strip_suffix(".bvlock").map(valid_id).unwrap_or(false)
                    || name
                        .strip_suffix(".bvfile")
                        .or_else(|| name.strip_prefix('.').and_then(|n| n.strip_suffix(".tmp")))
                        .is_some_and(crate::file_crypto::valid_id);
                if !valid
                    || !entry
                        .file_type()
                        .map_err(|_| VaultError::storage())?
                        .is_file()
                {
                    return Err(VaultError::storage());
                }
                entries.push(entry.path());
            }
            for path in entries {
                fs::remove_file(path).map_err(|_| VaultError::storage())?;
            }
            fs::remove_dir(dir).map_err(|_| VaultError::storage())?;
        }
        // Header is removed last; an interrupted deletion never silently retains usable files
        // after removing the header first. Filesystem deletion is not secure erasure on SSDs.
        fs::remove_file(header).map_err(|_| VaultError::storage())?;
        Ok(())
    }

    pub fn list_files(&self) -> Result<Vec<crate::file_crypto::VaultFile>, VaultError> {
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        let dir = self.root.join(format!("{}.files", vault.id));
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let meta = fs::symlink_metadata(&dir).map_err(|_| VaultError::storage())?;
        if !meta.file_type().is_dir() {
            return Err(VaultError::storage());
        }
        let mut files = Vec::new();
        for entry in fs::read_dir(dir).map_err(|_| VaultError::storage())? {
            let entry = entry.map_err(|_| VaultError::storage())?;
            if !entry
                .file_type()
                .map_err(|_| VaultError::storage())?
                .is_file()
            {
                continue;
            }
            let filename = entry.file_name();
            let Some(id) = filename
                .to_str()
                .and_then(|value| value.strip_suffix(".bvfile"))
            else {
                continue;
            };
            if !crate::file_crypto::valid_id(id) {
                continue;
            }
            let sealed = crate::file_crypto::read_regular(&entry.path(), 8 * 1024 * 1024)?;
            let (info, _) = crate::file_crypto::open(&vault.id, id, &vault.master_key, &sealed)?;
            files.push(info);
        }
        files.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(files)
    }

    pub fn list_nested_vaults(&self) -> Result<Vec<NestedVaultSummary>, VaultError> {
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        let dir = self.root.join(format!("{}.files", vault.id));
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut result = Vec::new();
        for entry in fs::read_dir(dir).map_err(|_| VaultError::storage())? {
            let entry = entry.map_err(|_| VaultError::storage())?;
            if !entry
                .file_type()
                .map_err(|_| VaultError::storage())?
                .is_file()
            {
                continue;
            }
            let name = entry.file_name();
            let Some(id) = name.to_str().and_then(|n| n.strip_suffix(".bvlock")) else {
                continue;
            };
            if !valid_id(id) {
                continue;
            }
            // File count is deliberately not exposed from the encrypted payload.
            result.push(NestedVaultSummary {
                id: id.into(),
                file_count: 0,
            });
        }
        result.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(result)
    }

    pub fn lock_selected_files(
        &self,
        name: String,
        password: String,
        ids: Vec<String>,
    ) -> Result<NestedVaultSummary, VaultError> {
        let password = Zeroizing::new(password);
        if name.trim().is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control)
        {
            return Err(VaultError::new(
                "invalid_vault_name",
                "Enter a name of 1–80 visible characters.",
            ));
        }
        if password.len() != 4 || !password.bytes().all(|digit| digit.is_ascii_digit()) {
            return Err(VaultError::new(
                "invalid_nested_pin",
                "Enter exactly four digits (0–9) for the nested vault PIN.",
            ));
        }
        if ids.is_empty() || ids.len() > 5 || ids.iter().any(|id| !valid_id(id)) {
            return Err(VaultError::new(
                "invalid_selection",
                "Select between 1 and 5 files.",
            ));
        }
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        if unique.len() != ids.len() {
            return Err(VaultError::new(
                "invalid_selection",
                "Duplicate file selection.",
            ));
        }
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        let dir = self.active_file_dir(vault)?;
        let mut files = Vec::new();
        let mut total_bytes = 0usize;
        for id in &ids {
            let bytes = crate::file_crypto::read_regular(
                &dir.join(format!("{id}.bvfile")),
                8 * 1024 * 1024,
            )?;
            // Authenticate every envelope against the parent before moving it.
            crate::file_crypto::open(&vault.id, id, &vault.master_key, &bytes)?;
            total_bytes += bytes.len();
            if total_bytes > 10 * 1024 * 1024 {
                return Err(VaultError::new(
                    "nested_vault_limit",
                    "Selected encrypted files exceed the 10 MiB nested vault limit.",
                ));
            }
            files.push((id.clone(), STANDARD.encode(&bytes)));
        }
        let child_id = crate::file_crypto::random_id();
        let mut salt = [0u8; SALT_LEN];
        OsRng.fill_bytes(&mut salt);
        let kdf = KdfRecord {
            algorithm: "argon2id-v19".into(),
            salt: STANDARD.encode(salt),
            m_cost: KDF_MEMORY_KIB,
            t_cost: KDF_ITERATIONS,
            p_cost: KDF_PARALLELISM,
            output_len: MASTER_KEY_LEN,
        };
        salt.zeroize();
        let kek = derive(&password, &kdf)?;
        let payload = Zeroizing::new(
            serde_json::to_vec(&NestedVaultPayload { name, files })
                .map_err(|_| VaultError::storage())?,
        );
        let inner = Zeroizing::new(
            serde_json::to_vec(&seal(&kek, &child_id, &payload)?)
                .map_err(|_| VaultError::storage())?,
        );
        let parent_wrapped = seal(&vault.master_key, &child_id, &inner)?;
        let record = NestedVaultRecord {
            version: 1,
            parent_id: vault.id.clone(),
            id: child_id.clone(),
            kdf,
            parent_wrapped,
        };
        let record_bytes = serde_json::to_vec(&record).map_err(|_| VaultError::storage())?;
        let path = dir.join(format!("{child_id}.bvlock"));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(&path).map_err(|_| VaultError::storage())?;
        if output
            .write_all(&record_bytes)
            .and_then(|_| output.sync_all())
            .is_err()
        {
            drop(output);
            let _ = fs::remove_file(&path);
            return Err(VaultError::storage());
        }
        File::open(&dir)
            .map_err(|_| VaultError::storage())?
            .sync_all()
            .map_err(|_| VaultError::storage())?;
        // Delete originals only AFTER the nested record is durable. On interrupted
        // deletion, duplicate ciphertext may remain, but data is not lost.
        for id in &ids {
            fs::remove_file(dir.join(format!("{id}.bvfile"))).map_err(|_| VaultError::storage())?;
        }
        File::open(&dir)
            .map_err(|_| VaultError::storage())?
            .sync_all()
            .map_err(|_| VaultError::storage())?;
        Ok(NestedVaultSummary {
            id: child_id,
            file_count: ids.len(),
        })
    }

    // Best-effort cleanup of encrypted, abandoned staging files only. Never
    // follow links, touch unknown names, or remove a staging file unless its
    // corresponding final envelope is present and authenticated.
    fn cleanup_authenticated_staging(
        &self,
        vault: &OpenVault,
        dir: &std::path::Path,
        nested_id: &str,
    ) -> Result<usize, VaultError> {
        let final_path = dir.join(format!("{nested_id}.bvlock"));
        let bytes = crate::file_crypto::read_regular(&final_path, 32 * 1024 * 1024)?;
        let record: NestedVaultRecord =
            serde_json::from_slice(&bytes).map_err(|_| VaultError::file_invalid())?;
        if record.version != 1 || record.id != nested_id || record.parent_id != vault.id {
            return Err(VaultError::file_invalid());
        }
        open(&vault.master_key, nested_id, &record.parent_wrapped)?;
        let prefix = format!(".{nested_id}.");
        let mut cleaned = 0;
        for entry in fs::read_dir(dir).map_err(|_| VaultError::storage())? {
            let entry = entry.map_err(|_| VaultError::storage())?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(suffix) = name
                .strip_prefix(&prefix)
                .and_then(|v| v.strip_suffix(".nested-tmp"))
            else {
                continue;
            };
            if !crate::file_crypto::valid_id(suffix) {
                continue;
            }
            let metadata = fs::symlink_metadata(entry.path()).map_err(|_| VaultError::storage())?;
            if !metadata.file_type().is_file() {
                continue;
            }
            fs::remove_file(entry.path()).map_err(|_| VaultError::storage())?;
            cleaned += 1;
        }
        if cleaned > 0 {
            File::open(dir)
                .map_err(|_| VaultError::storage())?
                .sync_all()
                .map_err(|_| VaultError::storage())?;
        }
        Ok(cleaned)
    }

    // Only an authenticated, byte-identical envelope can be reconciled. A mismatch
    // is never overwritten or deleted: it requires explicit human intervention.
    fn reconcile_nested_duplicates(
        &self,
        vault: &OpenVault,
        dir: &std::path::Path,
        payload: &NestedVaultPayload,
    ) -> Result<usize, VaultError> {
        let mut duplicates = Vec::new();
        for (id, encoded) in &payload.files {
            if !valid_id(id) {
                return Err(VaultError::file_invalid());
            }
            let expected = STANDARD
                .decode(encoded)
                .map_err(|_| VaultError::file_invalid())?;
            crate::file_crypto::open(&vault.id, id, &vault.master_key, &expected)?;
            let parent = dir.join(format!("{id}.bvfile"));
            match fs::symlink_metadata(&parent) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => return Err(VaultError::storage()),
                Ok(metadata) if !metadata.file_type().is_file() => {
                    return Err(VaultError::new(
                        "recovery_conflict",
                        "A conflicting parent file requires manual review.",
                    ));
                }
                Ok(_) => {}
            }
            let existing = crate::file_crypto::read_regular(&parent, 8 * 1024 * 1024)?;
            crate::file_crypto::open(&vault.id, id, &vault.master_key, &existing)?;
            if existing.as_slice() != expected.as_slice() {
                return Err(VaultError::new(
                    "recovery_conflict",
                    "Two authenticated but different copies share an ID. Neither was deleted.",
                ));
            }
            duplicates.push(parent);
        }
        // Validate the entire set before deleting anything. The nested record is
        // already durable and authenticated, so it is the canonical recovery copy.
        for parent in &duplicates {
            fs::remove_file(parent).map_err(|_| VaultError::storage())?;
        }
        if !duplicates.is_empty() {
            File::open(dir)
                .map_err(|_| VaultError::storage())?
                .sync_all()
                .map_err(|_| VaultError::storage())?;
        }
        Ok(duplicates.len())
    }

    pub fn unlock_nested_vault(
        &self,
        id: &str,
        password: String,
    ) -> Result<Vec<crate::file_crypto::VaultFile>, VaultError> {
        if !valid_id(id) {
            return Err(VaultError::file_invalid());
        }
        let password = Zeroizing::new(password);
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        self.check_nested_attempt(id)?;
        let dir = self.root.join(format!("{}.files", vault.id));
        let bytes =
            crate::file_crypto::read_regular(&dir.join(format!("{id}.bvlock")), 32 * 1024 * 1024)?;
        let record: NestedVaultRecord =
            serde_json::from_slice(&bytes).map_err(|_| VaultError::file_invalid())?;
        if record.version != 1 || record.id != id || record.parent_id != vault.id {
            return Err(VaultError::file_invalid());
        }
        let inner = open(&vault.master_key, id, &record.parent_wrapped)?;
        let wrapped: WrappedSecret =
            serde_json::from_slice(&inner).map_err(|_| VaultError::file_invalid())?;
        let kek = derive(&password, &record.kdf)?;
        let payload_bytes = match open(&kek, id, &wrapped) {
            Ok(bytes) => {
                self.record_nested_attempt(id, true)?;
                bytes
            }
            Err(error) => {
                self.record_nested_attempt(id, false)?;
                return Err(error);
            }
        };
        let payload: NestedVaultPayload =
            serde_json::from_slice(&payload_bytes).map_err(|_| VaultError::file_invalid())?;
        self.reconcile_nested_duplicates(vault, &dir, &payload)?;
        self.cleanup_authenticated_staging(vault, &dir, id)?;
        let mut result = Vec::new();
        for (file_id, bytes) in payload.files {
            let sealed = STANDARD
                .decode(bytes)
                .map_err(|_| VaultError::file_invalid())?;
            let (info, _) =
                crate::file_crypto::open(&vault.id, &file_id, &vault.master_key, &sealed)?;
            result.push(info);
        }
        Ok(result)
    }

    pub fn create_nested_text_file(
        &self,
        id: &str,
        password: String,
        name: String,
        content: String,
    ) -> Result<Vec<crate::file_crypto::VaultFile>, VaultError> {
        if !valid_id(id) {
            return Err(VaultError::file_invalid());
        }
        if password.len() != 4 || !password.bytes().all(|digit| digit.is_ascii_digit()) {
            return Err(VaultError::new(
                "invalid_nested_pin",
                "Enter exactly four digits (0–9).",
            ));
        }
        let extension = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if !["txt", "md", "json", "csv", "log"].contains(&extension.as_str()) {
            return Err(VaultError::file_name());
        }
        if content.len() as u64 > crate::file_crypto::MAX_FILE_BYTES {
            return Err(VaultError::file_large());
        }
        let password = Zeroizing::new(password);
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        self.check_nested_attempt(id)?;
        let dir = self.root.join(format!("{}.files", vault.id));
        let path = dir.join(format!("{id}.bvlock"));
        let bytes = crate::file_crypto::read_regular(&path, 32 * 1024 * 1024)?;
        let record: NestedVaultRecord =
            serde_json::from_slice(&bytes).map_err(|_| VaultError::file_invalid())?;
        if record.version != 1 || record.id != id || record.parent_id != vault.id {
            return Err(VaultError::file_invalid());
        }
        let inner = open(&vault.master_key, id, &record.parent_wrapped)?;
        let wrapped: WrappedSecret =
            serde_json::from_slice(&inner).map_err(|_| VaultError::file_invalid())?;
        let kek = derive(&password, &record.kdf)?;
        let payload_bytes = match open(&kek, id, &wrapped) {
            Ok(bytes) => {
                self.record_nested_attempt(id, true)?;
                bytes
            }
            Err(error) => {
                self.record_nested_attempt(id, false)?;
                return Err(error);
            }
        };
        let mut payload: NestedVaultPayload =
            serde_json::from_slice(&payload_bytes).map_err(|_| VaultError::file_invalid())?;
        if payload.files.len() >= 5 {
            return Err(VaultError::new(
                "nested_vault_limit",
                "A nested vault supports up to five files.",
            ));
        }
        for (file_id, encoded) in &payload.files {
            let sealed = STANDARD
                .decode(encoded)
                .map_err(|_| VaultError::file_invalid())?;
            let (info, _) =
                crate::file_crypto::open(&vault.id, file_id, &vault.master_key, &sealed)?;
            if info.name.eq_ignore_ascii_case(&name) {
                return Err(VaultError::new(
                    "duplicate_filename",
                    "A file with this name already exists in the nested vault.",
                ));
            }
        }
        let file_id = crate::file_crypto::random_id();
        let sealed = crate::file_crypto::seal(
            &vault.id,
            &file_id,
            &vault.master_key,
            &name,
            content.as_bytes(),
        )?;
        payload.files.push((file_id, STANDARD.encode(sealed)));
        let encoded_payload =
            Zeroizing::new(serde_json::to_vec(&payload).map_err(|_| VaultError::storage())?);
        if encoded_payload.len() > 20 * 1024 * 1024 {
            return Err(VaultError::new(
                "nested_vault_limit",
                "The nested vault has reached its storage limit.",
            ));
        }
        let new_inner = Zeroizing::new(
            serde_json::to_vec(&seal(&kek, id, &encoded_payload)?)
                .map_err(|_| VaultError::storage())?,
        );
        let updated = NestedVaultRecord {
            version: record.version,
            parent_id: record.parent_id,
            id: record.id,
            kdf: record.kdf,
            parent_wrapped: seal(&vault.master_key, id, &new_inner)?,
        };
        let encoded = serde_json::to_vec(&updated).map_err(|_| VaultError::storage())?;
        if encoded.len() > 32 * 1024 * 1024 {
            return Err(VaultError::storage());
        }
        // Atomic replacement: the old nested record remains intact until the new
        // ciphertext is completely written and synchronised.
        let temp = dir.join(format!(
            ".{id}.{}.nested-tmp",
            crate::file_crypto::random_id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let write_result = (|| -> std::io::Result<()> {
            let mut output = options.open(&temp)?;
            output.write_all(&encoded)?;
            output.sync_all()?;
            fs::rename(&temp, &path)?;
            File::open(&dir)?.sync_all()?;
            Ok(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(temp);
            return Err(VaultError::storage());
        }
        let mut result = Vec::new();
        for (file_id, encoded) in payload.files {
            let sealed = STANDARD
                .decode(encoded)
                .map_err(|_| VaultError::file_invalid())?;
            let (info, _) =
                crate::file_crypto::open(&vault.id, &file_id, &vault.master_key, &sealed)?;
            result.push(info);
        }
        Ok(result)
    }

    pub fn transfer_parent_file_to_nested(
        &self,
        id: &str,
        password: String,
        file_id: &str,
    ) -> Result<Vec<crate::file_crypto::VaultFile>, VaultError> {
        if !valid_id(id) || !crate::file_crypto::valid_id(file_id) {
            return Err(VaultError::file_invalid());
        }
        if password.len() != 4 || !password.bytes().all(|digit| digit.is_ascii_digit()) {
            return Err(VaultError::new(
                "invalid_nested_pin",
                "Enter exactly four digits (0–9).",
            ));
        }
        let password = Zeroizing::new(password);
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        self.check_nested_attempt(id)?;
        let dir = self.active_file_dir(vault)?;
        let path = dir.join(format!("{id}.bvlock"));
        let bytes = crate::file_crypto::read_regular(&path, 32 * 1024 * 1024)?;
        let record: NestedVaultRecord =
            serde_json::from_slice(&bytes).map_err(|_| VaultError::file_invalid())?;
        if record.version != 1 || record.id != id || record.parent_id != vault.id {
            return Err(VaultError::file_invalid());
        }
        let inner = open(&vault.master_key, id, &record.parent_wrapped)?;
        let wrapped: WrappedSecret =
            serde_json::from_slice(&inner).map_err(|_| VaultError::file_invalid())?;
        let kek = derive(&password, &record.kdf)?;
        let payload_bytes = match open(&kek, id, &wrapped) {
            Ok(bytes) => {
                self.record_nested_attempt(id, true)?;
                bytes
            }
            Err(error) => {
                self.record_nested_attempt(id, false)?;
                return Err(error);
            }
        };
        let mut payload: NestedVaultPayload =
            serde_json::from_slice(&payload_bytes).map_err(|_| VaultError::file_invalid())?;
        self.reconcile_nested_duplicates(vault, &dir, &payload)?;
        // Authenticate every existing envelope before changing the nested record.
        if payload.files.len() >= 5 {
            return Err(VaultError::new(
                "nested_vault_limit",
                "A nested vault supports up to five files.",
            ));
        }
        let mut inventory = Vec::new();
        let mut total_bytes = 0usize;
        for (candidate, encoded) in &payload.files {
            if !crate::file_crypto::valid_id(candidate) || candidate == file_id {
                return Err(VaultError::new(
                    "duplicate_file",
                    "This file is already in the nested vault.",
                ));
            }
            let sealed = STANDARD
                .decode(encoded)
                .map_err(|_| VaultError::file_invalid())?;
            let (info, _) =
                crate::file_crypto::open(&vault.id, candidate, &vault.master_key, &sealed)?;
            total_bytes += sealed.len();
            inventory.push(info);
        }
        let source = dir.join(format!("{file_id}.bvfile"));
        let sealed = crate::file_crypto::read_regular(&source, 8 * 1024 * 1024)?;
        let (info, _) = crate::file_crypto::open(&vault.id, file_id, &vault.master_key, &sealed)?;
        if inventory
            .iter()
            .any(|existing| existing.name.eq_ignore_ascii_case(&info.name))
        {
            return Err(VaultError::new(
                "duplicate_filename",
                "A file with this name already exists in the nested vault.",
            ));
        }
        total_bytes += sealed.len();
        if total_bytes > 10 * 1024 * 1024 {
            return Err(VaultError::new(
                "nested_vault_limit",
                "Encrypted files exceed the 10 MiB nested vault limit.",
            ));
        }
        payload
            .files
            .push((file_id.to_owned(), STANDARD.encode(&sealed)));
        let encoded_payload =
            Zeroizing::new(serde_json::to_vec(&payload).map_err(|_| VaultError::storage())?);
        if encoded_payload.len() > 20 * 1024 * 1024 {
            return Err(VaultError::new(
                "nested_vault_limit",
                "The nested vault has reached its storage limit.",
            ));
        }
        let new_inner = Zeroizing::new(
            serde_json::to_vec(&seal(&kek, id, &encoded_payload)?)
                .map_err(|_| VaultError::storage())?,
        );
        let updated = NestedVaultRecord {
            version: record.version,
            parent_id: record.parent_id,
            id: record.id,
            kdf: record.kdf,
            parent_wrapped: seal(&vault.master_key, id, &new_inner)?,
        };
        let encoded = serde_json::to_vec(&updated).map_err(|_| VaultError::storage())?;
        if encoded.len() > 32 * 1024 * 1024 {
            return Err(VaultError::storage());
        }
        // Commit the nested encrypted copy first. An interruption may leave duplicate
        // ciphertext, never an unencrypted file or a deleted sole copy.
        let temp = dir.join(format!(
            ".{id}.{}.nested-tmp",
            crate::file_crypto::random_id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let write_result = (|| -> std::io::Result<()> {
            let mut output = options.open(&temp)?;
            output.write_all(&encoded)?;
            transfer_checkpoint("before_temp_sync")?;
            output.sync_all()?;
            transfer_checkpoint("before_rename")?;
            fs::rename(&temp, &path)?;
            transfer_checkpoint("before_directory_sync")?;
            File::open(&dir)?.sync_all()?;
            transfer_checkpoint("before_source_unlink")?;
            Ok(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temp);
            return Err(VaultError::storage());
        }
        fs::remove_file(&source).map_err(|_| VaultError::storage())?;
        transfer_checkpoint("before_source_directory_sync").map_err(|_| VaultError::storage())?;
        File::open(&dir)
            .map_err(|_| VaultError::storage())?
            .sync_all()
            .map_err(|_| VaultError::storage())?;
        inventory.push(info);
        Ok(inventory)
    }

    pub fn manage_nested_file(
        &self,
        id: &str,
        password: String,
        file_id: &str,
        action: &str,
        name: Option<String>,
        content: Option<String>,
    ) -> Result<Vec<crate::file_crypto::VaultFile>, VaultError> {
        if !valid_id(id) || !valid_id(file_id) {
            return Err(VaultError::file_invalid());
        }
        if password.len() != 4 || !password.bytes().all(|v| v.is_ascii_digit()) {
            return Err(VaultError::new(
                "invalid_nested_pin",
                "Enter exactly four digits (0–9).",
            ));
        }
        if !["edit", "delete", "transfer_to_parent"].contains(&action) {
            return Err(VaultError::file_invalid());
        }
        let password = Zeroizing::new(password);
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        self.check_nested_attempt(id)?;
        let dir = self.root.join(format!("{}.files", vault.id));
        let path = dir.join(format!("{id}.bvlock"));
        let bytes = crate::file_crypto::read_regular(&path, 32 * 1024 * 1024)?;
        let record: NestedVaultRecord =
            serde_json::from_slice(&bytes).map_err(|_| VaultError::file_invalid())?;
        if record.version != 1 || record.id != id || record.parent_id != vault.id {
            return Err(VaultError::file_invalid());
        }
        let inner = open(&vault.master_key, id, &record.parent_wrapped)?;
        let wrapped: WrappedSecret =
            serde_json::from_slice(&inner).map_err(|_| VaultError::file_invalid())?;
        let kek = derive(&password, &record.kdf)?;
        let payload_bytes = match open(&kek, id, &wrapped) {
            Ok(bytes) => {
                self.record_nested_attempt(id, true)?;
                bytes
            }
            Err(error) => {
                self.record_nested_attempt(id, false)?;
                return Err(error);
            }
        };
        let mut payload: NestedVaultPayload =
            serde_json::from_slice(&payload_bytes).map_err(|_| VaultError::file_invalid())?;
        let mut inventory = Vec::new();
        let mut target = None;
        for (index, (candidate, encoded)) in payload.files.iter().enumerate() {
            if !valid_id(candidate) {
                return Err(VaultError::file_invalid());
            }
            let sealed = STANDARD
                .decode(encoded)
                .map_err(|_| VaultError::file_invalid())?;
            let (info, _) =
                crate::file_crypto::open(&vault.id, candidate, &vault.master_key, &sealed)?;
            if candidate == file_id {
                target = Some((index, info.clone(), sealed));
            }
            inventory.push(info);
        }
        let (index, _old, sealed) = target.ok_or_else(VaultError::file_unavailable)?;
        if action == "edit" {
            let name = name.ok_or_else(VaultError::file_name)?;
            let content = content.ok_or_else(VaultError::file_invalid)?;
            let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
            if !["txt", "md", "json", "csv", "log"].contains(&ext.as_str()) {
                return Err(VaultError::file_name());
            }
            if content.len() as u64 > crate::file_crypto::MAX_FILE_BYTES {
                return Err(VaultError::file_large());
            }
            crate::import_scan::scan(&name, content.as_bytes())
                .map_err(VaultError::scan_blocked)?;
            if inventory
                .iter()
                .any(|info| info.id != file_id && info.name.eq_ignore_ascii_case(&name))
            {
                return Err(VaultError::new(
                    "duplicate_filename",
                    "A file with this name already exists.",
                ));
            }
            let updated = crate::file_crypto::seal(
                &vault.id,
                file_id,
                &vault.master_key,
                &name,
                content.as_bytes(),
            )?;
            payload.files[index].1 = STANDARD.encode(updated);
        } else {
            if action == "transfer_to_parent" {
                let destination = dir.join(format!("{file_id}.bvfile"));
                match fs::symlink_metadata(&destination) {
                    Ok(metadata) if metadata.file_type().is_file() => {
                        let existing =
                            crate::file_crypto::read_regular(&destination, 8 * 1024 * 1024)?;
                        crate::file_crypto::open(&vault.id, file_id, &vault.master_key, &existing)?;
                        if existing.as_slice() != sealed.as_slice() {
                            return Err(VaultError::new(
                                "recovery_conflict",
                                "A different authenticated parent copy exists. Neither copy was changed.",
                            ));
                        }
                        // Identical ciphertext: resume an interrupted transfer.
                    }
                    Ok(_) => {
                        return Err(VaultError::new(
                            "recovery_conflict",
                            "The parent destination is not a regular file.",
                        ));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        transfer_checkpoint("nested_to_parent_before_copy")
                            .map_err(|_| VaultError::storage())?;
                        crate::file_crypto::persist_new(&dir, file_id, &sealed)?;
                    }
                    Err(_) => return Err(VaultError::storage()),
                }
            }
            if action == "transfer_to_parent" {
                transfer_checkpoint("nested_to_parent_after_copy")
                    .map_err(|_| VaultError::storage())?;
            }
            payload.files.remove(index);
        }
        let encoded_payload =
            Zeroizing::new(serde_json::to_vec(&payload).map_err(|_| VaultError::storage())?);
        if encoded_payload.len() > 20 * 1024 * 1024 {
            return Err(VaultError::storage());
        }
        let new_inner = Zeroizing::new(
            serde_json::to_vec(&seal(&kek, id, &encoded_payload)?)
                .map_err(|_| VaultError::storage())?,
        );
        let updated = NestedVaultRecord {
            version: record.version,
            parent_id: record.parent_id,
            id: record.id,
            kdf: record.kdf,
            parent_wrapped: seal(&vault.master_key, id, &new_inner)?,
        };
        let encoded = serde_json::to_vec(&updated).map_err(|_| VaultError::storage())?;
        if encoded.len() > 32 * 1024 * 1024 {
            return Err(VaultError::storage());
        }
        let temp = dir.join(format!(
            ".{id}.{}.nested-tmp",
            crate::file_crypto::random_id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| -> std::io::Result<()> {
            let mut output = options.open(&temp)?;
            output.write_all(&encoded)?;
            output.sync_all()?;
            if action == "transfer_to_parent" {
                transfer_checkpoint("nested_to_parent_before_rename")?;
            }
            fs::rename(&temp, &path)?;
            if action == "transfer_to_parent" {
                transfer_checkpoint("nested_to_parent_after_rename")?;
            }
            File::open(&dir)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
            return Err(VaultError::storage());
        }
        let mut result = Vec::new();
        for (candidate, encoded) in payload.files {
            let sealed = STANDARD
                .decode(encoded)
                .map_err(|_| VaultError::file_invalid())?;
            result.push(
                crate::file_crypto::open(&vault.id, &candidate, &vault.master_key, &sealed)?.0,
            );
        }
        Ok(result)
    }

    pub fn read_nested_text_file(
        &self,
        id: &str,
        password: String,
        file_id: &str,
    ) -> Result<String, VaultError> {
        if !valid_id(id) || !valid_id(file_id) {
            return Err(VaultError::file_invalid());
        }
        let password = Zeroizing::new(password);
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        self.check_nested_attempt(id)?;
        let dir = self.root.join(format!("{}.files", vault.id));
        let bytes =
            crate::file_crypto::read_regular(&dir.join(format!("{id}.bvlock")), 32 * 1024 * 1024)?;
        let record: NestedVaultRecord =
            serde_json::from_slice(&bytes).map_err(|_| VaultError::file_invalid())?;
        if record.version != 1 || record.id != id || record.parent_id != vault.id {
            return Err(VaultError::file_invalid());
        }
        let inner = open(&vault.master_key, id, &record.parent_wrapped)?;
        let wrapped: WrappedSecret =
            serde_json::from_slice(&inner).map_err(|_| VaultError::file_invalid())?;
        let kek = derive(&password, &record.kdf)?;
        let payload_bytes = match open(&kek, id, &wrapped) {
            Ok(bytes) => {
                self.record_nested_attempt(id, true)?;
                bytes
            }
            Err(error) => {
                self.record_nested_attempt(id, false)?;
                return Err(error);
            }
        };
        let payload: NestedVaultPayload =
            serde_json::from_slice(&payload_bytes).map_err(|_| VaultError::file_invalid())?;
        for (candidate, encoded) in payload.files {
            if candidate != file_id {
                continue;
            }
            let sealed = STANDARD
                .decode(encoded)
                .map_err(|_| VaultError::file_invalid())?;
            let (info, plaintext) =
                crate::file_crypto::open(&vault.id, &candidate, &vault.master_key, &sealed)?;
            let ext = info
                .name
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !["txt", "md", "json", "csv", "log"].contains(&ext.as_str()) {
                return Err(VaultError::file_invalid());
            }
            return String::from_utf8(plaintext.to_vec()).map_err(|_| VaultError::file_invalid());
        }
        Err(VaultError::file_unavailable())
    }

    pub fn restore_nested_vault(
        &self,
        id: &str,
        password: String,
    ) -> Result<Vec<crate::file_crypto::VaultFile>, VaultError> {
        if !valid_id(id) {
            return Err(VaultError::file_invalid());
        }
        let password = Zeroizing::new(password);
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        self.check_nested_attempt(id)?;
        let dir = self.root.join(format!("{}.files", vault.id));
        let path = dir.join(format!("{id}.bvlock"));
        let bytes = crate::file_crypto::read_regular(&path, 32 * 1024 * 1024)?;
        let record: NestedVaultRecord =
            serde_json::from_slice(&bytes).map_err(|_| VaultError::file_invalid())?;
        if record.version != 1 || record.id != id || record.parent_id != vault.id {
            return Err(VaultError::file_invalid());
        }
        let inner = open(&vault.master_key, id, &record.parent_wrapped)?;
        let wrapped: WrappedSecret =
            serde_json::from_slice(&inner).map_err(|_| VaultError::file_invalid())?;
        let kek = derive(&password, &record.kdf)?;
        let payload_bytes = match open(&kek, id, &wrapped) {
            Ok(bytes) => {
                self.record_nested_attempt(id, true)?;
                bytes
            }
            Err(error) => {
                self.record_nested_attempt(id, false)?;
                return Err(error);
            }
        };
        let payload: NestedVaultPayload =
            serde_json::from_slice(&payload_bytes).map_err(|_| VaultError::file_invalid())?;
        let mut restored = Vec::new();
        for (file_id, encoded) in &payload.files {
            if !valid_id(file_id) {
                return Err(VaultError::file_invalid());
            }
            let sealed = STANDARD
                .decode(encoded)
                .map_err(|_| VaultError::file_invalid())?;
            let (info, _) =
                crate::file_crypto::open(&vault.id, file_id, &vault.master_key, &sealed)?;
            let destination = dir.join(format!("{file_id}.bvfile"));
            match fs::symlink_metadata(&destination) {
                Ok(metadata) if metadata.file_type().is_file() => {
                    let existing = crate::file_crypto::read_regular(&destination, 8 * 1024 * 1024)?;
                    crate::file_crypto::open(&vault.id, file_id, &vault.master_key, &existing)?;
                    if existing.as_slice() != sealed.as_slice() {
                        return Err(VaultError::new(
                            "recovery_conflict",
                            "A different authenticated parent copy exists. No files were restored.",
                        ));
                    }
                }
                Ok(_) => {
                    return Err(VaultError::new(
                        "recovery_conflict",
                        "The parent destination is not a regular file.",
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(VaultError::storage()),
            }
            restored.push((info, sealed));
        }
        // Persist all encrypted envelopes before removing the child container.
        // An interruption can leave duplicate ciphertext but not lose the only copy.
        for (info, sealed) in &restored {
            if fs::symlink_metadata(dir.join(format!("{}.bvfile", info.id))).is_err() {
                transfer_checkpoint("restore_before_copy").map_err(|_| VaultError::storage())?;
                crate::file_crypto::persist_new(&dir, &info.id, sealed)?;
                transfer_checkpoint("restore_after_copy").map_err(|_| VaultError::storage())?;
            }
        }
        transfer_checkpoint("restore_before_unlink").map_err(|_| VaultError::storage())?;
        fs::remove_file(path).map_err(|_| VaultError::storage())?;
        transfer_checkpoint("restore_after_unlink").map_err(|_| VaultError::storage())?;
        File::open(&dir)
            .map_err(|_| VaultError::storage())?
            .sync_all()
            .map_err(|_| VaultError::storage())?;
        Ok(restored.into_iter().map(|(info, _)| info).collect())
    }

    // Verify and decrypt into zeroising Rust memory only. No plaintext file is written.
    pub fn decrypt_in_vault(
        &self,
        file_id: &str,
    ) -> Result<crate::file_crypto::VaultFile, VaultError> {
        if !crate::file_crypto::valid_id(file_id) {
            return Err(VaultError::file_invalid());
        }
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        let dir = self.root.join(format!("{}.files", vault.id));
        let sealed = crate::file_crypto::read_regular(
            &dir.join(format!("{file_id}.bvfile")),
            8 * 1024 * 1024,
        )?;
        let (info, _plaintext) =
            crate::file_crypto::open(&vault.id, file_id, &vault.master_key, &sealed)?;
        // _plaintext is dropped and zeroised before returning to the UI.
        Ok(info)
    }

    pub fn export_path(
        &self,
        file_id: &str,
        destination: &std::path::Path,
    ) -> Result<crate::file_crypto::VaultFile, VaultError> {
        let guard = self.lock_active()?;
        let vault = guard.as_ref().ok_or_else(VaultError::locked)?;
        let parent = destination
            .parent()
            .ok_or_else(VaultError::file_unavailable)?;
        let canonical_parent =
            fs::canonicalize(parent).map_err(|_| VaultError::file_unavailable())?;
        let canonical_root = fs::canonicalize(&self.root).map_err(|_| VaultError::storage())?;
        if canonical_parent.starts_with(&canonical_root) {
            return Err(VaultError::file_unavailable());
        }
        if !crate::file_crypto::valid_id(file_id) {
            return Err(VaultError::file_invalid());
        }
        let dir = self.root.join(format!("{}.files", vault.id));
        let meta = fs::symlink_metadata(&dir).map_err(|_| VaultError::storage())?;
        if !meta.file_type().is_dir() {
            return Err(VaultError::storage());
        }
        let sealed = crate::file_crypto::read_regular(
            &dir.join(format!("{file_id}.bvfile")),
            8 * 1024 * 1024,
        )?;
        let (info, plaintext) =
            crate::file_crypto::open(&vault.id, file_id, &vault.master_key, &sealed)?;
        crate::file_crypto::export_new(destination, &plaintext)?;
        Ok(info)
    }

    fn persist_new(&self, id: &str, bytes: &[u8]) -> Result<(), VaultError> {
        if bytes.len() as u64 > RECORD_MAX_BYTES {
            return Err(VaultError::storage());
        }
        let destination = self.file_path(id)?;
        if destination.exists() {
            return Err(VaultError::storage());
        }
        let temp = self.root.join(format!(".{id}.tmp"));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| -> std::io::Result<()> {
            let mut file = options.open(&temp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            fs::rename(&temp, &destination)?;
            File::open(&self.root)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
            return Err(VaultError::storage());
        }
        Ok(())
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn derive(
    password: &str,
    record: &KdfRecord,
) -> Result<Zeroizing<[u8; MASTER_KEY_LEN]>, VaultError> {
    if record.algorithm != "argon2id-v19"
        || record.m_cost != KDF_MEMORY_KIB
        || record.t_cost != KDF_ITERATIONS
        || record.p_cost != KDF_PARALLELISM
        || record.output_len != MASTER_KEY_LEN
    {
        return Err(VaultError::new(
            "unsupported_kdf",
            "This vault's key derivation parameters are unsupported.",
        ));
    }
    let salt = STANDARD
        .decode(&record.salt)
        .map_err(|_| VaultError::invalid())?;
    if salt.len() != SALT_LEN {
        return Err(VaultError::invalid());
    }
    let params = Params::new(
        KDF_MEMORY_KIB,
        KDF_ITERATIONS,
        KDF_PARALLELISM,
        Some(MASTER_KEY_LEN),
    )
    .map_err(|_| VaultError::invalid())?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut result = Zeroizing::new([0u8; MASTER_KEY_LEN]);
    argon2
        .hash_password_into(password.as_bytes(), &salt, result.as_mut())
        .map_err(|_| VaultError::invalid())?;
    Ok(result)
}
fn aad(id: &str) -> String {
    format!("black-vault:desktop:v1:vault-header:{id}")
}
fn seal(key: &[u8; MASTER_KEY_LEN], id: &str, data: &[u8]) -> Result<WrappedSecret, VaultError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| VaultError::storage())?;
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let context = aad(id);
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: data,
                aad: context.as_bytes(),
            },
        )
        .map_err(|_| VaultError::storage())?;
    Ok(WrappedSecret {
        nonce: STANDARD.encode(nonce),
        ciphertext: STANDARD.encode(ciphertext),
    })
}
fn open(
    key: &[u8; MASTER_KEY_LEN],
    id: &str,
    wrapped: &WrappedSecret,
) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    let nonce = STANDARD
        .decode(&wrapped.nonce)
        .map_err(|_| VaultError::invalid())?;
    if nonce.len() != NONCE_LEN {
        return Err(VaultError::invalid());
    }
    let ciphertext = STANDARD
        .decode(&wrapped.ciphertext)
        .map_err(|_| VaultError::invalid())?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| VaultError::invalid())?;
    let context = aad(id);
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: context.as_bytes(),
            },
        )
        .map_err(|_| VaultError::invalid())?;
    Ok(Zeroizing::new(plaintext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    fn store(dir: &TempDir) -> VaultStore {
        VaultStore::new(dir.path().join("vaults"))
    }
    const PASSWORD: &str = "correct horse battery staple";

    #[test]
    fn nested_file_roundtrip_keeps_contents_sealed() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        store.create("Parent".into(), PASSWORD.into()).unwrap();
        let file = store.import_example("notes").unwrap();
        for invalid in ["", "123", "12345", "12a4", "１２３４"] {
            assert_eq!(
                store
                    .lock_selected_files("Private".into(), invalid.into(), vec![file.id.clone()])
                    .unwrap_err()
                    .code,
                "invalid_nested_pin"
            );
        }
        let nested = store
            .lock_selected_files("Private".into(), "4827".into(), vec![file.id.clone()])
            .unwrap();
        assert!(store.list_files().unwrap().is_empty());
        assert_eq!(store.list_nested_vaults().unwrap().len(), 1);
        for _ in 0..5 {
            assert_eq!(
                store
                    .unlock_nested_vault(&nested.id, "0000".into())
                    .unwrap_err()
                    .code,
                "invalid_credentials"
            );
        }
        assert_eq!(
            store
                .unlock_nested_vault(&nested.id, "4827".into())
                .unwrap_err()
                .code,
            "nested_pin_cooldown"
        );
        store.nested_attempts.lock().unwrap().clear();
        assert_eq!(
            store
                .unlock_nested_vault(&nested.id, "4827".into())
                .unwrap(),
            vec![file.clone()]
        );
        assert_eq!(
            store
                .restore_nested_vault(&nested.id, "4827".into())
                .unwrap(),
            vec![file.clone()]
        );
        assert_eq!(store.list_files().unwrap(), vec![file]);
        assert!(store.list_nested_vaults().unwrap().is_empty());
        store.lock().unwrap();
        assert_eq!(
            store
                .unlock_nested_vault(&nested.id, String::new())
                .unwrap_err()
                .code,
            "vault_locked"
        );
    }

    #[test]
    fn native_lifecycle_survives_store_restart_and_bidirectional_transfer() {
        let dir = TempDir::new().unwrap();
        let initial = store(&dir);
        let parent = initial
            .create("Acceptance".into(), PASSWORD.into())
            .unwrap();
        let first = initial
            .create_text_file("first.txt", "Survives restart")
            .unwrap();
        let second = initial
            .create_text_file("second.md", "Second file")
            .unwrap();
        let nested = initial
            .lock_selected_files("Protected".into(), "0427".into(), vec![first.id.clone()])
            .unwrap();
        initial.lock().unwrap();
        drop(initial);
        let restarted = store(&dir);
        assert_eq!(
            restarted
                .unlock(parent.id.clone(), "wrong password".into())
                .unwrap_err()
                .code,
            "invalid_credentials"
        );
        restarted
            .unlock(parent.id.clone(), PASSWORD.into())
            .unwrap();
        assert_eq!(
            restarted
                .unlock_nested_vault(&nested.id, "9999".into())
                .unwrap_err()
                .code,
            "invalid_credentials"
        );
        assert_eq!(
            restarted
                .unlock_nested_vault(&nested.id, "0427".into())
                .unwrap(),
            vec![first.clone()]
        );
        restarted
            .transfer_parent_file_to_nested(&nested.id, "0427".into(), &second.id)
            .unwrap();
        assert!(restarted.list_files().unwrap().is_empty());
        restarted.lock().unwrap();
        drop(restarted);
        let final_session = store(&dir);
        final_session.unlock(parent.id, PASSWORD.into()).unwrap();
        assert_eq!(
            final_session
                .read_nested_text_file(&nested.id, "0427".into(), &second.id)
                .unwrap(),
            "Second file"
        );
        final_session
            .manage_nested_file(
                &nested.id,
                "0427".into(),
                &first.id,
                "transfer_to_parent",
                None,
                None,
            )
            .unwrap();
        assert_eq!(
            final_session.read_text_file(&first.id).unwrap(),
            "Survives restart"
        );
        assert_eq!(
            final_session
                .unlock_nested_vault(&nested.id, "0427".into())
                .unwrap(),
            vec![second]
        );
    }

    #[test]
    fn recovery_removes_only_known_regular_staging_files() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let parent = store.create("Parent".into(), PASSWORD.into()).unwrap();
        let first = store.create_text_file("one.txt", "First").unwrap();
        let nested = store
            .lock_selected_files("Nested".into(), "0427".into(), vec![first.id.clone()])
            .unwrap();
        let root = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", parent.id));
        let stale = root.join(format!(
            ".{}.{}.nested-tmp",
            nested.id,
            crate::file_crypto::random_id()
        ));
        let unknown = root.join(".unknown.nested-tmp");
        fs::write(&stale, b"abandoned encrypted staging").unwrap();
        fs::write(&unknown, b"unrelated").unwrap();
        #[cfg(unix)]
        let link = {
            let path = root.join(format!(
                ".{}.{}.nested-tmp",
                nested.id,
                crate::file_crypto::random_id()
            ));
            std::os::unix::fs::symlink(&unknown, &path).unwrap();
            path
        };
        store
            .unlock_nested_vault(&nested.id, "0427".into())
            .unwrap();
        assert!(!stale.exists());
        assert!(unknown.exists());
        #[cfg(unix)]
        assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    }

    #[test]
    fn crash_recovery_reconciles_identical_parent_duplicate_and_preserves_conflicts() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let parent = store.create("Parent".into(), PASSWORD.into()).unwrap();
        let first = store.create_text_file("one.txt", "First").unwrap();
        let second = store.create_text_file("two.txt", "Second").unwrap();
        let root = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", parent.id));
        let duplicate = fs::read(root.join(format!("{}.bvfile", first.id))).unwrap();
        let nested = store
            .lock_selected_files("Nested".into(), "0427".into(), vec![first.id.clone()])
            .unwrap();
        // Simulate crash after nested fsync but before source unlink.
        fs::write(root.join(format!("{}.bvfile", first.id)), &duplicate).unwrap();
        assert_eq!(
            store
                .unlock_nested_vault(&nested.id, "0427".into())
                .unwrap(),
            vec![first.clone()]
        );
        assert!(!root.join(format!("{}.bvfile", first.id)).exists());
        // Simulate conflicting authenticated parent envelope sharing the ID.
        let other = fs::read(root.join(format!("{}.bvfile", second.id))).unwrap();
        let (_, plaintext) = crate::file_crypto::open(
            &parent.id,
            &second.id,
            &store.active.lock().unwrap().as_ref().unwrap().master_key,
            &other,
        )
        .unwrap();
        let conflict = crate::file_crypto::seal(
            &parent.id,
            &first.id,
            &store.active.lock().unwrap().as_ref().unwrap().master_key,
            "one.txt",
            &plaintext,
        )
        .unwrap();
        fs::write(root.join(format!("{}.bvfile", first.id)), &conflict).unwrap();
        assert_eq!(
            store
                .unlock_nested_vault(&nested.id, "0427".into())
                .unwrap_err()
                .code,
            "recovery_conflict"
        );
        assert!(root.join(format!("{}.bvfile", first.id)).exists());
        assert_eq!(store.read_text_file(&first.id).unwrap(), "Second");
    }

    #[test]
    fn interrupted_nested_to_parent_transfer_can_resume() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let parent = store.create("Parent".into(), PASSWORD.into()).unwrap();
        let first = store.create_text_file("one.txt", "First").unwrap();
        let root = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", parent.id));
        let duplicate = fs::read(root.join(format!("{}.bvfile", first.id))).unwrap();
        let nested = store
            .lock_selected_files("Nested".into(), "0427".into(), vec![first.id.clone()])
            .unwrap();
        fs::write(root.join(format!("{}.bvfile", first.id)), &duplicate).unwrap();
        let remaining = store
            .manage_nested_file(
                &nested.id,
                "0427".into(),
                &first.id,
                "transfer_to_parent",
                None,
                None,
            )
            .unwrap();
        assert!(remaining.is_empty());
        assert_eq!(store.read_text_file(&first.id).unwrap(), "First");
        assert!(
            store
                .unlock_nested_vault(&nested.id, "0427".into())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn parent_to_nested_fault_matrix_recovers_without_plaintext_or_lost_files() {
        // Each case uses a separate disposable store and recreates the store
        // after the injected interruption, rather than relying on cached state.
        for stage in [
            "before_temp_sync",
            "before_rename",
            "before_directory_sync",
            "before_source_unlink",
            "before_source_directory_sync",
        ] {
            let dir = TempDir::new().unwrap();
            let initial = store(&dir);
            let parent = initial.create("Parent".into(), PASSWORD.into()).unwrap();
            let protected = initial
                .create_text_file("protected.txt", "Existing")
                .unwrap();
            let incoming = initial
                .create_text_file("incoming.txt", "Keep this safe")
                .unwrap();
            let nested = initial
                .lock_selected_files("Nested".into(), "0427".into(), vec![protected.id.clone()])
                .unwrap();
            let root = dir
                .path()
                .join("vaults")
                .join(format!("{}.files", parent.id));
            let source = root.join(format!("{}.bvfile", incoming.id));
            let original = fs::read(&source).unwrap();

            TRANSFER_FAILURE.with(|slot| slot.set(Some(stage)));
            let result =
                initial.transfer_parent_file_to_nested(&nested.id, "0427".into(), &incoming.id);
            TRANSFER_FAILURE.with(|slot| slot.set(None));
            assert_eq!(result.unwrap_err().code, "storage_unavailable", "{stage}");
            initial.lock().unwrap();
            drop(initial);

            let restarted = store(&dir);
            restarted
                .unlock(parent.id.clone(), PASSWORD.into())
                .unwrap();
            let nested_files = restarted
                .unlock_nested_vault(&nested.id, "0427".into())
                .unwrap();
            let is_committed = matches!(
                stage,
                "before_directory_sync" | "before_source_unlink" | "before_source_directory_sync"
            );
            assert_eq!(
                nested_files.iter().any(|file| file.id == incoming.id),
                is_committed,
                "{stage}"
            );
            if is_committed {
                assert!(!source.exists(), "{stage}: duplicate should be reconciled");
                assert_eq!(
                    restarted
                        .read_nested_text_file(&nested.id, "0427".into(), &incoming.id)
                        .unwrap(),
                    "Keep this safe",
                    "{stage}"
                );
            } else {
                assert_eq!(fs::read(&source).unwrap(), original, "{stage}");
                assert_eq!(
                    restarted.read_text_file(&incoming.id).unwrap(),
                    "Keep this safe",
                    "{stage}"
                );
            }
            assert_eq!(
                restarted
                    .read_nested_text_file(&nested.id, "0427".into(), &protected.id)
                    .unwrap(),
                "Existing",
                "{stage}"
            );
        }
    }

    #[test]
    fn nested_to_parent_fault_matrix_can_resume_after_restart() {
        for stage in [
            "nested_to_parent_before_copy",
            "nested_to_parent_after_copy",
            "nested_to_parent_before_rename",
            "nested_to_parent_after_rename",
        ] {
            let dir = TempDir::new().unwrap();
            let initial = store(&dir);
            let parent = initial.create("Parent".into(), PASSWORD.into()).unwrap();
            let file = initial
                .create_text_file("protected.txt", "Retain contents")
                .unwrap();
            let nested = initial
                .lock_selected_files("Nested".into(), "0427".into(), vec![file.id.clone()])
                .unwrap();
            TRANSFER_FAILURE.with(|slot| slot.set(Some(stage)));
            let result = initial.manage_nested_file(
                &nested.id,
                "0427".into(),
                &file.id,
                "transfer_to_parent",
                None,
                None,
            );
            TRANSFER_FAILURE.with(|slot| slot.set(None));
            assert_eq!(result.unwrap_err().code, "storage_unavailable", "{stage}");
            initial.lock().unwrap();
            drop(initial);
            let restarted = store(&dir);
            restarted.unlock(parent.id, PASSWORD.into()).unwrap();
            // Copy-first means the protected copy is still available after all
            // pre-commit interruptions, including an already copied parent.
            let inventory = restarted
                .unlock_nested_vault(&nested.id, "0427".into())
                .unwrap();
            let committed = stage == "nested_to_parent_after_rename";
            assert_eq!(
                inventory.iter().any(|item| item.id == file.id),
                !committed,
                "{stage}"
            );
            if !committed {
                let remaining = restarted
                    .manage_nested_file(
                        &nested.id,
                        "0427".into(),
                        &file.id,
                        "transfer_to_parent",
                        None,
                        None,
                    )
                    .unwrap();
                assert!(remaining.is_empty(), "{stage}");
            }
            assert_eq!(
                restarted.read_text_file(&file.id).unwrap(),
                "Retain contents",
                "{stage}"
            );
        }
    }

    #[test]
    fn restore_fault_matrix_resumes_identical_copies_after_restart() {
        for stage in [
            "restore_before_copy",
            "restore_after_copy",
            "restore_before_unlink",
            "restore_after_unlink",
        ] {
            let dir = TempDir::new().unwrap();
            let initial = store(&dir);
            let parent = initial.create("Parent".into(), PASSWORD.into()).unwrap();
            let first = initial
                .create_text_file("one.txt", "First content")
                .unwrap();
            let second = initial
                .create_text_file("two.txt", "Second content")
                .unwrap();
            let nested = initial
                .lock_selected_files(
                    "Nested".into(),
                    "0427".into(),
                    vec![first.id.clone(), second.id.clone()],
                )
                .unwrap();
            TRANSFER_FAILURE.with(|slot| slot.set(Some(stage)));
            let result = initial.restore_nested_vault(&nested.id, "0427".into());
            TRANSFER_FAILURE.with(|slot| slot.set(None));
            assert_eq!(result.unwrap_err().code, "storage_unavailable", "{stage}");
            initial.lock().unwrap();
            drop(initial);
            let restarted = store(&dir);
            restarted.unlock(parent.id, PASSWORD.into()).unwrap();
            if stage != "restore_after_unlink" {
                let restored = restarted
                    .restore_nested_vault(&nested.id, "0427".into())
                    .unwrap();
                assert_eq!(restored.len(), 2, "{stage}");
            }
            assert_eq!(
                restarted.read_text_file(&first.id).unwrap(),
                "First content",
                "{stage}"
            );
            assert_eq!(
                restarted.read_text_file(&second.id).unwrap(),
                "Second content",
                "{stage}"
            );
            assert!(
                restarted.list_nested_vaults().unwrap().is_empty(),
                "{stage}"
            );
        }
    }

    #[test]
    fn interruption_child_worker() {
        let Ok(root) = std::env::var("BV_TEST_EXIT_ROOT") else {
            return;
        };
        let stage = std::env::var("BV_TEST_EXIT_STAGE").unwrap();
        let worker = VaultStore::new(PathBuf::from(&root).join("vaults"));
        let parent = worker.create("Temporary".into(), PASSWORD.into()).unwrap();
        let first = worker
            .create_text_file("first.txt", "First survives")
            .unwrap();
        let second = worker
            .create_text_file("second.txt", "Second survives")
            .unwrap();
        let nested = worker
            .lock_selected_files("Nested".into(), "0427".into(), vec![first.id.clone()])
            .unwrap();
        let manifest = serde_json::json!({
            "parent": parent.id, "nested": nested.id,
            "first": first.id, "second": second.id
        });
        let path = PathBuf::from(&root).join("manifest.json");
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        File::open(&path).unwrap().sync_all().unwrap();
        File::open(&root).unwrap().sync_all().unwrap();
        if stage.starts_with("nested_to_parent_") {
            let _ = worker.manage_nested_file(
                &nested.id,
                "0427".into(),
                &first.id,
                "transfer_to_parent",
                None,
                None,
            );
        } else if stage.starts_with("restore_") {
            let _ = worker.restore_nested_vault(&nested.id, "0427".into());
        } else {
            let _ = worker.transfer_parent_file_to_nested(&nested.id, "0427".into(), &second.id);
        }
        panic!("checkpoint was not reached: {stage}");
    }

    #[test]
    fn interrupted_parent_staging_is_preserved_for_manual_review() {
        let dir = TempDir::new().unwrap();
        let initial = store(&dir);
        let parent = initial.create("Temporary".into(), PASSWORD.into()).unwrap();
        let file = initial
            .create_text_file("safe.txt", "Canonical content")
            .unwrap();
        let root = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", parent.id));
        let stage = root.join(format!(
            ".{}.{}.tmp",
            file.id,
            crate::file_crypto::random_id()
        ));
        let canonical = root.join(format!("{}.bvfile", file.id));
        let original = fs::read(&canonical).unwrap();
        fs::write(&stage, &original[..original.len() / 2]).unwrap();
        initial.lock().unwrap();
        drop(initial);

        let reopened = store(&dir);
        reopened.unlock(parent.id.clone(), PASSWORD.into()).unwrap();
        assert_eq!(
            reopened.read_text_file(&file.id).unwrap(),
            "Canonical content"
        );
        assert_eq!(fs::read(&canonical).unwrap(), original);
        assert!(
            stage.is_file(),
            "unverified staging must not be deleted automatically"
        );
        reopened.lock().unwrap();
        assert_eq!(
            reopened
                .delete_locked(&parent.id, &format!("DELETE {}", parent.id))
                .unwrap_err()
                .code,
            "storage_unavailable",
            "unknown staging names must prevent destructive vault deletion"
        );
        assert!(stage.is_file());
        assert!(canonical.is_file());
    }

    #[test]
    fn restore_conflict_preserves_authenticated_parent_and_nested_copies() {
        let dir = TempDir::new().unwrap();
        let initial = store(&dir);
        let parent = initial.create("Temporary".into(), PASSWORD.into()).unwrap();
        let first = initial
            .create_text_file("one.txt", "Nested version")
            .unwrap();
        let nested = initial
            .lock_selected_files("Nested".into(), "0427".into(), vec![first.id.clone()])
            .unwrap();
        let root = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", parent.id));
        let conflicting = crate::file_crypto::seal(
            &parent.id,
            &first.id,
            &initial.active.lock().unwrap().as_ref().unwrap().master_key,
            "one.txt",
            b"Different authenticated version",
        )
        .unwrap();
        let parent_copy = root.join(format!("{}.bvfile", first.id));
        fs::write(&parent_copy, &conflicting).unwrap();
        let child_copy = root.join(format!("{}.bvlock", nested.id));
        let child_original = fs::read(&child_copy).unwrap();
        initial.lock().unwrap();
        drop(initial);

        let reopened = store(&dir);
        reopened.unlock(parent.id, PASSWORD.into()).unwrap();
        assert_eq!(
            reopened
                .restore_nested_vault(&nested.id, "0427".into())
                .unwrap_err()
                .code,
            "recovery_conflict"
        );
        assert_eq!(fs::read(&parent_copy).unwrap(), conflicting);
        assert_eq!(fs::read(&child_copy).unwrap(), child_original);
        assert_eq!(
            reopened.read_text_file(&first.id).unwrap(),
            "Different authenticated version"
        );
        assert_eq!(
            reopened
                .read_nested_text_file(&nested.id, "0427".into(), &first.id)
                .unwrap(),
            "Nested version"
        );
    }

    #[test]
    fn subprocess_interruption_recovery_matrix() {
        for stage in [
            "before_temp_sync",
            "before_rename",
            "before_directory_sync",
            "before_source_unlink",
            "before_source_directory_sync",
            "nested_to_parent_before_copy",
            "nested_to_parent_after_copy",
            "nested_to_parent_before_rename",
            "nested_to_parent_after_rename",
            "restore_before_copy",
            "restore_after_copy",
            "restore_before_unlink",
            "restore_after_unlink",
        ] {
            let dir = TempDir::new().unwrap();
            let marker = dir.path().join("checkpoint");
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("vault::tests::interruption_child_worker")
                .env("BV_TEST_EXIT_ROOT", dir.path())
                .env("BV_TEST_EXIT_STAGE", stage)
                .env("BV_TEST_EXIT_MARKER", &marker)
                .output()
                .unwrap();
            assert_eq!(child.status.code(), Some(86), "{stage}: {:?}", child);
            assert_eq!(fs::read_to_string(&marker).unwrap(), stage, "{stage}");
            let manifest: serde_json::Value =
                serde_json::from_slice(&fs::read(dir.path().join("manifest.json")).unwrap())
                    .unwrap();
            let parent = manifest["parent"].as_str().unwrap();
            let nested = manifest["nested"].as_str().unwrap();
            let first = manifest["first"].as_str().unwrap();
            let second = manifest["second"].as_str().unwrap();
            let restarted = store(&dir);
            restarted.unlock(parent.into(), PASSWORD.into()).unwrap();
            if stage.starts_with("nested_to_parent_") {
                let committed = stage == "nested_to_parent_after_rename";
                let inventory = restarted
                    .unlock_nested_vault(nested, "0427".into())
                    .unwrap();
                assert_eq!(
                    inventory.iter().any(|file| file.id == first),
                    !committed,
                    "{stage}"
                );
                if !committed {
                    restarted
                        .manage_nested_file(
                            nested,
                            "0427".into(),
                            first,
                            "transfer_to_parent",
                            None,
                            None,
                        )
                        .unwrap();
                }
                assert_eq!(
                    restarted.read_text_file(first).unwrap(),
                    "First survives",
                    "{stage}"
                );
                assert_eq!(
                    restarted.read_text_file(second).unwrap(),
                    "Second survives",
                    "{stage}"
                );
            } else if stage.starts_with("restore_") {
                if stage != "restore_after_unlink" {
                    restarted
                        .restore_nested_vault(nested, "0427".into())
                        .unwrap();
                }
                assert_eq!(
                    restarted.read_text_file(first).unwrap(),
                    "First survives",
                    "{stage}"
                );
                assert_eq!(
                    restarted.read_text_file(second).unwrap(),
                    "Second survives",
                    "{stage}"
                );
            } else {
                let inventory = restarted
                    .unlock_nested_vault(nested, "0427".into())
                    .unwrap();
                let committed = matches!(
                    stage,
                    "before_directory_sync"
                        | "before_source_unlink"
                        | "before_source_directory_sync"
                );
                assert_eq!(
                    inventory.iter().any(|file| file.id == second),
                    committed,
                    "{stage}"
                );
                if committed {
                    assert_eq!(
                        restarted
                            .read_nested_text_file(nested, "0427".into(), second)
                            .unwrap(),
                        "Second survives",
                        "{stage}"
                    );
                } else {
                    assert_eq!(
                        restarted.read_text_file(second).unwrap(),
                        "Second survives",
                        "{stage}"
                    );
                }
                assert_eq!(
                    restarted
                        .read_nested_text_file(nested, "0427".into(), first)
                        .unwrap(),
                    "First survives",
                    "{stage}"
                );
            }
        }
    }

    #[test]
    fn parent_to_existing_nested_transfer_preserves_authenticated_ciphertext() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        store.create("Parent".into(), PASSWORD.into()).unwrap();
        let original = store
            .create_text_file("original.md", "Already protected")
            .unwrap();
        let incoming = store
            .create_text_file("incoming.txt", "Transfer secret")
            .unwrap();
        let nested = store
            .lock_selected_files(
                "Compartment".into(),
                "0427".into(),
                vec![original.id.clone()],
            )
            .unwrap();
        assert_eq!(
            store
                .transfer_parent_file_to_nested(&nested.id, "0000".into(), &incoming.id)
                .unwrap_err()
                .code,
            "invalid_credentials"
        );
        assert_eq!(store.list_files().unwrap(), vec![incoming.clone()]);
        let moved = store
            .transfer_parent_file_to_nested(&nested.id, "0427".into(), &incoming.id)
            .unwrap();
        assert_eq!(moved.len(), 2);
        assert!(store.list_files().unwrap().is_empty());
        assert_eq!(
            store
                .read_nested_text_file(&nested.id, "0427".into(), &incoming.id)
                .unwrap(),
            "Transfer secret"
        );
        assert_eq!(
            store
                .transfer_parent_file_to_nested(&nested.id, "0427".into(), &incoming.id)
                .unwrap_err()
                .code,
            "duplicate_file"
        );
        let returned = store
            .manage_nested_file(
                &nested.id,
                "0427".into(),
                &incoming.id,
                "transfer_to_parent",
                None,
                None,
            )
            .unwrap();
        assert_eq!(returned.len(), 1);
        assert_eq!(
            store.read_text_file(&incoming.id).unwrap(),
            "Transfer secret"
        );
        assert_eq!(
            store
                .unlock_nested_vault(&nested.id, "0427".into())
                .unwrap(),
            vec![original]
        );
    }

    #[test]
    fn nested_management_edits_deletes_and_transfers_encrypted_files() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        store.create("Parent".into(), PASSWORD.into()).unwrap();
        let source = store
            .create_text_file("first.md", "Initial secret")
            .unwrap();
        let nested = store
            .lock_selected_files("Private".into(), "0427".into(), vec![source.id.clone()])
            .unwrap();
        assert_eq!(
            store
                .read_nested_text_file(&nested.id, "0427".into(), &source.id)
                .unwrap(),
            "Initial secret"
        );
        assert_eq!(
            store
                .manage_nested_file(&nested.id, "0000".into(), &source.id, "delete", None, None)
                .unwrap_err()
                .code,
            "invalid_credentials"
        );
        let updated = store
            .manage_nested_file(
                &nested.id,
                "0427".into(),
                &source.id,
                "edit",
                Some("renamed.md".into()),
                Some("Changed secret".into()),
            )
            .unwrap();
        assert_eq!(updated[0].name, "renamed.md");
        assert_eq!(
            store
                .read_nested_text_file(&nested.id, "0427".into(), &source.id)
                .unwrap(),
            "Changed secret"
        );
        assert!(store.list_files().unwrap().is_empty());
        let transferred = store
            .manage_nested_file(
                &nested.id,
                "0427".into(),
                &source.id,
                "transfer_to_parent",
                None,
                None,
            )
            .unwrap();
        assert!(transferred.is_empty());
        assert_eq!(store.read_text_file(&source.id).unwrap(), "Changed secret");
        let child = store
            .create_nested_text_file(
                &nested.id,
                "0427".into(),
                "throwaway.txt".into(),
                "Delete me".into(),
            )
            .unwrap();
        assert_eq!(child.len(), 1);
        assert!(
            store
                .manage_nested_file(
                    &nested.id,
                    "0427".into(),
                    &child[0].id,
                    "delete",
                    None,
                    None
                )
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn nested_workspace_creates_files_without_plaintext_or_parent_leakage() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let parent = store.create("Parent".into(), PASSWORD.into()).unwrap();
        let source = store.import_example("notes").unwrap();
        let nested = store
            .lock_selected_files("Projects".into(), "0427".into(), vec![source.id.clone()])
            .unwrap();
        assert_eq!(
            store
                .create_nested_text_file(
                    &nested.id,
                    "0000".into(),
                    "draft.md".into(),
                    "# Secret".into()
                )
                .unwrap_err()
                .code,
            "invalid_credentials"
        );
        assert_eq!(
            store
                .create_nested_text_file(&nested.id, "0427".into(), "run.sh".into(), "bad".into())
                .unwrap_err()
                .code,
            "invalid_filename"
        );
        let files = store
            .create_nested_text_file(
                &nested.id,
                "0427".into(),
                "draft.md".into(),
                "# Secret".into(),
            )
            .unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|file| file.name == "draft.md"));
        assert!(store.list_files().unwrap().is_empty());
        assert_eq!(
            store
                .create_nested_text_file(
                    &nested.id,
                    "0427".into(),
                    "draft.md".into(),
                    "duplicate".into()
                )
                .unwrap_err()
                .code,
            "duplicate_filename"
        );
        let dir_path = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", parent.id));
        let entries: Vec<_> = std::fs::read_dir(&dir_path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries.len(), 1);
        let ciphertext = std::fs::read(dir_path.join(format!("{}.bvlock", nested.id))).unwrap();
        assert!(
            !ciphertext
                .windows(b"# Secret".len())
                .any(|window| window == b"# Secret")
        );
        assert_eq!(
            store
                .restore_nested_vault(&nested.id, "0427".into())
                .unwrap()
                .len(),
            2
        );
        assert_eq!(store.list_files().unwrap().len(), 2);
    }

    #[test]
    fn parent_file_management_updates_and_deletes_only_authenticated_envelopes() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        store.create("Managed".into(), PASSWORD.into()).unwrap();
        let first = store.create_text_file("first.md", "Original").unwrap();
        let second = store.create_text_file("second.md", "Another").unwrap();
        assert_eq!(
            store
                .update_text_file(&first.id, "second.md", "Collision")
                .unwrap_err()
                .code,
            "duplicate_filename"
        );
        assert_eq!(
            store
                .update_text_file(&first.id, "unsafe.sh", "No")
                .unwrap_err()
                .code,
            "invalid_filename"
        );
        let changed = store
            .update_text_file(&first.id, "renamed.md", "Updated")
            .unwrap();
        assert_eq!(changed.id, first.id);
        assert_eq!(store.read_text_file(&first.id).unwrap(), "Updated");
        assert_eq!(store.list_files().unwrap().len(), 2);
        store.delete_vault_file(&second.id).unwrap();
        assert_eq!(store.list_files().unwrap(), vec![changed]);
        store.lock().unwrap();
        assert_eq!(
            store.delete_vault_file(&first.id).unwrap_err().code,
            "vault_locked"
        );
    }

    #[test]
    fn text_documents_are_encrypted_in_vault_and_read_only_when_unlocked() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let vault = store.create("Documents".into(), PASSWORD.into()).unwrap();
        let created = store
            .create_text_file("draft.md", "# Private note")
            .unwrap();
        assert_eq!(store.read_text_file(&created.id).unwrap(), "# Private note");
        assert_eq!(store.list_files().unwrap(), vec![created.clone()]);
        let entries: Vec<_> = std::fs::read_dir(
            dir.path()
                .join("vaults")
                .join(format!("{}.files", vault.id)),
        )
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
        assert_eq!(entries, vec![format!("{}.bvfile", created.id)]);
        assert_eq!(
            store
                .create_text_file("run.sh", "echo unsafe")
                .unwrap_err()
                .code,
            "invalid_filename"
        );
        store.lock().unwrap();
        assert_eq!(
            store.read_text_file(&created.id).unwrap_err().code,
            "vault_locked"
        );
    }

    #[test]
    fn decrypt_in_vault_verifies_without_creating_plaintext_files() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let vault = store.create("Memory only".into(), PASSWORD.into()).unwrap();
        let encrypted = store.import_example("notes").unwrap();
        let before: Vec<_> = std::fs::read_dir(
            dir.path()
                .join("vaults")
                .join(format!("{}.files", vault.id)),
        )
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
        assert_eq!(store.decrypt_in_vault(&encrypted.id).unwrap(), encrypted);
        let after: Vec<_> = std::fs::read_dir(
            dir.path()
                .join("vaults")
                .join(format!("{}.files", vault.id)),
        )
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
        assert_eq!(before, after);
        assert_eq!(store.list_files().unwrap().len(), 1);
        store.lock().unwrap();
        assert_eq!(
            store.decrypt_in_vault(&encrypted.id).unwrap_err().code,
            "vault_locked"
        );
    }

    #[test]
    fn example_imports_use_same_scan_gate_and_delete_requires_exact_confirmation() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let vault = store.create("Examples".into(), PASSWORD.into()).unwrap();
        assert_eq!(
            store.import_example("blocked").unwrap_err().code,
            "import_scan_blocked"
        );
        assert_eq!(
            store.import_example("notes").unwrap().name,
            "example-notes.txt"
        );
        assert_eq!(
            store.import_example("csv").unwrap().name,
            "example-records.csv"
        );
        assert_eq!(
            store.import_example("sql").unwrap().name,
            "example-query.sql"
        );
        for (id, name) in [
            ("json", "example-settings.json"),
            ("markdown", "example-readme.md"),
            ("log", "example-audit.log"),
        ] {
            assert_eq!(store.import_example(id).unwrap().name, name);
        }
        assert_eq!(store.list_files().unwrap().len(), 6);
        assert_eq!(
            store
                .delete_locked(&vault.id, &format!("DELETE {}", vault.id))
                .unwrap_err()
                .code,
            "lock_before_delete"
        );
        store.lock().unwrap();
        assert_eq!(
            store.delete_locked(&vault.id, "DELETE").unwrap_err().code,
            "delete_confirmation_required"
        );
        assert_eq!(store.list().unwrap().len(), 1);
        store
            .delete_locked(&vault.id, &format!("DELETE {}", vault.id))
            .unwrap();
        assert!(store.list().unwrap().is_empty());
        assert!(
            !dir.path()
                .join("vaults")
                .join(format!("{}.files", vault.id))
                .exists()
        );
    }
    #[test]
    fn delete_refuses_unexpected_entries_without_removing_header() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let vault = store.create("Guard".into(), PASSWORD.into()).unwrap();
        store.lock().unwrap();
        let files = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", vault.id));
        fs::create_dir(&files).unwrap();
        fs::write(files.join("unexpected.txt"), b"keep").unwrap();
        assert!(
            store
                .delete_locked(&vault.id, &format!("DELETE {}", vault.id))
                .is_err()
        );
        assert!(store.file_path(&vault.id).unwrap().exists());
        assert!(files.join("unexpected.txt").exists());
    }

    #[test]
    fn blocked_import_never_creates_a_vault_file() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let vault = store
            .create("Scanner test".into(), PASSWORD.into())
            .unwrap();
        let source = dir.path().join("disguised.txt");
        fs::write(&source, b"MZnot-a-safe-document").unwrap();
        let error = store.import_path(&source).unwrap_err();
        assert_eq!(error.code, "import_scan_blocked");
        assert!(store.list_files().unwrap().is_empty());
        assert!(
            !dir.path()
                .join("vaults")
                .join(format!("{}.files", vault.id))
                .exists()
        );
        assert_eq!(fs::read(source).unwrap(), b"MZnot-a-safe-document");
    }

    #[test]
    fn creation_persists_only_encrypted_metadata_and_lock_clears_display_name() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let created = store
            .create("Private records".into(), PASSWORD.into())
            .unwrap();
        assert_eq!(created.name.as_deref(), Some("Private records"));
        let on_disk = fs::read_to_string(store.file_path(&created.id).unwrap()).unwrap();
        assert!(!on_disk.contains("Private records"));
        assert!(!on_disk.contains(PASSWORD));
        assert_eq!(store.list().unwrap()[0], created);
        store.lock().unwrap();
        let locked = store.list().unwrap();
        assert_eq!(locked[0].name, None);
        assert!(locked[0].is_locked);
        let reopened = VaultStore::new(dir.path().join("vaults"))
            .unlock(created.id, PASSWORD.into())
            .unwrap();
        assert_eq!(reopened.name.as_deref(), Some("Private records"));
    }

    #[test]
    fn wrong_password_does_not_unlock_or_modify_disk() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let vault = store.create("Personal".into(), PASSWORD.into()).unwrap();
        store.lock().unwrap();
        let path = store.file_path(&vault.id).unwrap();
        let before = fs::read(&path).unwrap();
        assert_eq!(
            store
                .unlock(vault.id.clone(), "wrong password".into())
                .unwrap_err()
                .code,
            "invalid_credentials"
        );
        assert!(store.list().unwrap()[0].is_locked);
        assert_eq!(fs::read(path).unwrap(), before);
    }

    #[test]
    fn tampered_ciphertext_and_cross_vault_swap_are_rejected() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let first = store.create("First".into(), PASSWORD.into()).unwrap();
        store.lock().unwrap();
        let second = store.create("Second".into(), PASSWORD.into()).unwrap();
        store.lock().unwrap();
        let a = store.file_path(&first.id).unwrap();
        let b = store.file_path(&second.id).unwrap();
        let mut first_record: VaultRecord = serde_json::from_slice(&fs::read(&a).unwrap()).unwrap();
        let second_record: VaultRecord = serde_json::from_slice(&fs::read(&b).unwrap()).unwrap();
        assert_ne!(first_record.kdf.salt, second_record.kdf.salt);
        assert_ne!(
            first_record.wrapped.ciphertext,
            second_record.wrapped.ciphertext
        );
        let mut bytes = STANDARD.decode(&first_record.wrapped.ciphertext).unwrap();
        bytes[0] ^= 1;
        first_record.wrapped.ciphertext = STANDARD.encode(bytes);
        fs::write(&a, serde_json::to_vec(&first_record).unwrap()).unwrap();
        assert_eq!(
            store
                .unlock(first.id.clone(), PASSWORD.into())
                .unwrap_err()
                .code,
            "invalid_credentials"
        );
        fs::write(&a, serde_json::to_vec(&second_record).unwrap()).unwrap();
        assert_eq!(
            store.unlock(first.id, PASSWORD.into()).unwrap_err().code,
            "unsupported_vault"
        );
    }

    #[test]
    fn a_single_vault_can_be_open_and_lock_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let first = store.create("One".into(), PASSWORD.into()).unwrap();
        assert_eq!(
            store
                .create("Two".into(), PASSWORD.into())
                .unwrap_err()
                .code,
            "vault_already_unlocked"
        );
        assert_eq!(
            store
                .unlock(first.id.clone(), PASSWORD.into())
                .unwrap_err()
                .code,
            "vault_already_unlocked"
        );
        store.lock().unwrap();
        store.lock().unwrap();
        assert_eq!(
            store
                .unlock(first.id, PASSWORD.into())
                .unwrap()
                .name
                .as_deref(),
            Some("One")
        );
    }

    #[test]
    fn invalid_inputs_and_traversal_are_rejected() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        assert_eq!(
            store.create(" ".into(), PASSWORD.into()).unwrap_err().code,
            "invalid_vault_name"
        );
        assert_eq!(
            store
                .create("Good".into(), "short".into())
                .unwrap_err()
                .code,
            "invalid_password"
        );
        assert_eq!(
            store
                .unlock("../secret".into(), PASSWORD.into())
                .unwrap_err()
                .code,
            "invalid_vault_id"
        );
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn key_derivation_matches_shared_browser_and_backend_vectors() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/crypto-v1.json")).unwrap();
        for fixture in fixtures.as_array().unwrap() {
            let params = &fixture["params"];
            let kdf = KdfRecord {
                algorithm: "argon2id-v19".into(),
                salt: params["salt"].as_str().unwrap().into(),
                m_cost: params["m_cost"].as_u64().unwrap() as u32,
                t_cost: params["t_cost"].as_u64().unwrap() as u32,
                p_cost: params["p_cost"].as_u64().unwrap() as u32,
                output_len: params["output_len"].as_u64().unwrap() as usize,
            };
            let actual = derive(fixture["password"].as_str().unwrap(), &kdf).unwrap();
            assert_eq!(
                STANDARD.encode(actual.as_ref()),
                fixture["key"].as_str().unwrap()
            );
        }
    }

    #[test]
    fn imported_files_survive_restart_and_export_identical_bytes() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("private-document.txt");
        let original = b"confidential \0 content and UTF-8 \xF0\x9F\x94\x92";
        fs::write(&source, original).unwrap();
        let store = store(&dir);
        let created = store.create("Personal".into(), PASSWORD.into()).unwrap();
        let first = store.import_path(&source).unwrap();
        let second = store.import_path(&source).unwrap();
        assert_ne!(first.id, second.id);
        assert_eq!(store.list_files().unwrap().len(), 2);
        let sealed_path = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", created.id))
            .join(format!("{}.bvfile", first.id));
        let sealed = fs::read_to_string(&sealed_path).unwrap();
        assert!(!sealed.contains("private-document"));
        assert!(!sealed.contains("confidential"));
        store.lock().unwrap();
        assert_eq!(store.list_files().unwrap_err().code, "vault_locked");
        drop(store);
        let restarted = VaultStore::new(dir.path().join("vaults"));
        restarted.unlock(created.id, PASSWORD.into()).unwrap();
        assert_eq!(restarted.list_files().unwrap().len(), 2);
        let output = dir.path().join("exported-document.txt");
        assert_eq!(restarted.export_path(&first.id, &output).unwrap(), first);
        assert_eq!(fs::read(&output).unwrap(), original);
        assert_eq!(fs::read(&source).unwrap(), original);
        assert_eq!(
            restarted.export_path(&first.id, &output).unwrap_err().code,
            "export_destination_exists"
        );
        assert_eq!(fs::read(output).unwrap(), original);
    }

    #[test]
    fn files_are_vault_bound_and_tampering_fails_closed() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("file.txt");
        fs::write(&source, b"hidden content").unwrap();
        let store = store(&dir);
        let first = store.create("First".into(), PASSWORD.into()).unwrap();
        let imported = store.import_path(&source).unwrap();
        let first_dir = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", first.id));
        let sealed_path = first_dir.join(format!("{}.bvfile", imported.id));
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&sealed_path).unwrap()).unwrap();
        let original = fs::read(&sealed_path).unwrap();
        value["content"] = "AAAA".into();
        fs::write(&sealed_path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            store.list_files().unwrap_err().code,
            "invalid_encrypted_file"
        );
        assert!(
            store
                .export_path(&imported.id, &dir.path().join("should-not-exist"))
                .is_err()
        );
        assert!(!dir.path().join("should-not-exist").exists());
        fs::write(&sealed_path, original).unwrap();
        store.lock().unwrap();
        let second = store.create("Second".into(), PASSWORD.into()).unwrap();
        let second_dir = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", second.id));
        fs::create_dir(&second_dir).unwrap();
        fs::copy(
            &sealed_path,
            second_dir.join(format!("{}.bvfile", imported.id)),
        )
        .unwrap();
        assert_eq!(
            store.list_files().unwrap_err().code,
            "invalid_encrypted_file"
        );
    }

    #[test]
    fn abandoned_encrypted_temporary_file_is_not_listed_or_exported() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("original.txt");
        fs::write(&source, b"important").unwrap();
        let store = store(&dir);
        let created = store.create("Test".into(), PASSWORD.into()).unwrap();
        let imported = store.import_path(&source).unwrap();
        let files_dir = dir
            .path()
            .join("vaults")
            .join(format!("{}.files", created.id));
        let sealed = fs::read(files_dir.join(format!("{}.bvfile", imported.id))).unwrap();
        let orphan_id = crate::file_crypto::random_id();
        fs::write(files_dir.join(format!(".{orphan_id}.tmp")), sealed).unwrap();
        assert_eq!(store.list_files().unwrap().len(), 1);
        let target_inside_storage = files_dir.join("plain.txt");
        assert!(
            store
                .export_path(&imported.id, &target_inside_storage)
                .is_err()
        );
        assert!(!target_inside_storage.exists());
    }

    #[test]
    fn unsupported_kdf_is_rejected_before_expensive_work() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let vault = store.create("Protected".into(), PASSWORD.into()).unwrap();
        store.lock().unwrap();
        let path = store.file_path(&vault.id).unwrap();
        let mut record: VaultRecord = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        record.kdf.m_cost = u32::MAX;
        fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        assert_eq!(
            store.unlock(vault.id, PASSWORD.into()).unwrap_err().code,
            "unsupported_kdf"
        );
    }
}
