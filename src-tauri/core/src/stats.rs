//! Parsing of `~/.claude/stats-cache.json` for the token-history view.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::CoreError;

/// Per-day token totals broken down by model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyTokens {
    pub date: String,
    #[serde(rename = "tokensByModel", default)]
    pub tokens_by_model: BTreeMap<String, u64>,
    /// Estimated API-list cost for the day (0 when read from the cache fallback).
    #[serde(rename = "costUsd", default)]
    pub cost_usd: f64,
}

impl DailyTokens {
    pub fn total(&self) -> u64 {
        self.tokens_by_model.values().copied().sum()
    }
}

/// Cumulative per-model usage totals.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    #[serde(rename = "inputTokens", default)]
    pub input_tokens: u64,
    #[serde(rename = "outputTokens", default)]
    pub output_tokens: u64,
    #[serde(rename = "cacheReadInputTokens", default)]
    pub cache_read_input_tokens: u64,
    #[serde(rename = "cacheCreationInputTokens", default)]
    pub cache_creation_input_tokens: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawStats {
    #[serde(rename = "dailyModelTokens", default)]
    daily_model_tokens: Vec<DailyTokens>,
    #[serde(rename = "modelUsage", default)]
    model_usage: BTreeMap<String, ModelUsage>,
    #[serde(rename = "totalSessions", default)]
    total_sessions: u64,
    #[serde(rename = "totalMessages", default)]
    total_messages: u64,
}

/// Shaped history sent to the frontend chart.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsHistory {
    /// Most recent `max_days` days, chronological (oldest first).
    pub days: Vec<DailyTokens>,
    /// All model names appearing in `days`, sorted for stable colour assignment.
    pub models: Vec<String>,
    pub model_usage: BTreeMap<String, ModelUsage>,
    pub total_sessions: u64,
    pub total_messages: u64,
    /// Estimated API-list cost across `days`.
    pub cost_usd: f64,
}

/// Parse `stats-cache.json`, keeping only the most recent `max_days` days.
pub fn parse_stats(raw: &str, max_days: usize) -> Result<StatsHistory, CoreError> {
    let mut parsed: RawStats = serde_json::from_str(raw)?;

    // ISO dates sort lexically; sort then take the tail so order is guaranteed.
    parsed.daily_model_tokens.sort_by(|a, b| a.date.cmp(&b.date));
    let days: Vec<DailyTokens> = parsed
        .daily_model_tokens
        .into_iter()
        .rev()
        .take(max_days)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let models: Vec<String> = days
        .iter()
        .flat_map(|d| d.tokens_by_model.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(StatsHistory {
        days,
        models,
        model_usage: parsed.model_usage,
        total_sessions: parsed.total_sessions,
        total_messages: parsed.total_messages,
        cost_usd: 0.0, // not derivable from the cache (no per-kind breakdown)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "version": 4,
        "dailyModelTokens": [
            {"date": "2026-05-13", "tokensByModel": {"claude-sonnet-4-6": 40902}},
            {"date": "2026-05-30", "tokensByModel": {"claude-sonnet-4-6": 98388, "claude-opus-4-8": 3277615}},
            {"date": "2026-05-24", "tokensByModel": {"claude-sonnet-4-6": 239611, "claude-opus-4-7": 70335}}
        ],
        "modelUsage": {
            "claude-opus-4-8": {"inputTokens": 22160, "outputTokens": 3273929, "cacheReadInputTokens": 23859332, "cacheCreationInputTokens": 4871567}
        },
        "totalSessions": 39,
        "totalMessages": 6443
    }"#;

    #[test]
    fn parses_and_sorts_recent_days() {
        let h = parse_stats(SAMPLE, 10).unwrap();
        let dates: Vec<&str> = h.days.iter().map(|d| d.date.as_str()).collect();
        assert_eq!(dates, vec!["2026-05-13", "2026-05-24", "2026-05-30"]);
        assert_eq!(h.total_sessions, 39);
        assert_eq!(h.total_messages, 6443);
    }

    #[test]
    fn keeps_only_max_days_most_recent() {
        let h = parse_stats(SAMPLE, 2).unwrap();
        let dates: Vec<&str> = h.days.iter().map(|d| d.date.as_str()).collect();
        assert_eq!(dates, vec!["2026-05-24", "2026-05-30"]);
    }

    #[test]
    fn collects_model_set_and_totals() {
        let h = parse_stats(SAMPLE, 10).unwrap();
        assert_eq!(
            h.models,
            vec!["claude-opus-4-7", "claude-opus-4-8", "claude-sonnet-4-6"]
        );
        let opus = &h.model_usage["claude-opus-4-8"];
        assert_eq!(opus.output_tokens, 3273929);
    }

    #[test]
    fn daily_total_sums_models() {
        let h = parse_stats(SAMPLE, 10).unwrap();
        let may30 = h.days.iter().find(|d| d.date == "2026-05-30").unwrap();
        assert_eq!(may30.total(), 98388 + 3277615);
    }

    #[test]
    fn handles_missing_sections() {
        let h = parse_stats(r#"{"version":4}"#, 10).unwrap();
        assert!(h.days.is_empty());
        assert!(h.models.is_empty());
        assert_eq!(h.total_sessions, 0);
    }
}
