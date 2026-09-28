# Black Vault Desktop

Sprint 01 introduces a standalone macOS application using Tauri 2, React, Vite, TypeScript and Tailwind CSS. The existing `backend/`, `frontend/` and cryptographic compatibility suite remain unchanged.

This is the desktop foundation. It displays native application information and opens the macOS file picker. Selecting a file returns its name and size, without reading, copying, uploading or encrypting its contents. Selection exists only in memory and can be cleared. Local vault operations belong to Sprint 02.

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

Frontend tests exercise the real typed wrapper with Tauri's mocked IPC transport and render the dashboard through React DOM. They cover selection, cancellation, recovery from errors, clearing the selection, safe filename rendering and browser-only rejection. Rust tests cover serialisation, metadata validation and IPC permissions for local/remote origins and window labels. The native picker itself must also be exercised manually on macOS.

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

Failures are serialised as `{ code, message }`. A native picker supplies the path inside Rust; JavaScript cannot request metadata for arbitrary paths. Only the basename and byte count cross IPC. Rust validates that the selection is a regular file and that its size fits JavaScript's safe integer range. Blocking picker work runs off the main event loop; a Rust mutex prevents concurrent dialogs.

`build.rs` registers the application commands with Tauri's permission system. Only the local `main` window receives their permissions. The webview has no filesystem, shell, HTTP or direct dialog-plugin permissions. The Rust dialog plugin is invoked by the dedicated command. Production CSP blocks remote resources and network connections; development allows the loopback Vite/HMR connection in addition to Tauri IPC, plus the inline scripts and styles injected by Vite. CSP remains enabled in both modes.

Selection/cancellation/rejection events go to process diagnostics without filenames, paths or content. Durable audit storage is future work. No file data, keys, passwords, selection history or database records are persisted by the desktop scaffold.

## Sprint 02

1. Extract or reuse the existing Rust crypto core without coupling the desktop executable to Axum/PostgreSQL; retain the shared compatibility vectors.
2. Design local vault creation, unlock and lock commands with keys owned by Rust and explicit cleanup.
3. Design local persistence and migration separately. This sprint adds no SQLite or database migration.
4. Replace metadata-only selection with an authorised local encryption workflow; revalidate the file at the time it is opened.
5. Add end-to-end vault tests, durable audit handling, recovery behaviour and distribution signing.

No file-sharing functionality is included.

## References

- [Tauri's Vite integration](https://v2.tauri.app/start/frontend/vite/)
- [Tauri command capabilities](https://v2.tauri.app/security/capabilities/)
- [Native dialog plugin](https://v2.tauri.app/plugin/dialog/)
