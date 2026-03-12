//! CryptoState and fetch_crypto() — Binance + Gamma price/event data + strategy runner.

use crate::live::global::{push_log_to, LogLevel};
use crate::shared::strategy::StrategyHealthTracker;
use crate::strategies::crypto::{
    BinanceLagStrategy, CryptoStrategyConfig, GammaMarket, LogicalArbStrategy, StoredCryptoSignal,
};
use std::collections::HashMap;
use std::sync::RwLock;

const GAMMA_EVENTS: &str = "https://gamma-api.polymarket.com/events?closed=false&limit=15";
const BINANCE_TICKER: &str = "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT";

pub struct CryptoState {
    pub btc_usdt: String,
    pub events: Vec<String>,
    pub strategy_configs: Vec<CryptoStrategyConfig>,
    pub signals: Vec<StoredCryptoSignal>,
    pub markets: Vec<GammaMarket>,
    /// Last known BTC price for momentum detection (legacy single-asset).
    pub last_btc_price: f64,
    /// Last known price per Binance symbol (multi-asset support).
    pub last_prices: HashMap<String, f64>,
    /// Health tracker for BinanceLag strategy.
    pub binance_lag_health: StrategyHealthTracker,
    /// Health tracker for LogicalArb strategy.
    pub logical_arb_health: StrategyHealthTracker,
    /// Confidence history ring buffer (last 10) for BinanceLag.
    pub binance_lag_confidence: Vec<f64>,
    /// Confidence history ring buffer (last 10) for LogicalArb.
    pub logical_arb_confidence: Vec<f64>,
}

impl Default for CryptoState {
    fn default() -> Self {
        Self {
            btc_usdt: String::new(),
            events: Vec::new(),
            strategy_configs: CryptoStrategyConfig::builtins(),
            signals: Vec::new(),
            markets: Vec::new(),
            last_btc_price: 0.0,
            last_prices: HashMap::new(),
            binance_lag_health: StrategyHealthTracker::default(),
            logical_arb_health: StrategyHealthTracker::default(),
            binance_lag_confidence: Vec::new(),
            logical_arb_confidence: Vec::new(),
        }
    }
}

pub fn fetch_crypto(
    crypto: &RwLock<CryptoState>,
    crypto_logs: &RwLock<Vec<(LogLevel, String)>>,
    binance_lag_assets: &[String],
) {
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            push_log_to(
                crypto_logs,
                LogLevel::Error,
                "HTTP client init failed".into(),
            );
            return;
        }
    };

    // ── BTC price ────────────────────────────────────────────────────────────
    push_log_to(
        crypto_logs,
        LogLevel::Info,
        "Fetching BTC/USDT (Binance)…".into(),
    );
    let prev_price = crypto.read().map(|c| c.last_btc_price).unwrap_or(0.0);
    match client.get(BINANCE_TICKER).send() {
        Ok(resp) => match resp.json::<serde_json::Value>() {
            Ok(json) => {
                let price_str = json.get("price").and_then(|p| p.as_str()).unwrap_or("—");
                let price: f64 = price_str.parse().unwrap_or(0.0);
                if let Ok(mut c) = crypto.write() {
                    c.btc_usdt = format!("BTC/USDT {}", price_str);
                    c.last_btc_price = price;
                }
                push_log_to(
                    crypto_logs,
                    LogLevel::Success,
                    format!("BTC/USDT {}", price_str),
                );
            }
            Err(_) => push_log_to(
                crypto_logs,
                LogLevel::Warning,
                "Binance ticker parse failed".into(),
            ),
        },
        Err(_) => push_log_to(
            crypto_logs,
            LogLevel::Error,
            "Binance request failed".into(),
        ),
    }

    // ── Gamma events ─────────────────────────────────────────────────────────
    push_log_to(crypto_logs, LogLevel::Info, "Fetching Gamma events…".into());
    match client.get(GAMMA_EVENTS).send() {
        Ok(resp) => match resp.json::<Vec<serde_json::Value>>() {
            Ok(arr) => {
                let lines: Vec<String> = arr
                    .iter()
                    .take(10)
                    .filter_map(|e| {
                        let title = e.get("title").and_then(|t| t.as_str())?;
                        let slug = e.get("slug").and_then(|s| s.as_str()).unwrap_or("");
                        Some(format!("{} | {}", title, slug))
                    })
                    .collect();
                if let Ok(mut c) = crypto.write() {
                    c.events = lines.clone();
                }
                push_log_to(
                    crypto_logs,
                    LogLevel::Success,
                    format!("Loaded {} Gamma events", lines.len()),
                );
            }
            Err(_) => push_log_to(
                crypto_logs,
                LogLevel::Warning,
                "Gamma events parse failed".into(),
            ),
        },
        Err(_) => push_log_to(crypto_logs, LogLevel::Error, "Gamma request failed".into()),
    }

    // ── Run enabled strategies ────────────────────────────────────────────────
    let enabled: Vec<String> = crypto
        .read()
        .map(|c| {
            c.strategy_configs
                .iter()
                .filter(|s| s.enabled)
                .map(|s| s.id.to_string())
                .collect()
        })
        .unwrap_or_default();

    if enabled.contains(&"binance_lag".to_string()) {
        push_log_to(crypto_logs, LogLevel::Info, "Running BinanceLag…".into());
        let assets = if binance_lag_assets.is_empty() {
            vec!["BTCUSDT".to_string()]
        } else {
            binance_lag_assets.to_vec()
        };
        let strategy = BinanceLagStrategy::with_symbols(assets.clone());
        let prev_prices = crypto
            .read()
            .map(|c| c.last_prices.clone())
            .unwrap_or_default();
        let signals = strategy.run_all_symbols(&prev_prices);
        let found = !signals.is_empty();
        if let Ok(mut c) = crypto.write() {
            // Update BTC legacy price from BTCUSDT if present
            if let Ok((new_price, _)) = strategy.run_symbol("BTCUSDT", prev_price) {
                c.last_btc_price = new_price;
                c.last_prices.insert("BTCUSDT".to_string(), new_price);
            }
            // Update per-symbol prices and store signals
            for sym in &assets {
                let pp = prev_prices.get(sym.as_str()).copied().unwrap_or(0.0);
                if let Ok((price, _)) = strategy.run_symbol(sym, pp) {
                    c.last_prices.insert(sym.clone(), price);
                }
            }
            if found {
                let avg_edge =
                    signals.iter().map(|s| s.edge_pct).sum::<f64>() / signals.len() as f64;
                push_confidence(&mut c.binance_lag_confidence, avg_edge);
                c.binance_lag_health.record_signal();
                c.signals.extend(signals);
            } else {
                c.binance_lag_health.record_no_signal();
            }
            let drain = c.signals.len().saturating_sub(100);
            if drain > 0 {
                c.signals.drain(0..drain);
            }
        }
        if found {
            push_log_to(
                crypto_logs,
                LogLevel::Success,
                format!("BinanceLag: signals found ({} assets)", assets.len()),
            );
        }
    }

    if enabled.contains(&"logical_arb".to_string()) {
        push_log_to(crypto_logs, LogLevel::Info, "Running LogicalArb…".into());
        let strategy = LogicalArbStrategy::new();
        match strategy.run() {
            Ok(sigs) => {
                let n = sigs.len();
                if let Ok(mut c) = crypto.write() {
                    if n > 0 {
                        let avg_edge = sigs.iter().map(|s| s.edge_pct).sum::<f64>() / n as f64;
                        push_confidence(&mut c.logical_arb_confidence, avg_edge);
                        c.logical_arb_health.record_signal();
                    } else {
                        c.logical_arb_health.record_no_signal();
                    }
                    c.signals.extend(sigs);
                    let drain = c.signals.len().saturating_sub(100);
                    if drain > 0 {
                        c.signals.drain(0..drain);
                    }
                }
                if n > 0 {
                    push_log_to(
                        crypto_logs,
                        LogLevel::Success,
                        format!("LogicalArb: {} signals", n),
                    );
                }
            }
            Err(e) => {
                if let Ok(mut c) = crypto.write() {
                    c.logical_arb_health.record_error();
                }
                push_log_to(
                    crypto_logs,
                    LogLevel::Warning,
                    format!("LogicalArb error: {}", e),
                );
            }
        }
    }
}

/// Push a confidence value into a 10-element ring buffer.
fn push_confidence(buf: &mut Vec<f64>, v: f64) {
    buf.push(v);
    if buf.len() > 10 {
        buf.drain(0..buf.len() - 10);
    }
}
