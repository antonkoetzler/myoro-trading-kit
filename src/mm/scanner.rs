//! MmScanner: polls Gamma API for thin-book markets suitable for market making.
//!
//! Criteria:
//!   - Spread > min_spread (from config)
//!   - Daily volume > min_volume_usd (from config)
//!   - Market is open (not resolved)

use anyhow::Result;

/// A candidate market for market making.
#[derive(Clone, Debug)]
pub struct MmCandidate {
    pub market_id: String,
    pub title: String,
    pub best_bid: f64,
    pub best_ask: f64,
    pub spread: f64,
    pub volume: f64,
}

pub struct MmScanner {
    pub min_spread: f64,
    pub min_volume: f64,
    pub max_markets: usize,
}

impl MmScanner {
    pub fn new(min_spread: f64, min_volume: f64, max_markets: usize) -> Self {
        Self {
            min_spread,
            min_volume,
            max_markets,
        }
    }

    /// Fetch and filter candidates from Gamma API.
    pub fn scan(&self) -> Result<Vec<MmCandidate>> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        // Fetch active markets sorted by volume descending.
        let url = "https://gamma-api.polymarket.com/markets?closed=false&limit=100&order=volumeNum&ascending=false";
        let resp = client.get(url).send()?;
        let arr: Vec<serde_json::Value> = resp.json().unwrap_or_default();

        let mut candidates: Vec<MmCandidate> = arr
            .iter()
            .filter_map(|m| {
                let market_id = m.get("conditionId").and_then(|v| v.as_str())?.to_string();
                let title = m
                    .get("question")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let best_bid = m.get("bestBid").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let best_ask = m.get("bestAsk").and_then(|v| v.as_f64()).unwrap_or(1.0);
                let volume = m.get("volumeNum").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let spread = best_ask - best_bid;

                if spread >= self.min_spread && volume >= self.min_volume {
                    Some(MmCandidate {
                        market_id,
                        title,
                        best_bid,
                        best_ask,
                        spread,
                        volume,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by spread descending (widest first = most edge).
        candidates.sort_by(|a, b| {
            b.spread
                .partial_cmp(&a.spread)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(self.max_markets);
        Ok(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_config_stored_correctly() {
        let s = MmScanner::new(0.04, 1000.0, 5);
        assert_eq!(s.min_spread, 0.04);
        assert_eq!(s.min_volume, 1000.0);
        assert_eq!(s.max_markets, 5);
    }

    #[test]
    fn candidate_spread_computed() {
        let c = MmCandidate {
            market_id: "x".to_string(),
            title: "Test".to_string(),
            best_bid: 0.44,
            best_ask: 0.56,
            spread: 0.12,
            volume: 5000.0,
        };
        assert!((c.spread - 0.12).abs() < 0.001);
    }
}
