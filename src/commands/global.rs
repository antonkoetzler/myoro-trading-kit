//! Global stats commands.

use crate::app_state::AppState;
use crate::commands::dto::shared::{GlobalStatsDto, LogEntryDto};
use tauri::State;

#[tauri::command]
pub fn get_global_stats(state: State<AppState>) -> GlobalStatsDto {
    let gs = state
        .live
        .global_stats
        .read()
        .map(|s| GlobalStatsDto {
            bankroll: s.bankroll,
            pnl: s.pnl,
            open_trades: s.open_trades,
            closed_trades: s.closed_trades,
            daily_loss_usd: s.daily_loss_usd,
            circuit_breaker_active: s.circuit_breaker_active,
        })
        .unwrap_or(GlobalStatsDto {
            bankroll: None,
            pnl: 0.0,
            open_trades: 0,
            closed_trades: 0,
            daily_loss_usd: 0.0,
            circuit_breaker_active: false,
        });
    gs
}

#[tauri::command]
pub fn get_logs(state: State<AppState>, domain: String) -> Vec<LogEntryDto> {
    use myoro_trading_kit::live::LogLevel;
    let raw = match domain.as_str() {
        "crypto" => state.live.get_crypto_logs(),
        "sports" => state.live.get_sports_logs(),
        "weather" => state.live.get_weather_logs(),
        "copy" => state.live.get_copy_logs(),
        "discover" => state.live.get_discover_logs(),
        _ => Vec::new(),
    };
    raw.into_iter()
        .map(|(level, msg)| LogEntryDto {
            level: match level {
                LogLevel::Info => "info",
                LogLevel::Success => "success",
                LogLevel::Warning => "warning",
                LogLevel::Error => "error",
            }
            .to_string(),
            message: msg,
        })
        .collect()
}

#[tauri::command]
pub fn reset_circuit_breaker(state: State<AppState>) {
    state.live.reset_circuit_breaker();
}
