# Sprint 01 — Desktop Foundation

Date: 28 September 2026  
Branch: `feat/desktop-migration`  
Starting commit: `f231e88`

## Result

A standalone Tauri 2 application scaffold is implemented in `desktop/`. A release macOS `.app` was built successfully for Apple Silicon and its ad-hoc signature passed `codesign --verify --deep --strict`.

**Interactive native verification remains blocked.** The computer-use tool returned `Computer Use permissions are not granted` when asked to launch/inspect the bundled app. Permission or a manual user check is required before calling the native window and picker fully verified. The Tauri development command did launch the debug executable without a reported runtime failure; that is not equivalent to visually verifying the window or completing a file selection.

## Completed work

- Inspected repository guidance, project handovers and the prior cryptographic compatibility report; confirmed the requested branch and a clean starting tree.
- Added independent npm and Cargo projects, with lockfiles, using Tauri 2.12, React 19, Vite 8, TypeScript and Tailwind CSS 4.
- Configured a native, resizable dark macOS window, application identity, icon and bundle settings. Minimum configured OS is macOS 13.3; only the current macOS 26.6.2 arm64 host was used for checks.
- Added a minimal responsive dashboard with native status, selected-file details, cancellation, error handling and clear-selection behaviour.
- Reused the existing `Card` and `cn` implementations and adapted the button styling to a native HTML button. No Next.js runtime or server authentication flow is embedded in the desktop application.
- Added typed `get_desktop_info` and `select_file` Tauri commands. The Rust file picker returns only a basename and byte count; it does not read file contents or persist a path.
- Limited command access to the main local webview. Production CSP is restrictive; development additionally permits local Vite/HMR and its injected inline scripts/styles. No generic filesystem, shell, HTTP or direct dialog-plugin commands are exposed to React.
- Added Rust metadata/serialisation and IPC authorisation tests, plus React/TypeScript tests for selection, cancellation, errors, clearing, filename escaping and the command boundary.
- Documented development, build, test, security and next-sprint steps in `desktop/README.md`.

No existing `backend/`, `frontend/` or shared crypto fixture files were changed. No PostgreSQL-to-SQLite migration, local vault implementation or file sharing was introduced. No commits or pushes were made.

## Validation

| Check | Result |
| --- | --- |
| Existing Rust suite, `cargo test --locked --quiet` | 92 passed |
| Existing frontend baseline, `npm test` | 15 passed; opt-in HTTP test skipped |
| Existing frontend with local API on port 18081 | 16 passed, including live HTTP and Rust↔TypeScript crypto exchange |
| Desktop frontend tests | 9 passed |
| Desktop Rust tests | 4 passed |
| Desktop TypeScript and Vite production build | Passed |
| Desktop Rust formatting | Passed |
| Desktop Clippy, all targets with warnings denied | Passed |
| Clean desktop npm install, `npm ci` | Passed |
| Tauri development command | Compiled and launched the debug process; stopped after checking startup output |
| Production `.app` build | Passed; arm64, approximately 10.23 MiB |
| macOS code signature verification | Passed; ad-hoc identity |
| Browser preview | Layout inspected; native-only actions disabled as intended |
| Native dashboard and file-picker interaction | Blocked by missing computer-use permissions; not claimed as passed |

During a regression run concurrent with the first release compilation, two existing crypto tests exceeded their five-second timeouts. The same unchanged suite passed after compilation completed. No crypto parameters, assertions or test files were weakened. Disposable live-API test accounts and their vault records were removed afterwards.

An initial npm dependency-resolution error was resolved by installing the current compatible Vitest release. The Rust IPC test initially expanded the macOS Info.plist embedding macro twice; a single shared context constructor corrected that compile error. Final tests include local-main-window success, other-window denial and remote-origin denial in the main window.

## Run and build

From the repository root:

```bash
cd desktop
npm ci
npm run desktop:dev
```

To create the standalone application:

```bash
npm run desktop:build
```

Output:

```text
desktop/src-tauri/target/release/bundle/macos/Black Vault.app
```

The bundle embeds the Vite assets. It does not depend on a running API, database, Docker or development server. It is ad-hoc signed for local testing, **not Developer ID signed or notarised**. Apple distribution signing and notarisation require the project's own credentials and a later distribution pass.

## Native verification still required

Once computer-use access is available, or manually in Finder:

1. Open `Black Vault.app` with Vite stopped.
2. Confirm the dashboard reports **Native desktop** and the local application version/platform.
3. Choose **Select a file**, cancel, and confirm cancellation leaves the dashboard usable.
4. Select the repository fixture `tests/fixtures/round-trip.txt`; confirm its basename and 104-byte size appear.
5. Clear the selection and select again. Confirm closing/reopening the app starts with no selection.
6. Resize the window and confirm the dashboard remains usable.

These steps validate the actual macOS WebKit and file-dialog path, beyond mocked IPC tests. They are the outstanding Sprint 01 acceptance check.

## Security review and remaining migration work

- The frontend cannot supply arbitrary paths to Rust. Metadata validation rejects directories/devices, inaccessible selections, non-Unicode filenames and sizes exceeding JavaScript's safe integer range. Rust owns the picker and prevents concurrent dialogs.
- File selections are ephemeral metadata. No plaintext file data, vault passwords or cryptographic keys are introduced into this scaffold.
- Selection events omit names and paths. They are process diagnostics, not a durable audit store.
- The command types are deliberately small and manually mirrored between Rust and TypeScript, with contract and IPC tests. Consider generated bindings if the command surface expands significantly.
- Sprint 02 should reuse/extract the existing Rust crypto core, preserve the shared deterministic vectors and keep decrypted keys in Rust-owned memory with explicit cleanup.
- Design local vault lifecycle and storage as a separate migration. Revalidate selected files when encryption actually opens them; this sprint's metadata is not a durable authorisation token.
- Do not copy server JWT/localStorage assumptions into local vault security. Define local access, recovery and persistence before implementing them.
- Intel/universal builds, earlier supported macOS versions, durable audit handling, distribution signing and notarisation remain unverified or future work.

Implementation references: [Tauri Vite integration](https://v2.tauri.app/start/frontend/vite/), [command capabilities](https://v2.tauri.app/security/capabilities/) and [native dialogs](https://v2.tauri.app/plugin/dialog/).
