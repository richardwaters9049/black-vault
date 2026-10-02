# Black Vault — Desktop Security Architecture

**Updated:** 1 October 2026
**Status:** As-built security architecture for Black Vault 0.1.0.

## Security objectives

Black Vault protects locally persisted vault metadata, filenames and file contents against offline inspection and unauthorised modification while a vault is locked. It minimises plaintext persistence, keeps privileged filesystem operations behind Rust/Tauri, and fails closed when authenticated encrypted records are malformed or tampered with.

It does **not** claim protection against a compromised host while unlocked, administrator-level memory inspection, guaranteed erasure from every OS-managed memory copy, physical power-loss durability on every filesystem, rollback detection without an independent trusted baseline, forgotten-password recovery, or independent security certification.

## Trust boundaries

### React/WebView

The WebView is presentation rather than the cryptographic authority. It requests operations through Tauri commands and receives authorised metadata/plaintext only when required for the current unlocked workflow. It does not receive vault master keys or arbitrary native filesystem authority.

The production CSP permits only the local application origin and Tauri IPC connection required by the runtime. Capabilities are scoped to the `main` window. Shell/process execution is not exposed.

### Rust/Tauri core

Rust owns vault sessions, cryptographic operations, native filesystem access, import screening and persistence/recovery. Password arguments are wrapped in zeroising buffers at the command/storage boundary. Active vault names and master keys use `Zeroizing` containers where practical.

Error messages are intentionally bounded so internal paths, keys and low-level storage details are not returned to the WebView.

### Local filesystem

The filesystem is treated as attacker-readable and attacker-modifiable while the vault is locked. Sensitive persistent values therefore require authenticated encryption. An attacker may still delete or roll back encrypted files; AES-GCM authenticity alone cannot prove freshness.

## Key hierarchy

```text
Master password
      │
      ▼
Argon2id (19,456 KiB, 2 iterations, parallelism 1)
      │ 16-byte random salt
      ▼
256-bit wrapping key
      │
      ├── authenticated encrypted vault metadata
      └── wraps random 256-bit vault master key
                              │
                              ▼
                  per-file authenticated encryption
                  encrypted filename + contents
```

The vault-header KDF record is versioned and validated before expensive derivation. Unsupported parameters fail closed. AES-GCM uses fresh random 96-bit nonces and contextual associated data.

File ciphertext is bound to its vault and file identifiers. Tests verify wrong keys, tampering and cross-vault ciphertext substitution fail authentication.

## Nested vaults

Nested vaults add a second local authentication boundary inside an unlocked parent vault. A four-digit PIN is processed with Argon2id, and the nested payload is additionally protected by the parent vault master key. Opening it therefore requires an unlocked parent and the correct PIN.

Repeated incorrect PIN attempts are throttled in the active session, and all PIN-authenticated nested operations use the same attempt controls. A four-digit PIN is a convenience boundary, not equivalent to a high-entropy master password.

## File handling

Imports are read through bounded regular-file helpers. On Unix, no-follow opening rejects symlinks and metadata is checked after opening. Vault headers use the same hardened read path.

The offline import scanner blocks high-confidence executable/script/container indicators, macro-capable extensions, the EICAR test signature and a small set of high-risk command patterns. It is a policy gate, **not antivirus**, malware detection or a safety verdict.

The current plaintext import limit is 5 MiB. Exports authenticate encrypted data before producing plaintext and refuse unsafe overwrites. Decrypt-in-vault verification does not create a plaintext copy in vault storage.

## Persistence and recovery

New and updated encrypted records use create-new temporary files where applicable, `sync_all`, rename and directory synchronisation. Unix files/directories receive restrictive modes where supported.

Nested transfer and restore operations use explicit persistence ordering. If interrupted, the design prefers recoverable duplicate ciphertext over losing the only authenticated copy. Recovery recognises narrowly defined staging files and validates corresponding encrypted state before cleanup.

The Rust suite includes fault matrices and a real subprocess interruption matrix around these boundaries.

## Session lifecycle

Only one root vault is active at a time. Explicit lock/sign-out and five minutes of inactivity clear the active session. Master-key material is held in best-effort-zeroising Rust containers and is never exposed through the JavaScript API.

Zeroisation reduces accidental retention; it is not a guarantee against compiler, allocator, swap, crash-dump or hostile-process copies.

## Security verification

The hardened baseline includes 46 Rust tests, 22 frontend tests, strict Clippy/Rustfmt verification, a production TypeScript/Vite build, production-bundle development-secret checks, `npm audit` with zero known vulnerabilities, and RustSec `cargo audit` with zero vulnerability advisories at the final audit.

See [Threat Model](../security/THREAT_MODEL.md) and [v0.1.0 Validation](../release/v0.1.0-validation.md).
