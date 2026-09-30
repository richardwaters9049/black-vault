# Cross-platform support

**Release target:** Black Vault 0.1.0
**Platforms:** macOS, Windows and Linux.

Black Vault uses Tauri 2 with a Rust core and React/Vite UI. Platform support is a release gate, not an assumption: a platform is supported only after its native CI build and a native packaged-app acceptance run pass.

| Platform | Automated source/build gate | Native packaged GUI acceptance | Distribution |
| --- | --- | --- | --- |
| macOS Apple Silicon | Local tests/build pass; CI added | Existing native acceptance plus final RC walkthrough required | Ad-hoc internal build; Developer ID/notarisation outstanding |
| Windows x64 | GitHub Actions gate added | Required on Windows before support claim | Signing/installer release work outstanding |
| Linux x64 | GitHub Actions gate added | Required on representative Linux desktop before support claim | Package format/repository distribution work outstanding |

The Rust storage implementation uses `#[cfg(unix)]` for Unix permission and `O_NOFOLLOW` hardening. Windows therefore follows portable Rust filesystem semantics but does **not** inherit Unix mode-bit protections. Windows filesystem ACL/reparse-point behaviour requires a dedicated security review before public Windows release.

The root `start.sh` is a macOS/Unix development convenience and is not the product launcher. End users launch the platform-native Tauri application. Windows development should use the npm/Tauri commands from `desktop/package.json`.

## Native acceptance matrix

On **each** operating system, use disposable data and verify:

1. First launch and vault creation.
2. Wrong master password rejected; correct password unlocks.
3. Native file picker and drag/drop import.
4. Encryption/decryption without plaintext persistence inside vault storage.
5. Nested vault creation, wrong/correct PIN, parent → nested and nested → parent transfer.
6. Restore and conflict-preservation behaviour.
7. Sign-out/relaunch and five-minute inactivity lock.
8. Window resize/minimum size, keyboard navigation and reduced-motion behaviour.
9. Back up the entire quiescent vault storage, restore into a disposable profile, relaunch and unlock.
10. Installer/package launch through the OS security mechanism.

Do not mark Windows or Linux as supported solely because compilation succeeds. Native execution, filesystem behaviour and packaging must pass on the target OS.
