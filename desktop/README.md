# Black Vault Desktop

Sprint 01 introduced a standalone macOS application using Tauri 2, React, Vite, TypeScript and Tailwind CSS. Sprint 02 now adds the first offline local-vault lifecycle: create, list, unlock and lock. The existing `backend/`, `frontend/` and cryptographic compatibility suite remain unchanged.

Vault headers are authenticated and encrypted locally. Each vault gets a separate random 256-bit master key, wrapped using a password-derived key. Display names are encrypted and hidden while locked. The original metadata-preview picker still returns metadata only. The separate **Import and encrypt** action in an unlocked vault now stores authenticated encrypted files (5 MiB per file), and **Export** explicitly creates a new plaintext file without overwriting an existing destination. SQLite metadata, account recovery and sharing are not implemented yet.

## Prerequisites

- macOS with Apple command-line developer tools (`xcode-select --install` if missing).
- Rust 1.90 or newer, installed through rustup.
- Node.js 22.12 or newer and npm; `.nvmrc` pins the version used for this sprint.
- No running API server, PostgreSQL or Docker is needed for the desktop application.

The bundle targets macOS 13.3 and newer. This sprint is tested on the host's Apple Silicon architecture; an Intel/universal build requires the corresponding Rust targets and separate validation.

## Development

Run from `desktop/`:

```bash
npm ci
npm run desktop:dev
```

The Tauri CLI starts Vite on `127.0.0.1:1420`, compiles Rust and launches the native window. The port is fixed so a collision fails visibly. Rust changes rebuild the application; React changes use hot reloading. The first Rust build downloads and compiles the macOS dependencies and takes longer.

`npm run dev` alone opens a browser preview. It cannot open the native picker; the dashboard reports that a desktop application is required.

## Production application

```bash
npm run desktop:build
```

This type-checks and bundles the React application, compiles Rust in release mode and creates:

```text
src-tauri/target/release/bundle/macos/Black Vault.app
```

Launch that bundle from Finder. It includes the built frontend and does not require Vite, Node.js or the existing backend to run.

The configuration uses an ad-hoc signature for local testing. This is not a Developer ID signed or notarised distribution. Before distribution, confirm the bundle identifier, configure your own signing identity and notarisation credentials, and validate the supported macOS versions. No signing credentials are stored in this repository.

## Checks

```bash
npm test
npm run typecheck
npm run build
npm run test:rust
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
```

Frontend tests exercise the typed wrapper with Tauri's mocked IPC transport and render the dashboard and vault controls. They cover selection, cancellation, errors, safe filename rendering, creation, locking, unlocking and browser-only rejection. Rust tests cover metadata validation, IPC permissions, local encrypted vault persistence, wrong passwords, tampering, independent vaults, invalid inputs and the shared Argon2id compatibility vectors. The native picker and complete lifecycle must also be exercised manually on macOS.

Run the original suites separately from the repository root:

```bash
(cd backend && cargo test --locked)
(cd frontend && npm test)
```

Those existing suites require their original prerequisites; the live HTTP test additionally requires `BLACK_VAULT_TEST_API` and a running backend. Avoid running the CPU-intensive crypto tests during a fresh release compilation: their existing five-second timeouts can expire under load. No crypto parameters or tests were relaxed for the desktop migration.

## Structure

```text
desktop/
├── src/App.tsx                   Dashboard and selection state
├── src/components/VaultWorkspace.tsx  Create, list, unlock and lock UI
├── src-tauri/src/vault.rs        Per-vault encryption and local lifecycle
├── src/components/ui/           Adapted existing UI components
├── src/lib/native.ts            Typed invoke boundary and error presentation
├── src-tauri/src/commands.rs    Native status, picker and metadata validation
├── src-tauri/capabilities/      Main-window command allow-list
├── src-tauri/tauri.conf.json    macOS window, CSP and bundle configuration
├── assets/icon.svg             Editable application icon source
├── package-lock.json           Locked JavaScript dependencies
└── src-tauri/Cargo.lock         Locked Rust dependencies
```

`Card` and `cn` are copied from the existing frontend. `Button` adapts the existing styling with native HTML button semantics, avoiding a second UI primitive dependency. These files are intentionally local copies so this scaffold builds independently; a shared UI package can follow when both applications need joint changes. The existing Next.js routing, authentication and server-dependent dashboard are not pulled into this desktop foundation.

## Command contract and security boundary

The Rust command return types use camelCase serialisation, mirrored by the TypeScript `Commands` mapping in `src/lib/native.ts`:

| Command | Arguments | Success result |
| --- | --- | --- |
| `get_desktop_info` | None | `{ appName, version, platform, architecture }` |
| `select_file` | None | `{ name, sizeBytes }`, or `null` on cancellation |
| `list_vaults` | None | Array of `{ id, name, isLocked }`; name is `null` when locked |
| `create_vault` | `{ name, password }` | Newly created and unlocked vault summary |
| `unlock_vault` | `{ vaultId, password }` | Unlocked vault summary or structured error |
| `lock_vault` | None | Releases active in-memory master key |
| `import_vault_file` | None; native picker | Encrypted file summary, or null on cancellation |
| `list_vault_files` | None; requires unlocked vault | Decrypted file summaries, with no plaintext contents |
| `export_vault_file` | `{ fileId }`; native save picker | Summary of exported plaintext file, or null on cancellation |

Failures are serialised as `{ code, message }`. A native picker supplies the path inside Rust; JavaScript cannot request metadata for arbitrary paths. Only the basename and byte count cross IPC. Rust validates that the selection is a regular file and that its size fits JavaScript's safe integer range. Blocking picker work runs off the main event loop; a Rust mutex prevents concurrent dialogs.

`build.rs` registers the application commands with Tauri's permission system. Only the local `main` window receives their permissions. The webview has no filesystem, shell, HTTP or direct dialog-plugin permissions. The Rust dialog plugin is invoked by the dedicated command. Production CSP blocks remote resources and network connections; development allows the loopback Vite/HMR connection in addition to Tauri IPC, plus the inline scripts and styles injected by Vite. CSP remains enabled in both modes.

Selection/cancellation/rejection events go to process diagnostics without filenames, paths or content. Durable audit storage is future work. Black Vault's vault storage persists authenticated encrypted headers and file envelopes, not plaintext file data, raw keys or passwords. A user-authorised export intentionally writes a new plaintext file at the chosen destination; protect or delete that exported copy separately.

## Sprint 02

**Implemented foundation:** Standalone Rust cryptography uses the same canonical Argon2id parameters and shared key-derivation test fixtures as the existing backend: v19, 19,456 KiB, 2 iterations, parallelism 1, 16-byte random salt, 32-byte KEK. Each vault receives a fresh random 32-byte master key; AES-256-GCM wraps it together with the display name, using a random 12-byte nonce and domain-separated vault-ID AAD. The disk header contains the version, opaque random ID, KDF parameters and authenticated ciphertext. The vaults reside under Tauri's application-local data directory, not in the source tree. On Unix, the vault directory is set to mode `0700` and new header files to `0600`; new headers are written using a create-new temporary file, sync, same-directory rename and directory sync. Only one vault can be unlocked per application session. Rust retains the key in zeroising memory, and locking drops it; JavaScript never receives a key. Locked names remain encrypted. This is best-effort process-memory clearing, not protection from malware controlling an unlocked host.

**Sprint 02 file storage delivered:** Independent random per-file 256-bit DEKs, wrapped under the active vault master key using AES-256-GCM with domain-separated vault/file AAD. The encrypted filename and bounded contents are stored together in a versioned authenticated envelope beneath the vault's private local directory. Rust revalidates the native-picked regular file (rejecting symlinks) and caps import at 5 MiB; exports use a native save picker and create-new semantics. The source is never deleted. **Remaining work:** design encrypted metadata and crash recovery; exercise the native application manually (create, quit, restart, wrong password, unlock, lock); add file-access and lifecycle regression tests. This implementation reads bounded files in memory; it is not streaming encryption. There is no SQLite database, account recovery, audit chain or sharing functionality in this slice. Losing a vault password currently means losing access; do not put irreplaceable data in this foundation build.

## References

- [Tauri's Vite integration](https://v2.tauri.app/start/frontend/vite/)
- [Tauri command capabilities](https://v2.tauri.app/security/capabilities/)
- [Native dialog plugin](https://v2.tauri.app/plugin/dialog/)

### Sprint 03 — Startup and sign-in
From the repository root use `./start.sh` to open the compiled app, `./start.sh build` to rebuild, `./start.sh dev` for Tauri development, and `./start.sh test` for desktop checks. Startup locks any previously open Rust vault before displaying a geometric splash and the vault-password sign-in screen. First-time setup creates a vault; subsequent launches require its password. Sign-out locks the Rust vault before hiding the dashboard. No separate account or online authentication is implemented. See `../docs/SPRINT_03_AUTH_EXPERIENCE.md` for acceptance and limitations.

### Inactivity lock
The React desktop session calls Rust `lock_vault` after five minutes without keyboard, pointer, wheel or touch activity. Wall-clock checking detects a delayed interval after sleep/resume, but this is not a native OS sleep listener. First-time setup includes development-only example placeholders, never prefilled credentials. See `../docs/SPRINT_04_AUTO_LOCK.md`.

### Blue/black identity and password visibility
`src/components/VaultMark.tsx` is the shared scalable SVG brand mark used by the splash, authentication and sidebar. Splash motion is CSS-based and honours reduced-motion preferences. First-time vault creation includes a masked-by-default Show/Hide password button; toggling it does not persist or change the password. Avoid using the public development example password with real files.

Splash polish: the large vault mark now completes a perspective Y-axis spin with a small overshoot during the 2.8-second startup reveal. The progress stroke tracks the longer reveal. Reduced-motion disables the spin; native lock/desktop initialisation still gates navigation.

Spin refinement: the splash logo now uses a single fast 0.95-second 2D revolution rather than a slow perspective Y-axis flip and overshoot, preventing the planar SVG from visually disappearing/snapping at the end. The overall startup reveal remains 2.8 seconds; reduced motion is unchanged.

### Pre-import screening
`src-tauri/src/import_scan.rs` runs in Rust after the native picker and before encryption/persistence. It blocks selected executable/script/macro/archive formats and known high-risk signatures/patterns. The scan is offline and bounded to 5 MiB. A pass is not antivirus clearance; the source remains unchanged. See `../docs/SPRINT_04_IMPORT_SCREENING.md` for threat model, tests and limitations.
