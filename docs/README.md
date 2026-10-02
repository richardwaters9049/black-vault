# Black Vault documentation

This directory contains the engineering record for Black Vault. For the current portfolio baseline, start with the authoritative documents below.

## Current authoritative documents

1. [As-built desktop architecture](architecture/Architecture.md) — current component and persistence design.
2. [Desktop security architecture](architecture/DESKTOP_SECURITY_ARCHITECTURE.md) — cryptography, trust boundaries and security controls.
3. [Threat model](security/THREAT_MODEL.md) — assets, attacker assumptions, controls and residual limitations.
4. [v0.1.0 validation record](release/v0.1.0-validation.md) — final test, audit and CI evidence.

## Secure sharing design — planned

The following documents define the next architecture phase. They are design requirements, not claims about the current v0.1.0 implementation.

- [Secure Transfer Protocol](security/SECURE_TRANSFER_PROTOCOL.md) — recipient encryption, sender authentication, package format and identity lifecycle.
- [Transfer Threat Model](security/TRANSFER_THREAT_MODEL.md) — hostile-channel threats, controls and residual limitations.
- [Secure Sharing Implementation Plan](architecture/SECURE_SHARING_IMPLEMENTATION_PLAN.md) — phased engineering and acceptance gates.

If an older sprint or release-candidate document conflicts with the current implemented architecture, the current authoritative documents take precedence.

## Project context

- [Project summary](project/PROJECT_SUMMARY.md)
- [Legacy architecture boundary](../legacy/README.md)
- [Engineering rules](AGENTS.md)

## Release and platform records

- [v0.1.0 validation record](release/v0.1.0-validation.md)
- [Cross-platform support notes](release/CROSS_PLATFORM_SUPPORT.md)
- [Historical release-candidate acceptance](release/RELEASE_0.1.0.md)

The historical release-candidate document records the earlier public-release evaluation. Public distribution is no longer planned; the validation record represents the final portfolio milestone.

## Historical engineering records

The sprint documents preserve implementation progression and are not the current architecture specification.

- [Sprint 01 — Desktop foundation](sprints/sprint-01/SPRINT_01_DESKTOP_FOUNDATION.md)
- [Sprint 02 — Vault foundation](sprints/sprint-02/SPRINT_02_VAULT_FOUNDATION.md)
- [Sprint 03 — Authentication](sprints/sprint-03/SPRINT_03_AUTH_EXPERIENCE.md)
- [Sprint 04 — Auto-lock](sprints/sprint-04/SPRINT_04_AUTO_LOCK.md) · [Import screening](sprints/sprint-04/SPRINT_04_IMPORT_SCREENING.md)
- [Sprint 05 — Examples and lifecycle](sprints/sprint-05/SPRINT_05_EXAMPLES_VAULT_LIFECYCLE.md)
- [Sprint 06 — Nested workspace](sprints/sprint-06/SPRINT_06_NESTED_VAULT_WORKSPACE.md)
- [Sprint 07 — File management](sprints/sprint-07/SPRINT_07_FILE_MANAGEMENT.md)
- [Sprint 08 — Nested file management](sprints/sprint-08/SPRINT_08_NESTED_FILE_MANAGEMENT.md)
- [Sprint 09 — Completion guide](sprints/sprint-09/SPRINT_09_COMPLETION_GUIDE.md)
- **Sprint 10:** [Completion](sprints/sprint-10/SPRINT_10_COMPLETION.md) · [Fault injection](sprints/sprint-10/SPRINT_10_FAULT_INJECTION.md) · [Security review](sprints/sprint-10/SPRINT_10_SECURITY_REVIEW.md) · [macOS RC checklist](sprints/sprint-10/SPRINT_10_MACOS_RC_CHECKLIST.md)

## Historical design/reference material

- [Historical encryption architecture](architecture/Encryption_Architecture.md)
- [Historical project handover](project/PROJECT_HANDOVER.md)
- [7 September notes](notes/7thSept2026_notes.md)
- [28 September notes](notes/28thSept2026_notes.md)

These records are retained to show the project's evolution. They may contain superseded implementation details, planned features that were not adopted, or release assumptions that no longer apply.
