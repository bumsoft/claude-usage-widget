//! The live subscription-usage endpoint and its types.
//!
//! `GET https://api.anthropic.com/api/oauth/usage` (OAuth bearer) returns the
//! same rolling-window utilization that Claude Code's `/usage` shows.

use serde::{Deserialize, Serialize};

use crate::{credentials::OauthCredentials, CoreError};

pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
pub const OAUTH_BETA: &str = "oauth-2025-04-20";

/// A single rolling-limit window (e.g. the 5-hour or 7-day window).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageWindow {
    /// Percent of the window consumed (0–100).
    #[serde(default)]
    pub utilization: f64,
    /// RFC3339 reset timestamp, passed through to the UI for countdowns.
    #[serde(default)]
    pub resets_at: Option<String>,
}

/// Pay-as-you-go overflow credits, shown only when the org has them enabled.
/// Field names are snake_case to match both the API payload and the frontend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtraUsage {
    #[serde(default)]
    pub is_enabled: bool,
    #[serde(default)]
    pub monthly_limit: Option<f64>,
    #[serde(default)]
    pub used_credits: Option<f64>,
    #[serde(default)]
    pub utilization: Option<f64>,
    #[serde(default)]
    pub currency: Option<String>,
}

/// Raw response from the usage endpoint. Unknown/extra fields are ignored.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawUsage {
    #[serde(default)]
    pub five_hour: Option<UsageWindow>,
    #[serde(default)]
    pub seven_day: Option<UsageWindow>,
    #[serde(default)]
    pub seven_day_opus: Option<UsageWindow>,
    #[serde(default)]
    pub seven_day_sonnet: Option<UsageWindow>,
    #[serde(default)]
    pub extra_usage: Option<ExtraUsage>,
}

/// What the frontend renders: usage windows plus a friendly plan label.
/// The access token is never included.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub plan: String,
    pub subscription_type: Option<String>,
    pub rate_limit_tier: Option<String>,
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
    pub seven_day_opus: Option<UsageWindow>,
    pub seven_day_sonnet: Option<UsageWindow>,
    pub extra_usage: Option<ExtraUsage>,
    pub fetched_at_ms: i64,
}

/// Parse a raw usage JSON payload.
pub fn parse_usage(raw: &str) -> Result<RawUsage, CoreError> {
    Ok(serde_json::from_str(raw)?)
}

/// Build a friendly plan name from the subscription type and rate-limit tier.
/// e.g. `("max", "default_claude_max_5x")` -> `"Claude Max 5x"`.
pub fn plan_label(subscription_type: Option<&str>, rate_limit_tier: Option<&str>) -> String {
    let tier = rate_limit_tier.unwrap_or("");
    let detail = if tier.contains("max_20x") {
        "Max 20x".to_string()
    } else if tier.contains("max_5x") {
        "Max 5x".to_string()
    } else if tier.contains("max") {
        "Max".to_string()
    } else if tier.contains("team") {
        "Team".to_string()
    } else if tier.contains("pro") {
        "Pro".to_string()
    } else if tier.contains("free") {
        "Free".to_string()
    } else {
        match subscription_type {
            Some("max") => "Max".to_string(),
            Some("pro") => "Pro".to_string(),
            Some(other) if !other.is_empty() => capitalize(other),
            _ => String::new(),
        }
    };

    if detail.is_empty() {
        "Claude".to_string()
    } else {
        format!("Claude {detail}")
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Call the usage endpoint with the given bearer token.
pub async fn fetch_raw_usage(
    client: &reqwest::Client,
    token: &str,
) -> Result<RawUsage, CoreError> {
    let resp = client
        .get(USAGE_URL)
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-beta", OAUTH_BETA)
        .header("User-Agent", "claude-usage-widget")
        .send()
        .await?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(CoreError::Unauthorized);
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .filter(|&s| s > 0);
        return Err(CoreError::RateLimited(retry_after));
    }
    if !status.is_success() {
        return Err(CoreError::Http(status.as_u16()));
    }
    Ok(resp.json::<RawUsage>().await?)
}

/// Fetch usage and assemble a snapshot for the given credentials.
pub async fn fetch_usage_snapshot(
    client: &reqwest::Client,
    creds: &OauthCredentials,
    now_ms: i64,
) -> Result<UsageSnapshot, CoreError> {
    let raw = fetch_raw_usage(client, &creds.access_token).await?;
    Ok(UsageSnapshot {
        plan: plan_label(
            creds.subscription_type.as_deref(),
            creds.rate_limit_tier.as_deref(),
        ),
        subscription_type: creds.subscription_type.clone(),
        rate_limit_tier: creds.rate_limit_tier.clone(),
        five_hour: raw.five_hour,
        seven_day: raw.seven_day,
        seven_day_opus: raw.seven_day_opus,
        seven_day_sonnet: raw.seven_day_sonnet,
        extra_usage: raw.extra_usage,
        fetched_at_ms: now_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "five_hour": {"utilization": 3.0, "resets_at": "2026-06-13T09:10:00+00:00"},
        "seven_day": {"utilization": 0.0, "resets_at": "2026-06-14T05:00:00+00:00"},
        "seven_day_opus": null,
        "seven_day_sonnet": {"utilization": 12.5, "resets_at": null},
        "extra_unknown_field": {"foo": 1}
    }"#;

    #[test]
    fn parses_usage_payload() {
        let u = parse_usage(SAMPLE).unwrap();
        assert_eq!(u.five_hour.as_ref().unwrap().utilization, 3.0);
        assert_eq!(
            u.five_hour.as_ref().unwrap().resets_at.as_deref(),
            Some("2026-06-13T09:10:00+00:00")
        );
        assert!(u.seven_day_opus.is_none());
        assert_eq!(u.seven_day_sonnet.as_ref().unwrap().utilization, 12.5);
        assert!(u.seven_day_sonnet.as_ref().unwrap().resets_at.is_none());
    }

    #[test]
    fn empty_object_yields_no_windows() {
        let u = parse_usage("{}").unwrap();
        assert!(u.five_hour.is_none());
        assert!(u.seven_day.is_none());
    }

    #[test]
    fn parses_extra_usage_when_present() {
        let raw = r#"{"five_hour":{"utilization":1.0},"extra_usage":{"is_enabled":true,"monthly_limit":50.0,"used_credits":12.5,"utilization":25.0,"currency":"USD"}}"#;
        let u = parse_usage(raw).unwrap();
        let extra = u.extra_usage.unwrap();
        assert!(extra.is_enabled);
        assert_eq!(extra.monthly_limit, Some(50.0));
        assert_eq!(extra.used_credits, Some(12.5));
        assert_eq!(extra.currency.as_deref(), Some("USD"));
    }

    #[test]
    fn extra_usage_absent_is_none() {
        let u = parse_usage(r#"{"five_hour":{"utilization":1.0}}"#).unwrap();
        assert!(u.extra_usage.is_none());
    }

    #[test]
    fn plan_label_variants() {
        assert_eq!(plan_label(Some("max"), Some("default_claude_max_5x")), "Claude Max 5x");
        assert_eq!(plan_label(Some("max"), Some("claude_max_20x")), "Claude Max 20x");
        assert_eq!(plan_label(Some("pro"), Some("claude_pro")), "Claude Pro");
        assert_eq!(plan_label(Some("max"), None), "Claude Max");
        assert_eq!(plan_label(None, None), "Claude");
        assert_eq!(plan_label(Some("enterprise"), None), "Claude Enterprise");
    }
}
