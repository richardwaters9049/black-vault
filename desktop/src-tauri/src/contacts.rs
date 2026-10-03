#![allow(dead_code)] // Phase C stays Rust-only until the contact security gate passes.

use crate::transfer::PublicIdentity;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const IDENTITY_MAGIC: &str = "BLACKVAULT-IDENTITY";
const IDENTITY_VERSION: u16 = 1;
const STORE_MAGIC: &str = "BLACKVAULT-CONTACTS";
const STORE_VERSION: u16 = 1;
const STORE_FILE: &str = "contacts.json";
const MAX_STORE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TrustState {
    Unverified,
    Verified,
    KeyChanged,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Contact {
    pub name: String,
    pub identity: PublicIdentity,
    pub trust: TrustState,
    pub previous_verified_fingerprint: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContactError {
    InvalidIdentity,
    InvalidName,
    NotFound,
    VerificationMismatch,
    CorruptStore,
    StorageFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityDocument {
    magic: String,
    version: u16,
    recipient_key: String,
    signing_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredContact {
    name: String,
    recipient_key: String,
    signing_key: String,
    trust: TrustState,
    previous_verified_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactStoreDocument {
    magic: String,
    version: u16,
    contacts: Vec<StoredContact>,
}

#[derive(Default)]
pub(crate) struct ContactBook {
    contacts: BTreeMap<String, Contact>,
}

impl ContactBook {
    pub(crate) fn import(
        &mut self,
        name: &str,
        identity: PublicIdentity,
    ) -> Result<TrustState, ContactError> {
        let name = validate_name(name)?;

        match self.contacts.get_mut(&name) {
            Some(existing) if existing.identity == identity => Ok(existing.trust),
            Some(existing) => {
                existing.previous_verified_fingerprint = match existing.trust {
                    TrustState::Verified => Some(existing.identity.fingerprint()),
                    TrustState::KeyChanged => existing.previous_verified_fingerprint,
                    TrustState::Unverified | TrustState::Revoked => None,
                };
                existing.identity = identity;
                existing.trust = TrustState::KeyChanged;
                Ok(TrustState::KeyChanged)
            }
            None => {
                self.contacts.insert(
                    name.clone(),
                    Contact {
                        name,
                        identity,
                        trust: TrustState::Unverified,
                        previous_verified_fingerprint: None,
                    },
                );
                Ok(TrustState::Unverified)
            }
        }
    }

    pub(crate) fn verify(
        &mut self,
        name: &str,
        expected_fingerprint: [u8; 32],
    ) -> Result<(), ContactError> {
        let contact = self.contacts.get_mut(name).ok_or(ContactError::NotFound)?;
        if contact.identity.fingerprint() != expected_fingerprint {
            return Err(ContactError::VerificationMismatch);
        }

        contact.trust = TrustState::Verified;
        contact.previous_verified_fingerprint = None;
        Ok(())
    }

    pub(crate) fn revoke(&mut self, name: &str) -> Result<(), ContactError> {
        let contact = self.contacts.get_mut(name).ok_or(ContactError::NotFound)?;
        contact.trust = TrustState::Revoked;
        contact.previous_verified_fingerprint = None;
        Ok(())
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Contact> {
        self.contacts.get(name)
    }

    fn to_document(&self) -> ContactStoreDocument {
        ContactStoreDocument {
            magic: STORE_MAGIC.to_owned(),
            version: STORE_VERSION,
            contacts: self
                .contacts
                .values()
                .map(|contact| StoredContact {
                    name: contact.name.clone(),
                    recipient_key: encode_hex(&contact.identity.recipient_key),
                    signing_key: encode_hex(&contact.identity.signing_key),
                    trust: contact.trust,
                    previous_verified_fingerprint: contact
                        .previous_verified_fingerprint
                        .as_ref()
                        .map(|fingerprint| encode_hex(fingerprint)),
                })
                .collect(),
        }
    }

    fn from_document(document: ContactStoreDocument) -> Result<Self, ContactError> {
        if document.magic != STORE_MAGIC || document.version != STORE_VERSION {
            return Err(ContactError::CorruptStore);
        }

        let mut contacts = BTreeMap::new();
        for stored in document.contacts {
            let name = validate_name(&stored.name).map_err(|_| ContactError::CorruptStore)?;
            if contacts.contains_key(&name) {
                return Err(ContactError::CorruptStore);
            }

            let identity = PublicIdentity {
                recipient_key: decode_hex_32(&stored.recipient_key)
                    .map_err(|_| ContactError::CorruptStore)?,
                signing_key: decode_hex_32(&stored.signing_key)
                    .map_err(|_| ContactError::CorruptStore)?,
            };
            let previous_verified_fingerprint = stored
                .previous_verified_fingerprint
                .map(|value| decode_hex_32(&value).map_err(|_| ContactError::CorruptStore))
                .transpose()?;

            if stored.trust != TrustState::KeyChanged && previous_verified_fingerprint.is_some() {
                return Err(ContactError::CorruptStore);
            }

            contacts.insert(
                name.clone(),
                Contact {
                    name,
                    identity,
                    trust: stored.trust,
                    previous_verified_fingerprint,
                },
            );
        }

        Ok(Self { contacts })
    }
}

pub(crate) struct ContactStore {
    root: PathBuf,
}

impl ContactStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn load(&self) -> Result<ContactBook, ContactError> {
        let path = self.root.join(STORE_FILE);
        if !path.exists() {
            return Ok(ContactBook::default());
        }

        let bytes = read_store_file(&path)?;
        let document = serde_json::from_slice(&bytes).map_err(|_| ContactError::CorruptStore)?;
        ContactBook::from_document(document)
    }

    pub(crate) fn save(&self, contacts: &ContactBook) -> Result<(), ContactError> {
        self.ensure_root()?;
        let bytes = serde_json::to_vec_pretty(&contacts.to_document())
            .map_err(|_| ContactError::StorageFailure)?;
        if bytes.len() as u64 > MAX_STORE_BYTES {
            return Err(ContactError::StorageFailure);
        }

        let temp = self
            .root
            .join(format!(".contacts.{}.tmp", crate::file_crypto::random_id()));
        let destination = self.root.join(STORE_FILE);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }

        let result = (|| -> std::io::Result<()> {
            let mut file = options.open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, &destination)?;
            crate::file_crypto::sync_directory(&self.root)?;
            Ok(())
        })();

        if result.is_err() {
            crate::file_crypto::cleanup_file(&temp);
            return Err(ContactError::StorageFailure);
        }
        Ok(())
    }

    fn ensure_root(&self) -> Result<(), ContactError> {
        fs::create_dir_all(&self.root).map_err(|_| ContactError::StorageFailure)?;
        let metadata =
            fs::symlink_metadata(&self.root).map_err(|_| ContactError::StorageFailure)?;
        if !metadata.file_type().is_dir() {
            return Err(ContactError::StorageFailure);
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))
                .map_err(|_| ContactError::StorageFailure)?;
        }
        Ok(())
    }
}

pub(crate) fn export_public_identity(identity: &PublicIdentity) -> Result<Vec<u8>, ContactError> {
    let document = IdentityDocument {
        magic: IDENTITY_MAGIC.to_owned(),
        version: IDENTITY_VERSION,
        recipient_key: encode_hex(&identity.recipient_key),
        signing_key: encode_hex(&identity.signing_key),
    };

    serde_json::to_vec_pretty(&document).map_err(|_| ContactError::InvalidIdentity)
}

pub(crate) fn import_public_identity(bytes: &[u8]) -> Result<PublicIdentity, ContactError> {
    let document: IdentityDocument =
        serde_json::from_slice(bytes).map_err(|_| ContactError::InvalidIdentity)?;

    if document.magic != IDENTITY_MAGIC || document.version != IDENTITY_VERSION {
        return Err(ContactError::InvalidIdentity);
    }

    Ok(PublicIdentity {
        recipient_key: decode_hex_32(&document.recipient_key)?,
        signing_key: decode_hex_32(&document.signing_key)?,
    })
}

pub(crate) fn format_fingerprint(identity: &PublicIdentity) -> String {
    identity
        .fingerprint()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| encode_hex(chunk))
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

fn validate_name(name: &str) -> Result<String, ContactError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 100 || trimmed.chars().any(char::is_control) {
        return Err(ContactError::InvalidName);
    }
    Ok(trimmed.to_owned())
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], ContactError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ContactError::InvalidIdentity);
    }

    let mut decoded = [0u8; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let text = std::str::from_utf8(pair).map_err(|_| ContactError::InvalidIdentity)?;
        decoded[index] = u8::from_str_radix(text, 16).map_err(|_| ContactError::InvalidIdentity)?;
    }
    Ok(decoded)
}

fn read_store_file(path: &Path) -> Result<Vec<u8>, ContactError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ContactError::StorageFailure)?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_STORE_BYTES {
        return Err(ContactError::CorruptStore);
    }

    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options
        .open(path)
        .map_err(|_| ContactError::StorageFailure)?;
    let opened = file.metadata().map_err(|_| ContactError::StorageFailure)?;
    if !opened.file_type().is_file() || opened.len() > MAX_STORE_BYTES {
        return Err(ContactError::CorruptStore);
    }

    use std::io::Read;
    let mut bytes = Vec::with_capacity(opened.len() as usize);
    file.take(MAX_STORE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ContactError::StorageFailure)?;
    if bytes.len() as u64 > MAX_STORE_BYTES {
        return Err(ContactError::CorruptStore);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::TransferIdentity;

    fn identity() -> PublicIdentity {
        TransferIdentity::generate().unwrap().public_identity()
    }

    #[test]
    fn public_identity_round_trips_without_private_material() {
        let public = identity();
        let encoded = export_public_identity(&public).unwrap();
        let decoded = import_public_identity(&encoded).unwrap();

        assert_eq!(decoded, public);
        assert!(!encoded.is_empty());
    }

    #[test]
    fn identity_import_rejects_unknown_fields_and_versions() {
        let public = identity();
        let mut value: serde_json::Value =
            serde_json::from_slice(&export_public_identity(&public).unwrap()).unwrap();
        value["unexpected"] = serde_json::json!(true);
        assert_eq!(
            import_public_identity(&serde_json::to_vec(&value).unwrap()),
            Err(ContactError::InvalidIdentity)
        );

        value.as_object_mut().unwrap().remove("unexpected");
        value["version"] = serde_json::json!(2);
        assert_eq!(
            import_public_identity(&serde_json::to_vec(&value).unwrap()),
            Err(ContactError::InvalidIdentity)
        );
    }

    #[test]
    fn new_contact_is_unverified() {
        let mut contacts = ContactBook::default();
        assert_eq!(
            contacts.import("Alice", identity()).unwrap(),
            TrustState::Unverified
        );
        assert_eq!(contacts.get("Alice").unwrap().trust, TrustState::Unverified);
    }

    #[test]
    fn verified_contact_stays_verified_when_same_identity_is_reimported() {
        let mut contacts = ContactBook::default();
        let public = identity();
        let fingerprint = public.fingerprint();

        contacts.import("Alice", public.clone()).unwrap();
        contacts.verify("Alice", fingerprint).unwrap();

        assert_eq!(
            contacts.import("Alice", public).unwrap(),
            TrustState::Verified
        );
    }

    #[test]
    fn changed_identity_never_inherits_verified_state() {
        let mut contacts = ContactBook::default();
        let first = identity();
        let replacement = identity();

        contacts.import("Alice", first.clone()).unwrap();
        contacts.verify("Alice", first.fingerprint()).unwrap();

        assert_eq!(
            contacts.import("Alice", replacement.clone()).unwrap(),
            TrustState::KeyChanged
        );
        let contact = contacts.get("Alice").unwrap();
        assert_eq!(contact.identity, replacement);
        assert_eq!(contact.trust, TrustState::KeyChanged);
        assert_eq!(
            contact.previous_verified_fingerprint,
            Some(first.fingerprint())
        );
    }

    #[test]
    fn verification_requires_exact_current_fingerprint() {
        let mut contacts = ContactBook::default();
        let first = identity();
        let replacement = identity();

        contacts.import("Alice", first.clone()).unwrap();
        contacts.import("Alice", replacement.clone()).unwrap();

        assert_eq!(
            contacts.verify("Alice", first.fingerprint()),
            Err(ContactError::VerificationMismatch)
        );
        assert_eq!(contacts.get("Alice").unwrap().trust, TrustState::KeyChanged);

        contacts.verify("Alice", replacement.fingerprint()).unwrap();
        assert_eq!(contacts.get("Alice").unwrap().trust, TrustState::Verified);
        assert_eq!(
            contacts.get("Alice").unwrap().previous_verified_fingerprint,
            None
        );
    }

    #[test]
    fn revoked_contact_does_not_regain_trust_on_reimport() {
        let mut contacts = ContactBook::default();
        let public = identity();

        contacts.import("Alice", public.clone()).unwrap();
        contacts.verify("Alice", public.fingerprint()).unwrap();
        contacts.revoke("Alice").unwrap();

        assert_eq!(
            contacts.import("Alice", public).unwrap(),
            TrustState::Revoked
        );
    }

    #[test]
    fn fingerprint_is_stable_and_grouped_for_human_comparison() {
        let public = identity();
        let first = format_fingerprint(&public);
        let second = format_fingerprint(&public);

        assert_eq!(first, second);
        assert_eq!(first.split(' ').count(), 8);
        assert_eq!(first.replace(' ', "").len(), 64);
    }

    #[test]
    fn contact_names_are_trimmed_and_control_characters_are_rejected() {
        let mut contacts = ContactBook::default();
        contacts.import("  Alice  ", identity()).unwrap();
        assert!(contacts.get("Alice").is_some());

        assert_eq!(
            contacts.import("Alice\nMallory", identity()),
            Err(ContactError::InvalidName)
        );
    }

    #[test]
    fn persisted_contacts_preserve_trust_and_key_change_history() {
        let temp = tempfile::tempdir().unwrap();
        let store = ContactStore::new(temp.path().join("contacts"));
        let first = identity();
        let replacement = identity();
        let mut contacts = ContactBook::default();

        contacts.import("Alice", first.clone()).unwrap();
        contacts.verify("Alice", first.fingerprint()).unwrap();
        contacts.import("Alice", replacement.clone()).unwrap();
        store.save(&contacts).unwrap();

        let loaded = store.load().unwrap();
        let alice = loaded.get("Alice").unwrap();
        assert_eq!(alice.identity, replacement);
        assert_eq!(alice.trust, TrustState::KeyChanged);
        assert_eq!(
            alice.previous_verified_fingerprint,
            Some(first.fingerprint())
        );
    }

    #[test]
    fn missing_store_loads_as_empty_contact_book() {
        let temp = tempfile::tempdir().unwrap();
        let store = ContactStore::new(temp.path().join("contacts"));

        assert!(store.load().unwrap().get("Alice").is_none());
    }

    #[test]
    fn corrupt_store_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("contacts");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(STORE_FILE), b"{not-json").unwrap();

        assert!(matches!(
            ContactStore::new(root).load(),
            Err(ContactError::CorruptStore)
        ));
    }

    #[test]
    fn duplicate_contact_names_in_store_are_rejected() {
        let public = identity();
        let stored = StoredContact {
            name: "Alice".to_owned(),
            recipient_key: encode_hex(&public.recipient_key),
            signing_key: encode_hex(&public.signing_key),
            trust: TrustState::Unverified,
            previous_verified_fingerprint: None,
        };
        let document = ContactStoreDocument {
            magic: STORE_MAGIC.to_owned(),
            version: STORE_VERSION,
            contacts: vec![stored.clone(), stored],
        };

        assert!(matches!(
            ContactBook::from_document(document),
            Err(ContactError::CorruptStore)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn contact_store_rejects_symlinked_store_file() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("contacts");
        fs::create_dir_all(&root).unwrap();
        let target = temp.path().join("target.json");
        fs::write(&target, b"{}").unwrap();
        symlink(&target, root.join(STORE_FILE)).unwrap();

        assert!(ContactStore::new(root).load().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn contact_store_uses_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("contacts");
        let store = ContactStore::new(root.clone());
        let mut contacts = ContactBook::default();
        contacts.import("Alice", identity()).unwrap();
        store.save(&contacts).unwrap();

        assert_eq!(
            fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(root.join(STORE_FILE))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
