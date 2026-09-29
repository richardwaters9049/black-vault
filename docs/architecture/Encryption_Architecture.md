# Black Vault Encryption Architecture

> **Historical web architecture — updated context 29 September 2026:** This document describes the original Next.js/Axum/PostgreSQL model, not the current desktop implementation plan. For the desktop direction and proposed security model, read [../project/PROJECT_SUMMARY.md](../project/PROJECT_SUMMARY.md) and [DESKTOP_SECURITY_ARCHITECTURE.md](DESKTOP_SECURITY_ARCHITECTURE.md). Retain this document for legacy compatibility; do not treat its browser sessions, JWT or PostgreSQL assumptions as desktop requirements.

## Overview

This document defines the encryption architecture for Black Vault, establishing the relationship between authentication, vault state, encryption capability, file keys, and encrypted files.

## Core Design Principles

1. **Separation of Concerns**: Authentication and encryption are separate capabilities
2. **Zero-Trust Backend**: The Rust backend never receives or stores plaintext files
3. **Client-Side Encryption**: All encryption/decryption operations happen in the frontend
4. **Key Hierarchy**: Master key protects individual file keys
5. **Minimal Plaintext Exposure**: Plaintext exists only where absolutely required

## Architecture Components

### 1. Authentication Layer (Existing)

**Purpose**: Grant access to the Black Vault API and dashboard

**Current Implementation**:
- User password → Argon2 hash → stored in database
- Successful login → JWT token → API access
- JWT stored in localStorage → session persistence

**Relationship to Encryption**: Authentication grants access to the vault but does NOT grant encryption capability.

### 2. Vault Key (New Component)

**Purpose**: Master encryption key that protects all file encryption keys

**Properties**:
- Generated randomly during registration (256-bit / 32 bytes)
- Never stored in plaintext
- Encrypted with a key derived from the user's vault password
- Stored in database as encrypted blob
- Kept in memory (session storage) when vault is unlocked
- Cleared from memory when vault is locked

**Lifecycle**:
1. **Registration**: Generate random Vault Key → encrypt with vault password → store encrypted Vault Key
2. **Login**: User provides vault password → decrypt Vault Key → store in session storage
3. **Vault Lock**: Clear Vault Key from session storage
4. **Vault Unlock**: User provides vault password → decrypt Vault Key → store in session storage
5. **Password Change**: Decrypt Vault Key with old password → re-encrypt with new password → update database

### 3. Vault Password (New Component)

**Purpose**: Separate password for unlocking encryption capability

**Properties**:
- Can be the same as or different from authentication password
- Used to derive an encryption key for the Vault Key
- Never stored in plaintext
- Never sent to the backend
- Used only client-side for Vault Key encryption/decryption

**Security Note**: The vault password is used for key derivation, not authentication. It never leaves the client.

### 4. File Encryption Keys (New Component)

**Purpose**: Encrypt individual files

**Properties**:
- Generated randomly for each file (256-bit / 32 bytes)
- Encrypted with the Vault Key
- Stored in database alongside file metadata
- Never stored in plaintext
- Never sent to backend in plaintext

**File Sharing**: When sharing a file, the file key is encrypted with the recipient's Vault Key and stored as an access grant.

### 5. Encrypted Files (New Component)

**Purpose**: Store encrypted file data

**Properties**:
- Encrypted with file-specific encryption key
- Stored in object storage (not database)
- Backend only handles encrypted blobs
- Never decrypted on the server

## Component Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                         Frontend (Next.js)                      │
│                                                                  │
│  ┌──────────────────┐         ┌──────────────────┐              │
│  │  Authentication  │         │  Vault State     │              │
│  │                  │         │                  │              │
│  │  • JWT Token     │         │  • Vault Key     │              │
│  │  • User ID       │         │  • Unlocked?     │              │
│  │  • localStorage  │         │  • sessionStorage │              │
│  └──────────────────┘         └──────────────────┘              │
│           │                              │                        │
│           │ Grants API access            │ Enables encryption     │
│           ▼                              ▼                        │
│  ┌──────────────────────────────────────────────────┐           │
│  │              Encryption Operations                │           │
│  │                                                   │           │
│  │  • Generate file keys                            │           │
│  │  • Encrypt files with file keys                 │           │
│  │  • Decrypt files with file keys                 │           │
│  │  • Encrypt file keys with Vault Key             │           │
│  │  • Decrypt file keys with Vault Key             │           │
│  └──────────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ HTTPS (encrypted data only)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Backend (Rust/Axum)                        │
│                                                                  │
│  ┌──────────────────┐         ┌──────────────────┐              │
│  │  Auth Middleware │         │  File API        │              │
│  │                  │         │                  │              │
│  │  • Validate JWT  │         │  • Upload        │              │
│  │  • Extract User │         │  • Download      │              │
│  └──────────────────┘         │  • Metadata      │              │
│           │                   └──────────────────┘              │
│           │                              │                        │
│           ▼                              ▼                        │
│  ┌──────────────────────────────────────────────────┐           │
│  │                    PostgreSQL                     │           │
│  │                                                   │           │
│  │  • users (id, email, password_hash)             │           │
│  │  • vault_keys (user_id, encrypted_vault_key)     │           │
│  │  • files (id, user_id, name, encrypted_key)     │           │
│  │  • file_shares (file_id, recipient_id, key)      │           │
│  └──────────────────────────────────────────────────┘           │
│                              │                                      │
│                              ▼                                      │
│  ┌──────────────────────────────────────────────────┐           │
│  │              Object Storage                       │           │
│  │                                                   │           │
│  │  • Encrypted file blobs (never decrypted)        │           │
│  └──────────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────────┘
```

## User Flows

### Registration Flow

1. User provides email, authentication password, and vault password
2. Frontend generates random Vault Key
3. Frontend derives encryption key from vault password (using Argon2 or similar)
4. Frontend encrypts Vault Key with derived key
5. Frontend sends email, auth password hash, and encrypted Vault Key to backend
6. Backend stores user record with auth password hash and encrypted Vault Key
7. Frontend stores Vault Key in session storage (vault unlocked)

### Login Flow

1. User provides email and authentication password
2. Backend validates credentials, returns JWT
3. Frontend stores JWT, redirects to dashboard
4. Dashboard shows "Vault Locked" state
5. User provides vault password
6. Frontend derives encryption key from vault password
7. Frontend requests encrypted Vault Key from backend
8. Frontend decrypts Vault Key, stores in session storage
9. Vault is now unlocked, encryption operations available

### File Upload Flow

1. User selects file
2. Frontend generates random file encryption key
3. Frontend encrypts file with file key
4. Frontend encrypts file key with Vault Key
5. Frontend sends encrypted file to backend (upload endpoint)
6. Frontend sends file metadata + encrypted file key to backend
7. Backend stores encrypted file in object storage
8. Backend stores file metadata and encrypted file key in database
9. Vault Key and file key never leave the frontend

### File Download Flow

1. User requests file download
2. Backend returns file metadata and encrypted file key
3. Frontend decrypts file key with Vault Key
4. Frontend downloads encrypted file from backend
5. Frontend decrypts file with file key
6. Frontend provides decrypted file to user
7. Vault Key and file key never leave the frontend

### File Sharing Flow

1. User selects file and recipient
2. Frontend decrypts file key with Vault Key
3. Frontend requests recipient's public Vault Key (or encrypted Vault Key)
4. Frontend encrypts file key with recipient's Vault Key
5. Frontend sends access grant to backend
6. Backend stores access grant in database
7. Recipient can decrypt file key with their Vault Key
8. No re-encryption of the actual file required

### Password Change Flow

1. User provides old vault password and new vault password
2. Frontend derives old encryption key from old vault password
3. Frontend requests encrypted Vault Key from backend
4. Frontend decrypts Vault Key with old key
5. Frontend derives new encryption key from new vault password
6. Frontend encrypts Vault Key with new key
7. Frontend sends updated encrypted Vault Key to backend
8. Backend updates encrypted Vault Key in database
9. No re-encryption of any files required

### Vault Lock Flow

1. User clicks "Lock Vault"
2. Frontend clears Vault Key from session storage
3. Frontend updates UI to "Vault Locked" state
4. User remains authenticated (JWT still valid)
5. Encryption operations disabled until vault is unlocked again

## Cryptographic Boundaries

### Frontend Responsibilities

- **Vault Key Generation**: Generate random Vault Key during registration
- **Vault Key Encryption**: Encrypt Vault Key with vault password-derived key
- **Vault Key Decryption**: Decrypt Vault Key when unlocking vault
- **File Key Generation**: Generate random file keys for each file
- **File Encryption**: Encrypt files with file keys
- **File Decryption**: Decrypt files with file keys
- **File Key Encryption**: Encrypt file keys with Vault Key
- **File Key Decryption**: Decrypt file keys with Vault Key
- **Key Derivation**: Derive encryption keys from vault password
- **Vault Key Storage**: Keep Vault Key in session storage when unlocked

### Backend Responsibilities

- **Authentication**: Validate JWT tokens
- **Authorization**: Ensure user has access to requested resources
- **Encrypted Data Storage**: Store encrypted Vault Keys, file keys, and file blobs
- **Metadata Management**: Store file metadata (name, size, type, timestamps)
- **Access Control**: Manage file sharing permissions
- **Audit Logging**: Log all security-relevant events
- **Never**: Decrypt any data, store plaintext keys, or perform encryption operations

### What Never Crosses the Boundary

- **Vault Key** (plaintext)
- **File Keys** (plaintext)
- **Vault Password** (any form)
- **Plaintext Files**
- **Derived Encryption Keys**

## Database Schema Extensions

### New Table: `vault_keys`

```sql
CREATE TABLE vault_keys (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    encrypted_vault_key TEXT NOT NULL,
    key_derivation_params JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
```

### New Table: `files`

```sql
CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    encrypted_file_key TEXT NOT NULL,
    storage_path TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
```

### New Table: `file_shares`

```sql
CREATE TABLE file_shares (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    shared_by_user_id UUID NOT NULL REFERENCES users(id),
    shared_with_user_id UUID NOT NULL REFERENCES users(id),
    encrypted_file_key TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP,
    UNIQUE(file_id, shared_with_user_id)
);
```

## Security Considerations

### Key Derivation

- Use Argon2id for key derivation from vault password
- Store derivation parameters (salt, memory, iterations, parallelism) with encrypted Vault Key
- Allow future migration to stronger parameters

### Key Storage

- Vault Key: Encrypted in database, plaintext in session storage when unlocked
- File Keys: Encrypted in database, never stored plaintext
- Session Storage: Cleared on vault lock, browser close, or timeout

### Transport Security

- All API communication over HTTPS
- Encrypted data sent over HTTPS (double encryption)
- JWT sent in Authorization header

### Failure Modes

- **Vault Key Decryption Failure**: User provided wrong vault password
- **File Key Decryption Failure**: Vault Key corrupted or wrong
- **Backend Unavailable**: Cannot access encrypted Vault Key or file data
- **Session Expired**: Vault Key cleared, user must re-enter vault password

### Recovery

- **Lost Vault Password**: Cannot decrypt Vault Key → all files inaccessible
- **Lost Vault Key**: Cannot decrypt any file keys → all files inaccessible
- **Future Enhancement**: Consider recovery keys or multi-factor recovery

## Implementation Phases

### Phase 1: Vault Key Infrastructure (Current Step)
- Add `vault_keys` table to database
- Create Rust crypto module for key generation and encryption
- Add backend API endpoints for Vault Key storage/retrieval
- Update registration flow to generate and store Vault Key
- Add vault unlock UI to frontend
- Implement vault lock functionality

### Phase 2: File Encryption
- Add `files` table to database
- Implement file upload/download endpoints
- Create frontend file encryption/decryption operations
- Add file management UI

### Phase 3: File Sharing
- Add `file_shares` table to database
- Implement sharing endpoints
- Create sharing UI
- Implement access control logic

### Phase 4: Password Change
- Implement vault password change flow
- Add password change UI
- Update Vault Key re-encryption logic

### Phase 5: Audit Logging
- Add audit events table
- Implement audit logging middleware
- Create audit log viewer

## Next Steps

**Immediate Next Step (Phase 1)**:
1. Create database migration for `vault_keys` table
2. Implement Rust crypto module with key generation and encryption primitives
3. Add backend endpoints for Vault Key storage and retrieval
4. Update registration to generate and store Vault Key
5. Add vault unlock UI to dashboard
6. Implement vault lock functionality

This architecture provides a solid foundation for secure file encryption while maintaining the separation between authentication and encryption capabilities.
