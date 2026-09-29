# Sprint 03 — Desktop startup and vault sign-in
**Date:** 29/09/2026
**Branch:** `feat/desktop-auth-experience`

## Implemented
- Root `./start.sh` defaults to opening the built macOS Black Vault.app bundle. `dev`, `build`, `test` and `help` target the desktop; `legacy-start`, `stop`, `restart`, `logs` retain Docker controls.
- Startup screen animates a geometric lock and progress stroke with a reduced-motion fallback. The UI waits for native initialisation and a brief brand reveal.
- On every webview mount, Rust's active vault is locked before the sign-in screen appears. Failure to lock fails closed and does not render the dashboard.
- First-time setup creates an encrypted vault using the existing Rust vault password mechanism. Returning users select a local vault and unlock it to enter the dashboard.
- Sign-out calls the Rust lock command before clearing the UI session and file selection. The existing dashboard Lock vault action also returns to sign-in. Incorrect credentials do not open the dashboard; password input clears after attempts.
- No online account, second password database, plaintext password persistence or new cryptographic format.

## Security boundary and limitations
This is **vault-password sign-in**, not a separate multi-user operating-system identity or independent application-account layer. Rust's existing vault authentication remains authoritative for decrypting files. UI route gating is not an independent IPC security boundary. Only one vault is unlocked at a time. The desktop webview must remain local and trusted. Automatic inactivity locking, OS sleep/window-hide handling, Touch ID, rate limiting and recovery are not yet implemented. A password cannot be recovered.

The current desktop bundle is ad-hoc signed, not Developer ID signed/notarised. Do not use for irreplaceable data.

## Manual acceptance
1. Run `./start.sh build` then `./start.sh`.
2. Confirm animation, first-time setup (or existing vault selection), and no dashboard before unlock.
3. Create or unlock a vault, import and export a small test file, then sign out.
4. Confirm the dashboard disappears; wrong password fails; correct password restores access.
5. Quit and relaunch: confirm the user must unlock again.
6. Check reduced-motion accessibility settings.

## Next
Native GUI acceptance, inactivity/sleep auto-lock, harden the sign-in/IPC boundary, improve file encryption streaming and crash recovery.
