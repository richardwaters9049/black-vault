# Sprint 08 — Nested file management

**Date:** 29/09/2026
**Branch:** `feat/desktop-nested-file-management`

## Implemented

- Click a file in **Protected files** to select it. The right-hand panel displays its authenticated name, size, ID and protection context.
- PIN-authorised read of supported text documents into the session-only editor; rename and edit via encrypted replacement; two-step permanent deletion.
- Individual **Move to parent** writes the authenticated encrypted envelope to the parent directory and synchronises it before updating the nested container. No plaintext export.
- Existing parent-vault operations, nested create and restore-all remain available.

## Security and limitations

Every nested read or mutation requires PIN re-entry. A four-digit PIN is only an additional convenience layer; the strong parent master password remains essential. Parent transfer is deliberately copy-before-remove: a crash between the two writes may temporarily leave duplicate ciphertext rather than lose the sole copy. There is no automatic crash reconciliation yet. File contents are exposed to the JavaScript editor only after successful PIN authentication, and cleared on close/lock. File metadata is shown only in an authenticated nested view. Transfers from parent into an *existing* nested vault and native GUI smoke testing remain outstanding.
