//! Widget configuration persisted to the app config directory.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

const DEFAULT_REFRESH_SECONDS: u32 = 90;
const MIN_REFRESH_SECONDS: u32 = 30;

/// User-tweakable widget settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// `id` of the source currently selected in the dropdown.
    #[serde(default)]
    pub selected_source_id: Option<String>,
    /// Extra `.credentials.json` paths the user added manually.
    #[serde(default)]
    pub custom_paths: Vec<String>,
    /// Poll interval for the live usage endpoint.
    #[serde(default = "default_refresh")]
    pub refresh_seconds: u32,
    /// Whether the window stays above other windows.
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    /// Whether the window is in the small minimized layout.
    #[serde(default)]
    pub compact: bool,
    /// Compact representation: `"bars"` or `"rings"`.
    #[serde(default = "default_compact_style")]
    pub compact_style: String,
}

fn default_refresh() -> u32 {
    DEFAULT_REFRESH_SECONDS
}

fn default_true() -> bool {
    true
}

fn default_compact_style() -> String {
    "bars".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            selected_source_id: None,
            custom_paths: Vec::new(),
            refresh_seconds: DEFAULT_REFRESH_SECONDS,
            always_on_top: true,
            compact: false,
            compact_style: default_compact_style(),
        }
    }
}

impl AppConfig {
    /// Clamp values that could break polling if a hand-edited config sets them too low.
    fn sanitized(mut self) -> Self {
        if self.refresh_seconds < MIN_REFRESH_SECONDS {
            self.refresh_seconds = MIN_REFRESH_SECONDS;
        }
        if self.compact_style != "rings" {
            self.compact_style = "bars".to_string();
        }
        self
    }
}

fn config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("config.json"))
}

/// Load config, falling back to defaults on any error.
pub fn load(app: &tauri::AppHandle) -> AppConfig {
    let Some(path) = config_path(app) else {
        return AppConfig::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<AppConfig>(&raw)
            .unwrap_or_default()
            .sanitized(),
        Err(_) => AppConfig::default(),
    }
}

/// Persist config to disk, creating the config directory if needed.
pub fn save(app: &tauri::AppHandle, cfg: &AppConfig) -> Result<(), String> {
    let path = config_path(app).ok_or("could not resolve config directory")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(&cfg.clone().sanitized()).map_err(|e| e.to_string())?;
    std::fs::write(&path, raw).map_err(|e| e.to_string())
}
