# Sprint 04 addendum — Offline pre-import static screening
**Date:** 29/09/2026
**Branch:** `feat/desktop-auto-lock` (existing uncommitted UI work retained).

## Threat model and implementation
`desktop/src-tauri/src/import_scan.rs` implements a lightweight offline **policy screen**, not a malware scanner or antivirus clearance. The native import command selects the source; `VaultStore::import_path` reads at most 5 MiB through the existing regular-file/no-follow path, screens those exact in-memory bytes, and **only then** creates the encrypted vault file directory and encrypts/persists. A blocked import returns `import_scan_blocked` with a category-specific user-facing message; it does not copy, quarantine, execute, modify or delete the source.

Current conservative checks: known executable/script/macro-capable extensions; executable headers (PE/MZ, ELF, Mach-O, fat binaries, shebang); EICAR test string; selected explicit command-execution text patterns (PowerShell encoded command and shell pipe downloaders); and unsupported compressed/archive/disk-image formats, including ZIP header masquerading as text. Archives are blocked because nested contents cannot currently be scanned. No network calls, external API or plaintext scan report are created.

**Important:** A file passing this gate is **not proven safe**. Signatures and heuristic strings are easy to evade; embedded PDF actions, novel malware, obfuscated scripts, encrypted containers, polyglots, worms and malicious content in otherwise ordinary documents may not be detected. SQL injection is normally a flaw in the importing/consuming application, not a property of an inert SQL file; ordinary SQL is not rejected. False positives are possible, particularly for security research samples. File content is never executed for scanning.

## Tests and manual acceptance
Rust tests cover blocked extensions, disguised executable headers, risky command strings, EICAR, archives, permitted ordinary text/SQL and the full vault import path rejecting a disguised executable **without creating a vault file directory**. Existing file roundtrip tests remain in place.

For native acceptance: use a disposable vault, import ordinary `.txt`, then try a harmless `.sh`, a text file beginning `MZ`, and a `.zip`; confirm each rejected import displays a reason and does not appear in the vault. Confirm source files remain unchanged. Do not test with live malware.

## Future
Add a maintained signature engine (for example locally updated ClamAV definitions), robust nested-format inspection with decompression limits, YARA rules under review, scan result versioning, and native UI progress/clear risk labels. Never advertise 'malware-free' from a static pass.
