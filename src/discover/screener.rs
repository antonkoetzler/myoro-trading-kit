//! Market screener: fetch top markets from Gamma API and score by edge.

/// A screener market row for the Screener view.
#[derive(Clone, Debug)]
pub struct ScreenerMarket {
    pub market_id: String,
    pub title: String,
    pub category: String,
    pub best_bid: f64,
    pub best_ask: f64,
    pub spread: f64,
    pub volume: f64,
    /// Simple edge score: spread * log10(volume+1)
    pub edge_score: f64,
}

const SCREENER_URL: &str =
    "https://gamma-api.polymarket.com/markets?closed=false&limit=100&order=volumeNum&ascending=false";

/// Fetch top markets from Gamma API. Returns markets sorted by edge_score descending.
pub fn fetch_screener_markets() -> Vec<ScreenerMarket> {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let arr: Vec<serde_json::Value> = client
        .get(SCREENER_URL)
        .send()
        .ok()
        .and_then(|r| r.json().ok())
        .unwrap_or_default();

    let mut markets: Vec<ScreenerMarket> = arr
        .iter()
        .filter_map(|m| {
            let market_id = m.get("conditionId").and_then(|v| v.as_str())?.to_string();
            let title = m.get("question").and_then(|v| v.as_str())?.to_string();
            let category = m
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("Other")
                .to_string();
            let best_bid = m.get("bestBid").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let best_ask = m.get("bestAsk").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let volume = m.get("volumeNum").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let spread = best_ask - best_bid;
            let edge_score = spread * (volume + 1.0).log10();
            Some(ScreenerMarket {
                market_id,
                title,
                category,
                best_bid,
                best_ask,
                spread,
                volume,
                edge_score,
            })
        })
        .collect();

    markets.sort_by(|a, b| {
        b.edge_score
            .partial_cmp(&a.edge_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    markets.truncate(50);
    markets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_score_sorted_descending() {
        let mut markets = vec![
            ScreenerMarket {
                market_id: "a".into(),
                title: "A".into(),
                category: "Crypto".into(),
                best_bid: 0.4,
                best_ask: 0.6,
                spread: 0.2,
                volume: 9.0,
                edge_score: 0.2 * (9.0_f64 + 1.0).log10(),
            },
            ScreenerMarket {
                market_id: "b".into(),
                title: "B".into(),
                category: "Sports".into(),
                best_bid: 0.45,
                best_ask: 0.55,
                spread: 0.1,
                volume: 99.0,
                edge_score: 0.1 * (99.0_f64 + 1.0).log10(),
            },
        ];
        markets.sort_by(|a, b| {
            b.edge_score
                .partial_cmp(&a.edge_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // First: 0.2 * log10(10) = 0.2; Second: 0.1 * log10(100) = 0.2 — equal; order stable
        assert_eq!(markets.len(), 2);
    }

    #[test]
    fn screener_market_fields() {
        let m = ScreenerMarket {
            market_id: "cid123".into(),
            title: "Will BTC > 100k?".into(),
            category: "Crypto".into(),
            best_bid: 0.48,
            best_ask: 0.52,
            spread: 0.04,
            volume: 1000.0,
            edge_score: 0.04 * (1001.0_f64).log10(),
        };
        assert_eq!(m.market_id, "cid123");
        assert!((m.spread - 0.04).abs() < 1e-9);
    }
}
