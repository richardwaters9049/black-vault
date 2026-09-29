# Sprint 10 — Completion handover

**Date:** 29/09/2026
**Branch:** `test/desktop-fault-injection`
**Outcome:** Security/recovery implementation and automated internal-release-candidate checks completed. This is **not** a public release approval.

## Delivered

- Test-only interruption hooks at 13 encrypted transfer and restore persistence boundaries. Dedicated tests verify returned I/O failure recovery, and an isolated child process exits abruptly at each checkpoint (exit code 86). Each run uses a fresh disposable vault and reopens it in a new store.
- Regression coverage for abandoned parent staging and two different valid authenticated ciphertext copies sharing a file ID. Conflicts are preserved rather than overwritten.
- Native Rust drag/drop tracing compile-gated to debug builds; reviewed UI errors no longer print raw exception objects. Fixed auth-panel overscroll CSS.
- Security review, backup policy, fault-injection scope/limitations and macOS candidate checklist.
- Verification: **45/45 Rust library tests, 21/21 frontend tests**, TypeScript/Vite production build, Rust formatting, Clippy with warnings denied, ad-hoc macOS app bundle and `codesign --verify --deep --strict`.

## Handover / outstanding distribution gates

The `Black Vault.app` under `desktop/src-tauri/target/release/bundle/macos/` is an **arm64 internal test build** (bundle identifier `com.blackvault.desktop`, version `0.1.0`, minimum macOS `13.3`). Build artefacts are intentionally not committed. Its signature is ad-hoc, and it has not been notarised.

Before public distribution: packaged GUI walkthrough with a disposable macOS profile, independently verified backup/restore, final frontend diagnostic-console cleanup, Developer ID signing and Apple notarisation, clean-machine Gatekeeper/permissions test, and external security assessment. The test harness is not literal SIGKILL or physical power-loss validation. Do not market an independently audited durability or security guarantee.

See `SPRINT_10_FAULT_INJECTION.md`, `SPRINT_10_SECURITY_REVIEW.md` and `SPRINT_10_MACOS_RC_CHECKLIST.md` for details.
