# Black Vault — Threat Model

**Updated:** 4 October 2026. **Scope:** 0.2.0-rc.1 offline desktop candidate.

## Assets and adversaries

Protected assets include root vault and wrapping keys; encrypted names/contents; authenticated file identities; the native/WebView filesystem boundary; recoverable ciphertext during interrupted operations; private transfer identities; and correct contact lifecycle transitions through application commands.

The primary adversary can read, copy, alter, replace or delete persisted files while a vault is locked. Other inputs include malformed documents/identity records, traversal, symlinks, substituted ciphertext, repeated nested PIN attempts and interrupted persistence. Private identity storage additionally relies on the OS credential store.

## Threats and controls

| Threat | Controls | Residual limitation |
| --- | --- | --- |
| Offline vault inspection | Argon2id wrapping and independent random keys; AES-256-GCM | Weak password entropy still matters |
| Ciphertext corruption/substitution | AEAD plus vault/file contextual binding | Does not detect rollback/deletion of all state |
| Arbitrary path access | Native picker/drop authority, ID validation, bounded file reads | Explicit picker/export intentionally accesses chosen files |
| Symlink replacement | Unix no-follow regular-file checks and restrictive modes | Windows ACL/reparse review remains separate |
| Risky import | Exact-byte bounded static policy before encryption | Not antivirus or comprehensive malware detection |
| UI session spoofing | Protected commands require native session; keys never returned to JS | Compromised native runtime/host is out of scope |
| Bypass UI biometric deletion | Fresh authorisation in native deletion command | No biometric factor when OS reports unavailable |
| Biometric cancellation/preflight error | Native fail-closed policy, timeout invalidation and tests | Real OS interaction requires hands-on acceptance |
| False biometric vault unlock claim | Password-based wrapping only; no standalone auth-success IPC | Enrolment/unwrapping remains deferred |
| Post-lock secret retention | Native session drop, best-effort zeroisation; immediate UI draft clearing and stale-response rejection | OS/runtime copies cannot be guaranteed erased |
| Nested brute force | Parent-key wrapping plus in-session PIN cooldown | Four-digit PIN low entropy; cooldown not durable |
| Interrupted transfer/restore | Authenticated staging, ordered persistence, fault/restart tests | Hardware/filesystem durability varies |
| Unsafe export overwrite | Authentication, destination checks and create-new semantics | Exported plaintext is outside encrypted storage |
| Contact key changed via app | Verification removed; explicit out-of-band re-verification | A display name cannot establish identity |
| Revoked contact reimport | Revoked state preserved | Revocation is local, not a global service |
| Contact persistence failure | Persist candidate before live-state update | Contacts JSON is public and not authenticated against valid local replacement |
| Contact-state tampering/rollback | Strict schema/bounds only; independent fingerprint checking | An attacker controlling valid local JSON can forge trust/history; no send/receive workflow currently relies on it |
| Identity-store corruption/unavailability | Bounded binary record, no plaintext fallback, explicit error | OS compromise and credential backup/lifecycle remain external |
| Malformed transfer bytes | Bounded Rust-only parser, signatures, suite/context binding, negative/vector tests | No shipping package workflow/replay database yet |
| Dependency defects | Lockfiles, dated audits and CI | Two desktop upstream warnings remain; legacy findings are recorded separately in the maintenance audit; new advisories can appear |

## Invariants

1. Vault master keys never cross the Rust-to-JavaScript boundary.
2. Locked vault storage does not require plaintext filenames, names or contents.
3. Protected ciphertext authenticates before producing plaintext or applying file changes.
4. JS IDs cannot become arbitrary native filesystem paths.
5. Recovery does not blindly delete unknown staging files or the only authenticated copy.
6. Import screening uses the exact bytes subsequently encrypted.
7. Successful native locking invalidates the active root-vault session; protected UI drafts clear immediately and late responses cannot restore them.
8. Biometric success cannot independently unlock cryptographic vault storage.
9. Imported/changed contact keys do not acquire verified trust merely from display names.
10. Private identity keys never enter React/localStorage; no plaintext credential fallback is allowed.

## Exclusions and validation

No protection is claimed against malware, an administrator, debugger, kernel or hostile hypervisor controlling the unlocked endpoint; captured passwords; physical-memory acquisition; deletion/rollback of every copy; forgotten-password loss; full credential-store compromise; comprehensive malware detection; or secure SSD erasure.

Automated/internal review is not independent penetration testing, certification or formal verification. Current candidate evidence is in the [validation record](../release/v0.2.0-validation.md); user-present acceptance is separately [tracked](../release/v0.2.0-manual-acceptance.md). The [transfer threat model](TRANSFER_THREAT_MODEL.md) includes future requirements that are not current shipping guarantees.

See the [4 October maintenance audit](../engineering/CODEBASE_AUDIT_2026-10-04.md) for the latest locking regressions and dependency findings.
