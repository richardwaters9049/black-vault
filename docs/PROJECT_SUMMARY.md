# Black Vault — Project Summary

**Updated:** 29 September 2026  
**Active branch:** `feat/desktop-migration`  
**Direction:** Offline-first, standalone cybersecurity desktop application.

## Overview

Black Vault is transitioning from a hosted Rust/Axum/PostgreSQL and Next.js encrypted-file application to a local-first desktop application using Tauri 2, React, Vite, TypeScript and Rust. macOS Apple Silicon is the initial target; Windows and Linux are future distribution targets. The desktop app must not require an account, API server, PostgreSQL, Docker or cloud service to perform its core vault operations.

The existing `backend/` and `frontend/` are preserved as working legacy implementations and cryptographic compatibility references. Do not delete, overwrite or destructively migrate their data. The authoritative desktop design is `DESKTOP_SECURITY_ARCHITECTURE.md`; older `Encryption_Architecture.md` documents the web-era design.

## Verified implementation at Sprint 01

- `desktop/` contains Tauri 2, React 19, Vite, TypeScript, Tailwind CSS and independent npm/Cargo lockfiles.
- A native dark dashboard, typed `get_desktop_info` and `select_file` commands, restricted main-window IPC and native macOS picker are implemented.
- The picker returns only filename and size, without file contents or persistent path.
- A release Apple Silicon `Black Vault.app` was built and its ad-hoc signature verified. This is not notarised distribution.
- Sprint 01 reported 92 existing Rust tests, 16 frontend tests with local API, nine desktop frontend tests and four desktop Rust tests passing.
- Native visual interaction and picker acceptance remain outstanding: computer-use permission prevented the previous agent from inspecting the bundled app. These results are historical, not a fresh test run on 29 September.
- No desktop vault, local database, encrypted desktop file storage, sharing, durable audit or isolated-process engine exists yet.

## Preserved cryptographic work

The legacy implementation has shared Rust/TypeScript deterministic vectors in `tests/fixtures/crypto-v1.json` and bidirectional interoperability tests. Canonical Argon2id uses a decoded 16-byte standard Base64 salt, version 19, memory 19456 KiB, two iterations, parallelism one and 32-byte output. The explicitly named legacy salt-text derivation is for provenance-verified recovery only, never automatic selection. AES-256-GCM uses fresh 12-byte nonces in production.

The web implementation supports browser vault setup/unlock/lock and version 2 encrypted file uploads, downloads and owner-scoped management, with a 5 MiB plaintext limit. Its file format and AAD domain strings must remain compatible. The web implementation's JWT, browser storage, API and PostgreSQL assumptions must not be copied into the desktop session model.

## Desktop product direction

1. Independent local vaults with separate vault master keys, metadata and lifecycle.
2. Rust-owned secrets and authorised Tauri commands; React is a presentation layer.
3. Encrypted persistent vault containers and file blobs; investigate authenticated metadata materialised into an SQLite in-memory connection while unlocked.
4. Atomic authenticated encrypted checkpoints after successful mutations, plus crash recovery and backup design. Never depend solely on a final save when locking.
5. A separate device/recipient cryptographic identity, distinct from the vault master key.
6. Later: versioned `.bvault` sharing packages, public-key recipient wrapping, Security Centre, durable tamper-evident audit, LAN transfer and optional encrypted relay.

Independent keys provide cryptographic compartmentalisation, not Qubes-style operating-system isolation. Separate restricted processes are a possible later security enhancement. Do not claim perfect memory erasure, resistance to a compromised unlocked host or rollback protection without an authenticated external baseline.

## Immediate work — Sprint 02

First complete the native release-app acceptance check in `SPRINT_01_DESKTOP_FOUNDATION.md`. Then extract/reuse Rust crypto without Axum/PostgreSQL coupling; document and prototype local vault creation, unlock, lock, independent key ownership, ephemeral metadata, encrypted persistence and crash-safe checkpointing. Preserve and run all existing compatibility tests. Obtain an architecture review before committing to a persistent format. Sharing, LAN, online relay and full file encryption are outside the initial prototype.

## Development commands

From repository root:

```bash
cd desktop
npm ci
npm run desktop:dev
npm test
npm run typecheck
npm run build
npm run test:rust
npm run desktop:build
```

Legacy regression suites: `(cd backend && cargo test --locked)` and `(cd frontend && npm test)`. Database tests require the original disposable PostgreSQL prerequisites. Never run the opt-in live API test against production.

**Engineering:** UK English; no automatic commit or push; never commit secrets; do not weaken tests or crypto to satisfy a build. See `AGENTS.md`, `DESKTOP_SECURITY_ARCHITECTURE.md` and `SPRINT_01_DESKTOP_FOUNDATION.md`.
