# Black Vault 0.1.0 release acceptance

**Branch:** `release/0.1.0`
**Status:** Internal release-candidate work.

## Completed

- Sprint 10 recovery/security regression suite merged to `main`.
- Documentation reorganisation merged to `main`.
- Release branch created from the combined tree.
- macOS arm64 app builds and passes structural `codesign --verify`.
- Cross-platform CI added for macOS arm64, Windows x64 and Linux x64.
- Cross-platform native acceptance matrix documented.

## Required before public release

- Final packaged macOS RC walkthrough using disposable data.
- Clean-profile backup/restore acceptance.
- Windows x64 CI green **and** packaged-app native acceptance on Windows.
- Linux x64 CI green **and** packaged-app native acceptance on a representative supported distribution.
- Windows filesystem ACL/reparse-point security review.
- Decide Linux package formats/support baseline.
- Production signing: Apple Developer ID/notarisation; Windows signing strategy; Linux package provenance/signing as appropriate.
- External security assessment before describing Black Vault as independently audited or production-security certified.

A green cross-platform compile/test matrix is necessary but does not replace target-OS runtime testing.
