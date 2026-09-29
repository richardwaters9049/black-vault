# Black Vault Engineering Rules

**Updated:** 29 September 2026. Read `PROJECT_SUMMARY.md` and `DESKTOP_SECURITY_ARCHITECTURE.md` before desktop changes. The active direction is local-first Tauri/Rust desktop; `backend/` and `frontend/` are preserved legacy implementations and compatibility references.

## Security

- Never persist plaintext file contents, vault passwords or decrypted master keys; never log secrets, sensitive filenames or plaintext paths.
- Keep decrypted keys and privileged file operations in Rust-owned state. Treat the WebView as a less-trusted presentation layer.
- Use independent vault master keys and separate recipient/device identity keys. Do not claim process isolation from independent keys alone.
- Version every cryptographic format; preserve shared Rust/TypeScript vectors and explicit legacy recovery. No silent migration.
- Use authenticated encryption, fresh nonces, domain separation, safe file handling and atomic crash-recoverable encrypted checkpoints.
- In-memory SQLite is a proposed experiment, not a verified security guarantee. Test temp files, journalling and recovery.
- No mandatory cloud/server/account for core desktop vault functionality. Do not import JWT/localStorage patterns into desktop sessions.
- Never commit secrets or weaken cryptography or tests to make builds pass.

## Implementation and testing

- Prefer safe Rust and typed `Result` errors; restrict Tauri commands/capabilities to the intended local window.
- Validate all untrusted input and authorise every sensitive operation.
- Add unit, integration, compatibility and security tests for each sensitive feature.
- Sensitive actions require an audit design; diagnostic logs alone are not a durable audit implementation.
- Preserve `backend/`, `frontend/`, old encrypted records and fixtures until an explicitly approved migration.
- Use UK English. Keep changes scoped, report checks and limitations, and do not commit or push automatically.
