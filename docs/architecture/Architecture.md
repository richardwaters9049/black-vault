# Black Vault — As-Built Architecture

**Updated:** 1 October 2026
**Status:** Black Vault 0.1.0 hardened desktop implementation.

## System overview

Black Vault is an offline-first encrypted desktop vault. The active product is a Tauri 2 application with a React/Vite/TypeScript presentation layer and a Rust security/storage core. It does not require an account, remote API, database server, Docker or an internet connection for normal operation.

```text
React 19 / TypeScript / Vite
        │
        │ restricted typed Tauri IPC
        ▼
Tauri 2 / Rust security and storage core
        │
        │ authenticated encrypted records
        ▼
Local application data
```

The WebView is not the authority for filesystem access or cryptographic keys. Native paths are obtained by Rust through the picker/drop boundary, while passwords are passed to Rust only for the operation that needs them. Vault master keys remain Rust-side.

## Rust vault core

The vault implementation is separated by responsibility:

| Module | Responsibility |
| --- | --- |
| `vault/session.rs` | Vault creation, unlock/lock, session state and nested-attempt throttling |
| `vault/crypto.rs` | Argon2id derivation and authenticated wrapping of vault records |
| `vault/files.rs` | Parent-vault import, encrypted text/file operations, export and deletion |
| `vault/nested.rs` | PIN-protected nested vaults, transfers, restore and recovery |
| `vault/models.rs` | Versioned persisted records and public summaries |
| `vault/error.rs` | Deliberately bounded user-facing errors |
| `file_crypto.rs` | Per-file encryption, authenticated envelopes and hardened filesystem helpers |
| `import_scan.rs` | Offline pre-import policy for high-confidence risky content |

The split is structural rather than cryptographic: all modules execute in the same process and share the same Rust trust boundary.

## Persistent model

A root vault has a random 256-bit master key. The user's password is processed with Argon2id to derive a wrapping key; that wrapping key protects the vault master key and encrypted vault metadata. The password is not the file-encryption key.

Each imported file receives its own random identifier and data-encryption material. Its filename and contents are authenticated and encrypted before persistence. A file envelope is bound to both its vault and file identity through AES-GCM associated data, preventing a valid ciphertext from being silently transplanted into another vault or file identity.

Nested vaults use a four-digit PIN-derived key and are additionally wrapped by the parent vault master key. Opening a nested vault therefore requires both an unlocked parent vault and the correct nested PIN.

## Filesystem and durability

Sensitive writes use restrictive Unix permissions where available, temporary files, `sync_all`, rename and directory synchronisation. Regular-file reads use bounded sizes and reject symlinks on Unix with no-follow semantics.

Transfer and restore operations are ordered so interruption favours duplicate encrypted data over silent data loss. Recovery recognises only known staging names and authenticates relevant encrypted records before cleanup or reconciliation. Fault-injection and subprocess-interruption tests exercise these persistence boundaries.

## UI and session boundary

The React layer uses typed native wrappers in `desktop/src/lib/native.ts`. The active desktop UI does not use browser local/session storage, cookies, IndexedDB, network fetches or WebSockets for vault state.

The application locks on explicit lock/sign-out and after five minutes of inactivity. Locking releases the active Rust session and its best-effort-zeroised secret buffers. This does not claim protection from malware, memory inspection or an administrator controlling the host while a vault is unlocked.

## Legacy code

`frontend/` and `backend/` preserve the earlier web architecture for history and cryptographic compatibility. They are not part of the active desktop runtime. `legacy/README.md` documents that boundary.

For security properties and threat assumptions, see [Desktop Security Architecture](DESKTOP_SECURITY_ARCHITECTURE.md) and the [Threat Model](../security/THREAT_MODEL.md).
