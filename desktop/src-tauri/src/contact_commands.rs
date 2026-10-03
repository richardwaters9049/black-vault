use crate::{
    contacts::{
        Contact, ContactBook, ContactError, ContactStore, TrustState, export_public_identity,
        format_fingerprint, import_public_identity,
    },
    identity_store::{IdentityManager, IdentityStoreError, OsSecretStore},
};
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};

pub(crate) struct ContactState {
    identity: IdentityManager<OsSecretStore>,
    store: ContactStore,
    contacts: Mutex<ContactBook>,
}

impl ContactState {
    pub(crate) fn load(root: PathBuf) -> Result<Self, ContactError> {
        let store = ContactStore::new(root);
        let contacts = store.load()?;
        Ok(Self {
            identity: IdentityManager::os_default(),
            store,
            contacts: Mutex::new(contacts),
        })
    }

    fn read_contacts(&self) -> Result<std::sync::MutexGuard<'_, ContactBook>, ContactError> {
        self.contacts
            .lock()
            .map_err(|_| ContactError::StorageFailure)
    }

    fn update_contacts<T>(
        &self,
        update: impl FnOnce(&mut ContactBook) -> Result<T, ContactError>,
    ) -> Result<T, ContactError> {
        let mut live = self.read_contacts()?;
        let mut candidate = live.clone();
        let result = update(&mut candidate)?;
        self.store.save(&candidate)?;
        *live = candidate;
        Ok(result)
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicIdentityView {
    fingerprint: String,
    document: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContactView {
    name: String,
    fingerprint: String,
    trust: TrustState,
    previous_verified_fingerprint: Option<String>,
}

impl From<Contact> for ContactView {
    fn from(contact: Contact) -> Self {
        Self {
            name: contact.name,
            fingerprint: format_fingerprint(&contact.identity),
            trust: contact.trust,
            previous_verified_fingerprint: contact
                .previous_verified_fingerprint
                .map(format_fingerprint_bytes),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ContactCommandError {
    code: &'static str,
    message: &'static str,
}

impl ContactCommandError {
    fn identity_unavailable() -> Self {
        Self {
            code: "identity_unavailable",
            message: "The transfer identity is unavailable. Please try again.",
        }
    }

    fn contact(error: ContactError) -> Self {
        match error {
            ContactError::InvalidIdentity => Self {
                code: "invalid_contact_identity",
                message: "The contact identity is invalid or unsupported.",
            },
            ContactError::InvalidName => Self {
                code: "invalid_contact_name",
                message: "Enter a valid contact name.",
            },
            ContactError::NotFound => Self {
                code: "contact_not_found",
                message: "The contact could not be found.",
            },
            ContactError::VerificationMismatch => Self {
                code: "fingerprint_mismatch",
                message: "The fingerprint does not match this contact's current identity.",
            },
            ContactError::CorruptStore | ContactError::StorageFailure => Self {
                code: "contact_storage_unavailable",
                message: "Contact data is unavailable. No trust change was applied.",
            },
        }
    }
}

impl From<ContactError> for ContactCommandError {
    fn from(error: ContactError) -> Self {
        Self::contact(error)
    }
}

impl From<IdentityStoreError> for ContactCommandError {
    fn from(_error: IdentityStoreError) -> Self {
        Self::identity_unavailable()
    }
}

#[tauri::command]
pub(crate) fn get_public_identity(
    state: tauri::State<'_, ContactState>,
) -> Result<PublicIdentityView, ContactCommandError> {
    let identity = state.identity.public_identity()?;
    let document = String::from_utf8(export_public_identity(&identity)?)
        .map_err(|_| ContactCommandError::identity_unavailable())?;
    Ok(PublicIdentityView {
        fingerprint: format_fingerprint(&identity),
        document,
    })
}

#[tauri::command]
pub(crate) fn list_contacts(
    state: tauri::State<'_, ContactState>,
) -> Result<Vec<ContactView>, ContactCommandError> {
    Ok(state
        .read_contacts()?
        .list()
        .into_iter()
        .map(ContactView::from)
        .collect())
}

#[tauri::command]
pub(crate) fn import_contact_identity(
    state: tauri::State<'_, ContactState>,
    name: String,
    document: String,
) -> Result<ContactView, ContactCommandError> {
    let identity = import_public_identity(document.as_bytes())?;
    Ok(state.update_contacts(|contacts| {
        contacts.import(&name, identity)?;
        contacts
            .get(name.trim())
            .cloned()
            .map(ContactView::from)
            .ok_or(ContactError::NotFound)
    })?)
}

#[tauri::command]
pub(crate) fn verify_contact_fingerprint(
    state: tauri::State<'_, ContactState>,
    name: String,
    fingerprint: String,
) -> Result<ContactView, ContactCommandError> {
    let expected = parse_fingerprint(&fingerprint)?;
    Ok(state.update_contacts(|contacts| {
        contacts.verify(&name, expected)?;
        contacts
            .get(&name)
            .cloned()
            .map(ContactView::from)
            .ok_or(ContactError::NotFound)
    })?)
}

#[tauri::command]
pub(crate) fn revoke_contact(
    state: tauri::State<'_, ContactState>,
    name: String,
) -> Result<ContactView, ContactCommandError> {
    Ok(state.update_contacts(|contacts| {
        contacts.revoke(&name)?;
        contacts
            .get(&name)
            .cloned()
            .map(ContactView::from)
            .ok_or(ContactError::NotFound)
    })?)
}

fn parse_fingerprint(value: &str) -> Result<[u8; 32], ContactCommandError> {
    let compact: String = value
        .chars()
        .filter(|character| *character != ' ')
        .collect();
    if compact.len() != 64 || !compact.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ContactCommandError {
            code: "invalid_fingerprint",
            message: "Enter the complete 64-character fingerprint.",
        });
    }

    let mut fingerprint = [0u8; 32];
    for (index, pair) in compact.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let text = std::str::from_utf8(pair).map_err(|_| ContactCommandError {
            code: "invalid_fingerprint",
            message: "Enter the complete 64-character fingerprint.",
        })?;
        fingerprint[index] = u8::from_str_radix(text, 16).map_err(|_| ContactCommandError {
            code: "invalid_fingerprint",
            message: "Enter the complete 64-character fingerprint.",
        })?;
    }
    Ok(fingerprint)
}

fn format_fingerprint_bytes(fingerprint: [u8; 32]) -> String {
    fingerprint
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| {
            chunk
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::TransferIdentity;

    #[test]
    fn fingerprint_parser_accepts_grouped_case_insensitive_values() {
        let identity = TransferIdentity::generate().unwrap().public_identity();
        let formatted = format_fingerprint(&identity);
        assert_eq!(
            parse_fingerprint(&formatted.to_lowercase()).unwrap(),
            identity.fingerprint()
        );
    }

    #[test]
    fn fingerprint_parser_rejects_partial_or_non_hex_values() {
        assert_eq!(
            parse_fingerprint("ABCD").unwrap_err().code,
            "invalid_fingerprint"
        );
        assert_eq!(
            parse_fingerprint(&"Z".repeat(64)).unwrap_err().code,
            "invalid_fingerprint"
        );
    }

    #[test]
    fn failed_persistence_does_not_change_live_contact_state() {
        let temp = tempfile::tempdir().unwrap();
        let blocker = temp.path().join("not-a-directory");
        std::fs::write(&blocker, b"block").unwrap();

        let state = ContactState {
            identity: IdentityManager::os_default(),
            store: ContactStore::new(blocker.join("contacts")),
            contacts: Mutex::new(ContactBook::default()),
        };
        let public = TransferIdentity::generate().unwrap().public_identity();

        assert!(matches!(
            state.update_contacts(|contacts| contacts.import("Alice", public)),
            Err(ContactError::StorageFailure)
        ));
        assert!(state.read_contacts().unwrap().get("Alice").is_none());
    }

    #[test]
    fn contact_view_contains_public_metadata_only() {
        let identity = TransferIdentity::generate().unwrap().public_identity();
        let view = ContactView::from(Contact {
            name: "Alice".to_owned(),
            identity: identity.clone(),
            trust: TrustState::Verified,
            previous_verified_fingerprint: None,
        });
        let value = serde_json::to_value(view).unwrap();

        assert_eq!(value["name"], "Alice");
        assert_eq!(value["trust"], "verified");
        assert_eq!(value["fingerprint"], format_fingerprint(&identity));
        assert!(value.get("recipientKey").is_none());
        assert!(value.get("signingKey").is_none());
        assert!(value.get("privateKey").is_none());
    }
}
