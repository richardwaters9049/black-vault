//! Fast, offline pre-import policy. This is not an antivirus engine or a safety verdict.
//! Only high-confidence indicators are blocked; unknown files remain unverified.
#[derive(Debug, PartialEq, Eq)]
pub enum ScanFinding {
    ExecutableType,
    ExecutableContent,
    KnownTestSignature,
    SuspiciousScript,
    OpaqueContainer,
}
impl ScanFinding {
    pub fn message(&self) -> &'static str {
        match self {
            Self::ExecutableType => {
                "Import blocked: executable, script or macro-capable file type."
            }
            Self::ExecutableContent => {
                "Import blocked: executable content detected regardless of extension."
            }
            Self::KnownTestSignature => "Import blocked: antivirus test signature detected.",
            Self::OpaqueContainer => {
                "Import blocked: compressed or disk-image content requires deeper scanning."
            }
            Self::SuspiciousScript => {
                "Import blocked: high-risk command execution pattern detected."
            }
        }
    }
}
pub fn scan(name: &str, bytes: &[u8]) -> Result<(), ScanFinding> {
    let extension = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    // Explicit deny-list for directly runnable content and Office macro-enabled formats.
    const BLOCKED: &[&str] = &[
        "app",
        "applescript",
        "bat",
        "cmd",
        "com",
        "dll",
        "dylib",
        "exe",
        "hta",
        "jar",
        "js",
        "jse",
        "lnk",
        "macho",
        "msi",
        "msp",
        "osax",
        "pl",
        "ps1",
        "psm1",
        "py",
        "rb",
        "scpt",
        "sh",
        "so",
        "vbe",
        "vbs",
        "wsf",
        "xlam",
        "xlsm",
        "xlsb",
        "docm",
        "dotm",
        "pptm",
        "potm",
        "ppam",
        "scr",
        "command",
        "workflow",
        "webloc",
        "desktop",
        "htm",
        "html",
        "svg",
        "zip",
        "rar",
        "7z",
        "gz",
        "tgz",
        "tar",
        "dmg",
        "iso",
        "pkg",
    ];
    if BLOCKED.contains(&extension.as_str()) {
        return Err(ScanFinding::ExecutableType);
    }
    if bytes.starts_with(&[b'P', b'K', 3, 4]) {
        return Err(ScanFinding::OpaqueContainer);
    }
    if bytes.starts_with(b"MZ")
        || bytes.starts_with(b"\x7fELF")
        || bytes.starts_with(b"\xca\xfe\xba\xbe")
        || bytes.starts_with(b"\xfe\xed\xfa\xce")
        || bytes.starts_with(b"\xce\xfa\xed\xfe")
        || bytes.starts_with(b"\xfe\xed\xfa\xcf")
        || bytes.starts_with(b"\xcf\xfa\xed\xfe")
        || bytes.starts_with(b"#!")
    {
        return Err(ScanFinding::ExecutableContent);
    }
    // Standard EICAR test string, split to avoid embedding a full test signature in the binary.
    let eicar = concat!(
        "X5O!P%@AP",
        "[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*"
    )
    .as_bytes();
    if bytes.windows(eicar.len()).any(|part| part == eicar) {
        return Err(ScanFinding::KnownTestSignature);
    }
    // These are narrow, contextual patterns. Do not treat SQL statements as malware:
    // SQL injection is a vulnerability in how applications process untrusted input.
    let prefix = &bytes[..bytes.len().min(256 * 1024)];
    let lower = String::from_utf8_lossy(prefix).to_ascii_lowercase();
    if lower.contains("powershell -encodedcommand")
        || lower.contains("powershell.exe -enc ")
        || lower.contains("curl ") && (lower.contains("| sh") || lower.contains("| bash"))
        || lower.contains("wget ") && (lower.contains("| sh") || lower.contains("| bash"))
    {
        return Err(ScanFinding::SuspiciousScript);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocks_runnable_extensions_case_insensitively() {
        for name in [
            "evil.EXE",
            "macro.docm",
            "run.sh",
            "payload.PS1",
            "launch.command",
        ] {
            assert_eq!(scan(name, b"plain bytes"), Err(ScanFinding::ExecutableType));
        }
    }
    #[test]
    fn blocks_disguised_executables_and_shebangs() {
        for bytes in [b"MZtest".as_slice(), b"\x7fELFdata", b"#!/bin/sh\necho hi"] {
            assert_eq!(
                scan("notes.txt", bytes),
                Err(ScanFinding::ExecutableContent)
            );
        }
    }
    #[test]
    fn blocks_high_risk_command_patterns() {
        assert_eq!(
            scan("notes.txt", b"curl https://example.invalid/x | sh"),
            Err(ScanFinding::SuspiciousScript)
        );
        assert_eq!(
            scan("notes.txt", b"powershell -EncodedCommand abc"),
            Err(ScanFinding::SuspiciousScript)
        );
    }
    #[test]
    fn rejects_archives_even_when_disguised_as_text() {
        assert_eq!(
            scan("archive.zip", b"data"),
            Err(ScanFinding::ExecutableType)
        );
        assert_eq!(
            scan("notes.txt", &[b'P', b'K', 3, 4, 0, 0]),
            Err(ScanFinding::OpaqueContainer)
        );
    }
    #[test]
    fn permits_normal_documents_and_sql_as_data() {
        assert_eq!(scan("report.txt", b"Quarterly report"), Ok(()));
        assert_eq!(
            scan("query.sql", b"SELECT * FROM users WHERE id = ?;"),
            Ok(())
        );
        assert_eq!(scan("photo.png", b"\x89PNG\r\n\x1a\n"), Ok(()));
    }
    #[test]
    fn catches_eicar_test_pattern() {
        let eicar = concat!(
            "X5O!P%@AP",
            "[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-ANTIVIRUS-TEST-FILE!$H+H*"
        )
        .as_bytes();
        assert_eq!(
            scan("sample.txt", &eicar),
            Err(ScanFinding::KnownTestSignature)
        );
    }
}
