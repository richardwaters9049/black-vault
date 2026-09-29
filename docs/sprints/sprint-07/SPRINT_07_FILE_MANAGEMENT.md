# Sprint 07 — Vault file management

**Date:** 29/09/2026
**Branch:** `feat/desktop-vault-file-management`

## Current implementation

- Parent-vault text documents (TXT, MD, JSON, CSV and LOG) can be opened, edited and renamed without plaintext export. The Rust boundary authenticates the original envelope, screens updated content and encrypts it with a new random per-file DEK under the same file ID.
- Updates use a private temporary ciphertext file, file sync, same-directory rename and directory sync. Filename collision checks are case-insensitive within the parent vault.
- Parent-vault encrypted files can be permanently deleted with an explicit two-step confirmation in the document list. Rust authenticates the selected envelope before deletion.
- The existing nested-vault workspace and encrypted restore-all flow remain unchanged.

## Outstanding

Individual transfers between parent and nested vaults, nested file editing/deletion, and arbitrary binary-file rename/delete remain future work. Deletion is permanent and is not a secure overwrite of SSD blocks, backups or snapshots. Atomic replacement protects against partially written new ciphertext but does not constitute a complete multi-operation crash-recovery protocol. Native UI acceptance is outstanding.
