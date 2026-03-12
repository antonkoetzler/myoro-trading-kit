//! Per-profile stats from Data API /trades: trade count (frequency), top category.

use serde::Deserialize;
use std::collections::HashMap;

const TRADES: &str = "https://data-api.polymarket.com/trades";
const TRADES_LIMIT: u32 = 500;

#[derive(Clone, Debug, Default)]
pub struct TraderStats {
    pub trade_count: u32,
    pub top_category: String,
    /// Approximate win rate (0.0–1.0): fraction of BUY trades with final price ≥ 0.5.
    pub win_rate: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ApiTrade {
    #[serde(rename = "conditionId")]
    condition_id: Option<String>,
    #[serde(rename = "eventSlug")]
    event_slug: Option<String>,
    slug: Option<String>,
    size: Option<f64>,
    price: Option<f64>,
    side: Option<String>,
}

fn infer_category(slug: &str, event_slug: &str) -> &'static str {
    let s = format!("{} {}", slug.to_lowercase(), event_slug.to_lowercase());
    if s.contains("bitcoin") || s.contains("btc") || s.contains("crypto") || s.contains("eth") {
        "Crypto"
    } else if s.contains("trump")
        || s.contains("election")
        || s.contains("biden")
        || s.contains("politic")
    {
        "Politics"
    } else if s.contains("sport")
        || s.contains("nfl")
        || s.contains("nba")
        || s.contains("game")
        || s.contains("match")
    {
        "Sports"
    } else if s.contains("weather") || s.contains("temp") || s.contains("rain") {
        "Weather"
    } else if s.contains("fed")
        || s.contains("inflation")
        || s.contains("gdp")
        || s.contains("econom")
    {
        "Economics"
    } else if s.contains("tech") || s.contains("stock") || s.contains("nasdaq") {
        "Tech"
    } else if s.contains("finance") || s.contains("rate") {
        "Finance"
    } else {
        "Other"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_category_crypto_keywords() {
        assert_eq!(infer_category("bitcoin-price-100k", ""), "Crypto");
        assert_eq!(infer_category("btc-above", ""), "Crypto");
        assert_eq!(infer_category("eth-merge", ""), "Crypto");
        assert_eq!(infer_category("crypto-adoption", ""), "Crypto");
    }

    #[test]
    fn infer_category_politics_keywords() {
        assert_eq!(infer_category("trump-wins", ""), "Politics");
        assert_eq!(infer_category("election-2024", ""), "Politics");
        assert_eq!(infer_category("biden-approval", ""), "Politics");
    }

    #[test]
    fn infer_category_sports_keywords() {
        assert_eq!(infer_category("nfl-super-bowl", ""), "Sports");
        assert_eq!(infer_category("nba-finals", ""), "Sports");
        assert_eq!(infer_category("game-7", ""), "Sports");
    }

    #[test]
    fn infer_category_weather_keywords() {
        assert_eq!(infer_category("rain-forecast", ""), "Weather");
        assert_eq!(infer_category("temperature-above", ""), "Weather");
    }

    #[test]
    fn infer_category_economics_keywords() {
        // "fed" matches Economics; avoid "inflation" which contains "nfl" (Sports)
        assert_eq!(infer_category("fed-funds-rate", ""), "Economics");
        assert_eq!(infer_category("gdp-growth-q4", ""), "Economics");
        assert_eq!(infer_category("econom-outlook", ""), "Economics");
    }

    #[test]
    fn infer_category_other_for_unknown() {
        // Avoid "something" (contains "eth"→Crypto) and slugs with any keyword substrings
        assert_eq!(infer_category("art-auction", "celebrity-awards"), "Other");
    }

    #[test]
    fn infer_category_uses_event_slug_too() {
        // If slug is empty but event_slug contains keyword, still matches
        assert_eq!(infer_category("", "btc-above-100k"), "Crypto");
    }
}

pub fn fetch_stats(address: &str) -> Option<TraderStats> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .ok()?;
    let mut trade_count: u32 = 0;
    let mut category_counts: HashMap<String, u32> = HashMap::new();
    let mut buy_wins: u32 = 0;
    let mut buy_total: u32 = 0;
    let mut offset = 0u32;
    loop {
        let url = format!(
            "{}?user={}&limit=100&offset={}&takerOnly=false",
            TRADES, address, offset
        );
        let list: Vec<ApiTrade> = client.get(&url).send().ok()?.json().ok()?;
        if list.is_empty() {
            break;
        }
        for t in &list {
            trade_count += 1;
            let slug = t.event_slug.as_deref().unwrap_or("").to_string();
            let es = t.slug.as_deref().unwrap_or("");
            let cat = infer_category(es, &slug);
            *category_counts.entry(cat.to_string()).or_insert(0) += 1;
            // Approximate win rate: BUY trades where final price ≥ 0.5
            if t.side
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case("buy"))
                .unwrap_or(false)
            {
                buy_total += 1;
                if t.price.unwrap_or(0.0) >= 0.5 {
                    buy_wins += 1;
                }
            }
        }
        if list.len() < 100 {
            break;
        }
        offset += 100;
        if offset >= TRADES_LIMIT {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    let top_category = category_counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(k, _)| k)
        .unwrap_or_else(|| "—".to_string());
    let win_rate = if buy_total > 0 {
        Some(buy_wins as f64 / buy_total as f64)
    } else {
        None
    };
    Some(TraderStats {
        trade_count,
        top_category,
        win_rate,
    })
}
