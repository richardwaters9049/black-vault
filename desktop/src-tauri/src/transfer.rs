#![allow(dead_code)] // Phase A is intentionally Rust-only until the protocol gate passes.

//! Rust-only foundation for transferable Black Vault packages.
//!
//! This module deliberately has no Tauri commands. Phase A proves the wire
//! format and cryptographic bindings before the WebView can depend on them.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hpke::{
    Deserializable, Kem as KemTrait, OpModeR, OpModeS, Serializable, aead::AesGcm256,
    kdf::HkdfSha256, kem::X25519HkdfSha256, single_shot_open, single_shot_seal,
};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::transfer_codec::{
    AEAD_ID, DecodeError, KDF_ID, KEM_ID, KEY_ID_LEN, MAGIC, MAX_PAYLOAD_LEN, PACKAGE_ID_LEN,
    SIGNATURE_LEN, VERSION, X25519_KEY_LEN, decode_package,
};

const ED25519_KEY_LEN: usize = 32;
const INFO: &[u8] = b"black-vault:transfer:v1:hpke";

type Kem = X25519HkdfSha256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferError {
    InvalidPackage,
    UnsupportedVersion,
    WrongRecipient,
    InvalidSignature,
    DecryptionFailed,
    PayloadTooLarge,
    RandomnessFailed,
}

pub(crate) struct TransferIdentity {
    recipient_private: <Kem as KemTrait>::PrivateKey,
    recipient_public: <Kem as KemTrait>::PublicKey,
    signing: SigningKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PublicIdentity {
    pub recipient_key: [u8; X25519_KEY_LEN],
    pub signing_key: [u8; ED25519_KEY_LEN],
}

impl PublicIdentity {
    pub(crate) fn fingerprint(&self) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(b"black-vault:identity:v1");
        digest.update(self.recipient_key);
        digest.update(self.signing_key);
        digest.finalize().into()
    }

    fn recipient_key_id(&self) -> [u8; KEY_ID_LEN] {
        key_id(b"black-vault:recipient-key:v1", &self.recipient_key)
    }

    fn signing_key_id(&self) -> [u8; KEY_ID_LEN] {
        key_id(b"black-vault:signing-key:v1", &self.signing_key)
    }
}

impl TransferIdentity {
    pub(crate) fn generate() -> Result<Self, TransferError> {
        let (recipient_private, recipient_public) = Kem::gen_keypair();

        let mut seed = Zeroizing::new([0u8; 32]);
        getrandom::fill(seed.as_mut()).map_err(|_| TransferError::RandomnessFailed)?;
        let signing = SigningKey::from_bytes(&seed);

        Ok(Self {
            recipient_private,
            recipient_public,
            signing,
        })
    }

    pub(crate) fn public_identity(&self) -> PublicIdentity {
        PublicIdentity {
            recipient_key: self.recipient_public.to_bytes().into(),
            signing_key: self.signing.verifying_key().to_bytes(),
        }
    }

    pub(crate) fn from_secret_bytes(secret: &[u8; 64]) -> Result<Self, TransferError> {
        let recipient_private = <Kem as KemTrait>::PrivateKey::from_bytes(&secret[..32])
            .map_err(|_| TransferError::InvalidPackage)?;
        let recipient_public = Kem::sk_to_pk(&recipient_private);
        let signing_seed: [u8; 32] = secret[32..]
            .try_into()
            .map_err(|_| TransferError::InvalidPackage)?;

        Ok(Self {
            recipient_private,
            recipient_public,
            signing: SigningKey::from_bytes(&signing_seed),
        })
    }

    pub(crate) fn secret_bytes(&self) -> Zeroizing<[u8; 64]> {
        let mut secret = Zeroizing::new([0u8; 64]);
        secret[..32].copy_from_slice(&self.recipient_private.to_bytes());
        secret[32..].copy_from_slice(&self.signing.to_bytes());
        secret
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TransferPackage {
    pub package_id: [u8; PACKAGE_ID_LEN],
    pub recipient_key_id: [u8; KEY_ID_LEN],
    pub sender_signing_key_id: [u8; KEY_ID_LEN],
    pub encapped_key: [u8; X25519_KEY_LEN],
    pub ciphertext: Vec<u8>,
    pub signature: [u8; SIGNATURE_LEN],
}

impl TransferPackage {
    pub(crate) fn encode(&self) -> Result<Vec<u8>, TransferError> {
        if self.ciphertext.len() > MAX_PAYLOAD_LEN {
            return Err(TransferError::PayloadTooLarge);
        }

        let ciphertext_len =
            u64::try_from(self.ciphertext.len()).map_err(|_| TransferError::PayloadTooLarge)?;
        let mut out = Vec::with_capacity(162 + self.ciphertext.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_be_bytes());
        out.extend_from_slice(&KEM_ID.to_be_bytes());
        out.extend_from_slice(&KDF_ID.to_be_bytes());
        out.extend_from_slice(&AEAD_ID.to_be_bytes());
        out.extend_from_slice(&self.package_id);
        out.extend_from_slice(&self.recipient_key_id);
        out.extend_from_slice(&self.sender_signing_key_id);
        out.extend_from_slice(&self.encapped_key);
        out.extend_from_slice(&ciphertext_len.to_be_bytes());
        out.extend_from_slice(&self.ciphertext);
        out.extend_from_slice(&self.signature);
        Ok(out)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, TransferError> {
        let decoded = decode_package(bytes).map_err(|error| match error {
            DecodeError::InvalidPackage => TransferError::InvalidPackage,
            DecodeError::UnsupportedVersion => TransferError::UnsupportedVersion,
            DecodeError::PayloadTooLarge => TransferError::PayloadTooLarge,
        })?;
        Ok(Self {
            package_id: decoded.package_id,
            recipient_key_id: decoded.recipient_key_id,
            sender_signing_key_id: decoded.sender_signing_key_id,
            encapped_key: decoded.encapped_key,
            ciphertext: decoded.ciphertext,
            signature: decoded.signature,
        })
    }

    fn signed_bytes(&self) -> Result<Vec<u8>, TransferError> {
        let mut unsigned = self.clone();
        unsigned.signature = [0u8; SIGNATURE_LEN];
        let mut encoded = unsigned.encode()?;
        encoded.truncate(encoded.len() - SIGNATURE_LEN);
        Ok(encoded)
    }
}

pub(crate) fn seal_for_recipient(
    sender: &TransferIdentity,
    recipient: &PublicIdentity,
    plaintext: &[u8],
) -> Result<TransferPackage, TransferError> {
    if plaintext.len() > MAX_PAYLOAD_LEN.saturating_sub(16) {
        return Err(TransferError::PayloadTooLarge);
    }

    let mut package_id = [0u8; PACKAGE_ID_LEN];
    getrandom::fill(&mut package_id).map_err(|_| TransferError::RandomnessFailed)?;

    let recipient_key_id = recipient.recipient_key_id();
    let sender_public = sender.public_identity();
    let sender_signing_key_id = sender_public.signing_key_id();
    let aad = package_aad(&package_id, &recipient_key_id, &sender_signing_key_id);

    let recipient_public = <Kem as KemTrait>::PublicKey::from_bytes(&recipient.recipient_key)
        .map_err(|_| TransferError::InvalidPackage)?;
    let (encapped_key, ciphertext) = single_shot_seal::<AesGcm256, HkdfSha256, Kem>(
        &OpModeS::Base,
        &recipient_public,
        INFO,
        plaintext,
        &aad,
    )
    .map_err(|_| TransferError::DecryptionFailed)?;

    let mut package = TransferPackage {
        package_id,
        recipient_key_id,
        sender_signing_key_id,
        encapped_key: encapped_key.to_bytes().into(),
        ciphertext,
        signature: [0u8; SIGNATURE_LEN],
    };
    package.signature = sender.signing.sign(&package.signed_bytes()?).to_bytes();
    Ok(package)
}

pub(crate) fn open_from_sender(
    recipient: &TransferIdentity,
    sender: &PublicIdentity,
    package: &TransferPackage,
) -> Result<Zeroizing<Vec<u8>>, TransferError> {
    let recipient_public = recipient.public_identity();
    if package.recipient_key_id != recipient_public.recipient_key_id() {
        return Err(TransferError::WrongRecipient);
    }
    if package.sender_signing_key_id != sender.signing_key_id() {
        return Err(TransferError::InvalidSignature);
    }

    let verifying_key = VerifyingKey::from_bytes(&sender.signing_key)
        .map_err(|_| TransferError::InvalidSignature)?;
    let signature = Signature::from_bytes(&package.signature);
    verifying_key
        .verify(&package.signed_bytes()?, &signature)
        .map_err(|_| TransferError::InvalidSignature)?;

    let encapped_key = <Kem as KemTrait>::EncappedKey::from_bytes(&package.encapped_key)
        .map_err(|_| TransferError::InvalidPackage)?;
    let aad = package_aad(
        &package.package_id,
        &package.recipient_key_id,
        &package.sender_signing_key_id,
    );
    let plaintext = single_shot_open::<AesGcm256, HkdfSha256, Kem>(
        &OpModeR::Base,
        &recipient.recipient_private,
        &encapped_key,
        INFO,
        &package.ciphertext,
        &aad,
    )
    .map_err(|_| TransferError::DecryptionFailed)?;

    Ok(Zeroizing::new(plaintext))
}

fn package_aad(
    package_id: &[u8; PACKAGE_ID_LEN],
    recipient_key_id: &[u8; KEY_ID_LEN],
    sender_signing_key_id: &[u8; KEY_ID_LEN],
) -> Vec<u8> {
    let mut aad = Vec::with_capacity(64);
    aad.extend_from_slice(b"black-vault:transfer:v1:payload");
    aad.extend_from_slice(package_id);
    aad.extend_from_slice(recipient_key_id);
    aad.extend_from_slice(sender_signing_key_id);
    aad
}

fn key_id(domain: &[u8], public_key: &[u8]) -> [u8; KEY_ID_LEN] {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(public_key);
    let digest = digest.finalize();
    let mut id = [0u8; KEY_ID_LEN];
    id.copy_from_slice(&digest[..KEY_ID_LEN]);
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identities() -> (TransferIdentity, TransferIdentity) {
        (
            TransferIdentity::generate().unwrap(),
            TransferIdentity::generate().unwrap(),
        )
    }

    #[test]
    fn package_round_trip_preserves_secret_payload() {
        let (alice, bob) = identities();
        let secret = b"Project Aurora: demonstration secret";
        let package = seal_for_recipient(&alice, &bob.public_identity(), secret).unwrap();
        let encoded = package.encode().unwrap();
        let decoded = TransferPackage::decode(&encoded).unwrap();

        let opened = open_from_sender(&bob, &alice.public_identity(), &decoded).unwrap();
        assert_eq!(opened.as_slice(), secret);
    }

    #[test]
    fn package_is_bound_to_the_intended_recipient() {
        let (alice, bob) = identities();
        let mallory = TransferIdentity::generate().unwrap();
        let package = seal_for_recipient(&alice, &bob.public_identity(), b"secret").unwrap();

        assert_eq!(
            open_from_sender(&mallory, &alice.public_identity(), &package),
            Err(TransferError::WrongRecipient)
        );
    }

    #[test]
    fn ciphertext_tampering_is_rejected_by_signature() {
        let (alice, bob) = identities();
        let mut package = seal_for_recipient(&alice, &bob.public_identity(), b"secret").unwrap();
        package.ciphertext[0] ^= 0x80;

        assert_eq!(
            open_from_sender(&bob, &alice.public_identity(), &package),
            Err(TransferError::InvalidSignature)
        );
    }

    #[test]
    fn sender_substitution_is_rejected() {
        let (alice, bob) = identities();
        let mallory = TransferIdentity::generate().unwrap();
        let package = seal_for_recipient(&alice, &bob.public_identity(), b"secret").unwrap();

        assert_eq!(
            open_from_sender(&bob, &mallory.public_identity(), &package),
            Err(TransferError::InvalidSignature)
        );
    }

    #[test]
    fn recipient_binding_tampering_is_rejected() {
        let (alice, bob) = identities();
        let mut package = seal_for_recipient(&alice, &bob.public_identity(), b"secret").unwrap();
        package.recipient_key_id[0] ^= 0x01;

        assert_eq!(
            open_from_sender(&bob, &alice.public_identity(), &package),
            Err(TransferError::WrongRecipient)
        );
    }

    #[test]
    fn package_id_tampering_is_rejected() {
        let (alice, bob) = identities();
        let mut package = seal_for_recipient(&alice, &bob.public_identity(), b"secret").unwrap();
        package.package_id[0] ^= 0x01;

        assert_eq!(
            open_from_sender(&bob, &alice.public_identity(), &package),
            Err(TransferError::InvalidSignature)
        );
    }

    #[test]
    fn decoder_rejects_truncation_and_trailing_data() {
        let (alice, bob) = identities();
        let package = seal_for_recipient(&alice, &bob.public_identity(), b"secret").unwrap();
        let encoded = package.encode().unwrap();

        assert_eq!(
            TransferPackage::decode(&encoded[..encoded.len() - 1]),
            Err(TransferError::InvalidPackage)
        );

        let mut trailing = encoded;
        trailing.push(0);
        assert_eq!(
            TransferPackage::decode(&trailing),
            Err(TransferError::InvalidPackage)
        );
    }

    #[test]
    fn algorithm_substitution_fails_closed() {
        let (alice, bob) = identities();
        let package = seal_for_recipient(&alice, &bob.public_identity(), b"secret").unwrap();
        let encoded = package.encode().unwrap();

        // KEM, KDF and AEAD identifiers occupy three consecutive u16 fields.
        for offset in [10usize, 12, 14] {
            let mut changed = encoded.clone();
            changed[offset..offset + 2].copy_from_slice(&0xffffu16.to_be_bytes());
            assert_eq!(
                TransferPackage::decode(&changed),
                Err(TransferError::InvalidPackage)
            );
        }
    }

    #[test]
    fn signature_tampering_is_rejected() {
        let (alice, bob) = identities();
        let mut package = seal_for_recipient(&alice, &bob.public_identity(), b"secret").unwrap();
        package.signature[0] ^= 0x01;

        assert_eq!(
            open_from_sender(&bob, &alice.public_identity(), &package),
            Err(TransferError::InvalidSignature)
        );
    }

    #[test]
    fn unsupported_version_fails_closed() {
        let (alice, bob) = identities();
        let package = seal_for_recipient(&alice, &bob.public_identity(), b"secret").unwrap();
        let mut encoded = package.encode().unwrap();
        encoded[8..10].copy_from_slice(&2u16.to_be_bytes());

        assert_eq!(
            TransferPackage::decode(&encoded),
            Err(TransferError::UnsupportedVersion)
        );
    }

    #[test]
    fn rfc9180_vector_opens_with_the_selected_suite() {
        // RFC 9180 vector: mode=base, KEM=0x20, KDF=0x01, AEAD=0x02.
        let sk = <Kem as KemTrait>::PrivateKey::from_bytes(&hex::<32>(
            "497b4502664cfea5d5af0b39934dac72242a74f8480451e1aee7d6a53320333d",
        ))
        .unwrap();
        let enc = <Kem as KemTrait>::EncappedKey::from_bytes(&hex::<32>(
            "6c93e09869df3402d7bf231bf540fadd35cd56be14f97178f0954db94b7fc256",
        ))
        .unwrap();
        let info = hex_vec("4f6465206f6e2061204772656369616e2055726e");
        let aad = hex_vec("436f756e742d30");
        let ciphertext = hex_vec(
            "e5d84cd531cfb583096e7cfa9641bd3079cf3a91cda813c52deb5f512be9931980a41de125a925cdad859d5b7a",
        );
        let expected = hex_vec("4265617574792069732074727574682c20747275746820626561757479");

        let opened = single_shot_open::<AesGcm256, HkdfSha256, Kem>(
            &OpModeR::Base,
            &sk,
            &enc,
            &info,
            &ciphertext,
            &aad,
        )
        .unwrap();
        assert_eq!(opened, expected);
    }

    #[test]
    fn decoder_survives_deterministic_hostile_input_corpus() {
        let (alice, bob) = identities();
        let package = seal_for_recipient(&alice, &bob.public_identity(), b"secret").unwrap();
        let valid = package.encode().unwrap();

        for cut in 0..valid.len() {
            let _ = TransferPackage::decode(&valid[..cut]);
        }
        for index in 0..valid.len() {
            for mask in [0x01u8, 0x80, 0xff] {
                let mut mutated = valid.clone();
                mutated[index] ^= mask;
                let _ = TransferPackage::decode(&mutated);
            }
        }
        for len in [0usize, 1, 7, 8, 15, 64, 128, 1024, 4096] {
            let _ = TransferPackage::decode(&vec![0xa5; len]);
        }
    }

    fn hex<const N: usize>(value: &str) -> [u8; N] {
        hex_vec(value).try_into().unwrap()
    }

    fn hex_vec(value: &str) -> Vec<u8> {
        assert_eq!(value.len() % 2, 0);
        (0..value.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn public_identity_fingerprint_is_stable_and_key_bound() {
        let alice = TransferIdentity::generate().unwrap();
        let bob = TransferIdentity::generate().unwrap();

        assert_eq!(
            alice.public_identity().fingerprint(),
            alice.public_identity().fingerprint()
        );
        assert_ne!(
            alice.public_identity().fingerprint(),
            bob.public_identity().fingerprint()
        );
    }
}
