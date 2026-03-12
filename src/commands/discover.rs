//! Discover tab commands.

use crate::app_state::AppState;
use crate::commands::dto::discover::*;
use myoro_trading_kit::discover::leaderboard::fetch_leaderboard;
use myoro_trading_kit::discover::trader_stats::fetch_stats;
use tauri::State;

#[tauri::command]
pub fn fetch_discover_leaderboard(
    _state: State<AppState>,
    category: String,
    period: String,
) -> Vec<LeaderboardEntryDto> {
    let cat = match category.to_uppercase().as_str() {
        "CRYPTO" => "CRYPTO",
        "SPORTS" => "SPORTS",
        "POLITICS" => "POLITICS",
        "WEATHER" => "WEATHER",
        _ => "OVERALL",
    };
    let per = match period.to_uppercase().as_str() {
        "DAY" => "DAY",
        "MONTH" => "MONTH",
        "ALL" => "ALL",
        _ => "WEEK",
    };
    fetch_leaderboard(cat, per, "PNL")
        .into_iter()
        .map(|e| LeaderboardEntryDto {
            rank: e.rank,
            proxy_wallet: e.proxy_wallet,
            user_name: e.user_name,
            vol: e.vol,
            pnl: e.pnl,
        })
        .collect()
}

#[tauri::command]
pub fn get_trader_profile(_state: State<AppState>, address: String) -> Option<TraderProfileDto> {
    fetch_stats(&address).map(|s| TraderProfileDto {
        address: address.clone(),
        trade_count: s.trade_count,
        top_category: s.top_category,
        win_rate: s.win_rate,
    })
}

#[tauri::command]
pub fn add_trader_to_copy(state: State<AppState>, address: String) -> bool {
    state.copy_monitor.trader_list().add(address)
}
