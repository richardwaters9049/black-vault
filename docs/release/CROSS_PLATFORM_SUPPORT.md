# Cross-platform support

**Updated:** 4 October 2026. **Current candidate:** 0.2.0-rc.1.

| Platform | Historical v0.1.0 evidence | Current candidate evidence | Current hands-on acceptance |
| --- | --- | --- | --- |
| macOS arm64 | CI/DMG and packaged GUI passed | Local gate, Keychain and app build/signature passed; release CI pending | Pending current-candidate walkthrough |
| Windows x64 | CI tests/NSIS passed | Release CI/credential acceptance pending | Not claimed |
| Linux x64 | CI tests/AppImage/DEB passed | Release CI pending; Secret Service requires an active desktop session | Not claimed |

The previous release remains documented in [v0.1.0 validation](v0.1.0-validation.md). Current results live in [v0.2.0 validation](v0.2.0-validation.md), with a [manual acceptance checklist](v0.2.0-manual-acceptance.md). Historical results do not automatically validate newer code.

## CI policy

PRs and `main` use one Ubuntu runner for frontend tests/build, Rust tests, formatting and strict Clippy. No native package is built there. `release/**` pushes and manual dispatch run macOS arm64, Windows x64 and Linux x64 package jobs; superseded runs are cancelled. Artifacts use the tested commit SHA in their names, preventing stale release-version labels.

macOS Keychain and Windows Credential Manager acceptance tests use isolated temporary credentials. Linux Secret Service acceptance remains a separate desktop-session test; ordinary headless Linux CI success does not establish it.

## Platform limits

Unix mode-bit/no-follow hardening is platform-specific; Windows ACL/reparse-point acceptance remains separate. `start.sh` is a Unix development convenience; Windows users use the npm/Tauri commands in `desktop/`. The configured macOS minimum is 13.3 and native local evidence targets Apple Silicon.

The macOS app is ad-hoc signed. Developer ID/notarisation, Windows distribution signing and public support commitments are outside this internal portfolio candidate. CI compile/package success is not equivalent to native GUI/security acceptance on each operating system.
