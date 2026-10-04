# Black Vault — Desktop Security Architecture

**Updated:** 4 October 2026. **Scope:** 0.2.0-rc.1 internal desktop candidate.

## Objectives and boundaries

Authenticated encryption protects local vault names, filenames and contents against offline inspection and unauthorised modification. Rust controls keys, root/nested authentication, authorised paths, import policy, persistence and recovery. React controls presentation and receives authorised metadata or text only for an active operation.

The production CSP permits local application resources and Tauri IPC. Capabilities apply to the local `main` window; command permissions do not grant a generic filesystem, HTTP or shell interface. Password arguments and active secrets use best-effort zeroising native buffers. Authorised editor text necessarily exists in WebView memory.

## Key hierarchy

Argon2id v19 uses 19,456 KiB, two iterations, parallelism one, a random 16-byte salt and a 32-byte output. Its wrapping key protects a randomly generated 256-bit vault master key and vault metadata. Header KDF/version bounds are validated before expensive processing.

Files receive independent random data-encryption keys. AES-256-GCM uses fresh random 96-bit nonces and domain-separated vault/file context in associated data. Wrong-key, tamper and cross-vault substitution tests enforce authentication.

Nested records require the parent master key and a separately derived four-digit PIN key. In-session attempt throttling reduces repeated attempts; the PIN remains low entropy and its cooldown is not durable across all restarts.

## Files and durability

Picker/drop selection is staged natively, then revalidated and bounded in Rust. Local imports are limited to 5 MiB. Unix regular-file/no-follow helpers reject symlinks, with restrictive storage modes where supported. Windows portable filesystem behaviour does not imply Unix permission or reparse-point guarantees.

Screening operates on the exact bytes subsequently encrypted. It rejects bounded executable/script/archive/signature indicators; it is not antivirus. Text saves re-encrypt into the vault without a plaintext working file. Explicit exports authenticate first and create a plaintext destination without unsafe overwrite.

Sensitive writes use narrow temporary names, file synchronisation, rename and directory synchronisation. Recovery authenticates known encrypted staging data and favours recoverable duplicate ciphertext over losing the only valid copy. Fault-injection and subprocess-interruption tests exercise ordering, not guaranteed physical power-loss durability on every device/filesystem.

## Authentication and destruction

Only successful Rust password authentication creates a root session. The locked Overview is not a security boundary; protected file commands independently require the native session. Startup, explicit lock and five-minute inactivity release the active root key. Protected panels and drafts are cleared as locking begins; operation-generation checks prevent late IPC responses from restoring editor content. Native failure is reported and does not become a successful UI lock. Delayed-timer/focus checks are not native sleep interception.

Permanent locked-vault deletion requires exact typed confirmation and fresh native biometrics when available. Cancellation, failed preflight, false success and timeout abort. Unavailable biometrics leave the typed-confirmation policy. The standalone biometric IPC command is removed; Touch ID cannot unlock vault master keys. See the [biometric model](../security/BIOMETRIC_SECURITY_MODEL.md).

## Identities and contacts

Independent X25519 recipient and Ed25519 signing private material is held in a versioned binary credential through macOS Keychain, Windows Credential Manager or Linux Secret Service. Missing credentials may initialise an identity; malformed or unavailable storage fails rather than silently replacing an identity or storing plaintext. These keys are not claimed to be Secure Enclave-backed or non-exportable. Rust rotation functionality is not exposed as a completed UI feature.

The UI receives public documents and fingerprints. Imports start unverified; explicit exact fingerprint comparison establishes verified state. Changed keys lose verification; revoked contacts do not silently regain trust on reimport. Contact updates persist a candidate before replacing live state.

Contacts contain public names, keys and trust history in versioned JSON outside vault encryption. Strict parsing, bounded regular-file reads, atomic persistence and restrictive Unix modes do **not** authenticate a valid malicious replacement of that JSON. Contact metadata privacy, durable rollback protection and an authenticated contact trust store are outside current guarantees. Reverify through an independent channel after restoring/replacing contact state.

## Transfer implementation status

The Rust-only foundation implements HPKE recipient encryption, mandatory Ed25519 signatures, version/suite binding, bounded canonical decoding and negative/RFC-vector tests. It does not provide a complete send/receive application. File package persistence, replay history, received-vault UI and cross-platform package workflow acceptance are deferred. See the [transfer status](SECURE_SHARING_IMPLEMENTATION_PLAN.md).

## Verification and limitations

Current results are dated in the [candidate record](../release/v0.2.0-validation.md); historical 46 Rust/22 frontend counts belong to v0.1.0. The [4 October maintenance audit](../engineering/CODEBASE_AUDIT_2026-10-04.md) records newer local checks. Two Linux GTK/Tauri/tooling warnings remain; the yanked yoke-derive version has been updated. Legacy dependency findings are tracked separately in that audit.

There is no forgotten-password recovery, all-state rollback protection, independent security certification, notarised public distribution, guaranteed memory erasure, or protection from a compromised unlocked host. See the [threat model](../security/THREAT_MODEL.md).
