# Biometric security model

**Updated:** 4 October 2026. **Scope:** 0.2.0-rc.1.

## Implemented use

macOS Local Authentication provides a fresh biometric prompt inside the native `delete_locked_vault` operation. Exact typed confirmation and locked-vault validation are still required. The React UI can describe the policy but cannot authorise or skip the native prompt by calling deletion directly.

| Native result | Deletion behaviour |
| --- | --- |
| Biometrics available and successful | Continue to existing native deletion validation |
| Cancellation, failure, false authentication result or timeout | Abort with a bounded error |
| Biometric status query error | Fail closed |
| Biometrics unavailable, including current Windows/Linux implementation | Existing exact typed confirmation policy |

A timeout invalidates the OS authentication context. The biometric policy suppresses the system-password fallback within the biometric prompt. Black Vault never receives fingerprint templates. Availability is rechecked natively at operation time; a stale UI status cannot bypass native policy.

## Vault unlock

Vault unlock remains password-based Argon2id wrapping-key derivation and AES-GCM authentication. A successful biometric signal alone cannot derive or unwrap a vault key. There is no standalone biometric-success IPC command, no cached biometric token and no vault-password storage introduced for this feature.

Biometric vault unlock/enrolment is deliberately excluded from this release. Implementing it later needs an OS-protected key-unwrapping design, explicit per-vault enrolment/revocation, reinstall/key-change handling and native cancellation/relaunch tests. OS login authentication must not be presented as cryptographic vault unlock.

## Limits and verification

Unavailable biometrics fall back to typed confirmation for deletion, not to equivalent two-factor authorisation. File deletion is not secure SSD erasure and does not remove external backups. Host compromise remains outside the protective model.

Native policy tests cover cancellation, unsuccessful responses, unavailable status and failed preflight. Real Touch ID success/cancellation requires a user present at the packaged app; mocked tests do not establish that evidence. See the [manual acceptance checklist](../release/v0.2.0-manual-acceptance.md).
