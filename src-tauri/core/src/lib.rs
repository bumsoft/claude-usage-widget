//! Platform-agnostic core for the Claude Usage widget.
//!
//! Everything here is free of Tauri / WebView dependencies so it can be unit
//! tested on any platform (`cargo test -p claude-usage-core`). The Tauri app
//! layer is a thin shell that does file IO, spawns `wsl.exe`, and exposes these
//! functions as commands.

pub mod credentials;
pub mod pricing;
pub mod sources;
pub mod stats;
pub mod transcripts;
pub mod usage;
pub mod wsl;

pub use credentials::{parse_credentials, OauthCredentials};
pub use sources::{Source, SourceKind};
pub use stats::{parse_stats, StatsHistory};
pub use usage::{fetch_usage_snapshot, plan_label, UsageSnapshot, UsageWindow};

use std::fmt;

/// Errors surfaced by the core layer. The app layer converts these into plain
/// strings for the frontend (the access token is never part of an error).
#[derive(Debug)]
pub enum CoreError {
    /// A credentials/usage/stats payload could not be parsed.
    Json(serde_json::Error),
    /// A network/transport failure talking to the usage endpoint.
    Network(String),
    /// The token was rejected (expired or revoked). The UI should prompt re-login.
    Unauthorized,
    /// HTTP 429 — rate limited. Carries the `Retry-After` seconds when provided.
    RateLimited(Option<u64>),
    /// The usage endpoint returned a non-success HTTP status.
    Http(u16),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Json(e) => write!(f, "parse error: {e}"),
            CoreError::Network(e) => write!(f, "network error: {e}"),
            CoreError::Unauthorized => {
                write!(f, "token expired or unauthorized — run Claude Code once to refresh it")
            }
            CoreError::RateLimited(Some(secs)) => {
                write!(f, "rate limited (HTTP 429); retry after {secs}s")
            }
            CoreError::RateLimited(None) => write!(f, "rate limited (HTTP 429)"),
            CoreError::Http(code) => write!(f, "usage endpoint returned HTTP {code}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self {
        CoreError::Json(e)
    }
}

impl From<reqwest::Error> for CoreError {
    fn from(e: reqwest::Error) -> Self {
        CoreError::Network(e.to_string())
    }
}
