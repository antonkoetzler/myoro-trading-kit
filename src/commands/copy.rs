//! Copy trading commands.

use crate::app_state::AppState;
use crate::commands::dto::copy::*;
use crate::commands::dto::shared::LogEntryDto;
use myoro_trading_kit::live::LogLevel;
use tauri::State;

#[tauri::command]
pub fn get_copy_state(state: State<AppState>) -> CopyStateDto {
    let traders = state.copy_monitor.trader_list().get_addresses();
    let is_running = state.copy_monitor.is_running();
    let recent_trades = state
        .copy_monitor
        .recent_trades(50)
        .into_iter()
        .map(|t| CopyTradeRowDto {
            user: t.user.clone(),
            side: t.side.clone(),
            size: t.size,
            price: t.price,
            title: t.title.clone(),
            outcome: t.outcome.clone(),
            ts: t.ts,
            tx: t.tx.clone(),
        })
        .collect();
    let logs = state
        .live
        .get_copy_logs()
        .into_iter()
        .map(|(l, m)| LogEntryDto {
            level: level_str(l).to_string(),
            message: m,
        })
        .collect();
    CopyStateDto {
        traders,
        is_running,
        recent_trades,
        logs,
    }
}

#[tauri::command]
pub fn add_copy_trader(state: State<AppState>, address: String) -> bool {
    state.copy_monitor.trader_list().add(address)
}

#[tauri::command]
pub fn remove_copy_trader(state: State<AppState>, idx: usize) {
    state.copy_monitor.trader_list().remove_at(idx);
}

#[tauri::command]
pub fn start_copy_trading(state: State<AppState>) {
    state.copy_monitor.set_running(true);
}

#[tauri::command]
pub fn stop_copy_trading(state: State<AppState>) {
    state.copy_monitor.set_running(false);
}

fn level_str(l: LogLevel) -> &'static str {
    match l {
        LogLevel::Info => "info",
        LogLevel::Success => "success",
        LogLevel::Warning => "warning",
        LogLevel::Error => "error",
    }
}
