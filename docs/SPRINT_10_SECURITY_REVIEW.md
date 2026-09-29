# Sprint 10 — Security review and macOS release candidate

**Review date:** 29/09/2026
**Branch:** `test/desktop-fault-injection`
**Status:** Internal testing candidate only; not an independently audited production release.

## Review of implemented controls

| Area | Observed implementation | Residual risk / required follow-up |
| --- | --- | --- |
| Master secret | Argon2id password derivation, random per-vault master key, AES-256-GCM authenticated envelopes; Rust active master key held in `Zeroizing` memory. | Rust and webview memory cannot be guaranteed absent from OS swap, crash reports or process snapshots. Consider OS-specific hardening and an external crypto review. |
| Nested PIN | Four-digit numeric validation, Argon2id wrapping, failed-attempt cooldown, parent-key wrapping and authentication before file access. | Four digits have limited entropy; PIN is not a replacement for the parent password. Review persistence of attempt throttling across app restarts and document offline threat model. |
| Filesystem | Owner-only permissions on vault directories and newly written files on Unix; bounded reads, regular-file checks and `O_NOFOLLOW` for encrypted reads; encrypted copy-first transfers with sync/rename. | Check all rename/destination symlink and race surfaces on every supported OS. Parent staging is conservatively preserved; no fully automatic recovery of unknown partial files. |
| Recovery | 13 checkpoints exercised via returned-error and separate child-process abrupt exit; 45 Rust regression tests at previous milestone; conflicting authenticated copies are preserved. | Abrupt exit is not SIGKILL or power-loss simulation. Hardware/fsync durability and simultaneous writers remain unproven. |
| UI / IPC | Scoped Tauri main-window command capability; restrictive production CSP; app boot requests Rust lock to fail closed; five-minute inactivity auto-lock. | Test rapid window close/reopen, focus transitions, suspend/resume, and clipboard/screenshot exposure manually. |
| Logging | Native file-selection audit intentionally redacts path/name; drag/drop development tracing exists in Rust and UI. | Remove or gate diagnostic tracing for release; never log raw exceptions, filesystem paths, filenames, PINs, passwords or plaintext. |
| Packaging | Tauri app target, macOS 13.3 minimum, identifier `com.blackvault.desktop`, signing identity `-` (ad-hoc). | Ad-hoc signing is not Developer ID signing/notarisation. Gate public distribution until trusted signing, notarisation and a clean-machine launch check. |

## Backup and recovery policy for testers

1. Quit and lock Black Vault before making a backup; do not copy a live vault during a transfer or edit.
2. Back up the **entire** application vault storage directory, including `.vault` headers, matching `.files` directories, nested `.bvlock` and encrypted `.bvfile` records. An individual encrypted file alone is insufficient for recovery.
3. Store backups in an independently protected location. Retain the master password securely; there is no password-reset bypass. A lost master password or damaged sole key header may make data unrecoverable.
4. Test restoration on a **separate disposable profile** first. Never merge divergent same-ID ciphertext or overwrite a `recovery_conflict` automatically.
5. Do not sync live vault folders between devices or allow concurrent writers; cross-device conflict resolution has not been designed or validated.
6. Before any real-data use, document tested backup/restore procedures and verify that OS backup software captures a quiescent, consistent snapshot.

## Release gates

- [x] Deterministic interruption and subprocess-exit regression coverage.
- [x] Conflict preservation and unknown parent-staging regression tests.
- [x] Redact raw errors in identified drag/drop diagnostic paths.
- [x] Compile-gate native Rust drag/drop diagnostic traces out of release binaries.
- [ ] Remove or compile-gate remaining frontend drag/drop console tracing before wider distribution; raw error objects are no longer logged by the reviewed paths.
- [x] Full Rust (45), frontend (21), TypeScript, Clippy and macOS app build pass for final tree.
- [ ] Native macOS .app smoke test: first setup, wrong password, unlock, import, nested PIN, both transfer directions, restore, sign-out and relaunch.
- [x] Verify ad-hoc package signature and architecture (arm64); confirm bundle ID, version and minimum macOS version.
- [ ] Inspect bundled assets and entitlements before distribution.
- [ ] Developer ID signing and Apple notarisation (required before public release; ad-hoc candidate only).
- [ ] Separate clean macOS account/machine test, including Gatekeeper, backup restore, sleep/wake and resize/accessibility.
- [ ] External security review before describing the application as production-grade.

**Do not distribute test vault data or release bundles containing actual credentials.**
