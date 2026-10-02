use super::*;

impl VaultStore {
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
            crate::file_crypto::cleanup_file(&path);
            return Err(VaultError::storage());
        }
        crate::file_crypto::sync_directory(&dir).map_err(|_| VaultError::storage())?;
        // Delete originals only AFTER the nested record is durable. On interrupted
        // deletion, duplicate ciphertext may remain, but data is not lost.
        for id in &ids {
            fs::remove_file(dir.join(format!("{id}.bvfile"))).map_err(|_| VaultError::storage())?;
        }
        crate::file_crypto::sync_directory(&dir).map_err(|_| VaultError::storage())?;
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
            crate::file_crypto::sync_directory(dir).map_err(|_| VaultError::storage())?;
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
            crate::file_crypto::sync_directory(dir).map_err(|_| VaultError::storage())?;
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
            crate::file_crypto::sync_directory(&dir)?;
            Ok(())
        })();
        if write_result.is_err() {
            crate::file_crypto::cleanup_file(&temp);
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
            crate::file_crypto::sync_directory(&dir)?;
            transfer_checkpoint("before_source_unlink")?;
            Ok(())
        })();
        if write_result.is_err() {
            crate::file_crypto::cleanup_file(&temp);
            return Err(VaultError::storage());
        }
        fs::remove_file(&source).map_err(|_| VaultError::storage())?;
        transfer_checkpoint("before_source_directory_sync").map_err(|_| VaultError::storage())?;
        crate::file_crypto::sync_directory(&dir).map_err(|_| VaultError::storage())?;
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
            crate::file_crypto::sync_directory(&dir)?;
            Ok(())
        })();
        if result.is_err() {
            crate::file_crypto::cleanup_file(&temp);
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
        crate::file_crypto::sync_directory(&dir).map_err(|_| VaultError::storage())?;
        Ok(restored.into_iter().map(|(info, _)| info).collect())
    }

    // Verify and decrypt into zeroising Rust memory only. No plaintext file is written.
}
