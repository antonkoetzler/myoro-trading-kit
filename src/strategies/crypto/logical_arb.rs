//! LogicalArbStrategy: detect when correlated Polymarket crypto markets are mispriced.
//!
//! Logic:
//!   Fetch all open crypto markets from Gamma API.
//!   Group markets that are logically nested (e.g., "BTC > $100K" and "BTC > $90K").
//!   If P("BTC > $90K") < P("BTC > $100K"), that is impossible → arb exists.
//!   Emit a signal to buy the underpriced market.

use super::{GammaMarket, StoredCryptoSignal};
use crate::shared::strategy::{Side, Signal, Strategy, StrategyMetadata};
use anyhow::Result;
use chrono::Utc;

const MIN_EDGE: f64 = 0.03;
const KELLY_FRACTION: f64 = 0.25;

pub struct LogicalArbStrategy;

impl LogicalArbStrategy {
    pub fn new() -> Self {
        Self
    }

    fn fetch_markets(client: &reqwest::blocking::Client) -> Result<Vec<GammaMarket>> {
        let url = "https://gamma-api.polymarket.com/markets?closed=false&limit=50&tag=crypto";
        let resp = client.get(url).send()?;
        let arr: Vec<serde_json::Value> = resp.json().unwrap_or_default();
        Ok(arr
            .iter()
            .filter_map(|m| {
                let id = m.get("conditionId").and_then(|v| v.as_str())?.to_string();
                let title = m.get("question").and_then(|v| v.as_str())?.to_string();
                let best_bid = m.get("bestBid").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let best_ask = m.get("bestAsk").and_then(|v| v.as_f64()).unwrap_or(1.0);
                let volume = m.get("volumeNum").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Some(GammaMarket {
                    id,
                    title,
                    best_bid,
                    best_ask,
                    volume,
                })
            })
            .collect())
    }

    /// Extract a threshold from a market title like "Will BTC reach $100,000?"
    /// Returns the numeric threshold if found.
    fn extract_threshold(title: &str) -> Option<f64> {
        // Look for patterns like "$100,000" or "$90k" or "100000"
        let lower = title.to_lowercase();
        // Only process BTC/ETH price level markets
        if !lower.contains("btc") && !lower.contains("bitcoin") {
            return None;
        }
        // Find dollar amounts: strip commas, parse number after "$"
        let bytes = title.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'$' {
                let rest = &title[i + 1..];
                let digits: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == ',')
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                if !digits.is_empty() {
                    if let Ok(n) = digits.parse::<f64>() {
                        return Some(n);
                    }
                }
            }
        }
        None
    }

    /// Find logical arb pairs: for two markets A (threshold_a > threshold_b),
    /// P(A) must be <= P(B). If violated, buy the underpriced one.
    pub fn find_arb_signals(markets: &[GammaMarket]) -> Vec<StoredCryptoSignal> {
        // Collect markets with BTC thresholds.
        let mut with_threshold: Vec<(f64, &GammaMarket)> = markets
            .iter()
            .filter_map(|m| {
                let t = Self::extract_threshold(&m.title)?;
                Some((t, m))
            })
            .collect();
        with_threshold.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut signals = Vec::new();
        // Compare each pair: higher threshold should have lower or equal probability.
        for i in 0..with_threshold.len() {
            for j in (i + 1)..with_threshold.len() {
                let (thresh_lo, mkt_lo) = &with_threshold[i]; // lower price target
                let (thresh_hi, mkt_hi) = &with_threshold[j]; // higher price target
                if thresh_hi <= thresh_lo {
                    continue;
                }
                let mid_lo = (mkt_lo.best_bid + mkt_lo.best_ask) / 2.0;
                let mid_hi = (mkt_hi.best_bid + mkt_hi.best_ask) / 2.0;
                // Logical constraint: P(BTC > hi) <= P(BTC > lo)
                // Violation: mid_hi > mid_lo  (higher target priced higher — impossible)
                if mid_hi > mid_lo + MIN_EDGE {
                    // Buy the lower-threshold market YES (it's underpriced)
                    let edge = mid_hi - mid_lo;
                    let kelly = (edge / (1.0 - mid_lo)) * KELLY_FRACTION;
                    signals.push(StoredCryptoSignal {
                        market_id: mkt_lo.id.clone(),
                        label: format!("Arb: {} vs {}", mkt_lo.title, mkt_hi.title),
                        side: "Yes".to_string(),
                        edge_pct: edge,
                        kelly_size: kelly.min(0.1),
                        strategy_id: "logical_arb".to_string(),
                        status: "pending".to_string(),
                        created_at: Utc::now(),
                    });
                }
            }
        }
        signals
    }

    pub fn run(&self) -> Result<Vec<StoredCryptoSignal>> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let markets = Self::fetch_markets(&client)?;
        Ok(Self::find_arb_signals(&markets))
    }
}

impl Strategy for LogicalArbStrategy {
    fn id(&self) -> &'static str {
        "logical_arb"
    }

    fn metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            name: "Logical Arb",
            domain: "crypto",
        }
    }

    fn signal(&self) -> Result<Option<Signal>> {
        let signals = self.run()?;
        Ok(signals.into_iter().next().map(|s| Signal {
            market_id: s.market_id,
            side: Side::Yes,
            confidence: s.edge_pct.min(1.0),
            edge_pct: s.edge_pct,
            kelly_size: s.kelly_size,
            auto_execute: false,
            strategy_id: s.strategy_id,
            metadata: None,
            stop_loss_pct: None,
            take_profit_pct: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_market(id: &str, title: &str, bid: f64, ask: f64) -> GammaMarket {
        GammaMarket {
            id: id.to_string(),
            title: title.to_string(),
            best_bid: bid,
            best_ask: ask,
            volume: 1000.0,
        }
    }

    #[test]
    fn extract_threshold_parses_btc_100k() {
        let t = LogicalArbStrategy::extract_threshold("Will BTC reach $100,000 by year end?");
        assert_eq!(t, Some(100_000.0));
    }

    #[test]
    fn extract_threshold_no_match_non_btc() {
        let t = LogicalArbStrategy::extract_threshold("Will ETH reach $5,000?");
        assert_eq!(t, None);
    }

    #[test]
    fn arb_detected_when_higher_threshold_priced_higher() {
        // P(BTC > $100K) = 0.60, P(BTC > $90K) = 0.50 → impossible, arb exists
        let markets = vec![
            make_market(
                "a",
                "Will BTC hit $90,000 this year?",
                0.48,
                0.52, // mid = 0.50
            ),
            make_market(
                "b",
                "Will BTC hit $100,000 this year?",
                0.58,
                0.62, // mid = 0.60
            ),
        ];
        let signals = LogicalArbStrategy::find_arb_signals(&markets);
        assert!(!signals.is_empty());
        assert!(signals[0].edge_pct >= MIN_EDGE);
    }

    #[test]
    fn no_arb_when_properly_ordered() {
        // P(BTC > $100K) = 0.30, P(BTC > $90K) = 0.50 → correct ordering
        let markets = vec![
            make_market("a", "Will BTC hit $90,000 this year?", 0.48, 0.52),
            make_market("b", "Will BTC hit $100,000 this year?", 0.28, 0.32),
        ];
        let signals = LogicalArbStrategy::find_arb_signals(&markets);
        assert!(signals.is_empty());
    }
}

impl Default for LogicalArbStrategy {
    fn default() -> Self {
        Self::new()
    }
}
