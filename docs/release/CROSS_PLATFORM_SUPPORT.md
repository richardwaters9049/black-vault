# Cross-platform support

**Release target:** Black Vault 0.1.0
**Platforms:** macOS, Windows and Linux.

Black Vault uses Tauri 2 with a Rust core and React/Vite UI. This document records cross-platform engineering evidence for the portfolio milestone; it is not a public distribution/support commitment.

| Platform | Automated source/build gate | Native packaged GUI acceptance | Distribution |
| --- | --- | --- | --- |
| macOS Apple Silicon | CI tests and DMG packaging pass | Hands-on packaged GUI acceptance passed | Ad-hoc signed; public notarisation not pursued |
| Windows x64 | CI tests and NSIS packaging pass | Not claimed as hands-on accepted | Package retained as engineering evidence |
| Linux x64 | CI tests and AppImage/DEB packaging pass | Not claimed as hands-on accepted | Packages retained as engineering evidence |

The Rust storage implementation uses `#[cfg(unix)]` for Unix permission and `O_NOFOLLOW` hardening. Windows therefore follows portable Rust filesystem semantics but does **not** inherit Unix mode-bit protections. Windows ACL/reparse-point behaviour remains an explicit limitation if the project is ever reconsidered for public Windows distribution.

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

Do not present Windows or Linux CI/package success as equivalent to hands-on target-OS security acceptance. The project is not currently planned for public distribution.
