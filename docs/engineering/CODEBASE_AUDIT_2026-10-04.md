# Codebase audit — 4 October 2026

## Scope and baseline

Maintenance branch: `chore/repository-audit`, based on main `8cbfcecbf55e1b9b5fa2dd85806df5dbe097982b`. The review covers the active desktop source, native command/storage boundaries, tests, manifests, launch scripts and documentation, plus security/error-handling maintenance of the preserved web implementation. It is an engineering review, not independent cryptographic certification.

## Security and locking

- Locking immediately removes protected operation panels, editor plaintext, PINs and selections while Rust drops the native session. Root-session success is recorded only after native confirmation; failures remain visible and can be retried.
- A shared operation-generation guard rejects late editor/import/contact/nested responses after unmount, view closure, vault change or a newer operation. Root list responses are rejected while locking. Regression tests cover stale plaintext and late filename metadata.
- The App owns one parent-file list used by the sidebar, editor and operation views. Removed the redundant VaultWorkspace authentication path and unused standalone selection IPC/capability. Password authentication and native destructive-action biometrics retain their existing boundaries.
- Native drop metadata rejects symlinks and files above 5 MiB before exposing the selection. Import still revalidates and screens the exact bytes natively. Encryption parameters and storage formats are unchanged.
- Native application startup returns a handled error instead of panicking with raw details. Locking still drops keys if the pending-drop mutex is poisoned.

## Errors and asynchronous state

Independent overview/contact/biometric reads no longer fail as one Promise group. An unavailable count is distinct from an empty collection; biometric status failure does not block password access. Native deletion independently enforces its authentication policy.

Successful file saves/deletes and nested transfers reconcile returned metadata without treating a later list failure as a failed mutation. Import/transfer refresh failures explicitly say that the write committed. Fixed Encryption/Decryption panels reject mismatched example or encrypted-file selections; multiple native drops clear the now-invalid selection. Comments explain these boundaries without narrating routine syntax.

Legacy browser authentication validates storage, handles blocked access, hydrates from a stable external-store snapshot and observes cross-tab changes. API handling preserves HTTP status and bounds malformed/network/abort errors. Legacy Rust auth errors are typed; JWT signing/verification explicitly uses HS256 and rejects expiration overflow. Test-only SQLx cleanup queries no longer break offline all-target compilation.

## Organisation and removal

| Location | Maintenance decision |
| --- | --- |
| `desktop/src/components/` | Removed unreachable AuthGate/AuthGlobe and duplicate VaultWorkspace, its superseded tests and unused card component |
| `desktop/src/data/` | Removed unused country-border data and its empty directory |
| `desktop/src/styles.css` | Removed unreferenced legacy animation/component rules; retained active and reduced-motion styles |
| `desktop/assets/` | Retained packaging master as icon-master.png, documented generation; removed obsolete unused SVG |
| `desktop/src-tauri/` | Retained native modules, generated icons, fuzz harness and tested transfer foundation; narrowed dead-code allowances |
| `frontend/` | Removed unused starter SVGs, form dependencies and shadcn CLI/CSS import; retained actual runtime dependencies |
| `backend/`, `frontend/`, `legacy/` | Kept stable historical paths and fixtures; README files now identify maintenance scope |
| `docs/`, `tests/fixtures/` | Preserved release/sprint evidence and shared cryptographic vectors |

An entry-point/import reachability review found no remaining unreachable active desktop TypeScript modules. This is evidence for the removed modules, not proof that every possible runtime branch is necessary. Deferred Rust transfer and identity rotation/deletion primitives remain because they are intentional, tested foundations. Default `./start.sh` launches the compiled Mac app; development is explicit. Docker legacy development ports bind to loopback.

## Local verification

All checks below ran locally on the development Mac, without starting GitHub Actions:

| Check | Result |
| --- | --- |
| Desktop TypeScript and Vitest | Passed; 45 tests |
| Desktop Rustfmt and strict all-target Clippy | Passed |
| Desktop Rust suite | 94 passed, 1 ignored native acceptance test |
| Desktop frontend/native release build | Passed; ad-hoc signed macOS app; strict codesign verification passed |
| Legacy frontend TypeScript, ESLint and production build | Passed |
| Legacy frontend suite with disposable API | 27 passed, including browser-to-Rust encrypted file round trip |
| Legacy backend Rustfmt and strict offline all-target Clippy | Passed |
| Legacy backend full suite with disposable PostgreSQL | 97 passed: 57 library, 17 auth API, 20 files API, 3 vectors |
| Legacy frontend production dependency audit | No reported vulnerabilities |
| Desktop Cargo audit | No vulnerability errors; two upstream warnings below |

The disposable API and compose database resources are removed after verification. Saved release packages are untouched. The rebuilt application is local development evidence; no public signing/notarisation, current Windows/Linux packaging or human-present Touch ID/GUI acceptance is established here. Earlier release validation remains dated historical evidence.

## Outstanding upstream dependency debt

- Desktop: `glib` 0.18.5 has [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429) and `proc-macro-error` 1.0.4 has [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370), inherited through the platform/tooling graph. These warnings remain visible; no local blanket suppression was added. Yanked yoke-derive versions were updated to 0.8.4.
- Legacy frontend: the full npm audit still reports five high-severity development-tool package findings in the braces/micromatch/fast-glob/Next ESLint chain. The offered forced fix downgrades eslint-config-next to 14.2.35, breaking alignment with Next 16.3.8; it was not applied. Production-only audit is clean. Follow an upstream compatible fix and repeat lint/build.
- Legacy backend: Cargo audit still reports [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071) for rsa 0.9.10 through sqlx-mysql in the resolved lockfile. SQLx default features are disabled; only PostgreSQL/JSON/macros/runtime/UUID/chrono/migrations are enabled. `cargo tree --locked -i rsa` reports no active dependent, and JWT uses AWS-LC without the RustCrypto provider. The lockfile finding remains visible because Cargo resolves optional SQLx dependencies; it is not a clean Cargo audit result.

These findings need compatible upstream dependency updates. Existing manual acceptance and distribution requirements remain in the [candidate checklist](../release/v0.2.0-manual-acceptance.md). Do not describe this maintenance pass as eliminating all technical debt or certifying the product.
