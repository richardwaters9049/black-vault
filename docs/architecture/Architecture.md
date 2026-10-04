# Black Vault — As-Built Architecture

**Updated:** 4 October 2026. **Scope:** 0.2.0-rc.1 internal desktop candidate.

## Runtime

React/Vite/TypeScript presents the workspace through restricted typed Tauri IPC. Rust owns root sessions, cryptography, native file selection, import screening, persistence and recovery. The active desktop has no account, network API, database service or mandatory internet connection. The old `frontend/` and `backend/` are reference implementations.

## Modules

| Module | Responsibility |
| --- | --- |
| `vault/session.rs` | Create/unlock/lock, active key and nested-attempt throttling |
| `vault/crypto.rs` | Argon2id derivation and authenticated vault-record wrapping |
| `vault/files.rs` | Parent encrypted import, text management, export and deletion |
| `vault/nested.rs` | Nested PIN compartments, transfer, restore and recovery |
| `vault/models.rs`, `vault/error.rs` | Versioned records and bounded public errors |
| `file_crypto.rs`, `import_scan.rs` | Per-file envelopes, hardened reads and bounded import policy |
| `biometric.rs` | OS biometric status and native destructive authorisation |
| `identity_store.rs` | Private transfer identity through OS credential storage |
| `contacts.rs`, `contact_commands.rs` | Public identity documents, fingerprints and contact lifecycle |
| `transfer.rs`, `transfer_codec.rs` | Rust-only signed recipient packages and bounded canonical decoding |

All modules share one native process; this is not OS/hypervisor isolation.

## Persistence

| Data | Protection and lifecycle |
| --- | --- |
| Root vault header | Random 256-bit master key wrapped using Argon2id password material; encrypted name |
| File envelopes | Independent random data keys; authenticated encrypted filename and content bound to vault/file IDs |
| Nested records | PIN derivation plus parent-master-key protection; four-digit PIN is a convenience boundary |
| Transfer private identity | Versioned binary OS credential; separate recipient/signing keys; no plaintext fallback |
| Contacts | Public keys, display names, trust and previous fingerprints in versioned JSON outside vault encryption |

The contacts JSON is not an authenticated anti-tampering trust anchor. OS credential storage is not represented as hardware-backed/non-exportable key storage. Vault backups alone do not include that credential.

Writes use narrow staging names, synchronisation and rename. Unix restrictive modes and no-follow reads apply where supported; Windows has separate filesystem limitations. Transfer/recovery ordering favours recoverable duplicate encrypted state over losing the only authenticated copy.

## UI and sessions

Overview and public contacts can be displayed without a root session. Protected file inventory and operations require successful native vault authentication. Passwords are never prefilled. The splash centres the symbol and title as one layout group; operation panels own their motion and respect reduced motion.

App owns the root session and one shared parent-file list. Editor and operation panels consume that list; a generation guard discards stale responses after closure, vault changes or unmount.

Startup, explicit locking and five-minute inactivity remove the native session. Explicit/inactivity locking immediately closes protected panels and clears drafts, PINs and selections while Rust locks; late list responses cannot restore filenames. A native lock failure remains visible and requires retry, rather than reporting a successful lock. Delayed timers/focus use wall-clock expiry; this is not a native sleep hook. Root secrets stay native; authorised text editing displays plaintext in WebView memory and saving re-encrypts it.

Permanent deletion checks exact typed confirmation and invokes fresh native biometrics when available. Standalone biometric verification is not exposed and biometrics do not unlock encrypted vaults.

## Transfer boundary

The Rust crypto/parser foundation supports in-memory signed recipient package round trips and negative/vector tests. Only identity/contact functionality is exposed in the current application. Package file send/receive, replay history and their UI remain future work; local parent/nested transfers are a different implemented feature.

See [security architecture](DESKTOP_SECURITY_ARCHITECTURE.md), [threat model](../security/THREAT_MODEL.md), [transfer status](SECURE_SHARING_IMPLEMENTATION_PLAN.md) and [candidate evidence](../release/v0.2.0-validation.md).

The [4 October maintenance audit](../engineering/CODEBASE_AUDIT_2026-10-04.md) records the latest local checks and dependency findings separately from release-candidate evidence.
