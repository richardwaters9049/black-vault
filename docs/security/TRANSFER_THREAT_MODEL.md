# Black Vault Transfer Threat Model

Updated: 2 October 2026  
Scope: proposed cross-user `.blackvault` transfer protocol

This document extends the local-vault threat model. The transfer channel is assumed hostile.

## Assets

- plaintext file contents;
- plaintext filenames and sensitive manifest metadata;
- recipient decryption private keys;
- sender signing private keys;
- package content-encryption material;
- verified contact/fingerprint state;
- authenticity of sender and recipient bindings;
- integrity and freshness state for received packages.

## Adversary capabilities

Assume an attacker may:

- read, copy, delay, reorder, replay, truncate and modify package bytes in transit;
- replace a package with an attacker-created package;
- send malformed packages designed to attack the parser;
- obtain public identity files and public fingerprints;
- trick a user into importing an attacker-controlled public identity;
- inspect filenames and metadata exposed outside encryption;
- retain encrypted packages indefinitely for later attack;
- control removable media or an untrusted cloud/email transport;
- restore old copies of package files.

The base model does not assume the attacker already controls the unlocked endpoint, kernel or administrator account. Those conditions can expose plaintext or keys while legitimately in use.

## Threats and required controls

| Threat | Required control | Residual limitation |
| --- | --- | --- |
| Transport reads package | Recipient public-key envelope + authenticated payload encryption | Traffic/transport metadata may remain visible |
| Package modified | AEAD authentication + sender signature | Endpoint compromise can bypass UI trust |
| Package replaced | Sender signature + verified sender fingerprint | User can still trust the wrong fingerprint |
| Recipient substituted before sending | Explicit recipient fingerprint/trust state in send confirmation | Requires trustworthy identity verification |
| Wrong recipient opens package | Recipient-key binding; no password fallback | Compromised recipient private key defeats this |
| Replay of valid package | Random authenticated package ID + accepted-ID history | Full local-state rollback can erase replay history |
| Metadata inference | Keep human-readable manifest/file metadata inside encryption | Outer routing fields and package byte size remain observable |
| Malformed package DoS | Strict parser bounds and limits before allocation/crypto | Resource exhaustion cannot be eliminated completely |
| Algorithm downgrade | Versioned suite IDs authenticated/signed; reject unsupported critical suites | Future migration must preserve downgrade rules |
| Signature stripping | Signature is mandatory for transferable v1 packages | Unsigned legacy/local formats remain separate |
| Sender key replacement | Key changes lose verified state and require re-verification | Offline revocation distribution is limited |
| Private key copied from disk | OS secure key storage where available; never JS/localStorage | Fully compromised unlocked OS may access operations/keys |
| Key reused for multiple purposes | Independent recipient and signing key pairs | More lifecycle state to manage |
| Partial/corrupt export | Temp write + sync + atomic publish where supported + verify output | Filesystem/hardware durability still has platform limits |
| Plaintext path traversal | Package contains logical IDs/names, never authoritative extraction paths | Export destination still requires native path controls |
| Malicious decrypted document | Existing import policy gate; never execute during parsing | Static screening is not antivirus |

## Identity bootstrap risk

Cryptography cannot prove that an arbitrary public key belongs to the human name displayed beside it.

For high-sensitivity transfers, users must verify the contact fingerprint over an independent trusted channel. Black Vault must never turn an imported display name into a trusted identity automatically.

## Compromise scenarios

### Sender signing key compromised

An attacker can forge future packages as that sender until recipients learn that the key is compromised. Offline Black Vault cannot guarantee instantaneous global revocation.

### Recipient encryption key compromised

An attacker who has retained packages encrypted to that long-term recipient identity may be able to decrypt them. Protocol v1 therefore does not claim forward secrecy.

### Endpoint compromised while unlocked

An attacker controlling the user's process, administrator account or kernel may capture plaintext or invoke legitimate decryption. Black Vault cannot cryptographically protect data from the endpoint at the moment the endpoint is authorised to read it.

### Transport compromised

This is an expected condition. Confidentiality and integrity of the package should remain intact if identities and endpoints are uncompromised.

## Security invariants

1. A transfer package never contains a password needed by its recipient.
2. Recipient private keys and signing private keys never enter React/WebView state.
3. No sensitive manifest metadata is displayed before package authentication.
4. A valid package for identity A cannot be opened as identity B.
5. Changing sender, recipient, package ID, suite ID or encrypted payload invalidates authentication/signature checks.
6. Unknown critical protocol features fail closed.
7. Imported package filenames never become filesystem paths.
8. An unverified contact is visually distinct from a verified contact at send and receive time.
9. A changed public key never silently inherits verified trust.
10. A failed import leaves no trusted/partially decrypted package state.
11. Completion is shown only after the output package has been written and verified.
12. Cross-user package security never relies on a four-digit nested-vault PIN.

## Review boundary

The protocol is not yet implemented and has not undergone independent cryptographic review. These controls are requirements for implementation, not claims about the current 0.1.0 application.
