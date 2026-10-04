# Black Vault v0.2.0-rc.1

Internal portfolio release candidate. Stable promotion waits for the current packaged-app manual acceptance record.

## Changes

- Updated authenticated vault workspace, operation-panel motion and centred startup branding.
- Empty password entry on Encryption/Decryption, including development builds.
- Native Touch ID confirmation for permanent locked-vault deletion; cancellation and status failures fail closed.
- Removed standalone biometric verification. Vault unlock remains password-based; no biometric vault enrolment is shipped.
- Public identities and trusted-contact fingerprint/key-change/revocation management.
- Reconciled architecture, threat model, biometric model and historical release evidence.
- Linux-only routine validation; full packaging reserved for release branches/manual runs.

## Validation

[Cross-platform run 37198649703](https://github.com/richardwaters9049/black-vault/actions/runs/37198649703) passed macOS arm64, Windows x64 and Linux x64 tests, Clippy and native packaging. The source commit is `6f90652d25a064a2bddc85d1193b97f534fe73b5`. Frontend: 29 tests on each OS. Rust: macOS 93, Windows 91, Linux 94; explicit Keychain and Credential Manager tests separately passed.

Packages: macOS DMG, Windows NSIS installer, Linux AppImage and DEB. SHA-256 checksums and provenance accompany the artifacts. macOS signing is ad-hoc; no notarised public-distribution claim is made. Three unresolved RustSec transitive warnings are recorded.

## Feature and acceptance boundaries

Cross-user package send/receive and replay tracking are deferred; only the Rust crypto/parser foundation and identity/contact UI are implemented. Contact JSON is public and not an authenticated defence against arbitrary local-state replacement. Real Touch ID, packaged GUI and Linux Secret Service acceptance remain pending for this candidate.

See [validation](v0.2.0-validation.md), [manual acceptance](v0.2.0-manual-acceptance.md) and [artifact manifest](v0.2.0-artifacts.json). Historical v0.1.0 evidence is preserved.
