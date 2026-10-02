use super::*;

impl VaultStore {
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
            crate::file_crypto::sync_directory(&dir)?;
            Ok(())
        })();
        if result.is_err() {
            crate::file_crypto::cleanup_file(&temp);
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
        crate::file_crypto::sync_directory(&dir).map_err(|_| VaultError::storage())
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

    pub(super) fn persist_new(&self, id: &str, bytes: &[u8]) -> Result<(), VaultError> {
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
            crate::file_crypto::sync_directory(&self.root)?;
            Ok(())
        })();
        if let Err(_error) = result {
            #[cfg(test)]
            eprintln!(
                "vault persist_new failed at {}: {_error:?}",
                self.root.display()
            );
            crate::file_crypto::cleanup_file(&temp);
            return Err(VaultError::storage());
        }
        Ok(())
    }
}
