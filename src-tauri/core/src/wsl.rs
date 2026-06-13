//! Pure helpers for locating WSL credential files from the Windows host.
//!
//! The actual `wsl.exe` invocation lives in the app layer; the parsing and
//! path-building done here is platform-agnostic and unit tested.

/// UNC prefix used by current Windows builds (Win10 2004+ / Win11).
pub const WSL_PREFIX_LOCALHOST: &str = r"\\wsl.localhost";
/// Legacy UNC prefix; some builds still expose distros here.
pub const WSL_PREFIX_DOLLAR: &str = r"\\wsl$";

/// Decode console output that may be UTF-16LE (`wsl.exe -l`) or UTF-8.
///
/// `wsl.exe -l -q` historically emits UTF-16LE. Some configurations emit UTF-8.
/// We detect UTF-16 by the density of interleaved NUL bytes.
fn decode_console(raw: &[u8]) -> String {
    let nul_count = raw.iter().filter(|&&b| b == 0).count();
    let looks_utf16 = raw.len() >= 2 && nul_count * 2 >= raw.len().saturating_sub(2);
    if looks_utf16 {
        decode_utf16le(raw)
    } else {
        String::from_utf8_lossy(raw).into_owned()
    }
}

fn decode_utf16le(raw: &[u8]) -> String {
    let bytes = if raw.starts_with(&[0xFF, 0xFE]) {
        &raw[2..]
    } else {
        raw
    };
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

/// Parse the distro names from `wsl.exe -l -q` output.
pub fn parse_wsl_list(raw: &[u8]) -> Vec<String> {
    decode_console(raw)
        .lines()
        .map(|line| line.trim().trim_matches('\u{0}').trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

/// Reject distro names that could escape the `\\wsl.localhost\<distro>\` root
/// when interpolated into a UNC path (path separators, parent refs, NUL).
pub fn is_valid_distro_name(distro: &str) -> bool {
    !distro.is_empty()
        && !distro.contains('\\')
        && !distro.contains('/')
        && !distro.contains("..")
        && !distro.contains('\u{0}')
}

/// Build the Windows UNC path to a distro's `.credentials.json`.
///
/// `posix_home` is the Linux `$HOME` (e.g. `/home/wlsbum`).
pub fn wsl_credentials_unc(prefix: &str, distro: &str, posix_home: &str) -> String {
    let rel = posix_home.trim_matches('/').replace('/', "\\");
    if rel.is_empty() {
        format!(r"{prefix}\{distro}\.claude\.credentials.json")
    } else {
        format!(r"{prefix}\{distro}\{rel}\.claude\.credentials.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_utf16le_distro_list() {
        // "Ubuntu\r\nDebian\r\n" as UTF-16LE with BOM.
        let mut raw = vec![0xFF, 0xFE];
        for ch in "Ubuntu\r\nDebian\r\n".encode_utf16() {
            raw.extend_from_slice(&ch.to_le_bytes());
        }
        assert_eq!(parse_wsl_list(&raw), vec!["Ubuntu", "Debian"]);
    }

    #[test]
    fn parses_utf8_distro_list() {
        let raw = b"Ubuntu\nUbuntu-22.04\n\n";
        assert_eq!(parse_wsl_list(raw), vec!["Ubuntu", "Ubuntu-22.04"]);
    }

    #[test]
    fn ignores_blank_and_whitespace_lines() {
        let raw = b"  Ubuntu  \n\n   \nkali-linux\n";
        assert_eq!(parse_wsl_list(raw), vec!["Ubuntu", "kali-linux"]);
    }

    #[test]
    fn validates_distro_names() {
        assert!(is_valid_distro_name("Ubuntu"));
        assert!(is_valid_distro_name("Ubuntu-22.04"));
        assert!(is_valid_distro_name("kali-linux"));
        assert!(!is_valid_distro_name(""));
        assert!(!is_valid_distro_name("..\\Windows"));
        assert!(!is_valid_distro_name("a/b"));
        assert!(!is_valid_distro_name("a\\b"));
    }

    #[test]
    fn builds_unc_path() {
        assert_eq!(
            wsl_credentials_unc(WSL_PREFIX_LOCALHOST, "Ubuntu", "/home/wlsbum"),
            r"\\wsl.localhost\Ubuntu\home\wlsbum\.claude\.credentials.json"
        );
        assert_eq!(
            wsl_credentials_unc(WSL_PREFIX_DOLLAR, "Debian", "/root"),
            r"\\wsl$\Debian\root\.claude\.credentials.json"
        );
    }
}
