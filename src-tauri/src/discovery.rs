//! Discovery of credential sources on the Windows host and in WSL distros.
//!
//! The pure parsing/path helpers live in `claude_usage_core::wsl`; this module
//! performs the actual filesystem checks and `wsl.exe` calls.

use std::collections::HashSet;
use std::path::Path;

use claude_usage_core::sources::{Source, SourceKind};
#[cfg(windows)]
use claude_usage_core::wsl;

/// Enumerate all credential sources: Windows host, every WSL distro, and any
/// user-added custom paths. Duplicates (by path) are removed, first wins.
pub fn discover(custom_paths: &[String]) -> Vec<Source> {
    let mut sources: Vec<Source> = Vec::new();

    if let Some(win) = windows_source() {
        sources.push(win);
    }
    sources.extend(wsl_sources());

    for (i, path) in custom_paths.iter().enumerate() {
        let exists = Path::new(path).is_file();
        sources.push(Source::new(
            format!("custom:{i}"),
            format!("Custom {}", i + 1),
            SourceKind::Custom,
            path.clone(),
            exists,
        ));
    }

    dedup(sources)
}

fn windows_source() -> Option<Source> {
    let profile = std::env::var("USERPROFILE").ok()?;
    let path = format!(r"{profile}\.claude\.credentials.json");
    let exists = Path::new(&path).is_file();
    Some(Source::new(
        "windows",
        "Windows",
        SourceKind::Windows,
        path,
        exists,
    ))
}

fn dedup(sources: Vec<Source>) -> Vec<Source> {
    let mut seen: HashSet<String> = HashSet::new();
    sources
        .into_iter()
        .filter(|s| seen.insert(s.credentials_path.to_lowercase()))
        .collect()
}

#[cfg(windows)]
fn wsl_sources() -> Vec<Source> {
    list_wsl_distros()
        .into_iter()
        .filter_map(|distro| wsl_source_for(&distro))
        .collect()
}

#[cfg(not(windows))]
fn wsl_sources() -> Vec<Source> {
    // Only meaningful on the Windows host; on other targets there is nothing to scan.
    Vec::new()
}

#[cfg(windows)]
fn list_wsl_distros() -> Vec<String> {
    use std::process::Command;
    match Command::new("wsl.exe").args(["-l", "-q"]).output() {
        Ok(out) => wsl::parse_wsl_list(&out.stdout),
        Err(_) => Vec::new(),
    }
}

#[cfg(windows)]
fn wsl_source_for(distro: &str) -> Option<Source> {
    if !wsl::is_valid_distro_name(distro) {
        return None;
    }
    let home = wsl_home(distro).unwrap_or_else(|| "/root".to_string());
    let candidates = [
        wsl::wsl_credentials_unc(wsl::WSL_PREFIX_LOCALHOST, distro, &home),
        wsl::wsl_credentials_unc(wsl::WSL_PREFIX_DOLLAR, distro, &home),
    ];

    let (path, exists) = match candidates.iter().find(|p| Path::new(p).is_file()) {
        Some(existing) => (existing.clone(), true),
        None => (candidates[0].clone(), false),
    };

    Some(Source::new(
        format!("wsl:{distro}"),
        format!("WSL: {distro}"),
        SourceKind::Wsl,
        path,
        exists,
    ))
}

#[cfg(windows)]
fn wsl_home(distro: &str) -> Option<String> {
    use std::process::Command;
    let out = Command::new("wsl.exe")
        .args(["-d", distro, "-e", "sh", "-c", "printf %s \"$HOME\""])
        .output()
        .ok()?;
    let home = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if home.is_empty() {
        None
    } else {
        Some(home)
    }
}
