//! Global stats, log level, and per-domain log helpers shared by all LiveState domains.

use std::sync::RwLock;

pub const MAX_LOGS: usize = 80;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Global stats shown on every tab (bankroll, P&L, open/closed trades).
pub struct GlobalStats {
    pub bankroll: Option<f64>,
    pub pnl: f64,
    pub open_trades: u32,
    pub closed_trades: u32,
    /// Cumulative realized loss for the current calendar day (USD).
    pub daily_loss_usd: f64,
    /// When true, all strategy signal generation is suppressed until reset.
    pub circuit_breaker_active: bool,
}

impl Default for GlobalStats {
    fn default() -> Self {
        Self {
            bankroll: None,
            pnl: 0.0,
            open_trades: 0,
            closed_trades: 0,
            daily_loss_usd: 0.0,
            circuit_breaker_active: false,
        }
    }
}

pub fn short_ts() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

pub fn truncate_log(g: &mut Vec<(LogLevel, String)>) {
    let drop = g.len().saturating_sub(MAX_LOGS);
    if drop > 0 {
        g.drain(0..drop);
    }
}

pub fn push_log_to(lock: &RwLock<Vec<(LogLevel, String)>>, level: LogLevel, msg: String) {
    if let Ok(mut g) = lock.write() {
        g.push((level, format!("{} {}", short_ts(), msg)));
        truncate_log(&mut g);
    }
}

pub fn read_logs(lock: &RwLock<Vec<(LogLevel, String)>>) -> Vec<(LogLevel, String)> {
    lock.read().map(|g| g.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_log_keeps_at_most_max_logs() {
        let mut v: Vec<(LogLevel, String)> = (0..MAX_LOGS + 10)
            .map(|i| (LogLevel::Info, i.to_string()))
            .collect();
        truncate_log(&mut v);
        assert_eq!(v.len(), MAX_LOGS);
        // oldest entries dropped: first entry should now be "10"
        assert_eq!(v[0].1, "10");
    }

    #[test]
    fn truncate_log_noop_when_under_limit() {
        let mut v: Vec<(LogLevel, String)> = vec![(LogLevel::Info, "a".into())];
        truncate_log(&mut v);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn push_log_to_appends_and_caps() {
        let lock: RwLock<Vec<(LogLevel, String)>> = RwLock::new(Vec::new());
        for i in 0..MAX_LOGS + 5 {
            push_log_to(&lock, LogLevel::Info, i.to_string());
        }
        let logs = read_logs(&lock);
        assert_eq!(logs.len(), MAX_LOGS);
    }

    #[test]
    fn read_logs_returns_all_entries() {
        let lock: RwLock<Vec<(LogLevel, String)>> = RwLock::new(vec![
            (LogLevel::Success, "ok".into()),
            (LogLevel::Error, "fail".into()),
        ]);
        let logs = read_logs(&lock);
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[1].0, LogLevel::Error);
    }

    #[test]
    fn global_stats_defaults() {
        let stats = GlobalStats::default();
        assert!(stats.bankroll.is_none());
        assert!((stats.pnl - 0.0).abs() < 1e-9);
        assert_eq!(stats.open_trades, 0);
        assert_eq!(stats.closed_trades, 0);
    }
}
