# Black Vault — Desktop Security Architecture

**Draft date:** 29 September 2026  
**Status:** Proposed design for review; only Sprint 01 desktop scaffolding is implemented.

## 1. Scope and security objectives

An offline-first desktop application built with Tauri 2, React/Vite/TypeScript and Rust. Core vault operations require no account, remote API, Docker, PostgreSQL or cloud service. Initial distribution target: macOS Apple Silicon. Preserve legacy encrypted formats and tests.

Protect persistent file contents, filenames, sensitive metadata and cryptographic keys against offline inspection of a locked vault. Minimise exposure during an unlocked session. Do not promise protection against malware or an administrator controlling an already-unlocked host, perfect memory erasure, or detection of deletion/rollback without an independent trusted baseline.

## 2. Trust boundaries

- React/WebView: presentation and authorised typed command requests only; never persist plaintext master keys or accept arbitrary filesystem paths as authorisation.
- Rust vault manager: owns sessions, passwords during derivation, keys, file handles, crypto and storage. Keep errors minimal and never log filenames, plaintext, passwords, paths or keys.
- Vaults: each has an independent random 256-bit master key, metadata and lifecycle. A compromise of the common process may affect all unlocked vaults; this is cryptographic compartmentalisation, not Qubes-style OS isolation.
- Persistent filesystem: assume an attacker can read, copy, corrupt, delete or replace local files. All sensitive persisted metadata and contents must be authenticated and encrypted.
- Future recipient identity: independent of vault master keys and protected separately.

## 3. Proposed key hierarchy

```text
Vault password --Argon2id--> wrapping key
                              |
                       wrapped vault key
                              |
                    independent vault master key
                              |
                  wrapped random per-file keys
                              |
                  encrypted content and filename

Separate recipient/device private identity key
     -> public-key sharing; never export vault master key
```

Reuse the canonical legacy Argon2id implementation and compatibility fixtures where applicable. Keep cryptographic versions explicit. Legacy salt-text recovery must be explicitly requested only for proven legacy records. Use fresh, domain-separated AES-256-GCM operations; never reuse a nonce with a key. Do not design new sharing crypto by improvising key conversion or inventing formats without review.

## 4. Proposed ephemeral metadata experiment

For each unlocked vault, prototype an SQLite in-memory connection populated only after authenticating and decrypting the vault's persisted metadata. Evaluate SQLite temporary storage, journalling, attached databases, extensions, backups, crash dumps and OS swapping; do not claim RAM-only execution eliminates every physical trace. Restrict SQLite's configuration and test for accidental plaintext files.

On successful mutation, write an authenticated encrypted checkpoint through a temporary file, flush/synchronise as appropriate and atomically replace the previous checkpoint. Include generation/version and recovery logic. A save-on-lock-only strategy is unacceptable because a crash would lose an entire session. Design a recoverable backup scheme without silently accepting stale or rolled-back state.

Lock must close active database connections, invalidate commands/sessions and clear accessible Rust secret buffers. Test in-flight operations and late completions so they cannot reopen a locked vault. Password changes should rewrap the vault key rather than silently rewrite every file.

## 5. File storage and sharing roadmap

Store encrypted file blobs separately from encrypted metadata. Preserve the legacy version 2 AES-GCM AAD domains and payload conventions for deliberate compatibility/import. Revalidate selected paths at the time of opening; protect against symlink/path races and unsafe export destinations. Do not lift the current 5 MiB web limit until chunked authenticated encryption, nonce allocation, size quotas and interrupted-write recovery are designed.

Later, define a versioned `.bvault` package using independently generated recipient/device identity keys, verified public-key fingerprints, recipient-specific wrapped file keys and authenticated metadata. Never share a vault password or vault master key. Support offline export/import first; LAN transport and optional encrypted relay come afterwards.

## 6. Audit, integrity and recovery

Sensitive actions should eventually produce a durable, minimised and tamper-evident local audit trail. Ordinary process diagnostics are not durable audit. AES-GCM detects unauthorised modifications to authenticated data but does not alone prevent deletion, replay or rollback. Specify trusted baseline, backup and recovery-key policies before claiming those protections.

## 7. Sprint 02 acceptance

- Existing Rust and TypeScript vectors and legacy regression tests remain green.
- Native bundled macOS app and picker manually verified.
- Architecture and versioned storage design reviewed before persistence format is fixed.
- Prototype creates independent vaults, rejects wrong passwords/tampering, locks and restarts without plaintext persistent metadata.
- Tests cover mutation checkpoints, crash/interrupted writes, database temporary files, session invalidation and corrupt backups.
- No automatic commits, pushes, destructive migrations or claims that planned controls are already implemented.
