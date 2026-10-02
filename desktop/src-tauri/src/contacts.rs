#![allow(dead_code)] // Phase C stays Rust-only until the contact security gate passes.

use crate::transfer::PublicIdentity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const IDENTITY_MAGIC: &str = "BLACKVAULT-IDENTITY";
const IDENTITY_VERSION: u16 = 1;

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContactError {
    InvalidIdentity,
    InvalidName,
    NotFound,
    VerificationMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityDocument {
    magic: String,
    version: u16,
    recipient_key: String,
    signing_key: String,
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
        Ok(())
    }

    pub(crate) fn revoke(&mut self, name: &str) -> Result<(), ContactError> {
        let contact = self.contacts.get_mut(name).ok_or(ContactError::NotFound)?;
        contact.trust = TrustState::Revoked;
        Ok(())
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Contact> {
        self.contacts.get(name)
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
        assert_eq!(contacts.get("Alice").unwrap().trust, TrustState::Verified);

        assert_eq!(
            contacts.import("Alice", replacement.clone()).unwrap(),
            TrustState::KeyChanged
        );
        let contact = contacts.get("Alice").unwrap();
        assert_eq!(contact.identity, replacement);
        assert_eq!(contact.trust, TrustState::KeyChanged);
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
}
