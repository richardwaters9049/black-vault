# Black Vault — Architecture Overview

**Updated:** 29 September 2026  
**Status:** Desktop-first direction; Sprint 01 foundation implemented.  
**Detailed proposed security design:** [DESKTOP_SECURITY_ARCHITECTURE.md](DESKTOP_SECURITY_ARCHITECTURE.md)

## Product

Black Vault is an offline-first desktop cybersecurity application for locally encrypted file storage, isolated vault workspaces and, later, secure recipient-to-recipient sharing. The primary app must operate without a mandatory server, account, cloud connection, PostgreSQL or Docker.

## Current implementation

```text
desktop/
  React 19 + Vite + TypeScript + Tailwind CSS
              |
       typed Tauri IPC
              |
  Tauri 2 / Rust native commands
    get_desktop_info | select_file
              |
      native macOS file picker
```

Sprint 01 returns selected-file metadata only. It does not yet encrypt, import, persist or share desktop files. The release Apple Silicon macOS application was built; manual native-window and picker acceptance is outstanding.

## Proposed target architecture (not yet implemented)

```text
Tauri / React presentation
         |
restricted typed IPC
         |
Rust vault/session manager
   |       |        |
Vault A  Vault B   Vault C   (independent keys and lifecycle)
   |       |        |
Authenticated encrypted metadata and file containers
         |
Local filesystem; no mandatory network dependency
```

Rust should own decrypted vault keys, file operations and metadata access. Investigate an SQLite in-memory database per unlocked vault, reconstructed from authenticated encrypted persistent metadata. Save encrypted atomic checkpoints after mutations and explicitly test interrupted writes and recovery. Cryptographic separation within one process is not OS-level process isolation.

## Legacy application (preserved)

- `backend/`: Rust/Axum API, SQLx and PostgreSQL.
- `frontend/`: Next.js web client and browser cryptography.
- `tests/fixtures/crypto-v1.json`: shared interoperability vectors.
- `Encryption_Architecture.md`: original web-era architecture, not the desktop implementation plan.
- `28thSept2026_notes.md`: detailed legacy crypto and version 2 file-workflow checkpoint.

Retain existing encrypted records and version semantics. Do not silently reinterpret or migrate old envelopes. Sharing must use recipient public-key identity and wrapped per-file keys, never the sender's vault master key or password.

## Delivery sequence

1. Sprint 01 native acceptance and release-bundle inspection.
2. Sprint 02 Rust crypto extraction, local vault lifecycle and ephemeral metadata feasibility prototype.
3. Desktop encrypted file workflows, persistence and recovery.
4. Versioned portable sharing and recipient identity.
5. Security Centre, durable audit and integrity verification.
6. LAN transport, optional encrypted relay and cross-platform release hardening.

For exact implementation versus design status, consult `PROJECT_SUMMARY.md`.
