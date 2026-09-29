# Sprint 02 — Offline Vault Foundation

Updated: 29 September 2026  
Branch: `feat/desktop-migration`  
Status: First lifecycle slice implemented; encrypted file storage is outstanding.

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

This is an encrypted **vault header**, not yet an encrypted filesystem. Selecting a file still reads metadata only; no file contents are imported or protected by this slice. There is no SQLite, recovery key, durable audit chain, sharing, LAN or cloud service. A forgotten vault password cannot currently be recovered. Best-effort zeroisation does not defeat a compromised host while unlocked. AEAD detects authenticated-field tampering, not rollback/deletion by an attacker controlling all local copies. Do not use the foundation build for irreplaceable data.

## Next implementation slice

1. Define versioned per-file DEKs, authenticated content/filename encryption and an explicit maximum import size. Preserve the existing web compatibility fixtures.
2. Revalidate the picker-authorised file when Rust opens it; do not accept arbitrary webview-provided filesystem paths.
3. Implement crash-safe authenticated file import and export, clean partial writes and negative tests (wrong key, altered metadata, interrupted writes, vault-to-vault isolation).
4. Design encrypted metadata container and an in-memory-only SQLite evaluation after unlock; prove plaintext does not enter WAL, journals, temporary files or checkpoints before adopting it.
5. Manually test create, quit, restart, wrong-password retry, unlock and lock in the macOS bundle. Add recovery guidance before any production use.

No commits or pushes have been performed by this implementation task.
