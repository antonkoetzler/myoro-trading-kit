//! Leaderboard types and fetch helpers.

use serde::Deserialize;

const LEADERBOARD: &str = "https://data-api.polymarket.com/v1/leaderboard";
const PAGE_SIZE: u32 = 50;
const MAX_OFFSET: u32 = 1000;

#[derive(Clone, Debug)]
pub struct LeaderboardEntry {
    pub rank: String,
    pub proxy_wallet: String,
    pub user_name: String,
    pub vol: f64,
    pub pnl: f64,
}

#[derive(Debug, Deserialize)]
struct ApiEntry {
    rank: Option<String>,
    #[serde(rename = "proxyWallet")]
    proxy_wallet: Option<String>,
    #[serde(rename = "userName")]
    user_name: Option<String>,
    vol: Option<f64>,
    pnl: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default)]
#[allow(clippy::upper_case_acronyms)] // Polymarket API expects OVERALL, CRYPTO, etc.
pub enum LeaderboardCategory {
    #[default]
    OVERALL,
    CRYPTO,
    SPORTS,
    POLITICS,
    CULTURE,
    WEATHER,
    ECONOMICS,
    TECH,
    FINANCE,
}

#[derive(Clone, Copy, Debug, Default)]
#[allow(clippy::upper_case_acronyms)] // API expects DAY, WEEK, etc.
pub enum TimePeriod {
    DAY,
    #[default]
    WEEK,
    MONTH,
    ALL,
}

#[derive(Clone, Copy, Debug, Default)]
#[allow(clippy::upper_case_acronyms)] // API expects PNL, VOL
pub enum OrderBy {
    #[default]
    PNL,
    VOL,
}

/// Fetch leaderboard pages for the given filters.
pub fn fetch_leaderboard(cat: &str, period: &str, order: &str) -> Vec<LeaderboardEntry> {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut all_entries: Vec<LeaderboardEntry> = Vec::new();
    let mut offset = 0u32;
    while offset <= MAX_OFFSET {
        let url = format!(
            "{}?category={}&timePeriod={}&orderBy={}&limit={}&offset={}",
            LEADERBOARD, cat, period, order, PAGE_SIZE, offset
        );
        let list: Vec<ApiEntry> = match client.get(&url).send().and_then(|r| r.json()) {
            Ok(l) => l,
            Err(_) => break,
        };
        let len = list.len();
        if len == 0 {
            break;
        }
        for e in list {
            let proxy_wallet = match &e.proxy_wallet {
                Some(s) if s.starts_with("0x") && s.len() >= 42 => s.clone(),
                _ => continue,
            };
            all_entries.push(LeaderboardEntry {
                rank: e.rank.unwrap_or_else(|| "—".to_string()),
                proxy_wallet,
                user_name: e.user_name.unwrap_or_else(|| "—".to_string()),
                vol: e.vol.unwrap_or(0.0),
                pnl: e.pnl.unwrap_or(0.0),
            });
        }
        if len < PAGE_SIZE as usize {
            break;
        }
        offset += PAGE_SIZE;
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    all_entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaderboard_category_defaults_to_overall() {
        let c = LeaderboardCategory::default();
        assert!(matches!(c, LeaderboardCategory::OVERALL));
    }

    #[test]
    fn time_period_defaults_to_week() {
        let p = TimePeriod::default();
        assert!(matches!(p, TimePeriod::WEEK));
    }

    #[test]
    fn order_by_defaults_to_pnl() {
        let o = OrderBy::default();
        assert!(matches!(o, OrderBy::PNL));
    }
}
