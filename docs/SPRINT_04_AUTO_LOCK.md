# Sprint 04 — Development examples and inactivity lock
**Date:** 29/09/2026
**Branch:** `feat/desktop-auto-lock`

## Changes
- First-time creation fields display example placeholders: `Development Vault` and `DevVault-Only-2026!`. They are not submitted or stored until explicitly typed; the example password is public and must **never** protect important files. Existing-vault unlock does not display an example password.
- An authenticated session locks through the existing Rust `lock_vault` IPC command after five minutes of inactivity. Pointer, keyboard, wheel and touch activity refresh the clock; the check uses wall-clock time, including when JavaScript timers are delayed by macOS sleep. Focus also checks elapsed time before allowing renewed activity.
- The UI only returns to sign-in after Rust confirms the lock. A failed IPC lock retains the session and shows an error; the interval retries. No password is persisted in browser storage.
- Added timeout boundary tests and retained existing authentication/file tests.

## Limitations
This is frontend-managed inactivity locking, not a native OS sleep/lock listener. A suspended webview cannot execute Rust IPC until resumed. Native background operations or file pickers can affect event timing. Rust remains the authority for file access, but IPC calls are not yet bound to independent per-session credentials. Next security work: native sleep/window-lock handling, operation cancellation on lock, and native GUI acceptance.

## Manual macOS acceptance
1. Run `./start.sh build && ./start.sh` and use only a disposable development vault.
2. Confirm the example values appear as placeholders; typing and submitting a real password is still required.
3. Import a harmless small file, sign out, unlock, export and compare bytes.
4. Unlock again, leave the app idle for five minutes and verify sign-in returns; check a wrong password cannot re-enter.
5. Repeat after Mac sleep longer than five minutes; the lock should run when the webview resumes. Check the error message if native IPC is unavailable.
