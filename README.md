# Black Vault

**Black Vault is an offline encrypted desktop vault with a Rust-controlled security boundary.** It uses Tauri 2, Rust, React, TypeScript, Vite and Tailwind CSS. Normal operation needs no account, API server, database server, Docker or internet connection.

**Current candidate: 0.2.0-rc.1.** The v0.1.0 portfolio release remains a historical milestone; its validation record is preserved. This candidate adds identities, trusted contacts, native destructive-action authentication and the redesigned workspace. It is an internal portfolio candidate, not a consumer security product or independently certified software.

## Implemented features

- Independent password-protected local vaults with random master keys.
- Native picker/drop import, encrypted filenames and contents, and a bounded offline policy gate.
- Parent and nested vault text creation/editing, export, deletion and bidirectional encrypted transfers.
- Four-digit nested PINs protected additionally by the parent vault key.
- Explicit locking, startup locking and five-minute inactivity locking, with immediate closure of protected views and rejection of stale responses.
- Atomic encrypted persistence, recovery and interruption regression tests.
- Public identity documents and independently verified contact fingerprints, with unverified, verified, key-changed and revoked states.
- OS credential storage for private identity material; no plaintext fallback.
- Native macOS Touch ID confirmation for permanent locked-vault deletion when available. Passwords still unlock vaults; Touch ID does not unwrap vault keys.
- Centred startup branding, motion within operation panels, keyboard controls and reduced-motion support.

Files are limited to **5 MiB** for local import. The import policy gate is not antivirus. Plaintext exports are explicit user-authorised copies outside vault storage.

## Transfer feature boundary

Recipient encryption, sender signatures, bounded package parsing and negative/vector tests exist in Rust. **The application does not yet send or receive `.blackvault` packages.** Package file export/import, received-vault UI, durable replay tracking and cross-platform transfer workflows remain a separate future feature. Trusted Contacts is an identity-management feature in this release.

## Verification

The [4 October maintenance audit](docs/engineering/CODEBASE_AUDIT_2026-10-04.md) records **263 passing local tests**, type/lint checks and a rebuilt ad-hoc signed macOS application, with remaining dependency findings explicitly tracked.

The local macOS candidate gate covers frontend tests/build, Rust tests, Rustfmt, strict Clippy, dependency audits and native application packaging. The dated [v0.2.0 validation record](docs/release/v0.2.0-validation.md) records exact results and unresolved acceptance items.

Routine PRs and `main` pushes use **Linux validation only**, without native packaging. `release/**` pushes and manual dispatch run macOS arm64, Windows x64 and Linux x64 tests, native credential-store checks where configured, and packaging. Obsolete runs are cancelled. Do not use the full matrix for every development change.

[Cross-platform support](docs/release/CROSS_PLATFORM_SUPPORT.md) distinguishes historical v0.1.0 evidence, current candidate results and hands-on OS acceptance. Build success alone does not prove GUI or security acceptance on another OS.

## Security model

Argon2id derives password wrapping material; AES-256-GCM protects encrypted local records. Each vault has a random 256-bit master key and files have independent random data-encryption keys. Keys and privileged filesystem operations stay in Rust. Authorised text editing necessarily exposes plaintext to the WebView in memory.

Public contact names, keys and trust state are stored outside the encrypted vault and are not an authenticated defence against arbitrary local-state tampering. Private transfer identities live in the OS credential store and are separate from vault passwords and master keys.

Read the [architecture](docs/architecture/Architecture.md), [security architecture](docs/architecture/DESKTOP_SECURITY_ARCHITECTURE.md), [threat model](docs/security/THREAT_MODEL.md) and [biometric model](docs/security/BIOMETRIC_SECURITY_MODEL.md). There is no forgotten-password recovery, independent security certification, rollback protection against replacement of all local state, guaranteed memory erasure, or protection from a compromised unlocked host.

## Run locally

**Directory:** repository root (`/Users/richy/Documents/Github/cyber-work/black-vault` on the development Mac).

```bash
./start.sh       # Launch the compiled macOS application
./start.sh dev   # Explicitly start native development mode
```

A local development environment file does not change the default compiled-app launch.

**Directory:** `desktop/` beneath that root. Requires Rust 1.90+, Node.js 22.12+ and the OS-specific Tauri prerequisites.

```bash
npm ci
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run desktop:build
```

The compiled macOS app is `desktop/src-tauri/target/release/bundle/macos/Black Vault.app`. It is ad-hoc signed and not notarised for public distribution.

## Repository and documentation

| Path | Purpose |
| --- | --- |
| `desktop/` | Active Tauri, React and Rust application |
| `docs/` | Current architecture, security, release evidence and historical sprints |
| `tests/fixtures/` | Shared cryptographic compatibility vectors |
| `legacy/`, `backend/`, `frontend/` | Preserved web-era reference implementation |

Start with the [documentation index](docs/README.md) and [desktop development guide](desktop/README.md). The final sprint records are [acceptance and scope](docs/sprints/sprint-11/SPRINT_11_RELEASE_ACCEPTANCE.md) and [release preparation](docs/sprints/sprint-12/SPRINT_12_RELEASE_PREPARATION.md). [v0.1.0 validation](docs/release/v0.1.0-validation.md) is historical evidence and does not certify newer functionality.

The [4 October codebase audit](docs/engineering/CODEBASE_AUDIT_2026-10-04.md) records security/locking fixes, unused-code removal, legacy maintenance and current local checks.
