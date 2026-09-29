# Sprint 02 — Offline Vault Foundation

Updated: 29 September 2026  
Branch: `feat/desktop-migration`  
Status: Vault lifecycle and bounded encrypted file import/list/export implemented; metadata, recovery and native acceptance outstanding.

## Delivered

- Added `desktop/src-tauri/src/vault.rs` as a standalone Rust local vault core. No Axum, PostgreSQL, Docker or account is required.
- Each new vault generates an independent, random 32-byte master key and 16-byte Argon2id salt.
- Canonical derivation: Argon2id v19; m=19,456 KiB, t=2, p=1, output=32 bytes. Shared `tests/fixtures/crypto-v1.json` vectors validate compatibility.
- The per-vault master key **and display name** are wrapped with AES-256-GCM, with a fresh 12-byte nonce and a domain-separated AAD bound to the opaque vault identifier. Header format is versioned; the implementation does not silently interpret legacy web envelopes as desktop vaults.
- Only version, opaque ID, fixed KDF settings/salt and the wrapped payload are written to the local header. A locked vault reveals only its opaque ID.
- Vault headers live beneath Tauri's application-local data directory. Unix storage uses a `0700` directory and `0600` file permissions, with create-new temporary write, file sync, rename and directory sync.
- Rust owns active key material using best-effort zeroising wrappers. Only one vault can be open; explicit lock releases its Rust-side key. Passwords are sent through authorised IPC only for create/unlock and React clears input state after each attempt.
- Registered `list_vaults`, `create_vault`, `unlock_vault`, `lock_vault` alongside the existing native commands, restricted to the main local Tauri window.
- Added a standalone desktop Vault Workspace with create, list, unlock, lock and structured error handling.

## Verification

- Rust: tests for create/restart/reopen, hidden locked names, wrong password, tampering, cross-vault header swap, independent salts and ciphertexts, one active vault, invalid IDs/inputs, rejected unsupported KDF cost, and canonical compatibility vectors.
- TypeScript: native-command contract and rendered vault lifecycle/error tests, plus the existing file-selection tests.
- Run: `cd desktop && npm run typecheck && npm test && npm run build && npm run test:rust`.
- Before distribution: manually validate the native macOS workflow and separately complete Developer ID signing/notarisation.

## Security limits

The original preview-only file picker still reads metadata only. The separate native Import and encrypt command now stores bounded encrypted file contents and filenames inside an unlocked vault. There is no SQLite, recovery key, durable audit chain, sharing, LAN or cloud service. A forgotten vault password cannot currently be recovered. Best-effort zeroisation does not defeat a compromised host while unlocked. AEAD detects authenticated-field tampering, not rollback/deletion by an attacker controlling all local copies. Do not use the foundation build for irreplaceable data.

## Next implementation slice

1. Extend the implemented versioned per-file DEK envelope beyond the current 5 MiB bounded in-memory import. Preserve the existing web compatibility fixtures.
2. Retain native-picker-authorised paths and strengthen file identity/revalidation against replacement races.
3. Add orphan temporary-file recovery and stronger interrupted-write/rollback tests; existing negative tests cover wrong keys, tampering and vault isolation.
4. Design encrypted metadata container and an in-memory-only SQLite evaluation after unlock; prove plaintext does not enter WAL, journals, temporary files or checkpoints before adopting it.
5. Manually test create, quit, restart, wrong-password retry, unlock and lock in the macOS bundle. Add recovery guidance before any production use.

No commits or pushes have been performed by this implementation task.

## Encrypted file storage slice — feat/desktop-file-encryption

The first bounded import/export implementation is now present:

- desktop/src-tauri/src/file_crypto.rs: v1 authenticated JSON envelope with a fresh random 32-byte DEK per file. AES-256-GCM wraps each DEK under the active vault master key and encrypts the filename plus file contents. Random nonces and distinct vault-ID/file-ID/purpose AAD bind each field. Source filenames and contents are not persisted as plaintext by Black Vault.
- Maximum import: **5 MiB**. Files are read in bounded memory; streaming/chunked support is not yet implemented. Encrypted envelopes have an 8 MiB read/write cap.
- import_vault_file: native picker supplies the source path entirely inside Rust; it revalidates the selected regular file and rejects symlinks. Original source files are left untouched.
- list_vault_files: requires the active unlocked vault; authenticates encrypted records before returning minimal display metadata. Locked vaults cannot list filenames.
- export_vault_file: native save picker selects the destination. Rust authenticates and decrypts, then creates a **new** plaintext file; existing destinations cannot be overwritten. Export within Black Vault's encrypted storage root is rejected.
- Private per-vault file directory with Unix 0700 permissions and 0600 encrypted files; create-new temporary file, sync, rename and directory sync. Each file receives a random opaque ID; duplicate source filenames are allowed.
- desktop/src/components/VaultFiles.tsx: import/list/export controls appear only inside an unlocked vault.
- Added tests for ciphertext randomness, round-trip, wrong keys, cross-vault copies, tampering, invalid filenames, size limits, symlinks, no-overwrite, lock restrictions and restart/reopen/export.

**Limitations:** A user-authorised export deliberately creates plaintext at the chosen destination. This bounded format is not streaming and decrypts records to list their names; a dedicated encrypted metadata index is future work. Unexpected interruption can leave an ignored temporary encrypted file; robust orphan recovery, storage rollback detection, filesystem durability across all platforms and native picker acceptance still require work. This is not production-ready and still has no password recovery.

**Verification:** 18 Rust tests and 14 frontend tests passed, along with TypeScript, frontend build, formatting and strict Clippy. Run a new native macOS bundle build and manual import/restart/export acceptance before treating the workflow as fully validated.
