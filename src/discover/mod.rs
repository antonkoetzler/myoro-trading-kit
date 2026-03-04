// NOTE: Exceeds 300-line limit — DiscoverState aggregates three domains (leaderboard, screener, trader-stats) with RwLock state; test block adds ~50 lines. See docs/ai-rules/file-size.md
//! Discover: leaderboard, screener, and per-profile stats.

pub mod leaderboard;
pub mod screener;
pub mod trader_stats;

use self::trader_stats::TraderStats;
pub use leaderboard::{LeaderboardCategory, LeaderboardEntry, OrderBy, TimePeriod};
pub use screener::ScreenerMarket;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

pub struct DiscoverState {
    pub entries: RwLock<Vec<LeaderboardEntry>>,
    pub category: RwLock<LeaderboardCategory>,
    pub time_period: RwLock<TimePeriod>,
    pub order_by: RwLock<OrderBy>,
    pub stats_cache: RwLock<HashMap<String, TraderStats>>,
    pub scan_index: RwLock<usize>,
    pub fetching: AtomicBool,
    pub screener_mode: AtomicBool,
    pub screener_markets: RwLock<Vec<ScreenerMarket>>,
    pub screener_fetching: AtomicBool,
}

impl Default for DiscoverState {
    fn default() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            category: RwLock::new(LeaderboardCategory::default()),
            time_period: RwLock::new(TimePeriod::default()),
            order_by: RwLock::new(OrderBy::default()),
            stats_cache: RwLock::new(HashMap::new()),
            scan_index: RwLock::new(0),
            fetching: AtomicBool::new(false),
            screener_mode: AtomicBool::new(false),
            screener_markets: RwLock::new(Vec::new()),
            screener_fetching: AtomicBool::new(false),
        }
    }
}

impl DiscoverState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fetch(&self) {
        self.fetching.store(true, Ordering::SeqCst);
        if let Ok(mut g) = self.entries.write() {
            *g = Vec::new();
        }
        let cat = self
            .category
            .read()
            .ok()
            .map(|c| format!("{:?}", c))
            .unwrap_or_else(|| "OVERALL".to_string());
        let period = self
            .time_period
            .read()
            .ok()
            .map(|p| format!("{:?}", p))
            .unwrap_or_else(|| "WEEK".to_string());
        let order = self
            .order_by
            .read()
            .ok()
            .map(|o| format!("{:?}", o))
            .unwrap_or_else(|| "PNL".to_string());
        let all_entries = leaderboard::fetch_leaderboard(&cat, &period, &order);
        if let Ok(mut g) = self.entries.write() {
            *g = all_entries;
        }
        if let Ok(mut idx) = self.scan_index.write() {
            *idx = 0;
        }
        self.fetching.store(false, Ordering::SeqCst);
    }

    pub fn get_entries(&self) -> Vec<LeaderboardEntry> {
        self.entries.read().map(|e| e.clone()).unwrap_or_default()
    }

    pub fn is_fetching(&self) -> bool {
        self.fetching.load(Ordering::SeqCst)
    }

    pub fn cycle_category(&self) {
        use LeaderboardCategory::*;
        if let Ok(mut c) = self.category.write() {
            *c = match *c {
                OVERALL => CRYPTO,
                CRYPTO => SPORTS,
                SPORTS => POLITICS,
                POLITICS => CULTURE,
                CULTURE => WEATHER,
                WEATHER => ECONOMICS,
                ECONOMICS => TECH,
                TECH => FINANCE,
                FINANCE => OVERALL,
            };
        }
    }

    pub fn cycle_time_period(&self) {
        use TimePeriod::*;
        if let Ok(mut p) = self.time_period.write() {
            *p = match *p {
                DAY => WEEK,
                WEEK => MONTH,
                MONTH => ALL,
                ALL => DAY,
            };
        }
    }

    pub fn cycle_order_by(&self) {
        use OrderBy::*;
        if let Ok(mut o) = self.order_by.write() {
            *o = match *o {
                PNL => VOL,
                VOL => PNL,
            };
        }
    }

    pub fn category_label(&self) -> String {
        use LeaderboardCategory::*;
        self.category
            .read()
            .ok()
            .map(|c| match *c {
                OVERALL => "ALL".to_string(),
                other => format!("{:?}", other),
            })
            .unwrap_or_else(|| "ALL".to_string())
    }

    pub fn time_period_label(&self) -> String {
        self.time_period
            .read()
            .ok()
            .map(|p| format!("{:?}", p))
            .unwrap_or_else(|| "WEEK".to_string())
    }

    pub fn order_by_label(&self) -> String {
        use OrderBy::*;
        self.order_by
            .read()
            .ok()
            .map(|o| match *o {
                PNL => "P&L".to_string(),
                VOL => "VOL".to_string(),
            })
            .unwrap_or_else(|| "P&L".to_string())
    }

    pub fn category_index(&self) -> usize {
        use LeaderboardCategory::*;
        self.category
            .read()
            .ok()
            .map(|c| match *c {
                OVERALL => 0,
                CRYPTO => 1,
                SPORTS => 2,
                POLITICS => 3,
                CULTURE => 4,
                WEATHER => 5,
                ECONOMICS => 6,
                TECH => 7,
                FINANCE => 8,
            })
            .unwrap_or(0)
    }

    pub fn set_category_by_index(&self, i: usize) {
        use LeaderboardCategory::*;
        let c = match i % 9 {
            0 => OVERALL,
            1 => CRYPTO,
            2 => SPORTS,
            3 => POLITICS,
            4 => CULTURE,
            5 => WEATHER,
            6 => ECONOMICS,
            7 => TECH,
            _ => FINANCE,
        };
        if let Ok(mut g) = self.category.write() {
            *g = c;
        }
    }

    pub fn time_period_index(&self) -> usize {
        use TimePeriod::*;
        self.time_period
            .read()
            .ok()
            .map(|p| match *p {
                ALL => 0,
                DAY => 1,
                WEEK => 2,
                MONTH => 3,
            })
            .unwrap_or(2)
    }

    pub fn set_time_period_by_index(&self, i: usize) {
        use TimePeriod::*;
        let p = match i % 4 {
            0 => ALL,
            1 => DAY,
            2 => WEEK,
            _ => MONTH,
        };
        if let Ok(mut g) = self.time_period.write() {
            *g = p;
        }
    }

    pub fn order_by_index(&self) -> usize {
        use OrderBy::*;
        self.order_by
            .read()
            .ok()
            .map(|o| match *o {
                PNL => 0,
                VOL => 1,
            })
            .unwrap_or(0)
    }

    pub fn set_order_by_index(&self, i: usize) {
        use OrderBy::*;
        let o = if i.is_multiple_of(2) { PNL } else { VOL };
        if let Ok(mut g) = self.order_by.write() {
            *g = o;
        }
    }

    pub fn get_stats(&self, address: &str) -> Option<TraderStats> {
        self.stats_cache.read().ok()?.get(address).cloned()
    }

    pub fn scan_next(&self) {
        let entries = self.get_entries();
        let len = entries.len();
        if len == 0 {
            return;
        }
        let mut idx = match self.scan_index.write() {
            Ok(g) => g,
            Err(_) => return,
        };
        let addr = entries[*idx].proxy_wallet.clone();
        *idx = (*idx + 1) % len;
        drop(idx);
        if let Some(stats) = self::trader_stats::fetch_stats(&addr) {
            if let Ok(mut cache) = self.stats_cache.write() {
                cache.insert(addr, stats);
            }
        }
    }

    pub fn is_screener_mode(&self) -> bool {
        self.screener_mode.load(Ordering::SeqCst)
    }

    pub fn toggle_screener_mode(&self) {
        let current = self.screener_mode.load(Ordering::SeqCst);
        self.screener_mode.store(!current, Ordering::SeqCst);
    }

    pub fn get_screener_markets(&self) -> Vec<ScreenerMarket> {
        self.screener_markets
            .read()
            .map(|m| m.clone())
            .unwrap_or_default()
    }

    pub fn fetch_screener(&self) {
        if self
            .screener_fetching
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        let markets = screener::fetch_screener_markets();
        if let Ok(mut g) = self.screener_markets.write() {
            *g = markets;
        }
        self.screener_fetching.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_state_initializes_defaults() {
        let state = DiscoverState::new();
        assert!(!state.is_fetching());
        assert!(!state.is_screener_mode());
        assert_eq!(state.get_entries().len(), 0);
        assert_eq!(state.category_label(), "ALL");
        assert_eq!(state.time_period_label(), "WEEK");
        assert_eq!(state.order_by_label(), "P&L");
    }

    #[test]
    fn cycle_category_wraps_around() {
        let state = DiscoverState::new();
        // Cycle through all 9 categories and back to OVERALL
        for _ in 0..9 {
            state.cycle_category();
        }
        assert_eq!(state.category_label(), "ALL");
    }

    #[test]
    fn set_category_by_index_all_values() {
        let state = DiscoverState::new();
        let labels = [
            "ALL",
            "CRYPTO",
            "SPORTS",
            "POLITICS",
            "CULTURE",
            "WEATHER",
            "ECONOMICS",
            "TECH",
            "FINANCE",
        ];
        for (i, expected) in labels.iter().enumerate() {
            state.set_category_by_index(i);
            assert_eq!(state.category_label(), *expected);
        }
    }

    #[test]
    fn toggle_screener_mode_flips() {
        let state = DiscoverState::new();
        assert!(!state.is_screener_mode());
        state.toggle_screener_mode();
        assert!(state.is_screener_mode());
        state.toggle_screener_mode();
        assert!(!state.is_screener_mode());
    }
}
