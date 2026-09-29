# Black Vault — macOS internal release candidate

Date: 29/09/2026. Build target: Tauri `.app` for the current host architecture, version 0.1.0.

Build from the repository root:

```bash
cd desktop
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run desktop:build
codesign --verify --deep --strict --verbose=2 'src-tauri/target/release/bundle/macos/Black Vault.app'
file 'src-tauri/target/release/bundle/macos/Black Vault.app/Contents/MacOS/black-vault-desktop'
```

The current `signingIdentity: "-"` produces an **ad-hoc signed** local build, not a notarised distributable. Do not submit or label it as publicly released. Test the packaged app with a disposable profile, not the development vault. Check the app launches without Vite, closes/locks cleanly, and survives relaunch. Inspect `codesign -dv --verbose=4` output before distribution.

Apple Developer ID signing/notarisation is a separate release gate requiring authorised credentials; do not store credentials in the repository. Do not change the bundle identifier or production storage path as part of a test build without a migration plan.

## Build verification (29/09/2026)

- Frontend: 21/21 tests passed; TypeScript/Vite production build passed.
- Rust: 45/45 library tests passed; Clippy passed with warnings denied.
- Packaged app: 10.82 MiB; arm64; identifier `com.blackvault.desktop`; version `0.1.0`; minimum macOS `13.3`.
- `codesign --verify --deep --strict` passed. Signature is ad-hoc, TeamIdentifier is unset, notarisation skipped because Apple credentials are not configured.
- Native GUI smoke test, clean-profile restore and Developer ID notarisation remain **unverified**.

## Sprint 10 close-out

The Sprint 10 code and automated checks are complete. The compiled application is an **internal release candidate**, not a public security-certified release. A real packaged-GUI smoke test and clean-profile backup/restore require human interaction with the macOS window and must not be represented as automated successes. Apple Developer ID credentials/notarisation and external security review remain release gates. The fault harness uses actual abrupt child-process exit (code 86), not SIGKILL or power-loss simulation.
