# Black Vault – Latest Development Summary

**Date: 07/09/2026**

## Latest Changes

### Development Demo User

Added a development-only seeded demo account to simplify testing of authenticated features.

**Credentials:**

- **Email:** `demo@blackvault.local`
- **Password:** `BlackVaultDev123!`

The backend checks for the account during startup after database migrations. If it does not exist, the password is securely hashed using the existing authentication service and the user is created. If it already exists, nothing changes.

Demo-user creation is controlled through Docker environment variables so it can be disabled outside development.

### SQLx Offline Cache

Regenerated the `backend/.sqlx/` query cache after the database query changes.

This is required because Docker builds the Rust backend with `SQLX_OFFLINE=true`, allowing it to compile without requiring PostgreSQL during the image build.

### Docker

Updated Docker Compose to support the demo-user configuration while retaining the dynamic host-port setup, allowing Black Vault to run alongside other local projects without fixed host ports.

## Current Status

The full Docker development environment is running successfully.

- PostgreSQL running
- SQLx migrations running automatically
- Rust backend online
- Next.js frontend online
- Demo login working
- Authenticated dashboard accessible

## Next Steps

1. **Test authentication and dashboard functionality.**
2. **Complete and test the vault setup/key-management flow.**
3. **Continue the encrypted file upload, download and decryption workflow.**
4. **Expand security and authorisation tests**, particularly file ownership and JWT validation.
5. **Begin production hardening**, including disabling demo users, securing secrets and reviewing the cryptographic architecture.
