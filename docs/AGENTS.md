# Black Vault — Engineering and Agent Rules

## Current baseline — 4 October 2026

Active candidate: `release/0.2.0-rc.1`, version `0.2.0-rc.1`. Root/desktop README, current architecture/threat model and `release/v0.2.0-validation.md` describe the current implementation. Earlier dated sprint descriptions below are historical context, not evidence that new candidate acceptance is complete. Never silently overwrite historical v0.1.0 release records.

Touch ID authorises permanent deletion natively when available; password authentication still unwraps vault keys. There is no standalone biometric-success IPC command. Trusted contacts/OS identity storage and Rust transfer primitives are implemented; end-user package send/receive, replay history and biometric vault enrolment are deferred. Contact metadata/trust JSON is not encrypted or authenticated against valid local replacement.

The [4 October maintenance audit](engineering/CODEBASE_AUDIT_2026-10-04.md) records the newer locking/error-handling implementation, cleanup and local verification. App owns the root session and parent-file list; stale asynchronous view responses must not restore protected data after closure or locking. Historical counts and obsolete components named below remain historical context.

Routine CI uses Linux validation. Reserve full native packaging for release branches/manual dispatch. Record real OS credential and packaged-GUI evidence separately; never tick human-present biometric/GUI acceptance from mocked/unit tests.


Updated: 29 September 2026. Active work: **offline-first desktop application**. Sprint 10 automated security/recovery work is complete; packaged-app acceptance and public distribution checks remain open. Read the root [README](../README.md), [desktop README](../desktop/README.md), [Sprint 02 record](sprints/sprint-02/SPRINT_02_VAULT_FOUNDATION.md) and [security design](architecture/DESKTOP_SECURITY_ARCHITECTURE.md) before changing security-sensitive code.

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

See [Sprint 02](sprints/sprint-02/SPRINT_02_VAULT_FOUNDATION.md) for current scope and [desktop security architecture](architecture/DESKTOP_SECURITY_ARCHITECTURE.md) for proposals that must not be represented as delivered.

### Sprint 03 desktop UX (29/09/2026)
- Current branch: `feat/desktop-auth-experience`. Native `./start.sh` launches the compiled macOS app; `dev`, `build`, `test`, `help` are desktop commands; `legacy-start` retains Docker start.
- React `SplashScreen` and `AuthGate` require vault-password unlock or first-time vault creation before rendering the dashboard. The Rust vault is locked on webview startup and sign-out. Do not treat the React gate as a separate security boundary or claim a multi-user login system.
- Follow `docs/sprints/sprint-03/SPRINT_03_AUTH_EXPERIENCE.md`. Next: manual native acceptance, inactivity/sleep auto-lock, IPC boundary review. After each task update relevant documentation and propose a commit message in `gc "Feat/Desktop: ..."` format; do not commit/push automatically.

### Sprint 04 (29/09/2026)
Branch `feat/desktop-auto-lock`: first-time vault creation has public example placeholders, not prefilled credentials. React inactivity tracking requests Rust `lock_vault` after five minutes and checks wall-clock expiry after timer delays. Do not claim native sleep interception or per-session IPC tokens. See `docs/sprints/sprint-04/SPRINT_04_AUTO_LOCK.md`. Manual native acceptance remains outstanding.

### Desktop brand polish (29/09/2026)
Stay on `feat/desktop-auto-lock` for this small UI change. Shared blue/black SVG `VaultMark` is used by splash, sign-in and sidebar; keep animation lightweight and respect reduced motion. Creation-only Show/Hide toggles the password input type without storing credentials. No Rust crypto changes.

Splash timing refinement: 2.8-second minimum brand reveal; the animated large SVG VaultMark spins on its Y-axis and settles upright, with reduced-motion support. Do not change the small static mark in the sign-in or sidebar. Startup must still fail closed if Rust lock/initialisation fails.

Splash refinement: fast .95s smooth planar logo revolution (no Y-axis flip/overshoot), while preserving 2.8s startup reveal and reduced-motion fallback. This supersedes the earlier Y-axis spin description above.

### Offline pre-import gate (29/09/2026)
Rust `import_scan::scan` must execute on the exact bounded in-memory bytes before `active_file_dir`, encryption or persistence in `VaultStore::import_path`. A rejection must not write into the vault, execute the source, or change the source file. This is a conservative offline static screen, NOT comprehensive malware detection. See `docs/sprints/sprint-04/SPRINT_04_IMPORT_SCREENING.md`. Do not present ordinary SQL statements as malware.

Example Files / deletion: sample IDs are strictly enumerated in Rust and use `VaultStore::import_bytes`, including the pre-import gate. `delete_locked` must reject active sessions, wrong confirmation, unknown entries and symlinks. It removes known encrypted files before the header. Do not describe filesystem deletion as secure erasure or forgotten-password deletion as recovery. See `docs/sprints/sprint-05/SPRINT_05_EXAMPLES_VAULT_LIFECYCLE.md`.
