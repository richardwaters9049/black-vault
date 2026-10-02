//! Canonical parser for the Black Vault transfer v1 outer package.
//!
//! Kept free of application state so the exact production parser can also be
//! compiled directly by the libFuzzer harness.

pub(crate) const MAGIC: &[u8; 8] = b"BLKVAULT";
pub(crate) const VERSION: u16 = 1;
pub(crate) const KEM_ID: u16 = 0x0020;
pub(crate) const KDF_ID: u16 = 0x0001;
pub(crate) const AEAD_ID: u16 = 0x0002;
pub(crate) const PACKAGE_ID_LEN: usize = 16;
pub(crate) const KEY_ID_LEN: usize = 16;
pub(crate) const X25519_KEY_LEN: usize = 32;
pub(crate) const SIGNATURE_LEN: usize = 64;
pub(crate) const MAX_PAYLOAD_LEN: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DecodedPackage {
    pub package_id: [u8; PACKAGE_ID_LEN],
    pub recipient_key_id: [u8; KEY_ID_LEN],
    pub sender_signing_key_id: [u8; KEY_ID_LEN],
    pub encapped_key: [u8; X25519_KEY_LEN],
    pub ciphertext: Vec<u8>,
    pub signature: [u8; SIGNATURE_LEN],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecodeError {
    InvalidPackage,
    UnsupportedVersion,
    PayloadTooLarge,
}

pub(crate) fn decode_package(bytes: &[u8]) -> Result<DecodedPackage, DecodeError> {
    const FIXED_LEN: usize = 8 + 2 + 2 + 2 + 2 + 16 + 16 + 16 + 32 + 8 + 64;
    if bytes.len() < FIXED_LEN || &bytes[..8] != MAGIC {
        return Err(DecodeError::InvalidPackage);
    }

    let mut cursor = 8;
    if take_u16(bytes, &mut cursor)? != VERSION {
        return Err(DecodeError::UnsupportedVersion);
    }
    if take_u16(bytes, &mut cursor)? != KEM_ID
        || take_u16(bytes, &mut cursor)? != KDF_ID
        || take_u16(bytes, &mut cursor)? != AEAD_ID
    {
        return Err(DecodeError::InvalidPackage);
    }

    let package_id = take_array::<PACKAGE_ID_LEN>(bytes, &mut cursor)?;
    let recipient_key_id = take_array::<KEY_ID_LEN>(bytes, &mut cursor)?;
    let sender_signing_key_id = take_array::<KEY_ID_LEN>(bytes, &mut cursor)?;
    let encapped_key = take_array::<X25519_KEY_LEN>(bytes, &mut cursor)?;
    let ciphertext_len =
        usize::try_from(take_u64(bytes, &mut cursor)?).map_err(|_| DecodeError::PayloadTooLarge)?;
    if ciphertext_len > MAX_PAYLOAD_LEN {
        return Err(DecodeError::PayloadTooLarge);
    }

    let expected = cursor
        .checked_add(ciphertext_len)
        .and_then(|n| n.checked_add(SIGNATURE_LEN))
        .ok_or(DecodeError::InvalidPackage)?;
    if expected != bytes.len() {
        return Err(DecodeError::InvalidPackage);
    }

    let ciphertext = bytes[cursor..cursor + ciphertext_len].to_vec();
    cursor += ciphertext_len;
    let signature = take_array::<SIGNATURE_LEN>(bytes, &mut cursor)?;
    Ok(DecodedPackage {
        package_id,
        recipient_key_id,
        sender_signing_key_id,
        encapped_key,
        ciphertext,
        signature,
    })
}

fn take_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, DecodeError> {
    Ok(u16::from_be_bytes(take_array::<2>(bytes, cursor)?))
}

fn take_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, DecodeError> {
    Ok(u64::from_be_bytes(take_array::<8>(bytes, cursor)?))
}

fn take_array<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Result<[u8; N], DecodeError> {
    let end = cursor.checked_add(N).ok_or(DecodeError::InvalidPackage)?;
    let slice = bytes.get(*cursor..end).ok_or(DecodeError::InvalidPackage)?;
    *cursor = end;
    slice.try_into().map_err(|_| DecodeError::InvalidPackage)
}
