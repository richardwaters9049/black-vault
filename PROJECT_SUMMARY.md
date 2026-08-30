# Black Vault - Project Summary for AI Developers

**Last updated:** Saturday 29 August 2026, 22:23 BST

## Project Overview

**Black Vault** is a security-focused digital vault and encrypted file platform built with a Rust backend, a Next.js frontend, and PostgreSQL. The application is designed around client-side encryption: sensitive plaintext and user-controlled encryption keys should remain in the browser wherever practical, while the backend handles authentication, authorisation, encrypted metadata, opaque encrypted payloads, and persistence.

The project should be treated as a security application, not a generic CRUD product.

The core principle is:

> Sensitive plaintext should remain on the client wherever practical.

## Current Status

- **Authentication backend:** Implemented and tested.
- **JWT-protected routes:** Implemented for user and vault APIs.
- **Vault key storage:** Implemented through the `vault_keys` table and protected API routes.
- **Frontend auth UI:** Login, registration, and dashboard pages exist.
- **Frontend crypto:** Vault key generation, Argon2id derivation, AES-GCM encryption/decryption, and tests exist.
- **Backend crypto module:** Implemented and tested, but not yet central to runtime file workflows.
- **Error handling:** Recently improved with structured API response status handling.
- **Vault metadata validation:** Server now validates vault setup metadata before persistence.
- **File workflows:** Not yet implemented; this is the next major product stage.
- **Audit logging:** Not yet implemented, but required for sensitive actions.
- **Project docs:** `PROJECT_HANDOVER.md` is detailed and useful; this summary is now refreshed to match current code.

## Current Repository Shape

```text
black-vault/
├── AGENTS.md
├── PROJECT_HANDOVER.md
├── PROJECT_SUMMARY.md
├── README.md
├── docker-compose.yml
├── backend/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── Dockerfile
│   ├── migrations/
│   │   ├── 001_create_users.sql
│   │   └── 002_create_vault_keys.sql
│   └── src/
│       ├── lib.rs
│       ├── main.rs
│       ├── api/
│       │   ├── auth.rs
│       │   ├── error.rs
│       │   ├── mod.rs
│       │   ├── user.rs
│       │   └── vault.rs
│       ├── config/
│       ├── crypto/
│       │   ├── derivation.rs
│       │   ├── encryption.rs
│       │   ├── error.rs
│       │   ├── keys.rs
│       │   └── mod.rs
│       ├── database/
│       │   ├── mod.rs
│       │   ├── users.rs
│       │   └── vault_keys.rs
│       ├── middleware/
│       ├── models/
│       └── services/
├── frontend/
│   ├── app/
│   │   ├── dashboard/
│   │   ├── login/
│   │   ├── register/
│   │   ├── layout.tsx
│   │   ├── page.tsx
│   │   └── globals.css
│   ├── components/
│   ├── lib/
│   │   ├── api.ts
│   │   ├── auth-context.tsx
│   │   ├── crypto.test.ts
│   │   ├── crypto.ts
│   │   ├── types.ts
│   │   └── utils.ts
│   ├── bun.lock
│   └── package.json
├── docs/
│   ├── Architecture.md
│   └── Encryption_Architecture.md
├── database/
├── scripts/
├── storage/
└── tests/
```

Always inspect the live repository before acting; this project is moving quickly.

## Technology Stack

### Backend

- Rust 2021
- Axum 0.8
- Tokio 1
- SQLx 0.8
- PostgreSQL
- Argon2 for authentication password hashing
- JSON Web Tokens via `jsonwebtoken`
- AES-GCM, Argon2, Base64, UUID, Chrono, Regex, Tracing

### Frontend

- Next.js 16.3.1
- React 19.2.8
- TypeScript 5
- Bun package/runtime workflow
- Tailwind CSS 4
- shadcn/ui and Base UI
- Noble crypto packages:
  - `@noble/ciphers`
  - `@noble/hashes`

### Infrastructure

- Docker and Docker Compose
- PostgreSQL 17 Alpine
- Local development database currently documented as `black_vault_db`

## Current API Endpoints

### Public

- `GET /health` - Health check.
- `POST /auth/register` - Create a user account.
- `POST /auth/login` - Authenticate and return a JWT.

### Protected

- `GET /user/me` - Return authenticated user details.
- `POST /vault/setup` - Store the authenticated user's encrypted vault key metadata.
- `GET /vault/key` - Retrieve the authenticated user's encrypted vault key metadata.

Protected routes use JWT middleware. User identity must be derived from the validated token, not from frontend-supplied `user_id` values.

## Database Schema

### `users`

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);
```

### `vault_keys`

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

The `vault_keys.user_id` foreign key means vault key records require an existing user. Tests must insert a user before creating a vault key.

## Security Model

Black Vault separates authentication from vault encryption:

- The **login password** is used for authentication and is hashed server-side with Argon2.
- The **vault password** is used client-side to derive an encryption key.
- The **vault key** is generated client-side.
- The **encrypted vault key** is stored server-side.
- The backend should not receive the plaintext vault password or plaintext vault key.

Current vault key envelope fields:

```text
encrypted_vault_key
key_derivation_salt
key_derivation_params
encryption_nonce
encryption_version
```

Current frontend crypto constants:

```text
Vault key: 32 bytes
File key: 32 bytes
AES-GCM nonce: 12 bytes
Argon2id output: 32 bytes
```

Current Argon2id parameters:

```text
t_cost = 2
m_cost = 19456
p_cost = 1
output_len = 32
```

## Error Handling State

The API now uses `ErrorResponse` constructors for explicit status handling:

- `bad_request`
- `conflict`
- `not_found`
- `internal_server_error`
- `from_service_error`

The JSON response shape remains:

```json
{
  "error": "Message"
}
```

Important direction:

- Avoid exposing database or cryptographic internals to clients.
- Log detailed server-side failures with `tracing`.
- Return stable, minimal client-facing messages.
- Move auth service errors from `String` to a typed enum in a future hardening pass.

## Recently Validated

The most recent known validation pass completed successfully:

```bash
cd backend
cargo test
cargo clippy --all-targets -- -D warnings

cd ../frontend
bun test
bunx tsc --noEmit
bun run lint
```

Observed results:

- Backend tests: `37 passed`
- Strict backend Clippy: passed
- Frontend crypto tests: `5 passed`
- TypeScript check: passed
- Frontend lint: passed

## Engineering Rules

From `AGENTS.md`:

- Never store plaintext files.
- Never log secrets.
- Never expose encryption keys.
- Validate all user input.
- Prefer safe Rust.
- Use `Result`-based error handling.
- All endpoints require authentication except deliberate public routes.
- All actions require authorisation checks.
- Every sensitive action must create an audit event.
- Every feature requires unit tests, integration tests, and security review.

## Known Gaps

- `PROJECT_HANDOVER.md` and this summary should remain aligned after future milestones.
- Auth service errors are still string-based internally.
- Vault API has validation tests, but needs full API integration tests.
- No audit event table or audit logging exists yet.
- No file metadata/file storage schema exists yet.
- No encrypted upload/download workflow exists yet.
- Dashboard unlock currently verifies the vault password but does not yet manage a complete key lifecycle for file operations.
- JWTs are stored in `localStorage`, which is acceptable for early development but should be revisited during security hardening.
- CORS is permissive in development and needs environment-aware production configuration.
- Timestamps should eventually use a deliberate timezone strategy.

## Immediate Next Steps

1. **Add backend API integration tests**
   - Registration.
   - Login.
   - `GET /user/me`.
   - Unauthenticated protected-route rejection.
   - Vault setup.
   - Vault key retrieval.
   - Duplicate vault setup rejection.
   - Invalid vault metadata rejection.

2. **Introduce typed auth/service errors**
   - Replace `Result<T, String>` in auth/JWT service layers.
   - Map typed errors to API responses at the edge.
   - Keep client-facing messages stable and non-leaky.

3. **Design audit logging**
   - Add `audit_events` migration.
   - Record authentication and vault setup/retrieval events.
   - Ensure no secrets, plaintext, or encrypted key material is logged.

4. **Design encrypted file metadata**
   - Add schema for encrypted files and wrapped file keys.
   - Define ownership checks.
   - Document encryption versioning and metadata format before implementation.

5. **Build file upload/download flow**
   - Encrypt files client-side.
   - Store encrypted payloads only.
   - Store metadata and wrapped file keys server-side.
   - Test tampering, ownership, and invalid metadata cases.

6. **Refresh frontend vault lifecycle**
   - Explicit unlock state.
   - Lock operation clears sensitive key material.
   - Avoid persistent storage of decrypted keys.
   - Prepare dashboard for encrypted file actions.

## Development Commands

Backend:

```bash
cd backend
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
```

Frontend:

```bash
cd frontend
bun install
bun test
bunx tsc --noEmit
bun run lint
```

Full local stack:

```bash
docker-compose up --build
```

## Commit Style

Preferred format:

```text
Backend/API: Improve vault error handling
Backend/Security: Add vault metadata validation
Frontend/API: Harden client error parsing
Docs/Summary: Refresh project status
FullStack/Security: Connect vault validation flow
Tests/API: Add vault integration coverage
```

Keep commits small and logically grouped.

## Working Notes For Future Agents

- Use UK English.
- Use Bun for frontend work.
- Do not reintroduce `@noble/argon2` without checking package availability and compatibility.
- Check installed Noble package exports before changing crypto imports.
- Do not weaken cryptography to satisfy tests.
- Do not remove tests because they are inconvenient.
- Do not commit secrets or `.env`.
- Treat the backend as the security boundary, even when the frontend validates too.
- Prefer surgical changes that preserve the existing architecture.
