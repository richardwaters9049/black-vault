# Sprint 10 — Deterministic transfer fault injection

**Date:** 29/09/2026
**Branch:** test/desktop-fault-injection

## First increment — parent → nested

A test-only thread-local checkpoint injects an I/O failure at five points of the encrypted parent-to-nested commit sequence: before temporary-file sync, before rename, after rename/before directory sync, after destination directory sync/before source unlink, and after source unlink/before final directory sync. No fault hook is active in production builds and no plaintext is written to disk by the transfer.

Each checkpoint runs against a separate temporary vault. The test drops the original store, opens a new one, authenticates the nested vault, checks whether the transfer is present according to its commit boundary, verifies the incoming content is still readable in exactly one canonical location after reconciliation, and checks the original protected file remains readable. This exercises the actual transfer implementation rather than manually constructing an interrupted state.

**Scope/limitations:** These are deterministic returned I/O errors, not actual SIGKILL, kernel faults, hardware power loss, or guaranteed filesystem persistence under power failure. After a rename but before directory sync, a live test filesystem normally exposes the renamed file; power-loss durability at that boundary is not proven. Additional work: inject nested → parent and restore checkpoints, test failing cleanup/unlink and conflicts, run process-kill integration on disposable native vault directories, review parent staging artefacts and backup/recovery documentation.

Do not describe this as an independently audited durability guarantee.

## Second increment — nested → parent and full restore

Added four further deterministic interruption checkpoints for individual nested → parent transfers (before encrypted parent copy, after copy, before nested-record rename, and after rename). A separate temporary vault and newly initialised store is used for every scenario. Tests authenticate the retained nested copy where applicable, resume an identical already-copied parent envelope, and confirm the transferred text is readable after recovery.

Added four full-restore checkpoints (before a parent envelope copy, after a copy, before nested-record unlink, and after unlink). The restore test uses two encrypted documents to exercise partial completion; after a simulated interruption and store restart, restoring again resumes identical parent copies. An interruption after the child unlink leaves the two authenticated parent files accessible without a child record.

The checkpoint implementation is compiled out of non-test builds. These are injected returned errors rather than a real process kill or power loss; durability at rename/directory-sync boundaries still needs separate native integration validation.

## Third increment — actual child-process termination

The integration-style Rust test `subprocess_interruption_recovery_matrix` launches a fresh copy of the Rust test executable for each of 13 commit checkpoints, using a distinct `tempfile::TempDir` as its entire vault root. A test-only environment variable selects the checkpoint; the child writes and syncs a checkpoint marker, then exits immediately with code 86 without unwinding or running ordinary error cleanup. The parent checks the exit code and marker, reopens the disposable store, authenticates encrypted content, and resumes incomplete transfers/restores. The test worker does not use the actual Tauri application data directory.

This is real abrupt child-process exit, **not** SIGKILL or a simulated machine power loss. The marker synchronisation is for test observability only. It does not prove hardware persistence or protect against kernel/filesystem faults. All exit hooks are `#[cfg(test)]` and excluded from production binaries.

## Fourth increment — conservative recovery and conflict regression

Added restart-based regression for abandoned parent-file staging: the canonical authenticated envelope must remain readable, and a random-name partial staging file is preserved for manual review rather than silently removed. The vault deletion guard must refuse an unknown staging filename rather than destroy it. This is intentionally conservative; automated parent-staging cleanup requires a separate authenticated recovery policy.

Added restore conflict regression: when a parent destination and nested envelope have the same file ID but different *valid authenticated* ciphertext, restoration returns `recovery_conflict`, preserves both byte-for-byte, and both original plaintext versions remain accessible via their respective authenticated views. No automatic overwrite is permitted.
