# Legacy Black Vault web stack

This directory is the entry point for Black Vault's **preserved historical web architecture**. It is not part of the active desktop runtime.

The legacy source remains in `../backend/` and `../frontend/` so historical documentation, commits and cryptographic compatibility work keep stable paths. The Docker definition remains at `../docker-compose.yml` because its build contexts refer to those preserved directories.

The active product is `../desktop/`: Tauri 2, Rust, React, Vite and TypeScript. Core desktop vault operations do not require this legacy API, PostgreSQL, Docker, accounts or network access.

## Legacy commands

From the repository root:

```bash
./legacy/start.sh start
./legacy/start.sh stop
./legacy/start.sh restart
./legacy/start.sh logs
```

Do not add new product features to the legacy stack. Changes here should be limited to preservation, compatibility/security fixes, or historical investigation.

Shared cryptographic fixtures live under `../tests/fixtures/`. They remain architecture-neutral test evidence and are intentionally used by the desktop Rust tests.
