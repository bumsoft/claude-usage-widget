//! Tauri commands exposed to the frontend.
//!
//! The access token never crosses into the frontend: credentials are read and
//! the usage endpoint is called entirely in Rust; only the resulting snapshot
//! (utilization + reset times + plan label) is returned.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use claude_usage_core as core;
use tauri::State;

use crate::{config, discovery, AppState};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// The `.claude` directory holding the credentials file (and `projects/`, `stats-cache.json`).
fn claude_dir_for(cred_path: &str) -> Option<PathBuf> {
    Path::new(cred_path).parent().map(Path::to_path_buf)
}

async fn read_file(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || std::fs::read_to_string(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Enumerate Windows + WSL + custom credential sources.
#[tauri::command]
pub async fn discover_sources(app: tauri::AppHandle) -> Result<Vec<core::Source>, String> {
    let custom = config::load(&app).custom_paths;
    tauri::async_runtime::spawn_blocking(move || discovery::discover(&custom))
        .await
        .map_err(|e| e.to_string())
}

/// Read the token from `path` and fetch the live usage snapshot.
#[tauri::command]
pub async fn fetch_usage(
    path: String,
    state: State<'_, AppState>,
) -> Result<core::UsageSnapshot, String> {
    let raw = read_file(path.clone())
        .await
        .map_err(|e| format!("cannot read credentials at {path}: {e}"))?;
    let creds = core::parse_credentials(&raw).map_err(|e| e.to_string())?;
    let client = state.http.clone();
    core::usage::fetch_usage_snapshot(&client, &creds, now_ms())
        .await
        .map_err(|e| e.to_string())
}

/// Read the local token-usage history for the selected source.
///
/// Primary source is the raw transcripts (always current, includes today);
/// `stats-cache.json` is a fallback when no transcripts are found.
#[tauri::command]
pub async fn read_stats(path: String, days: Option<usize>) -> Result<core::StatsHistory, String> {
    let claude_dir = claude_dir_for(&path)
        .ok_or_else(|| "cannot locate the .claude directory".to_string())?;
    let max_days = days.unwrap_or(14);
    let now = now_ms();

    // Primary: aggregate from raw transcripts.
    let projects_dir = claude_dir.join("projects");
    let history = tauri::async_runtime::spawn_blocking(move || {
        core::transcripts::aggregate_transcripts(&projects_dir, max_days, now)
    })
    .await
    .map_err(|e| e.to_string())?;
    if !history.days.is_empty() {
        return Ok(history);
    }

    // Fallback: the cached stats file (may lag behind real usage).
    let stats_path = claude_dir.join("stats-cache.json");
    let stats_str = stats_path.to_string_lossy().to_string();
    let raw = read_file(stats_str.clone())
        .await
        .map_err(|e| format!("no transcripts found and cannot read stats at {stats_str}: {e}"))?;
    core::parse_stats(&raw, max_days).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_config(app: tauri::AppHandle) -> config::AppConfig {
    config::load(&app)
}

#[tauri::command]
pub fn set_config(app: tauri::AppHandle, config: config::AppConfig) -> Result<(), String> {
    config::save(&app, &config)
}
