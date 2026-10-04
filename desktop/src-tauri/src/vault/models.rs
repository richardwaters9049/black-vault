use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::MASTER_KEY_LEN;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultSummary {
    pub id: String,
    // Names are encrypted on disk. Only the unlocked vault exposes its name.
    pub name: Option<String>,
    pub is_locked: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KdfRecord {
    pub(super) algorithm: String,
    pub(super) salt: String,
    pub(super) m_cost: u32,
    pub(super) t_cost: u32,
    pub(super) p_cost: u32,
    pub(super) output_len: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WrappedSecret {
    pub(super) nonce: String,
    pub(super) ciphertext: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VaultRecord {
    pub(super) version: u8,
    pub(super) id: String,
    pub(super) kdf: KdfRecord,
    pub(super) wrapped: WrappedSecret,
}

pub(super) struct OpenVault {
    pub(super) id: String,
    pub(super) name: Zeroizing<String>,
    // Zeroising is best-effort; host compromise while unlocked is out of scope.
    // Retained for encrypted-file operations; never exposed to JavaScript.
    pub(super) master_key: Zeroizing<[u8; MASTER_KEY_LEN]>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NestedVaultRecord {
    pub(super) version: u8,
    pub(super) parent_id: String,
    pub(super) id: String,
    pub(super) kdf: KdfRecord,
    // Double wrapping requires the parent master key and child password.
    pub(super) parent_wrapped: WrappedSecret,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NestedVaultPayload {
    pub(super) name: String,
    // Existing encrypted envelopes, never plaintext.
    pub(super) files: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NestedVaultSummary {
    pub id: String,
    pub file_count: usize,
}
