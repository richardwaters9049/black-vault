# Legacy Black Vault frontend

This Next.js application is a preserved web-era reference implementation. The active [desktop application](../desktop/README.md) runs offline and does not depend on this frontend, accounts or an API. Source paths remain stable for historical documentation and cryptographic compatibility tests; see the [legacy boundary](../legacy/README.md).

## Local maintenance

From `frontend/`, install the locked dependencies with `npm ci`. Development uses `npm run dev`; the API base URL is configured through `NEXT_PUBLIC_API_URL`. The Docker development stack is started from the repository root with `./legacy/start.sh start` and exposes ports only on loopback.

```bash
npx tsc --noEmit
npm run lint
npm test
npm run build
npm audit --omit=dev
```

The optional file API integration test requires a running legacy backend and migrated disposable database:

```bash
BLACK_VAULT_TEST_API=http://127.0.0.1:8080 npm test
```

It creates a disposable account and exercises encrypted file persistence, download and browser decryption. Delete the disposable database after verification.

## Authentication and errors

The browser auth store supplies a stable server snapshot, validates persisted token/user records, handles denied storage access and synchronises storage events. Logout clears in-memory state even when storage is unavailable. This remains the legacy browser-token model; desktop authentication stays native. API errors retain HTTP status and use bounded messages for malformed JSON, network failures and aborted requests.

Next.js and its ESLint configuration are aligned at 16.3.8. The [4 October audit](../docs/engineering/CODEBASE_AUDIT_2026-10-04.md) records verification and the remaining development-only ESLint dependency findings.
