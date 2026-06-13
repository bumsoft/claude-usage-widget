//! Shared types describing a credential source the widget can read from.

use serde::{Deserialize, Serialize};

/// Where a credential source originates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    /// `%USERPROFILE%\.claude` on the Windows host.
    Windows,
    /// A WSL distro reached over a UNC path.
    Wsl,
    /// A user-supplied path.
    Custom,
}

/// A selectable credential source shown in the widget's source dropdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// Stable identifier, e.g. `"windows"`, `"wsl:Ubuntu"`, `"custom:0"`.
    pub id: String,
    /// Human label, e.g. `"Windows"`, `"WSL: Ubuntu"`.
    pub label: String,
    pub kind: SourceKind,
    /// Windows-accessible path to `.credentials.json` (UNC for WSL distros).
    pub credentials_path: String,
    /// Whether the file currently exists / is readable.
    pub exists: bool,
}

impl Source {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        kind: SourceKind,
        credentials_path: impl Into<String>,
        exists: bool,
    ) -> Self {
        Source {
            id: id.into(),
            label: label.into(),
            kind,
            credentials_path: credentials_path.into(),
            exists,
        }
    }
}
