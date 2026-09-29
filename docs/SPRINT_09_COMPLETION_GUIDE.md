# Sprint 09 — Completion guide

**Date:** 29/09/2026
**Working branch:** feat/desktop-file-transfer-completion

## Phase 01 — Finish file transfers

**Implementation complete; awaiting native GUI acceptance.** Individual encrypted files can now move in both directions between the unlocked parent vault and an existing PIN-protected nested vault.

- Parent → nested: open the compartment, choose a parent file from **Move from parent vault**, re-enter the nested PIN and transfer. Rust authenticates the source and all existing child envelopes, checks duplicate IDs/names and the five-file / 10 MiB limits, commits and synchronises the updated nested ciphertext **before** removing the parent envelope. No plaintext export.
- Nested → parent: select a protected file, enter the PIN and use **Move to parent**. The encrypted parent envelope is persisted first; then the nested record is updated.
- Both directions preserve the original file ID and authenticated envelope. Existing full restore and create-new-compartment workflows remain available.
- Rust regression covers incorrect PIN, transfer, content round-trip, duplicate transfer and moving a file back.

**Known limitation:** a crash after the first durable write but before source removal can leave encrypted duplicate copies. Do not automatically delete one; first authenticate and reconcile them. This is explicitly deferred to Phase 02.

## Phase 02 — Crash recovery and data integrity (implemented; manual acceptance pending)

- Interrupted parent → nested transfers: after PIN authentication, compare the parent envelope with the authenticated nested envelope byte-for-byte. Remove the redundant parent copy only when identical; otherwise return recovery_conflict and preserve both.
- Interrupted nested → parent transfers: resume when the existing parent envelope is authenticated and byte-identical; refuse differing or non-regular destinations. Full restore also resumes identical already-restored envelopes.
- Encrypted staging writes now use unique random temporary filenames so an abandoned temp does not block future writes. On nested unlock, remove only correctly named regular abandoned nested temp files after authenticating the durable final record; symlinks and unrelated files remain untouched.
- Added regression simulations for both interrupted transfer directions, conflicting authenticated ciphertext and safe staging cleanup.
- Source removal still occurs after the destination has been written, synced and its directory synced. This protects the only durable copy under normal filesystem guarantees.

**Remaining validation:** Native macOS interruption/fault-injection at individual write, fsync, rename and unlink boundaries has not been demonstrated. A process crash may still leave encrypted staging artefacts for parent-file updates, and power-loss guarantees depend on the filesystem and storage hardware. Treat these as release-hardening acceptance items rather than a formal durability proof.

## Phase 03 — Native application testing and security regression (in progress)

Automated acceptance now exercises a fresh Rust store after process-equivalent drop/reinitialisation: incorrect master password, correct unlock, wrong nested PIN, nested inventory, parent → nested transfer, a second restart, nested text authentication, nested → parent transfer and content integrity. Frontend IPC regression verifies the exact native command and PIN/file-ID arguments.

**Native GUI checklist (must be observed in the running Tauri app; not claimed as completed):**
- Quit an existing app with Cmd+Q, stop its previous dev process, then run ./start.sh dev.
- Check split login at narrow and wide window sizes, first-vault creation, wrong password, correct unlock, sign-out and unlock again.
- Create two harmless test documents; create a nested vault with one. Unlock using an incorrect PIN then the correct PIN. Transfer the second parent file into the nested vault; inspect its metadata and text. Transfer one file back. Confirm inventories on both sides.
- Restart the application, unlock and verify content. Exercise delete confirmation and five-file limit with disposable test data.
- Check no plaintext exports, unexpected files, secrets in logs, or uncaught errors; review conflict behaviour using an isolated disposable test directory only.

Detailed controlled process-kill/fsync/unlink fault injection, native accessibility checks and release signing remain outstanding.

## Later phases

04 — Final UI polish and accessibility.
05 — Release documentation, backup/recovery policy and macOS signing/notarisation preparation.

Do not describe the application as production security-audited solely because automated tests pass.
