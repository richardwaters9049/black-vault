# Black Vault — Engineering and Agent Rules

Updated: 29 September 2026. Active work: **offline-first desktop migration**, currently the Sprint 02 vault foundation on `feat/desktop-migration`. Read the root [README](../README.md), [desktop README](../desktop/README.md), [Sprint 02 record](SPRINT_02_VAULT_FOUNDATION.md) and [security design](DESKTOP_SECURITY_ARCHITECTURE.md) before changing security-sensitive code.

## Scope and source of truth

- **`desktop/` is the active Tauri 2 / React / TypeScript / Rust product.** The `backend/` (Axum/PostgreSQL) and `frontend/` (Next.js) are preserved legacy/reference implementations. Do not reintroduce their mandatory account, API, database, Docker or cloud requirements into the desktop application.
- Sprint 01 implemented the native shell, restricted main-window IPC and metadata-only picker. Sprint 02 implements create/list/unlock/lock of encrypted local vault headers **and** bounded (5 MiB) native-picker-authorised encrypted file import/list/export. This is not a production-ready storage vault.
- Preserve `tests/fixtures/crypto-v1.json` and existing legacy compatibility tests. Never assume a desktop vault header and a web file envelope are interchangeable. Any new format needs a version, explicit provenance and tests.
- Documentation must distinguish **implemented**, **tested**, **proposed** and **not implemented** features. Use UK English. Update the root README, desktop README and relevant sprint notes when behaviour changes.

## Security boundary

- Rust owns cryptography, key material, authorised native file handles, persistence and sensitive state. React only receives the minimum display information. Never return passwords, KEKs, master keys, DEKs, plaintext contents, unrestricted filesystem paths or raw internal errors through IPC.
- Only the trusted local `main` Tauri window may invoke an explicitly allow-listed command. Keep CSP restrictive. Do not add generic filesystem, shell, network or unrestricted dialog access to the webview.
- Each vault requires an independently random master key and salt. Current header derivation is Argon2id v19, m=19,456 KiB, t=2, p=1, 32-byte output; wrapping uses AES-256-GCM, unique random 12-byte nonce and vault-ID-bound AAD. Validate versions and KDF limits **before** expensive processing.
- Validate all inputs and file sizes, reject traversal and symlink surprises, use generic credential/authentication errors, never log secrets or plaintext paths, and test tampering, wrong passwords and vault-to-vault isolation.
- Hold sensitive Rust buffers in best-effort zeroising containers; drop active key material on lock. Do not claim perfect memory erasure or protection from a compromised host while unlocked.
- Persist authenticated encrypted state using atomic, crash-conscious writes and recovery tests. Avoid plaintext metadata, journal, WAL, temporary-file or diagnostic leaks. Do not adopt SQLite until an in-memory-only lifecycle and checkpoint design is tested.
- AEAD protects authenticated data integrity but does not independently prevent rollback or deletion when an attacker controls every local copy. Do not overstate isolation: per-vault keys in one process are not Qubes-like OS or hypervisor isolation.
- Encrypted file import/export currently uses a bounded in-memory v1 envelope with an independently generated random per-file DEK, AES-256-GCM, vault/file-bound AAD and encrypted filename. Never claim streaming support, recovery, sharing, audit durability or notarised distribution exists until implemented and verified. There is currently **no password recovery**; avoid irreplaceable test data.

## Development workflow

- Inspect the actual files and Git status before editing. Keep changes scoped and preserve unrelated user work. Never silently rewrite historical documents or alter established crypto parameters to make a test pass.
- For code changes, provide full updated files when sharing code with the user, rather than disconnected snippets. Prefer safe Rust, typed TypeScript, narrow modules, `Result`-based errors and explicit state transitions.
- Add unit, integration and negative security tests for every sensitive feature. Include wrong-key, tampered-record, malformed-input, unauthorised IPC and restart/crash cases as relevant. Review the threat model when adding new trust boundaries.
- Use the native file picker as the authority for selection. A later import must revalidate and open the selected file in Rust; do not accept arbitrary paths supplied by JavaScript.
- Do not auto-commit, push, merge, delete branches, rotate secrets or alter distribution credentials without explicit user approval. Never include real passwords, signing keys or private data in source, tests or logs.

## Verification

From `desktop/`:

~~~bash
npm run typecheck
npm test
npm run build
npm run test:rust
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run desktop:build
~~~

Run the preserved legacy suites separately when touching shared formats: `(cd backend && cargo test --locked)` and `(cd frontend && npm test)` from the repository root, with their original prerequisites. Do not weaken test timeouts or crypto settings to conceal build contention.

**Latest recorded desktop checks (29 September 2026):** 18 Rust tests and 14 frontend tests passed; formatting, strict Clippy, TypeScript, frontend build and ad-hoc signed macOS bundle passed. These are dated results, not a claim that future edits are automatically verified. Native manual acceptance, Developer ID signing and notarisation are outstanding.

## Next implementation slice

1. Strengthen the existing v1 per-file DEK envelope with streaming/chunked support and explicit crash-recovery design.
2. Picker-authorised file import/export, crash-safe storage and negative tests.
3. Encrypted metadata container and tested in-memory-only SQLite evaluation.
4. Manual macOS create/restart/unlock/lock checks, recovery design and security review.

See [Sprint 02](SPRINT_02_VAULT_FOUNDATION.md) for current scope and [desktop security architecture](DESKTOP_SECURITY_ARCHITECTURE.md) for proposals that must not be represented as delivered.
