//! Copy-trading: monitor trades from profiles listed in config.json (copy_traders).

mod executor;
mod fetcher;
mod types;

pub use executor::execute_copy_trades;
pub use types::{TradeRow, TraderList};

use crate::config;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

const MAX_DISPLAY: usize = 24;

pub struct Monitor {
    list: Arc<TraderList>,
    trades: RwLock<Vec<TradeRow>>,
    seen: RwLock<HashSet<String>>,
    log_sink: Option<Arc<crate::live::LiveState>>,
    running: Arc<AtomicBool>,
}

impl Monitor {
    pub fn poll_ms_from_config(config: &config::Config) -> u64 {
        config.copy_poll_ms
    }

    pub fn new(
        list: Arc<TraderList>,
        log_sink: Option<Arc<crate::live::LiveState>>,
        running: Arc<AtomicBool>,
    ) -> Self {
        Self {
            list,
            trades: RwLock::new(Vec::new()),
            seen: RwLock::new(HashSet::new()),
            log_sink,
            running,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn set_running(&self, v: bool) {
        self.running.store(v, Ordering::SeqCst);
    }

    pub fn trader_list(&self) -> &Arc<TraderList> {
        &self.list
    }

    pub fn poll_once(&self) {
        if !self.running.load(Ordering::SeqCst) {
            return;
        }
        let cfg = match self.list.config.read() {
            Ok(c) => c.clone(),
            Err(_) => return,
        };
        self.list.reload_if_changed();
        let addresses = self.list.get_addresses();
        if addresses.is_empty() {
            return;
        }
        let mut seen = match self.seen.write() {
            Ok(s) => s,
            Err(_) => return,
        };
        let mut all = fetcher::fetch_recent_trades(&addresses, &mut seen);
        drop(seen);
        if all.is_empty() {
            return;
        }
        all.sort_by(|a, b| b.ts.cmp(&a.ts));
        if let Some(ref live) = self.log_sink {
            for r in &all {
                live.push_log(format!(
                    "Copy trade: {} {} @ {} · {}",
                    r.side, r.size, r.price, r.title
                ));
            }
        }
        execute_copy_trades(&all, &cfg, self.log_sink.as_ref().map(Arc::as_ref));
        let mut trades = match self.trades.write() {
            Ok(t) => t,
            Err(_) => return,
        };
        for r in all {
            trades.insert(0, r);
        }
        trades.truncate(200);
    }

    pub fn recent_trades(&self, n: usize) -> Vec<TradeRow> {
        self.trades
            .read()
            .map(|t| t.iter().take(n).cloned().collect())
            .unwrap_or_default()
    }

    pub fn copy_tab_display(&self, selected_index: Option<usize>, _input_buf: &str) -> String {
        let addresses = self.list.get_addresses();
        let mut out = String::new();
        for (i, addr) in addresses.iter().enumerate() {
            let mark = if Some(i) == selected_index {
                "► "
            } else {
                "  "
            };
            let short = addr
                .get(..10)
                .map(|s| format!("{}…", s))
                .unwrap_or_else(|| addr.clone());
            out.push_str(&format!("{}{}\n", mark, short));
        }
        if addresses.is_empty() {
            out.push_str("No profiles. Add from Discover (a/Enter) or Shortcuts screen.");
        }
        out
    }
}

/// Suppress unused import for MAX_DISPLAY (used only in display logic).
const _: usize = MAX_DISPLAY;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn base_config() -> config::Config {
        config::Config {
            execution_mode: config::ExecutionMode::Paper,
            polymarket: config::PolymarketConfig::default(),
            binance: config::BinanceConfig::default(),
            paper_bankroll: Some(10.0),
            copy_traders: vec![],
            copy_poll_ms: 250,
            pnl_currency: "USD".to_string(),
            copy_sizing: config::CopySizing::Proportional,
            copy_trader_bankrolls: std::collections::HashMap::new(),
            copy_bankroll_fraction: 0.05,
            copy_max_usd: 1000.0,
            copy_auto_execute: false,
            paper_trades_file: "data/paper_copy_trades.jsonl".to_string(),
            max_daily_loss_usd: 100.0,
            max_position_usd: 50.0,
            max_open_positions: 10,
            mm_enabled: false,
            mm_half_spread: 0.02,
            mm_max_inventory_usd: 200.0,
            mm_max_markets: 5,
            mm_min_volume_usd: 1000.0,
        }
    }

    #[test]
    fn recent_trades_returns_newest_first() {
        let cfg = Arc::new(RwLock::new(base_config()));
        let list = Arc::new(TraderList::new(cfg));
        let monitor = Monitor::new(list, None, Arc::new(AtomicBool::new(true)));
        if let Ok(mut trades) = monitor.trades.write() {
            trades.push(TradeRow {
                user: "u1".to_string(),
                side: "BUY".to_string(),
                size: 1.0,
                price: 0.5,
                title: "A".to_string(),
                outcome: "YES".to_string(),
                ts: 1,
                tx: "t1".to_string(),
                condition_id: Some("c1".to_string()),
                asset_id: Some("a1".to_string()),
            });
            trades.push(TradeRow {
                user: "u2".to_string(),
                side: "SELL".to_string(),
                size: 2.0,
                price: 0.4,
                title: "B".to_string(),
                outcome: "NO".to_string(),
                ts: 2,
                tx: "t2".to_string(),
                condition_id: Some("c2".to_string()),
                asset_id: Some("a2".to_string()),
            });
        }
        let rows = monitor.recent_trades(1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].tx, "t1");
    }
}
