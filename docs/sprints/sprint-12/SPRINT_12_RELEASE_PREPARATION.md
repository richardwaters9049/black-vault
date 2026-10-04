# Sprint 12 — release documentation and packaging

**Date:** 4 October 2026. **Candidate:** 0.2.0-rc.1.

## Engineering work

- Updated root/desktop README, current architecture, threat model and biometric model.
- Distinguished exposed identities/contacts from Rust-only transfer foundations and future send/receive.
- Preserved the historical v0.1.0 release and validation records.
- Aligned npm, npm lockfile, Cargo, Cargo lockfile and Tauri candidate versions.
- Replaced stale 0.1.0 CI artifact labels with commit-specific labels.
- Reserved three-OS packaging for release branches/manual dispatch; kept routine Linux validation.
- Prepared native candidate packaging and one full cross-platform CI gate.

## Promotion criteria

Record local gate results, CI run/artifacts and candidate acceptance before promoting to stable v0.2.0. Do not describe the candidate as public, notarised, independently audited or fully hands-on accepted across all platforms. No tag or hosted release should supersede v0.1.0 before promotion.

See [candidate record](../../release/v0.2.0-validation.md), [platform status](../../release/CROSS_PLATFORM_SUPPORT.md) and [documentation index](../../README.md).
