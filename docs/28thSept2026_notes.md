# Black Vault — 28 September 2026

## Outcome

Continued the existing Rust/Axum/PostgreSQL and Next.js application. The cryptographic compatibility checkpoint passed before file storage work began. Vault lifecycle tests, existing API/database tests and frontend checks passed before adding version 2 encrypted uploads.

The initial working tree was clean at `55e90c7`. npm is the configured frontend package manager (`package-lock.json`); no dependencies were added. The `rust_test` alias was unavailable, so formatting, checking and tests were run directly. Docker's daemon was unavailable; verification used the existing local PostgreSQL server. No existing encrypted records were migrated or rewritten.

## Cryptographic checkpoint

The previous Rust code passed Base64 salt **text** into Argon2id. `SaltString::from_b64` also rejected the browser's standard padded Base64 salt. The three new cross-language tests initially failed with `Invalid salt: contains invalid character: '='`. Existing Rust-only round trips had passed and did not detect this incompatibility.

Canonical derivation now decodes a 16-byte standard Base64 salt, uses Argon2id version 19 with memory 19456 KiB, two iterations, parallelism one and a 32-byte output. Both implementations reject unsupported derivation settings before allocating Argon2 memory. TypeScript now enforces 32-byte AES keys, 12-byte nonces, canonical Base64 and vault envelope metadata. Temporary wrapping keys and password byte buffers are wiped in `finally` blocks.

Shared deterministic vectors are in `tests/fixtures/crypto-v1.json`:

| Password | Raw salt bytes | Raw nonce bytes | Expected key (Base64) |
| --- | --- | --- | --- |
| `Black Vault compatibility` | `00` through `0f` | `00` through `0b` | `BsihHysC8Y1BbqHeXV6aCeoaXdDgvGupoa3cA2u+IOY=` |
| `Café 🔐 密碼` | `10` through `1f` | `0c` through `17` | `K3B9HM2bSnmdtu5ct3RUd3F8ESpopV2nJy7D3jMXmrk=` |

Both use plaintext bytes `ff` down through `e0`; the fixture pins ciphertext and its appended 16-byte GCM tag. Rust and TypeScript independently reproduce these bytes. Fixed nonces are confined to tests. Production encryption generates fresh random nonces.

The TypeScript suite builds and invokes `backend/examples/crypto_interop.rs`, exchanging fresh ciphertext with the actual Rust crypto functions in both directions. It also tests wrong passwords, bit changes to ciphertext and tags, malformed nonce encoding and incorrect nonce length. Rust separately reads the same fixture and reproduces deterministic AES-GCM encryption.

### Existing-record compatibility

Browser-created version 1 vault envelopes retain their original algorithm and format. No backend production caller previously invoked `derive_key`; it was only used by crypto unit tests. Nevertheless, the old algorithm remains available as the explicitly named `derive_key_legacy_salt_text` recovery function, with a regression test. It is not automatically selected and is not used for new records. Any externally created old Rust envelope needs provenance and explicit recovery before rewrapping; unversioned historical records must never be guessed or silently converted.

## Vault lifecycle

The dashboard now detects missing vaults and offers setup. It retrieves existing envelopes, rejects incorrect passwords and keeps the unlocked key in a `VaultSession` memory buffer. Lock, logout and component cleanup wipe that buffer and invalidate pending unlocks so late responses cannot restore access. Registration uses the same creation helper and wipes its generated key. Authentication callbacks are stable to avoid spurious dashboard cleanup on context rerenders. Password fields are cleared after submission.

Account authentication remains separate from vault unlocking. Passwords and plaintext keys are not persisted to browser storage or PostgreSQL. Wiping accessible JavaScript arrays cannot guarantee erasure of immutable strings, historical copies, browser internals or downloaded plaintext.

## Encrypted files: version 2

- A fresh random 256-bit file key encrypts each file and its UTF-8 filename separately.
- The unlocked vault key wraps the file key.
- AES-GCM additional authenticated data separates purposes: UTF-8 `black-vault:file:v2:contents`, `black-vault:file:v2:filename` and `black-vault:file:v2:wrapped-key`.
- `encrypted_filename` and `encrypted_file_key` contain standard Base64 of `12-byte nonce || ciphertext || 16-byte tag`. The wrapped-key blob is exactly 60 bytes.
- `encryption_nonce` contains the content nonce; `encrypted_data` contains the Base64 content ciphertext and appended tag on upload. PostgreSQL stores the decoded encrypted bytes.
- Contents and filename nonces are independently random and explicitly checked to differ. Wrapped-key nonces are independently random 96-bit values; as with random GCM nonces generally, collision probability is negligible, not mathematically zero.
- `POST /files/encrypted` requires authentication and an existing vault. A single SQL statement inserts the complete record and bytes, avoiding incomplete uploads.
- Files are limited to 5 MiB, encrypted filenames to 1024 plaintext UTF-8 bytes, and requests to 7 MiB. Empty files are supported.
- Existing owner-scoped list, download and deletion routes are reused. Version 2 content cannot be overwritten through the old upload route.
- Original version 1 file records and endpoints remain intact. The new UI labels unreadable/legacy records and refuses unsupported decryption instead of interpreting them as version 2.
- Upload uncertainty is surfaced to the user with a refresh-before-retry message. Retrying after a lost success response may create a duplicate, but cannot leave a half-uploaded version 2 record.

The dashboard lists decrypted filenames only while unlocked and offers encrypted upload and local download/decryption. Downloaded plaintext is intentionally handed to the browser as a Blob; the object URL is revoked and the accessible plaintext buffer wiped afterwards. Locking removes the file controls and name references.

PostgreSQL BYTEA is retained for this bounded first implementation. Streaming/chunked encryption and object storage are separate future work; do not remove the size limit without designing nonce allocation, authenticated chunk order, quotas and failure recovery.

## Verification results

- Initial Rust crypto tests: **15 passed**, before the incompatibility tests were added.
- Initial new cross-language suite: **3 failed**, exposing padded-salt rejection.
- Final Rust suite: **92 passed** — 52 unit/database tests, 17 authentication/vault API tests, 20 file API tests and 3 shared-vector/legacy tests.
- Frontend with the live local API enabled: **16 passed**. This includes live Rust↔TypeScript exchange, lifecycle cleanup/races, metadata rejection, file encryption/tamper rejection and a real API/PostgreSQL byte-for-byte file round trip.
- Frontend ESLint and production Next.js build: **passed**.
- Rust formatting and compilation, including `SQLX_OFFLINE=true cargo check --locked`: **passed**. Two pre-existing Rust formatting differences were normalised.
- Browser: registration with vault setup, incorrect-password rejection, successful unlock, encrypted upload, persisted filename after reload/unlock, explicit locking and logout were observed working.
- Browser download: the automation tool timed out waiting for a native download event, with no page error. **Saving a decrypted file through the browser is not confirmed by that check.** API retrieval and local decryption were independently verified byte-for-byte.
- Disposable browser and API test accounts, including their associated vault/file records, were removed after verification.
- Frontend Dockerfiles now use Node 22.23.2 and `npm ci`; the prior Node 20.9 image was below the installed Noble packages' declared minimum of 20.19. Container builds were not executed because Docker was unavailable.

### Re-running

From `backend`:

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
SQLX_OFFLINE=true cargo check --locked
```

Database tests need the migrated local test database configured through `DATABASE_URL` or `backend/.env`. They create UUID-labelled test rows and remove them.

From `frontend`:

```bash
npm ci
npm test
npm run lint
npm run build
```

`npm test` also needs Cargo because it builds the interop example. The live HTTP integration test is opt-in and otherwise reports one skip:

```bash
BLACK_VAULT_TEST_API=http://localhost:8080 npm test
```

Use a disposable local database for that HTTP test. It deletes its uploaded file but leaves its generated `file-e2e-<uuid>@blackvault.local` account because there is no account-deletion API; the account ID is reported for explicit database cleanup. Do not run it against production.

## Prioritised continuation

1. Confirm native browser download/save in a normal browser and add automated browser lifecycle coverage, including setup recovery after a lost response.
2. Harden authentication/session handling: bearer tokens remain in localStorage, there is no refresh/revocation design, CORS is broad, and authentication rate limiting is absent. These are existing concerns, not claims of production readiness.
3. Add durable audit storage and access-log UI. Changed vault/file operations emit structured events without keys, passwords or filenames; tracing output alone is not a durable audit trail.
4. Add upload idempotency and account quotas, then plan streaming/object storage for larger files.
5. Review deployment defaults, disable demo credentials outside development, verify Docker builds and establish a deployment-specific security review.

No claim is made that the server can authenticate ciphertext without keys: it validates metadata and lengths, while the browser checks GCM tags. Server compromise can still delete or roll back records. This implementation is not an external cryptographic audit.
