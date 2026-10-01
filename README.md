# Black Vault

**Black Vault is an offline-first encrypted desktop vault built to keep sensitive local files behind a Rust-controlled security boundary.**

The active application is built with **Tauri 2, Rust, React 19, Vite, TypeScript and Tailwind CSS**. Cryptographic operations, vault keys and privileged filesystem operations stay behind the Rust/Tauri boundary; the React UI receives only authorised application data.

> **Portfolio milestone — v0.1.0:** the application is feature-complete for its intended portfolio scope, has passed a six-phase internal hardening programme, and builds/packages in CI for macOS arm64, Windows x64 and Linux x64. It is not being distributed as a consumer security product or presented as independently security-certified.

## What Black Vault does

- Creates independent local encrypted vaults protected by a master password.
- Imports files through the native picker or drag and drop and stores encrypted envelopes rather than plaintext copies.
- Supports nested PIN-protected vaults and bidirectional encrypted file transfers.
- Supports encrypted text creation, editing, renaming, deletion, export and restore workflows.
- Performs bounded offline import screening before encrypted persistence.
- Locks explicitly, on sign-out, and after five minutes of inactivity.
- Includes crash/recovery and fault-injection coverage around encrypted transfer and restore persistence boundaries.
- Requires no account, remote API, database server, Docker container or internet connection to operate.

The current import limit is **5 MiB per file**. The import scanner is deliberately a high-confidence offline policy gate, not antivirus.

## Security model

Each vault uses an independently generated 256-bit master key. Password-derived key material uses Argon2id and authenticated encryption uses AES-256-GCM. Imported files receive independent random data-encryption keys, with filenames and contents encrypted together. Active master-key material is held by Rust and released with best-effort zeroisation when the vault locks.

The security boundary and its limitations matter as much as the algorithms. Read the [as-built architecture](docs/architecture/Architecture.md), [desktop security architecture](docs/architecture/DESKTOP_SECURITY_ARCHITECTURE.md), [threat model](docs/security/THREAT_MODEL.md) and [v0.1.0 validation record](docs/release/v0.1.0-validation.md) before making security claims about the project.

Black Vault does **not** claim protection from a compromised host while unlocked, guaranteed physical power-loss durability, independent security certification, or recovery of a forgotten master password.

## Verification and platform status

The current automated release gate runs on **macOS arm64, Windows x64 and Linux x64**. It executes the frontend suite, production frontend build, Rust tests, strict Clippy checks and a native Tauri package build.

Current automated baseline:

- **46/46 Rust library tests pass.**
- **22/22 frontend tests pass.**
- macOS arm64 DMG packaging passes.
- Windows x64 NSIS packaging passes.
- Linux x64 AppImage and DEB packaging pass.
- Packaged macOS GUI acceptance has passed.

Cross-platform CI/package success demonstrates build portability; it is not represented as equivalent to hands-on security acceptance on every operating system. The [v0.1.0 validation record](docs/release/v0.1.0-validation.md) captures the exact evidence and claim boundaries.

## Run and verify locally

**Desktop prerequisites:** Rust 1.90+, Node.js 22.12+ and npm, plus the native Tauri prerequisites for your operating system.

From the repository root:

~~~bash
./start.sh dev
~~~

Useful project commands:

~~~bash
./start.sh test
cd desktop
npm ci
npm test
npm run build
npm run test:rust
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run desktop:build
~~~

`start.sh` is the active desktop development convenience; it does not start Docker, PostgreSQL, the historical API or the historical web client. End users run the native Tauri package. See [desktop/README.md](desktop/README.md) for desktop development and IPC details.

The preserved web-era stack has its own entry point at [`legacy/`](legacy/README.md). It is retained for historical and cryptographic compatibility work only.

## Repository layout

~~~text
black-vault/
├── desktop/          Active Tauri + React + Rust desktop application
├── legacy/           Entry point and guidance for the preserved web-era stack
├── backend/          Preserved legacy Rust/Axum implementation and crypto reference
├── frontend/         Preserved legacy Next.js web application
├── tests/fixtures/   Architecture-neutral cryptographic compatibility vectors
└── docs/             Architecture, sprint, release and engineering documentation
~~~

The web frontend/backend are historical reference implementations, **not** the active Black Vault product. Normal development, testing and packaging should use `desktop/` and the root `start.sh`.

## Documentation

The [documentation index](docs/README.md) is the best starting point for detailed project records.

| Topic | Document |
| --- | --- |
| Project overview | [Project summary](docs/project/PROJECT_SUMMARY.md) |
| Desktop architecture | [Architecture](docs/architecture/Architecture.md) |
| Security design | [Desktop security architecture](docs/architecture/DESKTOP_SECURITY_ARCHITECTURE.md) |
| Threat model | [Threat model](docs/security/THREAT_MODEL.md) |
| Final engineering evidence | [v0.1.0 validation](docs/release/v0.1.0-validation.md) |
| Platform support | [Cross-platform support](docs/release/CROSS_PLATFORM_SUPPORT.md) |
| Engineering rules | [AGENTS.md](docs/AGENTS.md) |
| Sprint history | [Historical engineering records](docs/README.md#historical-engineering-records) |
| Recovery/fault testing | [Sprint 10 fault injection](docs/sprints/sprint-10/SPRINT_10_FAULT_INJECTION.md) |
| Security review | [Sprint 10 security review](docs/sprints/sprint-10/SPRINT_10_SECURITY_REVIEW.md) |

### Sprint progression

Sprints 01–10 document the move from the desktop foundation and vault lifecycle through authentication, auto-lock, offline import screening, nested vaults, encrypted file management, bidirectional transfers, recovery and release hardening. Rather than duplicating changing implementation detail here, use the [historical engineering records](docs/README.md#historical-engineering-records) for the historical progression and the [v0.1.0 validation record](docs/release/v0.1.0-validation.md) for the current state.

## Release position

Black Vault 0.1.0 is now treated as a **completed security-engineering portfolio project**. Cross-platform automated CI and native packaging are green, and the application has completed hands-on GUI acceptance on macOS. Public distribution is not planned, so production signing/notarisation is intentionally outside the current scope.

Development records describe the state at the time they were written. When an older sprint document conflicts with the current implementation, treat the [as-built architecture](docs/architecture/Architecture.md), [threat model](docs/security/THREAT_MODEL.md) and [v0.1.0 validation record](docs/release/v0.1.0-validation.md) as authoritative.
