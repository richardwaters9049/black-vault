# Black Vault Desktop

**Current candidate:** 0.2.0-rc.1. This is the active offline Tauri application. The web-era backend and frontend are historical references and are not runtime dependencies.

## Development and packaging

**Directory:** `/Users/richy/Documents/Github/cyber-work/black-vault/desktop` on the development Mac, or `desktop/` in another checkout.

```bash
npm ci
npm run desktop:dev
```

Vite binds to `127.0.0.1:1420`; Tauri provides native IPC and the application window. Browser-only preview cannot perform protected native operations.

```bash
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run desktop:build
```

Requires Node.js 22.12+, Rust 1.90+ and native Tauri prerequisites. On macOS, packaging creates `src-tauri/target/release/bundle/macos/Black Vault.app`. The app includes its frontend and needs no development server. Minimum configured macOS version is 13.3; the tested native target is Apple Silicon. Ad-hoc signing is not Developer ID signing or notarisation.

## Workspace and authentication

Startup locks any prior Rust session before native initialisation completes. The Overview may display non-sensitive information while locked. Encryption and Decryption require native vault creation/unlock before protected file controls appear. Neither tab prefills a password, including development builds. The UI cannot create a session without a successful Rust authentication response.

Lock/sign-out clears protected UI state after native locking; inactivity requests a lock after five minutes and checks wall-clock expiry after delayed timers or focus changes. This is not a native OS sleep listener.

Touch ID is confined to native confirmation of permanent vault deletion when available. It does not unlock a vault or recover a password. There is no standalone biometric-success IPC command. See the [biometric security model](../docs/security/BIOMETRIC_SECURITY_MODEL.md).

## Native command boundary

`src/lib/native.ts` maps typed calls to `src-tauri/src/commands.rs` and `contact_commands.rs`. `build.rs` registers the commands; `capabilities/default.json` permits them only for the local `main` window. No generic filesystem, shell or HTTP permission is granted. Production CSP restricts resources to the application and Tauri IPC.

| Command family | Authority and returned data |
| --- | --- |
| Vault lifecycle | Rust validates passwords/IDs; locked display names remain unavailable |
| Picker/drop import | Native selection is revalidated in Rust; no arbitrary JS filesystem paths |
| File and nested operations | Require the native root session and, where applicable, nested PIN |
| Text editor | Authorised plaintext in WebView memory; saves re-encrypt into the vault |
| Export | Explicit native save selection creates a plaintext copy; refuses unsafe overwrite |
| Locked-vault deletion | Exact typed confirmation plus fresh native biometrics when available |
| Identity/contact commands | Public identity and contact metadata only; private identity keys stay native |

Error payloads are bounded `{ code, message }` records. Release diagnostics do not log secrets, filenames or paths. Rust keeps active key material in best-effort zeroising containers; this is not guaranteed erasure from every OS/runtime copy.

## Storage and limits

Local vault files contain authenticated encrypted headers, filenames and contents, with per-vault and per-file key separation. Imports are bounded to 5 MiB and screened before encryption/persistence. The scanner is a conservative static policy, not antivirus. Nested four-digit PINs are convenience locks additionally protected by the parent vault key; they do not provide VM isolation or replace a strong master password.

Public contact documents and trust state are versioned JSON with bounded reads, strict validation, restrictive Unix permissions and atomic writes. They are not encrypted vault metadata or authenticated against arbitrary valid local replacement. OS credential storage separately protects the private transfer identity and fails without a plaintext fallback. Backing up vault files alone does not back up the OS identity.

## Transfer scope

`transfer.rs` and `transfer_codec.rs` implement Rust-only recipient encryption, sender signatures and parsing; a libFuzzer harness and deterministic hostile-input tests exercise decoding. Trusted Contacts implements public identity/fingerprint management. Package send/receive IPC, native package persistence, received-vault UI, replay history and complete transfer acceptance are deferred. See the [implementation status](../docs/architecture/SECURE_SHARING_IMPLEMENTATION_PLAN.md).

## Verification and acceptance

See the dated [candidate validation](../docs/release/v0.2.0-validation.md). Routine CI uses Linux checks; release branches/manual dispatch run the three-OS packaging matrix. The Linux matrix validates local vault functionality even without a graphical Secret Service session; that does not establish Linux identity-store acceptance.

The [candidate manual checklist](../docs/release/v0.2.0-manual-acceptance.md) covers packaged GUI, real Touch ID, clipboard, restart, backup/restore, keyboard navigation and OS acceptance. Use disposable data. Test success with mocked IPC does not prove these native interactions.

## References

- [As-built architecture](../docs/architecture/Architecture.md)
- [Desktop security architecture](../docs/architecture/DESKTOP_SECURITY_ARCHITECTURE.md)
- [Threat model](../docs/security/THREAT_MODEL.md)
- [Historical v0.1.0 record](../docs/release/v0.1.0-validation.md)
