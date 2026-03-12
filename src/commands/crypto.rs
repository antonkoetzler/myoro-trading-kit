//! Crypto tab commands.

use crate::app_state::AppState;
use crate::commands::dto::crypto::*;
use crate::commands::dto::shared::LogEntryDto;
use myoro_trading_kit::live::LogLevel;
use tauri::State;

#[tauri::command]
pub fn get_crypto_state(state: State<AppState>) -> CryptoStateDto {
    let crypto = state
        .live
        .crypto
        .read()
        .map(|c| c.clone_dto())
        .unwrap_or_default();
    let logs = state
        .live
        .get_crypto_logs()
        .into_iter()
        .map(|(l, m)| LogEntryDto {
            level: log_level_str(l).to_string(),
            message: m,
        })
        .collect();
    CryptoStateDto {
        btc_usdt: crypto.btc_usdt,
        events: crypto.events,
        strategies: crypto.strategies,
        signals: crypto.signals,
        markets: crypto.markets,
        binance_lag_confidence: crypto.binance_lag_confidence,
        logical_arb_confidence: crypto.logical_arb_confidence,
        logs,
    }
}

#[tauri::command]
pub fn toggle_crypto_strategy(state: State<AppState>, idx: usize, enabled: bool) {
    if let Ok(mut c) = state.live.crypto.write() {
        if let Some(s) = c.strategy_configs.get_mut(idx) {
            s.enabled = enabled;
        }
    }
}

#[tauri::command]
pub fn dismiss_crypto_signal(state: State<AppState>, market_id: String) {
    if let Ok(mut c) = state.live.crypto.write() {
        if let Some(sig) = c.signals.iter_mut().find(|s| s.market_id == market_id) {
            sig.status = "dismissed".to_string();
        }
    }
}

fn log_level_str(l: LogLevel) -> &'static str {
    match l {
        LogLevel::Info => "info",
        LogLevel::Success => "success",
        LogLevel::Warning => "warning",
        LogLevel::Error => "error",
    }
}

// Helper trait to convert CryptoState to intermediate DTO-ready struct
#[derive(Default)]
struct CryptoStateIntermediate {
    btc_usdt: String,
    events: Vec<String>,
    strategies: Vec<CryptoStrategyDto>,
    signals: Vec<CryptoSignalDto>,
    markets: Vec<GammaMarketDto>,
    binance_lag_confidence: Vec<f64>,
    logical_arb_confidence: Vec<f64>,
}

trait CryptoStateExt {
    fn clone_dto(&self) -> CryptoStateIntermediate;
}

impl CryptoStateExt for myoro_trading_kit::live::CryptoState {
    fn clone_dto(&self) -> CryptoStateIntermediate {
        CryptoStateIntermediate {
            btc_usdt: self.btc_usdt.clone(),
            events: self.events.clone(),
            strategies: self
                .strategy_configs
                .iter()
                .map(|s| CryptoStrategyDto {
                    id: s.id.to_string(),
                    name: s.name.to_string(),
                    description: s.description.to_string(),
                    enabled: s.enabled,
                    auto_execute: s.auto_execute,
                })
                .collect(),
            signals: self
                .signals
                .iter()
                .map(|s| CryptoSignalDto {
                    market_id: s.market_id.clone(),
                    label: s.label.clone(),
                    side: s.side.clone(),
                    edge_pct: s.edge_pct,
                    kelly_size: s.kelly_size,
                    strategy_id: s.strategy_id.clone(),
                    status: s.status.clone(),
                    created_at: s.created_at.to_rfc3339(),
                })
                .collect(),
            markets: self
                .markets
                .iter()
                .map(|m| GammaMarketDto {
                    id: m.id.clone(),
                    title: m.title.clone(),
                    best_bid: m.best_bid,
                    best_ask: m.best_ask,
                    volume: m.volume,
                })
                .collect(),
            binance_lag_confidence: self.binance_lag_confidence.clone(),
            logical_arb_confidence: self.logical_arb_confidence.clone(),
        }
    }
}
