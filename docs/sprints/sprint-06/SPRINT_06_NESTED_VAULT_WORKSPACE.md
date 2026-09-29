# Sprint 06 — Nested Vault Workspace

**Date:** 29/09/2026
**Branch:** `feat/desktop-nested-vault-workspace`

## Implemented

- Selecting a Locked Vault sidebar entry opens a dedicated compartment workspace, initially PIN-gated.
- Four ASCII digits, including leading zeroes, authenticate the nested record. Five failed attempts trigger a 60-second in-process cooldown shared across inspection, restoration and nested file creation.
- After authentication, the nested workspace displays its encrypted file inventory. Lock compartment clears the visible inventory and draft content without signing out of the parent vault. Switching main navigation clears the nested view.
- Users can create TXT, MD, JSON, CSV and LOG documents directly inside the nested container. Each write requires the four-digit PIN again. Rust checks filename validity, duplicate names, per-file 5 MiB and five-file limits.
- The new document is sealed as an authenticated parent-bound file envelope, included in the nested encrypted payload, then the nested container is atomically replaced via a same-directory temporary file, file sync, rename and directory sync. No plaintext file is created.
- The existing Restore to parent action restores all nested files as encrypted envelopes. Individual file transfer, in-place editing, rename and deletion are **not yet implemented**.

## Security limitations and acceptance

The nested PIN has only 10,000 combinations; it is a convenience gate on top of the unlocked parent vault, not a high-entropy independent password. Attempt limits are in-process, not tamper-resistant across restarts. Nested file metadata and names remain encrypted at rest, but the authenticated file list and drafts exist in React memory while viewed. This is cryptographic compartmentalisation in one process, **not** Qubes OS VM isolation. Native macOS UI acceptance, process crash recovery and adversarial rollback tests remain outstanding.

Verify: create parent vault, import file, lock it into a nested vault with a PIN beginning with zero, open via Locked Vaults, create a new text file, lock the nested compartment, reopen, confirm both entries, restore all to parent and verify the nested sidebar entry disappears. Run `./start.sh test` and restart the Tauri process after Rust changes.
