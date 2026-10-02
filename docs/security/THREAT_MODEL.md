# Black Vault — Threat Model

**Updated:** 1 October 2026
**Scope:** Black Vault 0.1.0 desktop application.

## Assets

Black Vault is designed to protect vault master keys and password-derived wrapping keys; encrypted filenames, contents and vault names at rest; integrity of encrypted records; the WebView/native filesystem boundary; and recoverable encrypted data across interrupted transfer/restore operations.

## Attacker assumptions

The primary model is an attacker who can inspect, copy, replace, corrupt or delete Black Vault's local persisted files while the vault is locked. The attacker may know the file format and application source code.

The model also considers malicious or malformed imported files, path traversal attempts, symlinks, ciphertext substitution, corrupted persistence records, repeated nested-PIN attempts and interruption/crash at sensitive persistence boundaries.

## Out of scope

Black Vault does not claim to defeat malware, debuggers or an administrator controlling the host while unlocked; kernel compromise; hostile hypervisors; physical memory acquisition; capture of a password before it reaches Black Vault; deletion/rollback of all encrypted state; forgotten-password recovery; vulnerabilities in the OS/runtime/cryptographic libraries; or comprehensive malware detection.

## Threats and controls

| Threat | Primary controls | Residual limitation |
| --- | --- | --- |
| Offline reading | Argon2id wrapping key, random vault key, AES-256-GCM | Weak passwords remain limited by their entropy |
| Ciphertext modification | AES-GCM authentication and contextual AAD | Attackers can still delete data |
| Ciphertext substitution | Vault/file identity in authenticated context; regression tests | Does not establish freshness against rollback |
| Path traversal | Strict ID validation; application-generated storage paths | Picker intentionally accesses user-selected external files |
| Symlink substitution | Bounded regular-file helpers; Unix `O_NOFOLLOW`; post-open checks | Windows reparse/ACL behaviour remains platform-specific |
| Malicious import | Offline extension/magic/signature/pattern gate; size bounds | Scanner is deliberately not antivirus |
| WebView filesystem authority | Native picker/drop staging, typed Tauri commands, origin/window checks | Native-core/runtime compromise is outside this boundary |
| Master key exposed to JS | Master key remains in Rust | Displayed/edited plaintext exists in process memory while in use |
| Secrets retained after lock | Session removal and `Zeroizing` containers | Zeroisation cannot guarantee removal from every runtime/OS copy |
| Nested PIN brute force | Parent-key wrapping and in-session cooldown | Four-digit PIN has low entropy; cooldown is not durable |
| Crash during transfer | Durable ordering, authenticated staging, recovery/fault tests | Hardware/filesystem durability can vary |
| Unsafe export overwrite | Destination validation and overwrite refusal | User can choose an externally exposed destination |
| Error/log leakage | Generic user-facing errors; minimised release diagnostics | Development builds contain limited diagnostics |
| Dependency vulnerability | Locked dependencies, CI, npm and RustSec audits | New advisories can appear after the audit date |

## Security invariants

1. Locked storage does not require plaintext filenames, vault names or file contents.
2. Vault master keys never cross the Rust-to-JavaScript command boundary.
3. Persisted ciphertext authenticates before destructive or plaintext-producing operations continue.
4. IPC-supplied IDs cannot become arbitrary filesystem paths.
5. Interrupted transfer/recovery must not silently destroy the only authenticated copy.
6. Unknown staging files are not automatically trusted or deleted.
7. Import screening operates on the exact bytes subsequently encrypted.
8. Locking invalidates the active root-vault session.

## Validation

These claims are supported by automated tests and the final Phase Six internal audit. Black Vault has not undergone independent penetration testing or formal cryptographic verification. It should be described as **security-engineered and internally audited**, not independently audited or certified.

See [Desktop Security Architecture](../architecture/DESKTOP_SECURITY_ARCHITECTURE.md) and [v0.1.0 Validation](../release/v0.1.0-validation.md).
