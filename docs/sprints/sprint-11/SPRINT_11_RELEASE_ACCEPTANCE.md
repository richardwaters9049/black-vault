# Sprint 11 — release acceptance and scope

**Date:** 4 October 2026. **Candidate:** 0.2.0-rc.1.

## Engineering work

- Reconciled the candidate scope with actual native IPC and UI.
- Restricted Touch ID to native destructive authorisation; removed misleading standalone verification UI/IPC.
- Added fail-closed biometric preflight and false-success tests.
- Kept cryptographic vault unlock password-based, with no development password prefill.
- Included centred splash branding and existing panel/reduced-motion behaviour.
- Executed the local frontend/Rust/release gate and isolated credential-store acceptance where available; exact outcomes live in the validation record.
- Created an explicit packaged-app acceptance checklist covering lifecycle, nested/file flows, contacts, clipboard, biometrics, accessibility and backup/restore.

## Acceptance boundary

The engineering scope is complete when the automated gate is recorded. User-present Touch ID and hands-on target-OS GUI acceptance remain separate pending evidence; they are not ticked automatically. Biometric vault unlock and cross-user send/receive are deliberately deferred features rather than unfinished promises for this release.

See [validation](../../release/v0.2.0-validation.md) and [manual checklist](../../release/v0.2.0-manual-acceptance.md).
