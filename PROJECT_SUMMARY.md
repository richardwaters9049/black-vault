# Black Vault - Project Summary for AI Developers

## Project Overview

**Black Vault** is a security-focused file encryption and transfer platform built with a Rust backend and Next.js frontend. The platform is designed to handle sensitive files with privacy and security as first-class requirements, following a zero-trust architecture where files are encrypted before permanent storage.

### Current Status
- **Authentication backend**: Complete and tested
- **Frontend development**: Underway (login, register, dashboard pages implemented)
- **File encryption workflows**: Next major development stage
- **Containerisation**: Docker setup ready

---

## Project Tree

```
black-vault/
├── AGENTS.md                          # Engineering rules for AI agents
├── README.md                          # Main project documentation
├── docker-compose.yml                 # Docker orchestration
├── .env.docker                        # Docker environment variables
├── .gitignore
│
├── backend/                           # Rust API Server
│   ├── Cargo.toml                     # Rust dependencies
│   ├── Cargo.lock
│   ├── Dockerfile                     # Production Docker image
│   ├── README.md
│   ├── src/
│   │   ├── main.rs                    # Application entry point
│   │   ├── api/                       # API route handlers
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs                # Register/login endpoints
│   │   │   ├── user.rs                # User endpoints
│   │   │   └── error.rs               # Error handling
│   │   ├── config/
│   │   │   └── mod.rs                 # Configuration management
│   │   ├── database/
│   │   │   ├── mod.rs                 # Database connection
│   │   │   └── users.rs               # User queries
│   │   ├── middleware/
│   │   │   ├── mod.rs
│   │   │   └── auth.rs                # JWT authentication middleware
│   │   ├── models/
│   │   │   ├── mod.rs
│   │   │   └── user.rs                # User data models
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs                # Authentication logic
│   │   │   └── jwt.rs                 # JWT token generation
│   │   └── crypto/                    # (Empty - encryption module to be implemented)
│   ├── migrations/
│   │   └── 001_create_users.sql       # Database schema
│   ├── tests/                         # Backend tests
│   └── docs/                          # Backend documentation
│
├── frontend/                          # Next.js Frontend
│   ├── package.json                   # Node dependencies
│   ├── package-lock.json
│   ├── tsconfig.json                  # TypeScript configuration
│   ├── next.config.ts                 # Next.js configuration
│   ├── components.json                # shadcn/ui configuration
│   ├── Dockerfile                     # Production Docker image
│   ├── Dockerfile.dev                 # Development Docker image
│   ├── AGENTS.md                      # Next.js version warning
│   ├── CLAUDE.md                      # Claude-specific notes
│   ├── README.md
│   ├── app/
│   │   ├── layout.tsx                 # Root layout
│   │   ├── page.tsx                   # Landing page
│   │   ├── globals.css                # Global styles
│   │   ├── login/
│   │   │   └── page.tsx               # Login page
│   │   ├── register/
│   │   │   └── page.tsx               # Registration page
│   │   └── dashboard/
│   │       └── page.tsx               # Dashboard page
│   ├── components/
│   │   └── ui/                        # shadcn/ui components
│   │       ├── button.tsx
│   │       ├── card.tsx
│   │       ├── input.tsx
│   │       └── label.tsx
│   ├── lib/                           # Utility functions
│   └── public/                        # Static assets
│
├── database/                          # (Empty - database data)
├── storage/                           # (Empty - encrypted file storage)
├── docs/
│   └── Architecture.md                # Architecture documentation
├── scripts/
│   └── docker-dev.sh                  # Docker development script
└── tests/                             # (Empty - integration tests)
```

---

## Technology Stack

### Backend (Rust)
- **Framework**: Axum 0.8 (async web framework)
- **Runtime**: Tokio 1 (async runtime)
- **Database**: PostgreSQL via SQLx 0.8
- **Authentication**: 
  - JWT (jsonwebtoken 11.0.0)
  - Argon2 (password hashing)
- **Serialization**: Serde + Serde JSON
- **HTTP**: Tower-HTTP (CORS, tracing)
- **Utilities**: UUID, Chrono, Regex, Tracing

### Frontend (Next.js)
- **Framework**: Next.js 16.3.1 (React 19.2.8)
- **Language**: TypeScript 5
- **Styling**: TailwindCSS 4
- **UI Components**: shadcn/ui + Base UI
- **Forms**: React Hook Form + Zod validation
- **Icons**: Lucide React

### Infrastructure
- **Containerisation**: Docker + Docker Compose
- **Database**: PostgreSQL 17 (Alpine)
- **Network**: Bridge network for service communication

---

## Architecture

```
┌─────────────────────────────┐
│        Next.js Frontend     │
│                             │
│  - Authentication UI         │
│  - File management UI       │
│  - Upload / download        │
│  - Security status          │
└──────────────┬──────────────┘
               │
               │ HTTPS / API
               ▼
┌─────────────────────────────┐
│        Rust API (Axum)       │
│                             │
│  - Authentication            │
│  - Authorisation             │
│  - File operations           │
│  - Validation                │
│  - Security controls         │
└──────────────┬──────────────┘
               │
       ┌───────┴────────┐
       ▼                ▼
┌──────────────┐  ┌──────────────┐
│ PostgreSQL   │  │ File/Object  │
│              │  │ Storage      │
│ - Users      │  │ Encrypted    │
│ - Metadata   │  │ Files        │
│ - Permissions│  │              │
└──────────────┘  └──────────────┘
```

---

## Current API Endpoints

### Public Routes
- `GET /health` - Health check
- `POST /auth/register` - User registration
- `POST /auth/login` - User login

### Protected Routes (Requires JWT)
- `GET /user/me` - Get current user info

---

## Database Schema

### Users Table
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);
```

---

## Engineering Rules (CRITICAL)

### Security Rules
- **Never store plaintext files**
- **Never log secrets**
- **Never expose encryption keys**
- **Validate all user input**

### Rust Development
- **Prefer safe Rust**
- **Avoid unsafe blocks unless justified**
- **Use Result-based error handling**

### API Development
- **All endpoints require authentication**
- **All actions require authorisation checks**
- **Every sensitive action must create an audit event**

### Testing Requirements
Every feature requires:
- Unit tests
- Integration tests
- Security review

---

## Core Principles

1. **Security First** - Every feature considered from security perspective before implementation
2. **Least Privilege** - Users, services, and database operations have only required access
3. **Defence in Depth** - Multiple security layers (auth, authorisation, validation, encryption, logging)
4. **Minimise Plaintext Exposure** - Sensitive data exists only where required and for minimal time
5. **Fail Securely** - Unexpected failures should not grant access to protected resources
6. **No Security by Obscurity** - Security from sound design, not hiding implementation
7. **Clear Separation** - Frontend, API, auth, business logic, database, crypto have defined responsibilities

---

## Development Setup

### Prerequisites
- Docker and Docker Compose
- Rust (for local backend development)
- Node.js 20+ (for local frontend development)

### Running with Docker
```bash
# Set required environment variables
export JWT_SECRET="your-secret-key"
export JWT_EXPIRATION="3600"

# Start all services
docker-compose up --build

# Services will be available at:
# - Frontend: http://localhost:3000
# - Backend API: http://localhost:8080
# - PostgreSQL: localhost:5432
```

### Local Development
```bash
# Backend
cd backend
cargo run

# Frontend
cd frontend
npm install
npm run dev
```

---

## Important Notes for AI Developers

### Next.js Version Warning
The frontend uses Next.js 16.3.1 with breaking changes from previous versions. **Always check** `node_modules/next/dist/docs/` before writing frontend code, as APIs and conventions may differ from training data.

### Security Mindset
This is a security-focused application. Every code change must consider:
- Input validation
- Authentication/authorisation
- Data exposure
- Error handling
- Logging (never log secrets)
- Encryption key management

### Current Implementation Status
- ✅ User registration with Argon2 password hashing
- ✅ User login with JWT authentication
- ✅ Protected routes with middleware
- ✅ Frontend auth UI (login, register, dashboard)
- ⏳ File encryption (crypto module empty - to be implemented)
- ⏳ File upload/download workflows
- ⏳ File sharing/transfer functionality
- ⏳ Audit logging
- ⏳ Integration tests

### File Structure Conventions
- Backend follows standard Rust module structure
- Frontend uses Next.js App Router
- Database migrations in `backend/migrations/`
- UI components in `frontend/components/ui/` (shadcn/ui)

### Environment Variables
Required for Docker:
- `JWT_SECRET` - Secret key for JWT signing
- `JWT_EXPIRATION` - Token expiration time (seconds, default: 3600)

---

## Planned Features (Roadmap)

1. **File Encryption Module** - Implement crypto operations in `backend/src/crypto/`
2. **File Upload/Download** - Secure file transfer endpoints
3. **File Storage** - Encrypted file object storage
4. **Access Control** - File permissions and sharing
5. **Audit Logging** - Security event tracking
6. **Redis Cache** - Session management and caching
7. **Comprehensive Testing** - Unit and integration tests
8. **Security Review** - Professional security audit

---

## Contact & Context

This project follows strict security engineering practices. When working on this codebase:
- Always reference `AGENTS.md` for engineering rules
- Consider security implications of every change
- Follow the principle of least privilege
- Ensure proper error handling without exposing sensitive information
- Write tests for all new features

The goal is to create a professional security tool, not a generic SaaS application.
