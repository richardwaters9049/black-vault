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
pub struct VaultStore {
    root: PathBuf,
    active: Mutex<Option<OpenVault>>,
}
impl VaultStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            active: Mutex::new(None),
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
        let sealed = crate::file_crypto::seal(&vault.id, &id, &vault.master_key, name, &bytes)?;
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
                let valid = name
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
    fn decrypt_in_vault_verifies_without_creating_plaintext_files() {
        let dir = TempDir::new().unwrap();
        let store = store(&dir);
        let vault = store.create("Memory only".into(), PASSWORD.into()).unwrap();
        let encrypted = store.import_example("notes").unwrap();
        let before: Vec<_> = std::fs::read_dir(dir.path().join("vaults").join(format!("{}.files", vault.id)))
            .unwrap().map(|entry| entry.unwrap().file_name()).collect();
        assert_eq!(store.decrypt_in_vault(&encrypted.id).unwrap(), encrypted);
        let after: Vec<_> = std::fs::read_dir(dir.path().join("vaults").join(format!("{}.files", vault.id)))
            .unwrap().map(|entry| entry.unwrap().file_name()).collect();
        assert_eq!(before, after);
        assert_eq!(store.list_files().unwrap().len(), 1);
        store.lock().unwrap();
        assert_eq!(store.decrypt_in_vault(&encrypted.id).unwrap_err().code, "vault_locked");
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
