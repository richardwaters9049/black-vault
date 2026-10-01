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
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    let mut manifest_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    manifest_file.write_all(&manifest_bytes).unwrap();
    manifest_file.sync_all().unwrap();
    drop(manifest_file);
    crate::file_crypto::sync_directory(std::path::Path::new(&root)).unwrap();
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
            serde_json::from_slice(&fs::read(dir.path().join("manifest.json")).unwrap()).unwrap();
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
                "before_directory_sync" | "before_source_unlink" | "before_source_directory_sync"
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
        serde_json::from_str(include_str!("../../../../tests/fixtures/crypto-v1.json")).unwrap();
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
