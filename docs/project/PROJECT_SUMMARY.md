# Black Vault — Project Summary

**Updated:** 4 October 2026. **Candidate:** 0.2.0-rc.1. **Latest maintenance branch:** `chore/repository-audit`.

## Current product

Black Vault is an offline Tauri 2 / Rust / React / TypeScript desktop vault. Normal operation requires no account, API, PostgreSQL, Docker or network service. `desktop/` is the active application; `backend/` and `frontend/` retain stable paths as legacy implementations and cryptographic compatibility references. See the [root README](../../README.md) and [as-built architecture](../architecture/Architecture.md).

Implemented functionality includes password-protected independent local vaults, bounded native picker/drop imports, encrypted names and contents, parent/nested text management, explicit export, deletion, encrypted transfers and crash recovery. Nested four-digit PINs are additionally protected by the parent key; they are convenience compartments, not operating-system isolation.

Startup and five-minute inactivity request native locking. Protected views clear immediately when locking starts; late IPC responses cannot restore plaintext or filenames. Native success is required to report a locked session. Touch ID is confined to permanent locked-vault deletion when available and does not unwrap vault keys.

Public identities and contact fingerprint management are implemented. Private transfer identities use OS credential storage without a plaintext fallback. Public contact JSON is outside encrypted vault storage and is not authenticated against arbitrary valid local replacement.

## Transfer and future scope

Rust recipient-encryption, signature and bounded-parser primitives are tested foundations. End-user `.blackvault` package send/receive, package persistence, received-vault UI and durable replay history remain deferred. See the [implementation status](../architecture/SECURE_SHARING_IMPLEMENTATION_PLAN.md). Streaming/chunked storage, authenticated contact-state protection and OS/process isolation remain separate future design work. No forgotten-password recovery or guaranteed memory erasure is claimed.

## Latest maintenance and verification

The [4 October audit](../engineering/CODEBASE_AUDIT_2026-10-04.md) records security/locking and error-handling fixes, concise code comments, verified unused-code/assets/dependency removal and legacy maintenance. Local results total 263 passing tests: 45 desktop UI, 94 desktop Rust, 97 legacy Rust and 27 legacy frontend, including the live disposable API round trip. One native acceptance test remains ignored. TypeScript, ESLint, Rustfmt, strict Clippy, builds and ad-hoc macOS codesign verification passed.

Desktop dependency warnings and legacy development/optional-lockfile findings remain documented; this is not a claim of zero technical debt or independent security certification. Disposable test resources were removed. Archived release packages and dated sprint/release evidence are preserved.

## Release acceptance and history

The [candidate validation record](../release/v0.2.0-validation.md) distinguishes local and cross-platform automated evidence from manual OS/credential/GUI acceptance. Current maintenance has local macOS evidence; it does not establish new Windows/Linux packaging or human-present Touch ID acceptance. Public Developer ID signing and notarisation remain outstanding. See the [manual checklist](../release/v0.2.0-manual-acceptance.md).

Historical milestones remain in the [Sprint 01 record](../sprints/sprint-01/SPRINT_01_DESKTOP_FOUNDATION.md), [sprint index](../README.md) and [v0.1.0 validation](../release/v0.1.0-validation.md). Their counts and scope do not describe the current application. Shared vectors in `tests/fixtures/crypto-v1.json` and bidirectional compatibility tests remain preserved; desktop and web formats are not interchangeable.

## Development

From the repository root (`/Users/richy/Documents/Github/cyber-work/black-vault` on the development Mac):

```bash
./start.sh       # Compiled macOS application
./start.sh dev   # Explicit native development mode
```

Use the [desktop guide](../../desktop/README.md) for build/test commands and prerequisites. Legacy database/API tests require disposable PostgreSQL and must not target production. Engineering rules are in [AGENTS.md](../AGENTS.md).
