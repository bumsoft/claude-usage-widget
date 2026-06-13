//! Token-history aggregation straight from Claude Code's raw transcripts.
//!
//! `stats-cache.json` lags behind (it is only recomputed periodically), so the
//! daily chart is built by scanning `projects/**/*.jsonl` and summing the
//! `message.usage` of each assistant turn. This always includes today.
//!
//! Days are bucketed by the UTC date in each line's `timestamp` (the first 10
//! chars), which needs no date parsing and no timezone library.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde_json::Value;

use crate::pricing;
use crate::stats::{DailyTokens, ModelUsage, StatsHistory};

const DAY_MS: i64 = 86_400_000;

#[derive(Default, Clone, Copy)]
pub(crate) struct Counts {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
}

impl Counts {
    /// Sum of all token kinds — used only to drop genuinely empty rows.
    pub(crate) fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_creation
    }
    /// "Fresh" tokens charted per day: new input + output + cache writes,
    /// excluding cache reads (which re-read existing context and would
    /// otherwise dominate and flatten the chart).
    pub(crate) fn work(&self) -> u64 {
        self.input + self.output + self.cache_creation
    }
    fn add(&mut self, other: &Counts) {
        self.input += other.input;
        self.output += other.output;
        self.cache_read += other.cache_read;
        self.cache_creation += other.cache_creation;
    }
}

type DayMap = BTreeMap<String, BTreeMap<String, Counts>>;
type ModelMap = BTreeMap<String, Counts>;

/// Parse one transcript line into `(utc_date, model, counts)`.
///
/// Returns `None` for non-assistant lines, synthetic models (`<...>`), or
/// rows whose total token count is zero.
pub(crate) fn parse_usage_line(line: &str) -> Option<(String, String, Counts)> {
    let v: Value = serde_json::from_str(line).ok()?;
    if v.get("type")?.as_str()? != "assistant" {
        return None;
    }
    let date = v.get("timestamp")?.as_str()?.get(..10)?.to_string();
    let msg = v.get("message")?;
    let model = msg.get("model")?.as_str()?;
    if model.is_empty() || model.starts_with('<') {
        return None;
    }
    let usage = msg.get("usage")?;
    let get = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
    let counts = Counts {
        input: get("input_tokens"),
        output: get("output_tokens"),
        cache_read: get("cache_read_input_tokens"),
        cache_creation: get("cache_creation_input_tokens"),
    };
    if counts.total() == 0 {
        return None;
    }
    Some((date, model.to_string(), counts))
}

/// Walk `projects/**/*.jsonl` and aggregate token usage by day and model.
/// Files older than the window (by mtime) are skipped to limit IO.
pub fn aggregate_transcripts(projects_dir: &Path, max_days: usize, now_ms: i64) -> StatsHistory {
    let mut by_day: DayMap = BTreeMap::new();
    let mut model_totals: ModelMap = BTreeMap::new();
    let cutoff_ms = now_ms - ((max_days as i64) + 2) * DAY_MS;

    if let Ok(projects) = fs::read_dir(projects_dir) {
        for project in projects.flatten() {
            let dir = project.path();
            if !dir.is_dir() {
                continue;
            }
            let Ok(files) = fs::read_dir(&dir) else {
                continue;
            };
            for file in files.flatten() {
                let path = file.path();
                if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }
                if file_older_than(&path, cutoff_ms) {
                    continue;
                }
                accumulate_file(&path, &mut by_day, &mut model_totals);
            }
        }
    }

    build_history(by_day, model_totals, max_days)
}

fn file_older_than(path: &Path, cutoff_ms: i64) -> bool {
    match fs::metadata(path).and_then(|m| m.modified()) {
        Ok(time) => match time.duration_since(UNIX_EPOCH) {
            Ok(d) => (d.as_millis() as i64) < cutoff_ms,
            Err(_) => false,
        },
        Err(_) => false,
    }
}

fn accumulate_file(path: &Path, by_day: &mut DayMap, model_totals: &mut ModelMap) {
    let Ok(file) = fs::File::open(path) else {
        return;
    };
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if let Some((date, model, counts)) = parse_usage_line(&line) {
            by_day
                .entry(date)
                .or_default()
                .entry(model.clone())
                .or_default()
                .add(&counts);
            model_totals.entry(model).or_default().add(&counts);
        }
    }
}

pub(crate) fn build_history(by_day: DayMap, model_totals: ModelMap, max_days: usize) -> StatsHistory {
    let dates: Vec<String> = by_day.keys().cloned().collect();
    let start = dates.len().saturating_sub(max_days);

    let mut total_cost = 0.0;
    let days: Vec<DailyTokens> = dates[start..]
        .iter()
        .map(|date| {
            let mut day_cost = 0.0;
            let tokens_by_model = by_day[date]
                .iter()
                .map(|(model, c)| {
                    day_cost +=
                        pricing::cost_usd(model, c.input, c.output, c.cache_creation, c.cache_read);
                    (model.clone(), c.work())
                })
                .collect();
            total_cost += day_cost;
            DailyTokens {
                date: date.clone(),
                tokens_by_model,
                cost_usd: day_cost,
            }
        })
        .collect();

    let models: Vec<String> = days
        .iter()
        .flat_map(|d| d.tokens_by_model.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let model_usage: BTreeMap<String, ModelUsage> = model_totals
        .iter()
        .map(|(model, c)| {
            (
                model.clone(),
                ModelUsage {
                    input_tokens: c.input,
                    output_tokens: c.output,
                    cache_read_input_tokens: c.cache_read,
                    cache_creation_input_tokens: c.cache_creation,
                },
            )
        })
        .collect();

    StatsHistory {
        days,
        models,
        model_usage,
        total_sessions: 0,
        total_messages: 0,
        cost_usd: total_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(ts: &str, ty: &str, model: &str, input: u64, output: u64) -> String {
        format!(
            r#"{{"type":"{ty}","timestamp":"{ts}","message":{{"model":"{model}","usage":{{"input_tokens":{input},"output_tokens":{output},"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}}}}"#
        )
    }

    #[test]
    fn parses_assistant_usage_line() {
        let l = line("2026-06-13T05:04:34.451Z", "assistant", "claude-opus-4-8", 10, 20);
        let (date, model, counts) = parse_usage_line(&l).unwrap();
        assert_eq!(date, "2026-06-13");
        assert_eq!(model, "claude-opus-4-8");
        assert_eq!(counts.total(), 30);
    }

    #[test]
    fn skips_synthetic_zero_and_non_assistant() {
        assert!(parse_usage_line(&line("2026-06-13T00:00:00Z", "assistant", "<synthetic>", 0, 0)).is_none());
        assert!(parse_usage_line(&line("2026-06-13T00:00:00Z", "assistant", "claude-opus-4-8", 0, 0)).is_none());
        assert!(parse_usage_line(&line("2026-06-13T00:00:00Z", "user", "claude-opus-4-8", 5, 5)).is_none());
        assert!(parse_usage_line("not json").is_none());
    }

    #[test]
    fn build_history_windows_and_aggregates() {
        let mut by_day: DayMap = BTreeMap::new();
        for (date, model, total) in [
            ("2026-06-10", "claude-opus-4-8", 100u64),
            ("2026-06-12", "claude-opus-4-8", 200),
            ("2026-06-12", "claude-sonnet-4-6", 50),
            ("2026-06-13", "claude-opus-4-8", 300),
        ] {
            by_day.entry(date.to_string()).or_default().insert(
                model.to_string(),
                Counts { output: total, ..Default::default() },
            );
        }
        let mut totals: ModelMap = BTreeMap::new();
        totals.insert("claude-opus-4-8".into(), Counts { output: 600, ..Default::default() });
        totals.insert("claude-sonnet-4-6".into(), Counts { output: 50, ..Default::default() });

        let h = build_history(by_day, totals, 2);
        let dates: Vec<&str> = h.days.iter().map(|d| d.date.as_str()).collect();
        assert_eq!(dates, vec!["2026-06-12", "2026-06-13"]); // most recent 2 days
        assert_eq!(h.models, vec!["claude-opus-4-8", "claude-sonnet-4-6"]);
        let day12 = h.days.iter().find(|d| d.date == "2026-06-12").unwrap();
        assert_eq!(day12.tokens_by_model["claude-sonnet-4-6"], 50);
    }
}
