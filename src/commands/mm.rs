//! Market making commands.

use crate::app_state::AppState;
use crate::commands::dto::mm::*;
use crate::commands::dto::shared::LogEntryDto;
use myoro_trading_kit::config;
use myoro_trading_kit::live::LogLevel;
use std::sync::atomic::Ordering;
use tauri::State;

#[tauri::command]
pub fn get_mm_state(state: State<AppState>) -> MmStateDto {
    let logs: Vec<LogEntryDto> = state
        .live
        .mm_logs
        .read()
        .map(|l| {
            l.iter()
                .map(|(lv, m)| LogEntryDto {
                    level: level_str(*lv).to_string(),
                    message: m.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    state
        .live
        .mm
        .read()
        .map(|mm| MmStateDto {
            active_quotes: mm
                .active_quotes
                .iter()
                .map(|q| ActiveQuoteDto {
                    market_id: q.market_id.clone(),
                    side: q.side.to_string(),
                    price: q.price,
                    size: q.size,
                    order_id: q.order_id.clone(),
                })
                .collect(),
            inventory: mm
                .inventory
                .values()
                .map(|inv| InventoryEntryDto {
                    market_id: inv.market_id.clone(),
                    net_yes: inv.net_yes,
                    realized_pnl: inv.realized_pnl,
                    volume: inv.volume,
                })
                .collect(),
            total_realized_pnl: mm.total_realized_pnl,
            fill_count: mm.fill_count,
            running: state.mm_running.load(Ordering::SeqCst),
            logs: logs.clone(),
        })
        .unwrap_or_else(|_| MmStateDto {
            active_quotes: Vec::new(),
            inventory: Vec::new(),
            total_realized_pnl: 0.0,
            fill_count: 0,
            running: false,
            logs,
        })
}

#[tauri::command]
pub fn start_mm(state: State<AppState>) {
    state.mm_running.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn stop_mm(state: State<AppState>) {
    state.mm_running.store(false, Ordering::SeqCst);
}

#[tauri::command]
pub fn save_mm_config(
    state: State<AppState>,
    mm_half_spread: f64,
    mm_max_inventory_usd: f64,
    mm_max_markets: u32,
    mm_min_volume_usd: f64,
) -> Result<(), String> {
    if let Ok(mut c) = state.config.write() {
        c.mm_half_spread = mm_half_spread.clamp(0.001, 0.5);
        c.mm_max_inventory_usd = mm_max_inventory_usd.max(0.0);
        c.mm_max_markets = mm_max_markets;
        c.mm_min_volume_usd = mm_min_volume_usd.max(0.0);
        config::save_config(&c).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn level_str(l: LogLevel) -> &'static str {
    match l {
        LogLevel::Info => "info",
        LogLevel::Success => "success",
        LogLevel::Warning => "warning",
        LogLevel::Error => "error",
    }
}
