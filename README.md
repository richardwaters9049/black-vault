# Black Vault

Black Vault is a security-focused file encryption and transfer platform designed for handling sensitive files with privacy and security as first-class requirements.

The project is being built as a full-stack application with a Rust backend and a Next.js frontend. The long-term goal is to provide a controlled environment where users can securely encrypt, upload, store, transfer and decrypt sensitive files without exposing plaintext data unnecessarily to the server.

> **Project status:** Authentication backend complete and tested. Frontend development is underway. File encryption, secure file storage/transfer workflows and full containerisation are the next major development stages.

---

## Table of Contents

- [Project Aim](#project-aim)
- [Expected Outcome](#expected-outcome)
- [Core Principles](#core-principles)
- [Architecture](#architecture)
- [Technology Stack](#technology-stack)
- [Current Features](#current-features)
- [Authentication](#authentication)
- [API](#api)
- [Frontend](#frontend)
- [Security Model](#security-model)
- [Planned Encryption Workflow](#planned-encryption-workflow)
- [Docker and Deployment](#docker-and-deployment)
- [Project Structure](#project-structure)
- [Environment Configuration](#environment-configuration)
- [Running the Project](#running-the-project)
- [Testing](#testing)
- [Development Principles](#development-principles)
- [Roadmap](#roadmap)
- [Expected Final Product](#expected-final-product)
- [Licence](#licence)

---

## Project Aim

The aim of Black Vault is to build a secure, modern platform for protecting and transferring sensitive files.

The system should minimise unnecessary exposure of plaintext information and use strong cryptographic and application-security practices throughout the entire lifecycle of a file.

The platform is intended to provide:

- Secure user authentication
- Strong password protection
- Secure file encryption and decryption
- Controlled file uploads and downloads
- Secure file sharing and transfer
- Protected storage
- Clear ownership and access controls
- Auditable security events
- A simple and professional security-focused user interface
- A deployment model suitable for containerised environments

Security is not intended to be added after the application is built. It is a core architectural requirement.

---

## Expected Outcome

The finished Black Vault platform should allow an authenticated user to:

1. Create and securely access an account.
2. Upload a sensitive file.
3. Encrypt the file using strong modern cryptography.
4. Store or transfer the encrypted file without unnecessarily exposing plaintext.
5. Control who can access a protected file.
6. Download an encrypted file.
7. Decrypt the file when authorised.
8. Remove files securely.
9. Manage their account and protected resources.
10. Understand the security state of their files through a clear interface.

The final product should feel like a serious security tool rather than a generic SaaS application.

---

## Core Principles

### Security First

Every feature should be considered from a security perspective before implementation.

### Least Privilege

Users, services and database operations should have only the access they require.

### Defence in Depth

Security should not depend on a single mechanism. Authentication, authorisation, validation, encryption, secure transport, database constraints and logging should work together.

### Minimise Plaintext Exposure

Sensitive plaintext data should exist only where it is required and for as short a period as practical.

### Fail Securely

Unexpected failures should not accidentally grant access to protected resources.

### No Security by Obscurity

Security should come from sound cryptographic and architectural design rather than hiding URLs, endpoints or implementation details.

### Clear Separation of Responsibilities

The frontend, API, authentication layer, business logic, database and cryptographic operations should have clearly defined responsibilities.

---

## Architecture

Black Vault is being designed as a full-stack application:

```text
┌─────────────────────────────┐
│        Next.js Frontend     │
│                             │
│  Authentication             │
│  File management UI         │
│  Upload / download          │
│  Security status            │
└──────────────┬──────────────┘
               │
               │ HTTPS / API
               ▼
┌─────────────────────────────┐
│        Rust API             │
│          Axum               │
│                             │
│  Authentication             │
│  Authorisation              │
│  File operations            │
│  Validation                 │
│  Security controls          │
└──────────────┬──────────────┘
               │
       ┌───────┴────────┐
       ▼                ▼
┌──────────────┐  ┌──────────────┐
│ PostgreSQL   │  │ File/Object  │
│              │  │ Storage      │
│ Users        │  │ Encrypted    │
│ Metadata     │  │ Files        │
│ Permissions  │  │              │
└──────────────┘  └──────────────┘