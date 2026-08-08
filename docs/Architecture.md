# Black Vault Architecture

## Overview

Black Vault is a secure file transfer and encrypted storage platform.

The system follows a zero-trust architecture where files are encrypted before permanent storage.

## Core Principles

- No plaintext file storage
- Minimal server trust
- Strong authentication
- Complete auditability
- Least privilege access

## Components

Frontend:
NextJS application

Backend:
Rust Axum API

Database:
PostgreSQL

Cache:
Redis

Storage:
Encrypted file objects
