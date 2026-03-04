//! Live data orchestrator. Background thread calls fetch_all() every 8s; TUI reads via Arc<LiveState>.

pub mod crypto;
pub mod global;
pub mod portfolio;
pub mod sports;
pub mod weather;

// Re-export frequently used types at the live:: level for backwards compatibility.
pub use crypto::CryptoState;
pub use global::{push_log_to, read_logs, GlobalStats, LogLevel};
pub use portfolio::PortfolioState;
#[allow(unused_imports)]
pub use sports::{League, LiveMatchSnapshot, SportsState, StoredSignal, StrategyConfig};
pub use weather::WeatherState;

use crate::config::Config;
use crate::mm::MmState;
use std::sync::RwLock;

pub struct LiveState {
    pub crypto: RwLock<CryptoState>,
    pub sports: RwLock<SportsState>,
    pub weather: RwLock<WeatherState>,
    pub portfolio: RwLock<PortfolioState>,
    pub mm: RwLock<MmState>,
    pub crypto_logs: RwLock<Vec<(LogLevel, String)>>,
    pub sports_logs: RwLock<Vec<(LogLevel, String)>>,
    pub weather_logs: RwLock<Vec<(LogLevel, String)>>,
    pub copy_logs: RwLock<Vec<(LogLevel, String)>>,
    pub discover_logs: RwLock<Vec<(LogLevel, String)>>,
    pub mm_logs: RwLock<Vec<(LogLevel, String)>>,
    pub global_stats: RwLock<GlobalStats>,
}

impl Default for LiveState {
    fn default() -> Self {
        Self {
            crypto: RwLock::new(CryptoState::default()),
            sports: RwLock::new(SportsState::default()),
            weather: RwLock::new(WeatherState::default()),
            portfolio: RwLock::new(PortfolioState::default()),
            mm: RwLock::new(MmState::default()),
            crypto_logs: RwLock::new(Vec::new()),
            sports_logs: RwLock::new(Vec::new()),
            weather_logs: RwLock::new(Vec::new()),
            copy_logs: RwLock::new(Vec::new()),
            discover_logs: RwLock::new(Vec::new()),
            mm_logs: RwLock::new(Vec::new()),
            global_stats: RwLock::new(GlobalStats::default()),
        }
    }
}

impl LiveState {
    pub fn fetch_all(&self) {
        crypto::fetch_crypto(&self.crypto, &self.crypto_logs);
        sports::fetch_sports(&self.sports, &self.sports_logs);
        weather::fetch_weather(&self.weather, &self.weather_logs);
        if let Ok(mut p) = self.portfolio.write() {
            p.refresh();
        }
    }

    /// Run one MM cycle. Called separately with config (config not stored in LiveState).
    pub fn run_mm(&self, config: &Config) {
        crate::mm::run_mm_cycle(config, &self.mm, &self.mm_logs);
    }

    pub fn push_log(&self, s: String) {
        self.push_copy_log(LogLevel::Info, s);
    }

    pub fn push_crypto_log(&self, level: LogLevel, s: String) {
        push_log_to(&self.crypto_logs, level, s);
    }

    pub fn push_sports_log(&self, level: LogLevel, s: String) {
        push_log_to(&self.sports_logs, level, s);
    }

    pub fn push_weather_log(&self, level: LogLevel, s: String) {
        push_log_to(&self.weather_logs, level, s);
    }

    pub fn push_copy_log(&self, level: LogLevel, s: String) {
        push_log_to(&self.copy_logs, level, s);
    }

    pub fn get_crypto_logs(&self) -> Vec<(LogLevel, String)> {
        read_logs(&self.crypto_logs)
    }

    pub fn get_sports_logs(&self) -> Vec<(LogLevel, String)> {
        read_logs(&self.sports_logs)
    }

    pub fn get_weather_logs(&self) -> Vec<(LogLevel, String)> {
        read_logs(&self.weather_logs)
    }

    pub fn get_copy_logs(&self) -> Vec<(LogLevel, String)> {
        read_logs(&self.copy_logs)
    }

    pub fn get_discover_logs(&self) -> Vec<(LogLevel, String)> {
        read_logs(&self.discover_logs)
    }

    pub fn last_log_is_error(&self, tab: u8) -> bool {
        let logs = match tab {
            0 => self.get_crypto_logs(),
            1 => self.get_sports_logs(),
            2 => self.get_weather_logs(),
            _ => return false,
        };
        logs.last()
            .map(|(l, _)| *l == LogLevel::Error)
            .unwrap_or(false)
    }

    pub fn set_bankroll(&self, v: Option<f64>) {
        if let Ok(mut s) = self.global_stats.write() {
            s.bankroll = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_state_initializes_all_fields() {
        let state = LiveState::default();
        assert!(state.crypto.read().is_ok());
        assert!(state.sports.read().is_ok());
        assert!(state.weather.read().is_ok());
        assert!(state.portfolio.read().is_ok());
        assert!(state.global_stats.read().is_ok());
    }

    #[test]
    fn set_bankroll_stores_value() {
        let state = LiveState::default();
        state.set_bankroll(Some(5000.0));
        let bankroll = state
            .global_stats
            .read()
            .map(|s| s.bankroll)
            .unwrap_or(None);
        assert_eq!(bankroll, Some(5000.0));
    }

    #[test]
    fn set_bankroll_clears_value() {
        let state = LiveState::default();
        state.set_bankroll(Some(1000.0));
        state.set_bankroll(None);
        let bankroll = state
            .global_stats
            .read()
            .map(|s| s.bankroll)
            .unwrap_or(Some(99.0));
        assert!(bankroll.is_none());
    }

    #[test]
    fn push_copy_log_readable_via_get_copy_logs() {
        let state = LiveState::default();
        state.push_copy_log(LogLevel::Info, "test message".into());
        let logs = state.get_copy_logs();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].1.contains("test message"));
    }

    #[test]
    fn last_log_is_error_returns_false_for_empty() {
        let state = LiveState::default();
        assert!(!state.last_log_is_error(0));
        assert!(!state.last_log_is_error(1));
    }

    #[test]
    fn last_log_is_error_detects_error_level() {
        let state = LiveState::default();
        state.push_crypto_log(LogLevel::Error, "crash".into());
        assert!(state.last_log_is_error(0));
    }

    #[test]
    fn live_state_concurrent_reads_no_deadlock() {
        use std::sync::Arc;
        let state = Arc::new(LiveState::default());
        let s1 = Arc::clone(&state);
        let s2 = Arc::clone(&state);
        let t1 = std::thread::spawn(move || {
            for _ in 0..50 {
                let _ = s1.get_crypto_logs();
            }
        });
        let t2 = std::thread::spawn(move || {
            for _ in 0..50 {
                s2.push_copy_log(LogLevel::Info, "concurrent".into());
            }
        });
        t1.join().unwrap();
        t2.join().unwrap();
    }
}
