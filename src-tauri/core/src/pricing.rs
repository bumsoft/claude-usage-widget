//! Estimated API-list pricing, used to show the dollar value of usage.
//!
//! A subscription has no per-token cost, so these figures are an *estimate* of
//! what the same tokens would cost on the pay-as-you-go API (公개 단가 기준).
//! Prices are USD per 1,000,000 tokens.

/// Per-million-token prices for one model family.
#[derive(Debug, Clone, Copy)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    pub cache_write: f64,
    pub cache_read: f64,
}

/// Resolve a model id to its price tier. Unknown models fall back to the
/// Sonnet tier as a middle-of-the-road estimate.
pub fn price_for(model: &str) -> Price {
    if model.contains("opus") {
        Price { input: 15.0, output: 75.0, cache_write: 18.75, cache_read: 1.50 }
    } else if model.contains("haiku") {
        Price { input: 1.0, output: 5.0, cache_write: 1.25, cache_read: 0.10 }
    } else {
        // sonnet, fable, and any unrecognized model
        Price { input: 3.0, output: 15.0, cache_write: 3.75, cache_read: 0.30 }
    }
}

/// Estimated USD cost for one model's token counts.
pub fn cost_usd(
    model: &str,
    input: u64,
    output: u64,
    cache_write: u64,
    cache_read: u64,
) -> f64 {
    let p = price_for(model);
    (input as f64 * p.input
        + output as f64 * p.output
        + cache_write as f64 * p.cache_write
        + cache_read as f64 * p.cache_read)
        / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_costs_more_than_sonnet() {
        let opus = cost_usd("claude-opus-4-8", 1_000_000, 1_000_000, 0, 0);
        let sonnet = cost_usd("claude-sonnet-4-6", 1_000_000, 1_000_000, 0, 0);
        assert_eq!(opus, 15.0 + 75.0);
        assert_eq!(sonnet, 3.0 + 15.0);
        assert!(opus > sonnet);
    }

    #[test]
    fn includes_cache_pricing() {
        // 1M cache reads on opus = $1.50; 1M cache writes = $18.75
        let c = cost_usd("claude-opus-4-8", 0, 0, 1_000_000, 1_000_000);
        assert!((c - (18.75 + 1.50)).abs() < 1e-9);
    }

    #[test]
    fn unknown_model_uses_sonnet_tier() {
        assert_eq!(
            cost_usd("claude-fable-5", 1_000_000, 0, 0, 0),
            cost_usd("claude-sonnet-4-6", 1_000_000, 0, 0, 0),
        );
    }
}
