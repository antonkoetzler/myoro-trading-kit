//! Config commands: read and write application settings.

use crate::app_state::AppState;
use crate::commands::dto::shared::ConfigDto;
use myoro_trading_kit::config;
use tauri::State;

#[tauri::command]
pub fn get_config(state: State<AppState>) -> ConfigDto {
    let cfg = state.config.read().map(|c| c.clone()).unwrap_or_default();
    let mode = match cfg.execution_mode {
        config::ExecutionMode::Paper => "paper",
        config::ExecutionMode::Live => "live",
    };
    ConfigDto {
        execution_mode: mode.to_string(),
        paper_bankroll: cfg.paper_bankroll,
        pnl_currency: cfg.pnl_currency.clone(),
        copy_traders: cfg.copy_traders.clone(),
        copy_poll_ms: cfg.copy_poll_ms,
        copy_bankroll_fraction: cfg.copy_bankroll_fraction,
        copy_max_usd: cfg.copy_max_usd,
        copy_auto_execute: cfg.copy_auto_execute,
        max_daily_loss_usd: cfg.max_daily_loss_usd,
        max_position_usd: cfg.max_position_usd,
        max_open_positions: cfg.max_open_positions,
        mm_enabled: cfg.mm_enabled,
        mm_half_spread: cfg.mm_half_spread,
        mm_max_inventory_usd: cfg.mm_max_inventory_usd,
        mm_max_markets: cfg.mm_max_markets,
        mm_min_volume_usd: cfg.mm_min_volume_usd,
        binance_lag_assets: cfg.binance_lag_assets.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn save_config_settings(
    state: State<AppState>,
    paper_bankroll: Option<f64>,
    pnl_currency: String,
    max_daily_loss_usd: f64,
    max_position_usd: f64,
    max_open_positions: u32,
    mm_enabled: bool,
    mm_half_spread: f64,
    mm_max_inventory_usd: f64,
    mm_max_markets: u32,
    mm_min_volume_usd: f64,
    binance_lag_assets: Vec<String>,
) -> Result<(), String> {
    if let Ok(mut c) = state.config.write() {
        c.paper_bankroll = paper_bankroll;
        c.pnl_currency = pnl_currency;
        c.max_daily_loss_usd = max_daily_loss_usd;
        c.max_position_usd = max_position_usd;
        c.max_open_positions = max_open_positions;
        c.mm_enabled = mm_enabled;
        c.mm_half_spread = mm_half_spread;
        c.mm_max_inventory_usd = mm_max_inventory_usd;
        c.mm_max_markets = mm_max_markets;
        c.mm_min_volume_usd = mm_min_volume_usd;
        c.binance_lag_assets = binance_lag_assets;
        config::save_config(&c).map_err(|e| e.to_string())?;
    }
    Ok(())
}
