use super::*;

impl VaultStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            active: Mutex::new(None),
            nested_attempts: Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub(super) fn lock_active(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<OpenVault>>, VaultError> {
        self.active.lock().map_err(|_| {
            VaultError::new(
                "vault_state_unavailable",
                "The vault session is unavailable.",
            )
        })
    }
    pub(super) fn check_nested_attempt(&self, id: &str) -> Result<(), VaultError> {
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

    pub(super) fn record_nested_attempt(&self, id: &str, success: bool) -> Result<(), VaultError> {
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

    pub(super) fn ensure_root(&self) -> Result<(), VaultError> {
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
    pub(super) fn file_path(&self, id: &str) -> Result<PathBuf, VaultError> {
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
        let bytes = crate::file_crypto::read_regular(&path, RECORD_MAX_BYTES).map_err(|error| {
            if error.code == "file_unavailable" {
                VaultError::new("vault_not_found", "This local vault does not exist.")
            } else {
                VaultError::new("invalid_vault_record", "The vault header is invalid.")
            }
        })?;
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

    pub(super) fn active_file_dir(&self, vault: &OpenVault) -> Result<PathBuf, VaultError> {
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
}
