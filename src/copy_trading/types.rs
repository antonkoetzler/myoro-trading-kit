//! Copy-trading types: TraderList, TradeRow, and address helpers.

use serde::{Deserialize, Serialize};

use crate::config;
use std::sync::{Arc, RwLock};

pub(super) fn is_valid_address(s: &str) -> bool {
    let s = s.trim();
    s.starts_with("0x") && s.len() == 42 && s[2..].chars().all(|c| c.is_ascii_hexdigit())
}

pub(super) fn normalize_address(s: &str) -> String {
    let s = s.trim();
    if s.starts_with("0x") {
        s.to_string()
    } else {
        format!("0x{}", s)
    }
}

/// Copy trader list backed by config.json.
pub struct TraderList {
    pub(super) config: Arc<RwLock<config::Config>>,
}

impl TraderList {
    pub fn new(config: Arc<RwLock<config::Config>>) -> Self {
        Self { config }
    }

    pub fn reload_if_changed(&self) {
        // Config is in memory; reload from file if we want external edits. For now no-op.
    }

    pub fn get_addresses(&self) -> Vec<String> {
        self.config
            .read()
            .map(|c| c.copy_traders.clone())
            .unwrap_or_default()
    }

    pub fn add(&self, addr: String) -> bool {
        let n = normalize_address(&addr);
        if !is_valid_address(&n) {
            return false;
        }
        if let Ok(mut c) = self.config.write() {
            if c.copy_traders.contains(&n) {
                return true;
            }
            c.copy_traders.push(n);
            let _ = config::save_config(&c);
            true
        } else {
            false
        }
    }

    pub fn remove_at(&self, index: usize) {
        if let Ok(mut c) = self.config.write() {
            if index < c.copy_traders.len() {
                c.copy_traders.remove(index);
                let _ = config::save_config(&c);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.config
            .read()
            .map(|c| c.copy_traders.len())
            .unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Debug)]
pub struct TradeRow {
    pub user: String,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub title: String,
    pub outcome: String,
    pub ts: i64,
    pub tx: String,
    pub condition_id: Option<String>,
    pub asset_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiTrade {
    #[serde(rename = "proxyWallet")]
    pub proxy_wallet: Option<String>,
    pub side: Option<String>,
    pub size: Option<f64>,
    pub price: Option<f64>,
    pub title: Option<String>,
    pub outcome: Option<String>,
    #[serde(rename = "conditionId")]
    pub condition_id: Option<String>,
    pub asset: Option<String>,
    pub timestamp: Option<i64>,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct PaperTradeRecord<'a> {
    pub timestamp: String,
    pub source_timestamp: i64,
    pub condition_id: &'a str,
    pub asset_id: Option<&'a str>,
    pub side: &'a str,
    pub size: f64,
    pub price: f64,
    pub title: &'a str,
    pub outcome: &'a str,
    pub source_trader_address: &'a str,
    pub source_transaction_hash: &'a str,
}

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
    fn trader_list_validates_and_removes() {
        let cfg = Arc::new(RwLock::new(base_config()));
        let list = TraderList::new(Arc::clone(&cfg));
        assert!(!list.add("bad".to_string()));
        assert!(list.add("0x1234567890123456789012345678901234567890".to_string()));
        assert_eq!(list.len(), 1);
        list.remove_at(0);
        assert_eq!(list.len(), 0);
    }
}
