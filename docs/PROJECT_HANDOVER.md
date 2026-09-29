# Black Vault — AI Engineering Handover

> **Desktop migration notice — 29 September 2026:** This long handover is a historical reference for the original Next.js/Rust/Axum/PostgreSQL application. The active direction is an offline-first Tauri 2 desktop app on `feat/desktop-migration`. Read `PROJECT_SUMMARY.md`, `Architecture.md`, `DESKTOP_SECURITY_ARCHITECTURE.md` and `SPRINT_01_DESKTOP_FOUNDATION.md` first. The old backend/frontend and crypto fixtures must be preserved. Do not interpret the legacy API, JWT, browser-key or PostgreSQL instructions below as requirements for the new desktop runtime.

## 1. Project Overview

**Project:** Black Vault
**Repository:** `black-vault`
**Backend:** Rust
**Frontend:** Next.js / TypeScript
**Database:** PostgreSQL
**Purpose:** Build a secure, privacy-focused digital vault for storing sensitive user data and files with client-side encryption.

Black Vault is being developed as a security-focused application rather than a conventional CRUD application.

The core architectural principle is:

> **Sensitive plaintext should remain on the client wherever practical.**

The backend should primarily handle authentication, encrypted data storage, metadata, access control, and API operations. Cryptographic operations involving user secrets should be performed client-side wherever possible.

The project is being developed incrementally, with automated tests expected for security-sensitive functionality.

---

# 2. Current Repository Structure

The project currently contains:

```text
black-vault/
├── backend/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── Dockerfile
│   ├── migrations/
│   │   ├── 001_create_users.sql
│   │   └── 002_create_vault_keys.sql
│   ├── src/
│   │   ├── api/
│   │   │   ├── mod.rs
│   │   │   ├── user.rs
│   │   │   └── vault.rs
│   │   ├── config/
│   │   ├── crypto/
│   │   │   ├── mod.rs
│   │   │   ├── derivation.rs
│   │   │   ├── encryption.rs
│   │   │   ├── error.rs
│   │   │   └── keys.rs
│   │   ├── database/
│   │   │   ├── mod.rs
│   │   │   ├── user.rs
│   │   │   └── vault_keys.rs
│   │   ├── models/
│   │   ├── services/
│   │   └── main.rs
│   └── tests/
├── frontend/
│   ├── app/
│   │   ├── dashboard/
│   │   └── register/
│   ├── lib/
│   │   ├── api.ts
│   │   ├── crypto.ts
│   │   └── types.ts
│   └── package.json
├── docs/
│   ├── Encryption_Architecture.md
│   └── ...
├── docker-compose.yml
├── PROJECT_SUMMARY.md
└── README.md
```

The exact repository contents should always be checked before making assumptions.

---

# 3. Development Environment

The primary development environment is macOS.

Backend:

- Rust
- Cargo
- SQLx
- PostgreSQL
- Tokio
- Axum
- dotenvy
- UUID

Frontend:

- Next.js
- React
- TypeScript
- Bun
- Tailwind CSS
- shadcn/ui

Cryptography:

- Rust cryptographic implementation
- Browser/client-side cryptography
- `@noble/ciphers`
- `@noble/hashes`

The project uses PostgreSQL locally.

Current development database:

```text
black_vault_db
```

Current local connection:

```text
postgres://richy@localhost:5432/black_vault_db
```

Do not hard-code credentials or secrets into application source code.

---

# 4. Environment Variables

The local backend `.env` currently contains development configuration similar to:

```text
DATABASE_URL=postgres://richy@localhost:5432/black_vault_db
JWT_SECRET=black-vault-development-secret-change-this
JWT_EXPIRATION=3600
```

The JWT secret is explicitly a development secret.

It must **not** be treated as production-safe.

Future production deployment must use:

- a strong randomly generated secret
- environment/secret management
- no secrets committed to Git
- separate development/staging/production credentials

---

# 5. Database

SQLx migrations have been successfully applied.

Current migrations:

```text
001_create_users.sql
002_create_vault_keys.sql
```

PostgreSQL currently contains:

```text
_sqlx_migrations
users
vault_keys
```

The `vault_keys` table is:

```sql
CREATE TABLE vault_keys (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    encrypted_vault_key TEXT NOT NULL,
    key_derivation_salt TEXT NOT NULL,
    key_derivation_params JSONB NOT NULL,
    encryption_nonce TEXT NOT NULL,
    encryption_version TEXT NOT NULL DEFAULT '1',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_vault_keys_user_id ON vault_keys(user_id);
```

Important:

`vault_keys.user_id` is a foreign key to `users.id`.

This means a vault key cannot be created for a user that does not exist.

This previously caused a failing test because the test generated:

```rust
let user_id = Uuid::new_v4();
```

without inserting the corresponding user first.

The test was corrected so the database relationship is respected.

---

# 6. Database User Layer

The backend currently contains user database operations.

The user record contains:

```rust
pub struct UserRecord {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
}
```

Implemented operations include:

```text
create_user
find_user_by_email
find_user_by_id
```

Automated tests currently cover:

- creating users
- duplicate email rejection
- finding a user by email
- handling unknown email lookups

Test emails are generated using UUIDs to avoid collisions.

---

# 7. Vault Key Database Layer

The backend contains:

```text
backend/src/database/vault_keys.rs
```

The database record contains:

```rust
pub struct VaultKeyRecord {
    pub user_id: Uuid,
    pub encrypted_vault_key: String,
    pub key_derivation_salt: String,
    pub key_derivation_params: serde_json::Value,
    pub encryption_nonce: String,
    pub encryption_version: String,
}
```

Implemented operations:

```text
create_vault_key
get_vault_key
```

`create_vault_key` inserts the encrypted vault key and returns the stored record.

`get_vault_key` retrieves the vault key associated with a user.

The database layer deliberately does not perform cryptographic operations itself.

---

# 8. Backend Cryptography

The crypto module currently contains:

```text
backend/src/crypto/
├── mod.rs
├── derivation.rs
├── encryption.rs
├── error.rs
└── keys.rs
```

`mod.rs` exports:

```rust
pub mod derivation;
pub mod encryption;
pub mod error;
pub mod keys;
```

The error type currently contains:

```text
KeyGenerationFailed
EncryptionFailed
DecryptionFailed
KeyDerivationFailed
InvalidKeyLength
InvalidNonce
EncodingError
```

Some error variants are currently unused.

This currently produces a compiler warning:

```text
variants `KeyGenerationFailed` and `EncodingError` are never constructed
```

This is not currently breaking the build.

Future work should either:

1. use those variants appropriately, or
2. remove them if they are unnecessary.

Do not blindly suppress warnings simply to achieve a clean build.

---

# 9. Frontend Cryptography

The frontend has:

```text
frontend/lib/crypto.ts
```

The frontend crypto layer currently supports:

- vault key generation
- file key generation
- Argon2id key derivation
- AES-256-GCM encryption
- AES-256-GCM decryption
- vault key encryption
- vault key decryption
- Base64 encoding/decoding

Important constants:

```text
Vault key: 256-bit
File key: 256-bit
AES-GCM nonce: 96-bit
```

Argon2id parameters currently used:

```text
t_cost = 2
m_cost = 19456
p_cost = 1
output_len = 32
```

The cryptographic tests currently pass.

Frontend crypto tests:

```text
5 pass
0 fail
10 expect() calls
```

Tests cover:

1. Encrypt/decrypt with correct password
2. Failure with wrong password
3. Unique salt and nonce generation
4. Correct Argon2id parameters
5. 256-bit vault key generation

---

# 10. Important npm/Bun Dependency History

The original frontend implementation attempted to use:

```text
@noble/argon2
```

However:

```text
bun add @noble/argon2
```

returned:

```text
404
```

Therefore the project should **not reintroduce `@noble/argon2` without first verifying package availability and compatibility**.

Currently installed Noble packages include:

```text
@noble/ciphers
@noble/hashes
```

Current versions observed:

```text
@noble/ciphers@2.3.0
@noble/hashes@2.3.0
```

There were also import compatibility issues involving:

```text
@noble/ciphers/aes
@noble/ciphers/utils.js
```

These were resolved by adapting the implementation to the APIs actually exposed by the installed package version.

Before modifying crypto imports, inspect:

```text
node_modules/@noble/ciphers/
```

and its package exports rather than assuming an older API.

---

# 11. Test Status

The backend currently has:

```text
33 passed
0 failed
```

The frontend crypto test suite currently has:

```text
5 passed
0 failed
```

The full backend test suite must remain green after changes.

A security-sensitive change should normally include or update tests.

Never remove tests simply because they are inconvenient or failing.

---

# 12. Current Architecture

The intended architecture is approximately:

```text
                    ┌─────────────────────┐
                    │      Browser        │
                    │                     │
                    │  Next.js Frontend   │
                    │                     │
                    │  User Password      │
                    │       │             │
                    │       ▼             │
                    │    Argon2id         │
                    │       │             │
                    │       ▼             │
                    │ Derived Key         │
                    │       │             │
                    │       ▼             │
                    │ AES-256-GCM         │
                    │       │             │
                    │       ▼             │
                    │ Encrypted Vault Key │
                    └─────────┬───────────┘
                              │
                              │ HTTPS
                              ▼
                    ┌─────────────────────┐
                    │     Rust API        │
                    │      Axum           │
                    │                     │
                    │ Authentication      │
                    │ Authorization       │
                    │ Vault API           │
                    │ Database access     │
                    └─────────┬───────────┘
                              │
                              ▼
                    ┌─────────────────────┐
                    │    PostgreSQL       │
                    │                     │
                    │ users               │
                    │ vault_keys          │
                    │ encrypted data      │
                    └─────────────────────┘
```

The server should never need the user's vault password in plaintext.

---

# 13. Security Model

The primary security objective is to prevent the backend from being able to decrypt user vault contents.

The conceptual model is:

```text
User password
      │
      ▼
   Argon2id
      │
      ▼
Derived encryption key
      │
      ▼
Decrypt vault key
      │
      ▼
Vault key
      │
      ├── File key A
      ├── File key B
      ├── File key C
      └── ...
```

Individual files should eventually use separate random file encryption keys rather than using one master key directly for every file.

The encrypted vault key can then act as a protected root/key-encryption mechanism.

---

# 14. Critical Security Principles

Future AI agents must follow these principles.

## Never store plaintext passwords

Passwords must only be used for key derivation/authentication operations.

Never persist:

```text
vault password
plaintext file contents
plaintext vault secrets
```

on the backend unless there is an explicit architectural reason and security review.

## Never reuse AES-GCM nonces

Every AES-GCM encryption operation must use a unique nonce.

Random 96-bit nonces are currently being generated.

This must remain true.

## Never reuse encryption keys incorrectly

Vault keys and file keys should have clearly defined responsibilities.

Do not casually reuse one key across unrelated encryption domains.

## Never commit secrets

Do not commit:

```text
.env
JWT secrets
database passwords
API keys
private keys
test credentials
```

unless the value is deliberately fake and clearly development-only.

## Do not weaken cryptography to make tests pass

If a cryptographic test fails, investigate the implementation.

Do not disable authentication, validation, encryption, or integrity checks simply to make tests pass.

---

# 15. Authentication Roadmap

Authentication is currently under development.

The next authentication improvements should include:

1. Registration
2. Password hashing
3. Login
4. JWT generation
5. JWT validation
6. Protected routes
7. Authentication middleware
8. User identity extraction
9. Logout/token invalidation strategy
10. Refresh token strategy if required

Password hashing and vault key derivation are conceptually separate.

The login password hash is for authentication.

The vault password/key derivation mechanism is for encryption.

Do not accidentally couple the two in a way that makes the server capable of decrypting the vault.

---

# 16. Authorization Roadmap

Every protected vault operation must verify that the authenticated user owns the requested resource.

Never trust:

```text
user_id
```

supplied by the frontend.

The server should derive the authenticated identity from the validated authentication token/session.

For example:

```text
Authenticated user
        │
        ▼
JWT claims
        │
        ▼
Server-side user ID
        │
        ▼
Database query
        │
        ▼
Only that user's vault
```

A request such as:

```text
GET /vault/{another-users-id}
```

must not allow access to another user's encrypted vault data.

---

# 17. Vault API Roadmap

The vault API should eventually support:

```text
Create vault
Retrieve vault metadata
Store encrypted vault key
Retrieve encrypted vault key
Update vault configuration
Delete vault
```

Later:

```text
Create encrypted file
Upload encrypted file
Download encrypted file
Delete encrypted file
List encrypted files
```

The backend should treat encrypted payloads as opaque data wherever possible.

---

# 18. File Encryption Roadmap

The eventual file encryption model should be approximately:

```text
Random File Key
       │
       ▼
AES-256-GCM
       │
       ▼
Encrypted File
```

The file key should itself be protected by the user's vault key.

Conceptually:

```text
Vault Key
   │
   ├── encrypt File Key A
   ├── encrypt File Key B
   └── encrypt File Key C

File Key A
   │
   ▼
File A encryption
```

This allows individual file keys to be rotated without necessarily re-encrypting every file.

---

# 19. Encryption Metadata

Encrypted objects should eventually contain sufficient metadata to support cryptographic versioning.

Current vault key records contain:

```text
encrypted_vault_key
key_derivation_salt
key_derivation_params
encryption_nonce
encryption_version
```

Future encrypted file metadata may include:

```text
encryption_version
algorithm
nonce
authentication tag
key identifier
encrypted file key
ciphertext size
content hash
created_at
updated_at
```

Avoid storing redundant fields if they are already encoded by the chosen format.

The final encrypted data format should be explicitly documented.

---

# 20. Cryptographic Versioning

The current:

```text
encryption_version = "1"
```

should be treated as a foundation for future cryptographic migrations.

Future versions may change:

- KDF
- KDF parameters
- cipher
- nonce format
- key sizes
- encrypted payload format

Migration code should be able to identify the encryption version and process it appropriately.

Do not silently reinterpret old encrypted data using new cryptographic parameters.

---

# 21. API Validation

Future API endpoints must validate:

- UUID formats
- required fields
- string lengths
- email format
- password requirements
- encrypted payload structure
- nonce lengths
- key lengths
- encryption versions
- JSON metadata

Do not rely solely on TypeScript frontend validation.

The backend is the security boundary.

---

# 22. Error Handling

Avoid exposing sensitive implementation details through API errors.

Bad:

```text
Postgres error: relation users violates constraint...
```

Better:

```text
Internal server error
```

while logging the detailed error server-side.

Authentication errors should avoid revealing whether an account exists when that distinction creates an account-enumeration vulnerability.

---

# 23. Database Improvements

Future database work should include:

- timestamps using a consistent timezone strategy
- appropriate indexes
- foreign-key constraints
- unique constraints
- migration tests
- transaction boundaries
- connection pool configuration
- production database configuration
- backup/recovery strategy

Potential future tables:

```text
users
vaults
vault_keys
files
file_keys
sessions
refresh_tokens
audit_events
```

The schema should remain normalized where appropriate without overengineering prematurely.

---

# 24. Testing Strategy

The project should eventually have tests at several levels.

## Unit tests

Test:

- cryptographic primitives
- validation
- parsing
- serialization
- key generation
- authentication logic

## Database tests

Test:

- user creation
- duplicate users
- vault creation
- vault retrieval
- foreign-key behaviour
- cascade deletion
- authorization boundaries

## API integration tests

Test:

```text
register
login
authenticated request
unauthenticated request
unauthorized request
vault creation
vault retrieval
vault deletion
```

## Security tests

Explicitly test:

```text
User A cannot access User B's vault
User A cannot modify User B's vault
Invalid JWT rejected
Expired JWT rejected
Invalid encryption version rejected
Invalid nonce rejected
Wrong vault password fails
Tampered ciphertext fails
```

Security boundary tests are especially important.

---

# 25. Frontend Roadmap

The frontend currently contains registration and dashboard foundations.

Future improvements:

1. Login page
2. Authentication state
3. Protected dashboard
4. Vault unlock flow
5. Secure key lifecycle
6. File upload
7. File encryption
8. Encrypted file download/decryption
9. Vault locking
10. Automatic inactivity locking
11. Secure error handling
12. Loading/error states
13. Better UX around encryption operations

The frontend should avoid unnecessary exposure of secrets through:

- React state
- logs
- error messages
- URL parameters
- localStorage

Carefully evaluate whether sensitive keys need to exist in memory and for how long.

---

# 26. Key Lifecycle

A future implementation should explicitly define:

```text
Generate
Store
Unlock
Use
Rotate
Lock
Destroy
```

For every sensitive key.

In particular, determine when decrypted vault keys should be removed from memory.

The frontend should eventually support a clear:

```text
LOCK VAULT
```

operation.

When locked:

- decrypted vault key should no longer be available to normal application code
- sensitive state should be cleared where practical
- encrypted data can remain accessible

---

# 27. Threat Model

The project should eventually document and test against:

### Threats

- stolen database
- compromised API server
- malicious authenticated user
- unauthorized API requests
- JWT theft
- database injection
- XSS
- CSRF where applicable
- compromised browser session
- malicious file upload
- ciphertext tampering
- replay attacks
- brute-force password attacks
- credential stuffing
- account enumeration
- compromised dependencies

### Important distinction

Client-side encryption can strongly reduce the impact of a stolen database or compromised backend, but it does **not** automatically protect against:

- compromised client device
- malicious JavaScript delivered to the browser
- XSS
- stolen authenticated session
- compromised browser extensions

The security architecture must acknowledge these limitations.

---

# 28. Dependency Security

Future AI agents should periodically inspect dependencies.

Backend:

```bash
cargo audit
cargo outdated
```

Frontend:

```bash
bun audit
```

where supported.

Do not blindly upgrade cryptographic dependencies.

For crypto libraries:

1. inspect changelog
2. inspect API changes
3. inspect security advisories
4. update implementation
5. run tests
6. review encryption compatibility

---

# 29. Docker / DevOps Roadmap

The project already contains Docker configuration.

Future improvements:

- consistent development containers
- PostgreSQL container
- backend container
- frontend container
- health checks
- persistent database volume
- environment-specific configuration
- hot reload
- reproducible builds
- production Dockerfiles
- non-root containers
- minimal production images
- security scanning
- CI pipeline

Eventually:

```text
Git push
    │
    ▼
CI
 ├── Rust formatting
 ├── Rust clippy
 ├── Rust tests
 ├── SQLx checks
 ├── Frontend lint
 ├── TypeScript checks
 ├── Frontend tests
 ├── Dependency audit
 └── Security checks
```

---

# 30. Code Quality

Before committing backend changes:

```bash
cargo fmt
cargo check
cargo test
```

Consider:

```bash
cargo clippy
```

Frontend:

```bash
bunx tsc --noEmit
bun test
```

Do not commit known compilation errors.

Warnings should be investigated rather than routinely suppressed.

---

# 31. Git Workflow

The developer prefers simple Git aliases:

```text
gs = git status
ga = git add .
gc = git commit -m
gp = git push
```

Preferred commit style is category-based:

```text
Frontend: NextJS frontend added
Backend: Authentication foundation added
FullStack: Connect vault API to frontend
DevOps: Docker development environment improved
Docs: Encryption architecture documented
Security: Harden vault authorization
Tests: Add vault encryption coverage
Refactor: Simplify crypto module
Fix: Resolve vault key persistence error
Chore: Update dependencies
```

Keep commits logically separated.

Do not create one giant commit containing unrelated frontend, backend, documentation, and DevOps work when the changes can reasonably be separated.

---

# 32. Current Git State

The project has previously reached:

```text
On branch main
Your branch is up to date with 'origin/main'.
nothing to commit, working tree clean
```

Always run:

```bash
gs
```

before beginning significant work.

After changes:

```bash
gs
```

Review exactly what changed before committing.

---

# 33. Shell Clipboard Workflow

The developer has custom shell functions:

```text
cpfile
cpc
```

`cpfile` copies one file to the macOS clipboard.

Example:

```bash
cpfile src/crypto/error.rs
```

`cpc` copies multiple files with filename separators.

Example:

```bash
cpc src/crypto/derivation.rs src/crypto/encryption.rs src/crypto/keys.rs
```

This is useful when requesting code review from another AI.

When asked to inspect existing code, prefer asking the developer to use these commands rather than requesting them to manually reformat files.

---

# 34. Developer Preferences

Important working preferences:

- Commands should specify the directory when relevant.
- Do not provide code in artificial stages when the developer asks for a file.
- When asked for a complete file, provide the complete file.
- Avoid unnecessary changes outside the requested scope.
- Preserve existing architecture unless there is a clear reason to change it.
- Test changes before considering work complete.
- Prefer small, logically separated commits.
- Use UK English.
- Git commit messages use:
  `Category: Description`
- The developer uses Bun for frontend package management.
- The developer uses Rust for the backend.
- The developer wants security to be a first-class concern rather than an afterthought.

---

# 35. Immediate Next Steps

The next AI agent should work through these in order.

## Step 1 — Inspect current repository

Run:

```bash
gs
```

Then inspect:

```text
backend/src/
frontend/
docs/
backend/migrations/
docker-compose.yml
```

Do not assume the repository still exactly matches this handover.

---

## Step 2 — Verify all tests

Backend:

```bash
cd backend
cargo test
```

Frontend:

```bash
cd ../frontend
bun test
bunx tsc --noEmit
```

Resolve any regression before adding new functionality.

---

## Step 3 — Review authentication

Inspect:

```text
backend/src/api/user.rs
backend/src/api/mod.rs
backend/src/main.rs
```

Determine the current state of:

- registration
- password hashing
- JWT generation
- JWT validation
- protected routes

Then complete the authentication flow.

---

## Step 4 — Review vault API

Inspect:

```text
backend/src/api/vault.rs
backend/src/database/vault_keys.rs
frontend/lib/api.ts
frontend/lib/types.ts
```

Confirm that the authenticated user's identity controls vault access.

Do not trust arbitrary `user_id` values from the frontend.

---

## Step 5 — Complete authorization

Add tests proving:

```text
User A cannot retrieve User B's vault key.
User A cannot modify User B's vault key.
Unauthenticated users cannot access vault endpoints.
Invalid JWTs are rejected.
```

This should be considered a high-priority security milestone.

---

## Step 6 — Review cryptographic implementation

Inspect all files under:

```text
backend/src/crypto/
frontend/lib/crypto.ts
```

Verify that both sides agree on:

- key sizes
- nonce sizes
- encoding
- Argon2 parameters
- AES-GCM behaviour
- encryption version
- metadata format

Document the final format in:

```text
docs/Encryption_Architecture.md
```

---

## Step 7 — Design the encrypted file system

Before implementing file uploads, define:

```text
Vault key
File key
File metadata
Encrypted file format
Key wrapping format
Nonce handling
Integrity validation
Versioning
```

Document the design first.

Then implement.

---

# 36. Medium-Term Roadmap

After authentication and vault authorization are stable:

### Phase 1 — Authentication

```text
Registration
Login
JWT
Protected routes
Authorization middleware
```

### Phase 2 — Vault

```text
Create vault
Unlock vault
Lock vault
Retrieve encrypted vault key
Decrypt vault key client-side
```

### Phase 3 — Files

```text
Generate file key
Encrypt file client-side
Upload ciphertext
Store metadata
Download ciphertext
Decrypt client-side
```

### Phase 4 — Security

```text
Rate limiting
Security headers
Input validation
Audit logging
Session hardening
Dependency auditing
Threat model
Security testing
```

### Phase 5 — Production

```text
CI/CD
Production Docker images
Managed PostgreSQL
Secret management
Backups
Monitoring
Logging
Alerting
Disaster recovery
```

---

# 37. Long-Term Improvements

Potential future features:

- encrypted folders
- file versioning
- secure sharing
- encrypted notes
- password manager functionality
- TOTP secrets
- recovery mechanisms
- key rotation
- multi-device vault access
- device authorization
- WebAuthn/passkeys
- hardware security key support
- audit logs
- suspicious login detection
- configurable KDF parameters
- cryptographic migration framework

These should not be implemented before the fundamental authentication, authorization, encryption, and key-management model is correct.

---

# 38. What NOT To Do

Do not:

- store plaintext vault passwords
- send plaintext vault passwords to the backend unnecessarily
- trust frontend-provided user IDs
- reuse AES-GCM nonces
- disable authentication to simplify development
- commit `.env` secrets
- blindly upgrade crypto dependencies
- remove failing security tests
- suppress warnings without understanding them
- mix unrelated changes into one commit
- introduce a second cryptographic model without documenting why
- implement file storage before defining the encrypted file format
- assume the frontend is a trusted security boundary

---

# 39. Definition of Done

A feature is not considered complete merely because it compiles.

For security-sensitive features, completion should mean:

```text
Implementation complete
        │
        ▼
Unit tests
        │
        ▼
Integration tests
        │
        ▼
Authorization tests
        │
        ▼
Type checking
        │
        ▼
Linting / formatting
        │
        ▼
Security review
        │
        ▼
Documentation
        │
        ▼
Git commit
```

The final goal is a Black Vault system where:

> **The server can authenticate the user and store encrypted data without possessing the information required to decrypt the user's vault.**

That principle should guide architectural decisions going forward.
