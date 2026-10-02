#![allow(dead_code)] // Phase B remains Rust-only until the identity security gate passes.

//! Persistent transfer identity storage.
//!
//! Private identity bytes are kept behind this Rust-only abstraction. The
//! production backend uses the operating system credential store.

use crate::transfer::{PublicIdentity, TransferIdentity};
use zeroize::Zeroizing;

const SERVICE: &str = "com.blackvault.desktop";
const ACCOUNT: &str = "transfer-identity-v1";
const RECORD_VERSION: u8 = 1;
const SECRET_LEN: usize = 64;
const RECORD_LEN: usize = 1 + SECRET_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdentityStoreError {
    Unavailable,
    NotFound,
    Corrupt,
    StorageFailure,
    CryptoFailure,
}

pub(crate) trait SecretStore {
    fn get(&self) -> Result<Zeroizing<Vec<u8>>, IdentityStoreError>;
    fn set(&self, secret: &[u8]) -> Result<(), IdentityStoreError>;
    fn delete(&self) -> Result<(), IdentityStoreError>;
}

pub(crate) struct IdentityManager<S = OsSecretStore> {
    store: S,
}

impl IdentityManager<OsSecretStore> {
    pub(crate) fn os_default() -> Self {
        Self {
            store: OsSecretStore,
        }
    }
}
impl<S: SecretStore> IdentityManager<S> {
    #[cfg(test)]
    fn new(store: S) -> Self {
        Self { store }
    }

    pub(crate) fn load_or_create(&self) -> Result<TransferIdentity, IdentityStoreError> {
        match self.load() {
            Ok(identity) => Ok(identity),
            Err(IdentityStoreError::NotFound) => self.create(),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn load(&self) -> Result<TransferIdentity, IdentityStoreError> {
        let record = self.store.get()?;
        decode_identity(&record)
    }

    pub(crate) fn public_identity(&self) -> Result<PublicIdentity, IdentityStoreError> {
        Ok(self.load_or_create()?.public_identity())
    }

    pub(crate) fn rotate(&self) -> Result<PublicIdentity, IdentityStoreError> {
        let identity =
            TransferIdentity::generate().map_err(|_| IdentityStoreError::CryptoFailure)?;
        self.persist(&identity)?;
        Ok(identity.public_identity())
    }

    pub(crate) fn delete(&self) -> Result<(), IdentityStoreError> {
        self.store.delete()
    }
    fn create(&self) -> Result<TransferIdentity, IdentityStoreError> {
        let identity =
            TransferIdentity::generate().map_err(|_| IdentityStoreError::CryptoFailure)?;
        self.persist(&identity)?;
        Ok(identity)
    }

    fn persist(&self, identity: &TransferIdentity) -> Result<(), IdentityStoreError> {
        let secret = identity.secret_bytes();
        let mut record = Zeroizing::new(Vec::with_capacity(RECORD_LEN));
        record.push(RECORD_VERSION);
        record.extend_from_slice(secret.as_ref());
        self.store.set(&record)
    }
}

fn decode_identity(record: &[u8]) -> Result<TransferIdentity, IdentityStoreError> {
    if record.len() != RECORD_LEN || record[0] != RECORD_VERSION {
        return Err(IdentityStoreError::Corrupt);
    }

    let mut secret = Zeroizing::new([0u8; SECRET_LEN]);
    secret.copy_from_slice(&record[1..]);
    TransferIdentity::from_secret_bytes(&secret).map_err(|_| IdentityStoreError::Corrupt)
}

pub(crate) struct OsSecretStore;

impl OsSecretStore {
    fn entry() -> Result<keyring::Entry, IdentityStoreError> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(map_keyring_error)
    }
}
impl SecretStore for OsSecretStore {
    fn get(&self) -> Result<Zeroizing<Vec<u8>>, IdentityStoreError> {
        let secret = Self::entry()?.get_secret().map_err(map_keyring_error)?;
        Ok(Zeroizing::new(secret))
    }

    fn set(&self, secret: &[u8]) -> Result<(), IdentityStoreError> {
        Self::entry()?.set_secret(secret).map_err(map_keyring_error)
    }

    fn delete(&self) -> Result<(), IdentityStoreError> {
        Self::entry()?
            .delete_credential()
            .map_err(map_keyring_error)
    }
}

fn map_keyring_error(error: keyring::Error) -> IdentityStoreError {
    match error {
        keyring::Error::NoEntry => IdentityStoreError::NotFound,
        keyring::Error::NoStorageAccess(_)
        | keyring::Error::NoDefaultStore
        | keyring::Error::NotSupportedByStore(_) => IdentityStoreError::Unavailable,
        keyring::Error::BadEncoding(_)
        | keyring::Error::BadDataFormat(_, _)
        | keyring::Error::BadStoreFormat(_) => IdentityStoreError::Corrupt,
        _ => IdentityStoreError::StorageFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryStore {
        value: Mutex<Option<Vec<u8>>>,
        fail_set: Mutex<bool>,
        fail_delete: Mutex<bool>,
    }

    impl MemoryStore {
        fn fail_next_set(&self) {
            *self.fail_set.lock().unwrap() = true;
        }

        fn fail_next_delete(&self) {
            *self.fail_delete.lock().unwrap() = true;
        }
    }

    impl SecretStore for MemoryStore {
        fn get(&self) -> Result<Zeroizing<Vec<u8>>, IdentityStoreError> {
            self.value
                .lock()
                .unwrap()
                .clone()
                .map(Zeroizing::new)
                .ok_or(IdentityStoreError::NotFound)
        }

        fn set(&self, secret: &[u8]) -> Result<(), IdentityStoreError> {
            let mut fail = self.fail_set.lock().unwrap();
            if *fail {
                *fail = false;
                return Err(IdentityStoreError::StorageFailure);
            }
            *self.value.lock().unwrap() = Some(secret.to_vec());
            Ok(())
        }

        fn delete(&self) -> Result<(), IdentityStoreError> {
            let mut fail = self.fail_delete.lock().unwrap();
            if *fail {
                *fail = false;
                return Err(IdentityStoreError::StorageFailure);
            }
            let removed = self.value.lock().unwrap().take();
            if removed.is_some() {
                Ok(())
            } else {
                Err(IdentityStoreError::NotFound)
            }
        }
    }

    #[test]
    fn first_use_creates_and_subsequent_load_preserves_identity() {
        let manager = IdentityManager::new(MemoryStore::default());
        let first = manager.load_or_create().unwrap().public_identity();
        let second = manager.load_or_create().unwrap().public_identity();

        assert_eq!(first, second);
        assert_eq!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn rotation_replaces_both_identity_keys() {
        let manager = IdentityManager::new(MemoryStore::default());
        let before = manager.public_identity().unwrap();
        let after = manager.rotate().unwrap();

        assert_ne!(before.recipient_key, after.recipient_key);
        assert_ne!(before.signing_key, after.signing_key);
        assert_ne!(before.fingerprint(), after.fingerprint());
        assert_eq!(manager.public_identity().unwrap(), after);
    }
    #[test]
    fn failed_rotation_preserves_the_existing_identity() {
        let manager = IdentityManager::new(MemoryStore::default());
        let before = manager.public_identity().unwrap();
        manager.store.fail_next_set();

        assert!(matches!(
            manager.rotate(),
            Err(IdentityStoreError::StorageFailure)
        ));
        assert_eq!(manager.public_identity().unwrap(), before);
    }

    #[test]
    fn failed_initial_persistence_does_not_create_an_identity() {
        let manager = IdentityManager::new(MemoryStore::default());
        manager.store.fail_next_set();

        assert!(matches!(
            manager.load_or_create(),
            Err(IdentityStoreError::StorageFailure)
        ));
        assert!(matches!(manager.load(), Err(IdentityStoreError::NotFound)));
    }

    #[test]
    fn failed_delete_preserves_the_existing_identity() {
        let manager = IdentityManager::new(MemoryStore::default());
        let before = manager.public_identity().unwrap();
        manager.store.fail_next_delete();

        assert!(matches!(
            manager.delete(),
            Err(IdentityStoreError::StorageFailure)
        ));
        assert_eq!(manager.public_identity().unwrap(), before);
    }

    #[test]
    fn corrupt_or_unknown_record_never_regenerates_silently() {
        let manager = IdentityManager::new(MemoryStore::default());
        manager.store.set(&[99, 1, 2, 3]).unwrap();

        assert!(matches!(manager.load(), Err(IdentityStoreError::Corrupt)));
        assert!(matches!(
            manager.load_or_create(),
            Err(IdentityStoreError::Corrupt)
        ));
    }

    #[test]
    fn delete_removes_persisted_identity() {
        let manager = IdentityManager::new(MemoryStore::default());
        let before = manager.public_identity().unwrap();
        manager.delete().unwrap();

        assert!(matches!(manager.load(), Err(IdentityStoreError::NotFound)));
        let recreated = manager.public_identity().unwrap();
        assert_ne!(before.fingerprint(), recreated.fingerprint());
    }

    #[test]
    fn encoded_record_contains_only_version_and_private_key_material() {
        let manager = IdentityManager::new(MemoryStore::default());
        let identity = manager.load_or_create().unwrap();
        let record = manager.store.get().unwrap();

        assert_eq!(record.len(), RECORD_LEN);
        assert_eq!(record[0], RECORD_VERSION);
        assert_eq!(&record[1..], identity.secret_bytes().as_ref());
    }

    #[test]
    fn secret_round_trip_derives_the_same_public_identity() {
        let identity = TransferIdentity::generate().unwrap();
        let secret = identity.secret_bytes();
        let restored = TransferIdentity::from_secret_bytes(&secret).unwrap();

        assert_eq!(identity.public_identity(), restored.public_identity());
    }

    fn platform_secret_round_trip(account_prefix: &str) {
        let mut suffix = [0u8; 8];
        getrandom::fill(&mut suffix).unwrap();
        let account = format!(
            "{account_prefix}-{}",
            suffix
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let entry = keyring::Entry::new(SERVICE, &account).unwrap();
        let secret = [0x5au8; RECORD_LEN];

        entry.set_secret(&secret).unwrap();
        let loaded = entry.get_secret().unwrap();
        assert_eq!(loaded, secret);
        entry.delete_credential().unwrap();
        assert!(matches!(entry.get_secret(), Err(keyring::Error::NoEntry)));
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "touches the real macOS Keychain; run explicitly as a platform acceptance test"]
    fn macos_keychain_binary_secret_round_trip() {
        platform_secret_round_trip("phase-b-macos-test");
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "touches Windows Credential Manager; run explicitly as a platform acceptance test"]
    fn windows_credential_manager_binary_secret_round_trip() {
        platform_secret_round_trip("phase-b-windows-test");
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires an active Linux Secret Service session"]
    fn linux_secret_service_binary_secret_round_trip() {
        platform_secret_round_trip("phase-b-linux-test");
    }
}
