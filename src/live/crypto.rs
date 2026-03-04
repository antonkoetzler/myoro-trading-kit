//! CryptoState and fetch_crypto() — Binance + Gamma price/event data + strategy runner.

use crate::live::global::{push_log_to, LogLevel};
use crate::strategies::crypto::{
    BinanceLagStrategy, CryptoStrategyConfig, GammaMarket, LogicalArbStrategy, StoredCryptoSignal,
};
use std::sync::RwLock;

const GAMMA_EVENTS: &str = "https://gamma-api.polymarket.com/events?closed=false&limit=15";
const BINANCE_TICKER: &str = "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT";

pub struct CryptoState {
    pub btc_usdt: String,
    pub events: Vec<String>,
    pub strategy_configs: Vec<CryptoStrategyConfig>,
    pub signals: Vec<StoredCryptoSignal>,
    pub markets: Vec<GammaMarket>,
    /// Last known BTC price for momentum detection.
    pub last_btc_price: f64,
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
        }
    }
}

pub fn fetch_crypto(crypto: &RwLock<CryptoState>, crypto_logs: &RwLock<Vec<(LogLevel, String)>>) {
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
        let strategy = BinanceLagStrategy::new();
        match strategy.run(prev_price) {
            Ok((new_price, Some(signal))) => {
                if let Ok(mut c) = crypto.write() {
                    c.last_btc_price = new_price;
                    c.signals.push(signal);
                    let drain = c.signals.len().saturating_sub(100);
                    if drain > 0 {
                        c.signals.drain(0..drain);
                    }
                }
                push_log_to(
                    crypto_logs,
                    LogLevel::Success,
                    "BinanceLag: signal found".into(),
                );
            }
            Ok((new_price, None)) => {
                if let Ok(mut c) = crypto.write() {
                    c.last_btc_price = new_price;
                }
            }
            Err(e) => push_log_to(
                crypto_logs,
                LogLevel::Warning,
                format!("BinanceLag error: {}", e),
            ),
        }
    }

    if enabled.contains(&"logical_arb".to_string()) {
        push_log_to(crypto_logs, LogLevel::Info, "Running LogicalArb…".into());
        let strategy = LogicalArbStrategy::new();
        match strategy.run() {
            Ok(sigs) => {
                let n = sigs.len();
                if let Ok(mut c) = crypto.write() {
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
            Err(e) => push_log_to(
                crypto_logs,
                LogLevel::Warning,
                format!("LogicalArb error: {}", e),
            ),
        }
    }
}
