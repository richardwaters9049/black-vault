# Black Vault Secure Transfer Protocol

> Current status (4 October 2026): Rust in-memory encryption/signature/parser foundations and identity/contact management exist. This specification also includes future requirements. End-user package send/receive, durable replay tracking and the received-vault UI are excluded from 0.2.0-rc.1. See [implementation status](../architecture/SECURE_SHARING_IMPLEMENTATION_PLAN.md).

Updated: 2 October 2026  
Status: design specification — not yet implemented

## Purpose

Black Vault transfers highly sensitive files as encrypted vault packages. The transport is deliberately outside the trust boundary: a package may be copied by USB, AirDrop, email, cloud storage, SFTP or another channel without that channel receiving the plaintext or the content-encryption key.

The security objective is not to hide that a transfer occurred from the local operating system. It is to provide confidentiality, integrity, recipient binding and sender authenticity for the package itself.

## Design rules

1. Do not invent cryptographic primitives.
2. Do not share vault passwords between users.
3. Use separate keys for encryption and signing.
4. Encrypt filenames and other sensitive metadata with the payload.
5. Authenticate before releasing plaintext.
6. Bind the package version, package ID, sender, recipient and payload to the cryptographic context.
7. Private identity keys never cross the Rust-to-WebView boundary.
8. Treat every imported package and manifest field as attacker-controlled until verified.
9. A copied package must not become readable merely because the transport is compromised.
10. Security failures are fail-closed and use generic user-facing errors.

## Cryptographic identities

Each Black Vault identity has two independent long-term key pairs:

- X25519 recipient key pair — receives encrypted package keys.
- Ed25519 signing key pair — signs outgoing package manifests.

The public identity contains the public keys, protocol version and a fingerprint derived from canonical public identity bytes. Private keys remain Rust-side and must be protected at rest using an operating-system secure storage facility where the platform implementation permits it.

A contact is not trusted merely because a public key was imported. Black Vault records whether its fingerprint has been verified through a separate trusted channel.

## Package encryption

For every transfer Black Vault generates fresh random package encryption material. The recipient-key envelope uses an RFC 9180 HPKE suite based on DHKEM(X25519, HKDF-SHA256). The exact AEAD suite and library must be fixed before implementation and covered by published RFC/library test vectors.

The encrypted payload contains the sensitive manifest and file records. Existing AES-256-GCM file-encryption code may be reused only where its nonce/AAD invariants remain valid under the package format; reuse is not assumed merely because the primitive is the same.

No password-derived key protects a cross-user transfer package.

## Sender authentication

The sender signs the canonical outer package metadata and cryptographic commitment to the encrypted payload using the sender's Ed25519 signing key.

The recipient must verify:

1. the package format/version is supported;
2. the recipient key identifier matches a local identity;
3. the sender public identity is known or is explicitly presented as unverified;
4. the Ed25519 signature is valid;
5. the HPKE recipient envelope opens successfully;
6. authenticated payload decryption succeeds;
7. the inner manifest agrees with the authenticated outer bindings.

No filenames or file contents are shown before these checks succeed.

## Proposed package layout

```text
secret.blackvault
├── magic / format version
├── package ID
├── cryptographic suite IDs
├── recipient key ID
├── sender signing key ID
├── HPKE encapsulated key material
├── encrypted payload
│   ├── private manifest
│   │   ├── package ID
│   │   ├── sender identity binding
│   │   ├── recipient identity binding
│   │   ├── created-at value
│   │   └── file records
│   └── encrypted file records
└── sender signature
```

The outer header contains only information required to route and verify the package. Human-readable vault names, filenames, file count, MIME-like labels and descriptive metadata belong inside authenticated encryption unless a concrete interoperability requirement proves otherwise.

## Canonical encoding

Protocol v1 uses an explicit big-endian binary encoding. It does not use Rust's in-memory representation or a general-purpose serializer for signed bytes.

The v1 outer package fields are encoded in this exact order:

```text
8 bytes   magic = BLKVAULT
2 bytes   version = 0x0001
2 bytes   HPKE KEM ID = 0x0020 (DHKEM X25519/HKDF-SHA256)
2 bytes   HPKE KDF ID = 0x0001 (HKDF-SHA256)
2 bytes   HPKE AEAD ID = 0x0002 (AES-256-GCM)
16 bytes  random package ID
16 bytes  recipient key ID
16 bytes  sender signing-key ID
32 bytes  HPKE encapsulated key
8 bytes   ciphertext length, unsigned big-endian
N bytes   HPKE ciphertext
64 bytes  Ed25519 signature
```

The Ed25519 signature covers every preceding byte, including the ciphertext length and ciphertext. HPKE associated data independently binds the package ID, recipient key ID and sender signing-key ID to payload authentication.

Protocol v1 has no optional or unknown fields. Any unsupported version, algorithm identifier, inconsistent length, truncation or trailing byte fails closed. Future extensibility requires a new explicitly reviewed version rather than parser guesswork.

### Interoperability and parser validation

The selected HPKE suite is checked against an RFC 9180 known-answer vector for base mode with X25519/HKDF-SHA256/AES-256-GCM, rather than relying only on Black Vault encrypt/decrypt round trips.

The production byte parser is isolated in `src/transfer_codec.rs`. Unit tests exercise deterministic truncation, mutation and arbitrary-input corpora. The same parser source is compiled by the libFuzzer target in `fuzz/fuzz_targets/transfer_decode.rs`, avoiding a test-only parser implementation.

Fuzzing is a developer security gate and does not change the project's stable Rust toolchain. Run it explicitly with nightly:

```bash
cd desktop/src-tauri
cargo +nightly fuzz run transfer_decode fuzz/corpus/transfer_decode -- -max_total_time=60 -max_len=8192
```

A bounded 20-second AddressSanitizer/libFuzzer run completed during Phase A with 16,282,808 executions and no crash. Longer fuzzing remains appropriate before a high-assurance release.

## Replay and rollback

Every package has a cryptographically random 128-bit-or-larger package ID included in the authenticated/signature context.

Black Vault maintains a local record of successfully accepted package IDs. Re-importing an already accepted package is visibly identified as a replay/duplicate rather than silently presented as new.

This local record cannot by itself prevent rollback if an attacker can restore the entire application state to an older snapshot. Strong rollback detection requires an external monotonic/trusted state and remains a documented limitation for an offline application.

## Identity verification

A public identity is displayed with a human-verifiable fingerprint. Users may exchange identity files through any channel, but high-sensitivity contacts should verify the fingerprint independently, preferably in person or over an already authenticated channel.

A future QR workflow may encode the same canonical public identity. QR scanning improves usability; it does not by itself establish trust.

The UI distinguishes:

- Unverified identity
- Verified identity
- Key changed — verification required
- Revoked/retired identity

A changed key must never silently inherit the previous key's verified status.

## Key rotation and compromise

Encryption and signing keys have independent lifecycle metadata. Rotation creates new keys; it does not mutate historical signatures.

If a recipient encryption key is compromised, packages encrypted to that key may be exposed if the attacker has retained ciphertext. Black Vault must not claim forward secrecy for asynchronous stored packages unless a separately reviewed protocol provides it.

If a signing key is compromised, historical signatures cannot automatically be treated as trustworthy after the compromise point without trusted time/revocation infrastructure. Offline revocation has unavoidable distribution limitations.

## Import pipeline

Imported `.blackvault` packages are parsed with strict bounds before expensive or allocating operations:

1. validate magic and version;
2. enforce total package and field-size limits;
3. reject duplicate/ambiguous fields and unsupported critical algorithms;
4. identify the intended local recipient;
5. verify sender signature according to trust state;
6. open the recipient envelope;
7. authenticate/decrypt the private manifest;
8. validate file counts, lengths and identifiers;
9. expose metadata to the UI only after authentication;
10. decrypt file content only on explicit user action.

Package parsing must not execute embedded content or infer paths from package filenames.

## Export pipeline

1. User opens a local vault.
2. User selects files.
3. User selects a recipient whose identity status is clearly displayed.
4. Rust creates a fresh package ID and encryption material.
5. Rust encrypts sensitive metadata and file content.
6. Rust creates the recipient envelope.
7. Rust signs the canonical package commitment.
8. Rust writes to a restrictive temporary file, synchronises it, then atomically publishes the completed package where supported.
9. UI reports completion only after durable write succeeds.

The WebView receives status/progress events, never private keys or plaintext cryptographic key material.

## Progress model

Progress must describe real states, not invented percentages:

```text
Preparing files
Scanning selected files
Encrypting package
Binding recipient
Signing package
Writing package
Verifying output
Complete
```

For large files, byte progress may be added only if the Rust implementation can report genuine processed/total byte counts.

## Multi-recipient packages

Not part of protocol v1.

A later version may encrypt one content key to several recipient public keys. That changes metadata leakage, revocation and recipient-privacy properties and therefore requires a separate review.

## Explicit non-goals for v1

- Anonymous communication
- Hiding network/transport metadata
- Protection after the recipient deliberately exports plaintext
- Protection against a fully compromised unlocked endpoint
- Guaranteed secure deletion on modern storage
- Central account recovery
- Server-mediated identity verification
- Multi-recipient group messaging
- Claims of independent certification

## Required implementation gates

Before transfer is called complete:

- protocol parser fuzzing;
- RFC/library interoperability test vectors;
- malformed/truncated/oversized package tests;
- wrong-recipient tests;
- signature substitution tests;
- payload bit-flip tests;
- sender/recipient binding substitution tests;
- duplicate/replay tests;
- symlink/path and interrupted-write tests;
- cross-platform package round trips;
- private-key persistence/storage review on macOS, Windows and Linux;
- dependency audit;
- manual GUI acceptance;
- independent cryptographic/security review before high-assurance deployment.

## Standards basis

- RFC 9180 — Hybrid Public Key Encryption (HPKE)
- RFC 8032 — Ed25519/EdDSA
- OWASP Cryptographic Storage Cheat Sheet
- OWASP Key Management Cheat Sheet

This document fixes the intended security properties. Exact crate selection and wire-level byte encoding remain implementation decisions and must be reviewed before code is merged.
