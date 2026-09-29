# Black Vault

**Black Vault is an offline-first, security-focused desktop vault for sensitive files.** The active product is a standalone macOS application built with Tauri 2, Rust, React 19, Vite, TypeScript and Tailwind CSS. Cryptographic operations and key material remain in Rust, behind a restricted native-command boundary.

> **Status — 29 September 2026:** Sprint 01 desktop foundation and the Sprint 02 vault lifecycle plus bounded encrypted file-storage slice are implemented. You can create, list, unlock and lock vaults, then import, list and explicitly export encrypted files. File import is currently limited to **5 MiB per file**. This is a development build, not a production-ready vault. See [Sprint 02](docs/SPRINT_02_VAULT_FOUNDATION.md).

## Current capabilities

| Area | Status |
| --- | --- |
| Standalone macOS Apple Silicon desktop app | Implemented; local ad-hoc signed bundle |
| Native file picker | Implemented; returns basename and size only, not file contents |
| Local vault creation and listing | Implemented |
| Password-based unlock and explicit lock | Implemented |
| Independent per-vault master keys | Implemented; keys remain in Rust |
| Encrypted vault display names | Implemented; hidden while locked |
| Encrypted file import, storage and export | Implemented for files up to 5 MiB; native picker only |
| Encrypted metadata database and crash recovery | **Not implemented** |
| Recovery keys, file sharing, LAN/cloud, durable audit | **Not implemented** |
| Developer ID signing, notarisation and native UI acceptance | Outstanding |

### What is protected today?

Each vault has an independently generated random 256-bit master key. A password-derived key (Argon2id v19; 19,456 KiB memory, 2 iterations, parallelism 1) wraps the master key and vault display name using AES-256-GCM, with a random nonce and vault-bound authenticated associated data. The versioned encrypted header, non-secret KDF information and separately authenticated per-file envelopes are persisted under Tauri's local application-data directory. Each imported file has a fresh random 256-bit DEK wrapped under the active vault master key; the filename and contents are encrypted together. One vault can be unlocked at a time; explicit locking releases the Rust-held master key with best-effort zeroisation. JavaScript receives only authorised display information.

**Important:** the original metadata-only file picker still only previews a file; use **Import and encrypt** inside an unlocked vault to store an authenticated encrypted copy. Export explicitly writes a new plaintext file and refuses to overwrite an existing destination. There is no password-recovery mechanism yet, so do not put irreplaceable data in this build. Best-effort memory clearing cannot protect an unlocked application from a compromised host, and authenticated encryption alone cannot detect rollback or deletion of every local copy.

## Run the desktop application

**Prerequisites:** macOS 13.3+, Apple command-line developer tools, Rust 1.90+ and Node.js 22.12+ with npm. Initial development targets Apple Silicon; Intel/universal builds need separate validation. No API, account, PostgreSQL, Docker or internet service is required to operate the desktop application.

From the repository root:

~~~bash
cd desktop
npm ci
npm run desktop:dev
~~~

A browser-only Vite preview cannot access native commands. Use the Tauri window for the picker and vault lifecycle.

Build the local macOS application:

~~~bash
cd desktop
npm run desktop:build
open "src-tauri/target/release/bundle/macos/Black Vault.app"
~~~

The local bundle is ad-hoc signed, **not** Developer ID signed or notarised for distribution. See [desktop/README.md](desktop/README.md) for setup, IPC contract and release notes.

## Verification

The latest desktop checks completed on 29 September 2026:

- **18 Rust tests passed** (including encrypted-header persistence, wrong passwords, tamper/cross-vault rejection, lock/reopen and shared Argon2id fixtures).
- **14 frontend tests passed** (including native-command contract, vault controls and file-picker behaviour).
- Rust formatting and strict Clippy checks, TypeScript type-check, frontend production build and native macOS app build passed.
- Native end-to-end manual acceptance remains outstanding; the application has not been notarised.

To rerun from desktop/:

~~~bash
npm run typecheck
npm test
npm run build
npm run test:rust
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run desktop:build
~~~

## Repository layout

~~~text
black-vault/
├── desktop/          Active offline-first Tauri + React + Rust application
├── backend/          Preserved legacy Rust/Axum implementation and crypto reference
├── frontend/         Preserved legacy Next.js web application
├── tests/fixtures/   Shared cryptographic compatibility vectors
└── docs/             Architecture, sprint records, engineering rules and history
~~~

The previous browser/server version supported authenticated vault workflows and version 2 client-encrypted file upload/retrieval (5 MiB limit). **That is historical web functionality, not a capability of the new desktop application.** The legacy applications and fixtures remain available for reference and regression checks; do not silently reinterpret their encryption envelopes or remove them during migration.

## Architecture and direction

The active desktop design separates the React interface from the Rust security boundary. Tauri capabilities permit only the main local window to invoke an explicit command allow-list. Rust owns the vault lifecycle, password derivation, master keys and local encrypted headers. No remote API is needed.

The longer-term direction is independently encrypted vault environments, per-file data-encryption keys, authenticated file/filename encryption, local encrypted metadata, robust atomic checkpoints, recovery and controlled sharing. The proposed in-memory SQLite approach must be tested for plaintext disk leakage before adoption. Independent keys within one process provide **cryptographic separation, not OS- or hypervisor-level isolation**.

### Next steps

1. Extend the current bounded in-memory file encryption to chunked/streaming import/export and strengthen crash/recovery tests.
2. Add durable encrypted metadata, recovery strategy and further negative security tests.
3. Evaluate encrypted metadata persistence and prove no plaintext SQLite temporary, journal or WAL leakage.
4. Manually test the native lifecycle across application restart, incorrect password and lock/unlock.
5. Design recovery and audit before production use; complete Developer ID signing/notarisation for distribution.

## Documentation

- [Desktop development and commands](desktop/README.md)
- [Project summary](docs/PROJECT_SUMMARY.md)
- [Desktop-first architecture](docs/Architecture.md)
- [Proposed desktop security architecture](docs/DESKTOP_SECURITY_ARCHITECTURE.md)
- [Sprint 01 — Desktop foundation](docs/SPRINT_01_DESKTOP_FOUNDATION.md)
- [Sprint 02 — Vault foundation](docs/SPRINT_02_VAULT_FOUNDATION.md)
- [Engineering and agent rules](docs/AGENTS.md)
- [Historical web-era encryption design](docs/Encryption_Architecture.md)
- [Historical project handover](docs/PROJECT_HANDOVER.md)

Development is ongoing. Treat security claims as bounded by implemented behaviour and verified tests, not by planned features.

### Sprint 03 — Native launch and vault sign-in (29/09/2026)
Run `./start.sh` from the repository root to launch the compiled macOS app (`./start.sh build` first if needed). `./start.sh dev` runs Tauri development; `./start.sh test` runs desktop checks; `./start.sh legacy-start` preserves the previous Docker workflow. On startup, a short animated splash precedes first-vault setup or password-based vault unlock. The dashboard appears only after vault authentication. Sign-out locks the Rust vault and clears the UI session. This is local vault authentication, **not** an online or separate multi-user account. See [Sprint 03](docs/SPRINT_03_AUTH_EXPERIENCE.md).
