# Secure Sharing Implementation Plan

Updated: 2 October 2026  
Status: partially implemented; end-user transfer workflows deferred

## Implementation status — 4 October 2026

| Phase | Current status |
| --- | --- |
| A — protocol foundation | Rust in-memory signed recipient packages, bounded parser, RFC vector and negative/corpus tests implemented; fuzz harness exists |
| B — platform key protection | OS credential abstraction and corruption/failure tests implemented; current platform acceptance recorded separately |
| C — contacts | Public identity, fingerprint, changed-key and revoked lifecycle UI/IPC implemented |
| D/E — send/receive | Native package file persistence, package workflow IPC, Received Vaults and replay history not implemented |
| F — dashboard | Local-vault dashboard/Encryption/Decryption/Contacts implemented; proposed received-vault product model deferred |
| G — hardening | Local candidate gates recorded; full cross-platform transfer and independent review not claimed |

Contact metadata/trust is currently versioned public JSON, not an authenticated defence against valid attacker-controlled local replacement. A future send/receive workflow needs an explicit trusted-state design and replay baseline. OS identity rotation exists in Rust but is not a finished UI workflow. The phase descriptions below remain requirements where not explicitly implemented.

## Product model

Black Vault becomes a secure vault-transfer application with four primary areas:

1. Dashboard
2. My Vaults
3. Received Vaults
4. Trusted Contacts

A local vault remains the working container. A transferable `.blackvault` package is an immutable encrypted export addressed to a recipient identity.

## Phase A — protocol foundation

No UI redesign depends on unfinished cryptography.

- Select reviewed Rust crates implementing the required RFC primitives.
- Freeze canonical package encoding and maximum field/package sizes.
- Implement identity key types with strict separation between encryption and signing.
- Implement deterministic public identity fingerprints.
- Implement package parser/serializer with no filesystem side effects.
- Add RFC/library test vectors and negative tests.
- Add fuzz targets for package parsing.

Exit gate: Rust can create and open a package entirely in memory, including wrong-recipient, tamper and signature-substitution tests.

## Phase B — platform key protection

Implement a small Rust key-storage abstraction.

Current design:

- store the versioned X25519 private key and Ed25519 seed as one binary credential;
- use macOS Keychain, Windows Credential Manager and Linux Secret Service;
- fail explicitly if secure storage is unavailable; there is no plaintext file fallback;
- derive public keys and fingerprints from the private identity instead of persisting a second source of truth;
- treat corrupt or unknown identity records as errors rather than silently replacing the user's identity;
- keep creation, loading, deletion and rotation entirely Rust-side.

Do not claim Secure Enclave, hardware-backed or non-exportable keys: the selected cross-platform credential API does not guarantee those properties for these key types.

Platform evidence starts with an explicit macOS Keychain binary-secret acceptance test. Windows Credential Manager and Linux Secret Service require their own cross-platform CI/manual acceptance gates.

Exit gate: private identity material survives restart without entering WebView/browser storage and has platform-specific tests/manual acceptance.

## Phase C — contacts and identity verification

Add:

- public identity export/import;
- fingerprint display;
- unverified/verified/key-changed/revoked states;
- explicit fingerprint verification workflow;
- no automatic trust from display names;
- duplicate/key-change handling.

Exit gate: replacing a trusted contact's key cannot silently retain trusted status.

## Phase D — package export

Inside a local vault:

1. select one or more encrypted files;
2. choose a recipient;
3. confirm recipient fingerprint/trust status;
4. create encrypted package in Rust;
5. display real operation stages;
6. atomically save the completed `.blackvault` package.

The UI must not receive plaintext private keys or content-encryption keys.

Exit gate: packages created on macOS, Windows and Linux are byte-format interoperable and open on the other platforms.

## Phase E — received vaults

Add a Received Vaults area.

Before unlock, show only authenticated/safe outer information. After verification/decryption, show the sender trust state and decrypted private manifest.

States:

- package detected;
- verifying format;
- verifying sender;
- addressed to you;
- decrypting manifest;
- verified and locked;
- open;
- duplicate/replayed;
- invalid/rejected.

Exit gate: malformed, tampered, wrong-recipient and unsupported packages never expose filenames/content.

## Phase F — dashboard UX

Reorganise the current React UI around the product model:

```text
Dashboard
├── My Vaults
│   ├── Create vault
│   └── Open vault
│       ├── Files
│       ├── Encrypt
│       └── Decrypt
├── Received Vaults
└── Trusted Contacts
```

Nested vaults remain a local compartment feature and are not presented as the cross-user transfer mechanism.

## Phase G — hardening gate

Before calling secure sharing complete:

- full Rust/UI regression suite;
- parser fuzzing campaign;
- dependency audits;
- cryptographic test vectors;
- corruption and interruption matrices;
- cross-platform round trips;
- metadata leakage review;
- key compromise/rotation exercises;
- GUI privacy review;
- external security/cryptographic review.

## Progress UX

Use stage events generated by Rust:

```text
Preparing
Scanning
Encrypting
Binding recipient
Signing
Writing
Verifying
Complete
```

Receive:

```text
Reading package
Checking format
Verifying sender
Checking recipient
Decrypting manifest
Verifying contents
Ready
```

No simulated percentages. Byte percentages may be introduced later only for genuinely measurable streaming work.

## What we reuse

- Rust-controlled filesystem boundary
- AES-256-GCM local-vault machinery where its invariants fit
- Argon2id local vault-password wrapping
- zeroising secret buffers
- bounded regular-file reads/no-follow behaviour
- import policy scanner
- durable temp/sync/rename write patterns
- inactivity/session lock
- existing failure/recovery testing patterns

## What is new

- asymmetric user identities
- trusted-contact state
- recipient public-key encryption/envelope
- sender signatures
- transferable package parser/serializer
- replay history
- key rotation/revocation state
- received-vault workflow
- cross-platform private-key storage abstraction

## Deliberately deferred

- cloud accounts
- central key directory
- server-side file relay
- multi-recipient groups
- automatic revocation service
- anonymous transfer
- mobile clients
- public distribution claims

Keeping these out of v1 preserves a small protocol surface and allows the offline security model to remain understandable.
