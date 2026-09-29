# Example Files and vault lifecycle — 29/09/2026

Branch: `feat/desktop-example-files-vault-lifecycle`.

## Example Files
A new sidebar entry under Overview opens a built-in example library. Four fixed, offline sample IDs are recognised **inside Rust**, not arbitrary frontend paths: plain text, CSV, parameterised SQL text and a harmless script deliberately rejected by the static pre-import gate. Examples go through the same `VaultStore::import_bytes` screening, per-file encryption and persistence path as normal file imports. No example is downloaded, executed or placed in the user's filesystem. Successful examples appear in the vault file list when returning to Overview.

## Forgotten passwords and deletion
There is **no password reset, escrow or backdoor**. The vault master key is encrypted using a password-derived key. If the password is lost and no accessible backup exists, encrypted data cannot be recovered by Black Vault. The sign-in screen offers permanent deletion of a **locked** local vault without knowing its password, but requires typing `DELETE <full 32-character vault ID>` exactly. It removes recognised encrypted file records, then the files directory, and the vault header **last**. Unknown entries, symlinks, or an unlocked active session prevent deletion. If deletion is interrupted, some files may have been removed while the header remains; there is no rollback. Backups, external copies, filesystem snapshots and SSD remanence are not securely erased. This is a destructive local reset, not a way to recover data.

Future: authenticated password change while unlocked (rewrap the existing master key), user-controlled offline recovery key generated during creation, separately reviewed backup/restore and a stronger transactional deletion design. Never store example passwords in release builds.

## Manual acceptance
Create a disposable vault; open Example Files; import notes, CSV and SQL; confirm they appear under Overview / Encrypted files. Try the blocked script; confirm an error and no new vault entry. Sign out, choose Forgot password / Delete vault, verify incomplete confirmation disables deletion, type the exact full-ID confirmation, verify the vault disappears and a new one can be created. Do not test deletion with irreplaceable data.

### Encryption and Decryption interaction (29/09/2026)
- Encryption and Decryption are separate tabs in the unlocked vault's full-width file workspace.
- Example Files is an expandable sidebar button; its four local sample cards can be dragged into Encryption. The drop only transfers a fixed sample ID, and Rust selects the bytes, scans them and encrypts them through the existing native gate. A blocked example remains blocked.
- Encrypted file rows can be dragged into Decryption or exported by button; the Rust export picker chooses a new plaintext destination and refuses overwrites. Arbitrary operating-system file drops are deliberately **not** accepted; use the secure macOS picker to import real files, so JavaScript never supplies filesystem paths.
- While native work is pending, a subtle blue animated icon/progress indicator runs; green confirmation only appears after the native promise resolves successfully, red on failure, and cancellation is neutral. Reduced-motion users get static indicators. The animation is indeterminate and does not claim measured cryptographic progress.
- Manually check dragging built-in examples, rejecting the script, encryption tab import picker, dragging encrypted entries to Decryption, cancellation and successful export; automated DOM and Rust tests do not replace native GUI acceptance.

### Native Finder drop and explicit confirmation
- The macOS webview reports Finder drag/drop position; Rust independently captures the one-file native drop path. JavaScript never submits an arbitrary filesystem path for import.
- Dropping a Finder file into Encryption stages its filename and size. Encrypt explicitly consumes the native pending path and uses the existing regular-file limit, static scan and encryption pipeline.
- Click an Example File in the sidebar if WebKit HTML drag data is unavailable. The selected example still requires Encrypt.
- Decryption stages a stored encrypted file; Decrypt invokes the existing native save/export picker.
- A pending Finder path is cleared when the vault locks. Native Finder drag/drop requires manual macOS GUI acceptance testing.

### Local development password prefill
- AuthGate reads VITE_DEV_VAULT_PASSWORD only when Vite is in development mode.
- Store any disposable development credential in desktop/.env.development.local (already Git-ignored), never tracked source. The password is not automatically submitted.
- The release app launched with ./start.sh does not include the development-only prefill; use ./start.sh dev for development.
