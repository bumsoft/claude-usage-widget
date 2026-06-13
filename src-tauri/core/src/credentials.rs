//! Parsing of `~/.claude/.credentials.json`.
//!
//! Only the `claudeAiOauth` block is read. The file is treated as read-only:
//! token refresh is owned by Claude Code, so the widget never writes it back.

use serde::Deserialize;

use crate::CoreError;

/// Top-level shape of `.credentials.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct CredentialsFile {
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: OauthCredentials,
}

/// The OAuth credentials Claude Code stores for the subscription.
#[derive(Debug, Clone, Deserialize)]
pub struct OauthCredentials {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken", default)]
    pub refresh_token: Option<String>,
    /// Expiry as epoch milliseconds.
    #[serde(rename = "expiresAt", default)]
    pub expires_at: Option<i64>,
    #[serde(rename = "subscriptionType", default)]
    pub subscription_type: Option<String>,
    #[serde(rename = "rateLimitTier", default)]
    pub rate_limit_tier: Option<String>,
}

impl OauthCredentials {
    /// True when the stored access token is past its expiry.
    pub fn is_expired(&self, now_ms: i64) -> bool {
        match self.expires_at {
            Some(exp) => now_ms >= exp,
            None => false,
        }
    }
}

/// Parse the contents of a `.credentials.json` file.
pub fn parse_credentials(raw: &str) -> Result<OauthCredentials, CoreError> {
    let file: CredentialsFile = serde_json::from_str(raw)?;
    Ok(file.claude_ai_oauth)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "claudeAiOauth": {
            "accessToken": "sk-ant-oat-xxx",
            "refreshToken": "sk-ant-ort-yyy",
            "expiresAt": 1750000000000,
            "scopes": ["user:inference", "user:profile"],
            "subscriptionType": "max",
            "rateLimitTier": "default_claude_max_5x"
        }
    }"#;

    #[test]
    fn parses_full_credentials() {
        let creds = parse_credentials(SAMPLE).unwrap();
        assert_eq!(creds.access_token, "sk-ant-oat-xxx");
        assert_eq!(creds.subscription_type.as_deref(), Some("max"));
        assert_eq!(creds.rate_limit_tier.as_deref(), Some("default_claude_max_5x"));
        assert_eq!(creds.expires_at, Some(1750000000000));
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        let raw = r#"{"claudeAiOauth":{"accessToken":"tok"}}"#;
        let creds = parse_credentials(raw).unwrap();
        assert_eq!(creds.access_token, "tok");
        assert!(creds.refresh_token.is_none());
        assert!(!creds.is_expired(0));
    }

    #[test]
    fn detects_expiry() {
        let creds = parse_credentials(SAMPLE).unwrap();
        assert!(!creds.is_expired(1740000000000));
        assert!(creds.is_expired(1760000000000));
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(parse_credentials("not json").is_err());
        assert!(parse_credentials("{}").is_err());
    }
}
