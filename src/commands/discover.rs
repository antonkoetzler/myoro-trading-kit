//! Discover tab commands.

use crate::app_state::AppState;
use crate::commands::dto::discover::*;
use myoro_trading_kit::discover::leaderboard::fetch_leaderboard;
use myoro_trading_kit::discover::trader_stats::fetch_stats;
use tauri::State;

#[tauri::command]
pub async fn fetch_discover_leaderboard(
    category: String,
    period: String,
) -> Vec<LeaderboardEntryDto> {
    let cat = match category.to_uppercase().as_str() {
        "CRYPTO" => "CRYPTO",
        "SPORTS" => "SPORTS",
        "POLITICS" => "POLITICS",
        "WEATHER" => "WEATHER",
        _ => "OVERALL",
    }
    .to_string();
    let per = match period.to_uppercase().as_str() {
        "DAY" => "DAY",
        "MONTH" => "MONTH",
        "ALL" => "ALL",
        _ => "WEEK",
    }
    .to_string();
    tokio::task::spawn_blocking(move || {
        fetch_leaderboard(&cat, &per, "PNL")
            .into_iter()
            .map(|e| LeaderboardEntryDto {
                rank: e.rank,
                proxy_wallet: e.proxy_wallet,
                user_name: e.user_name,
                vol: e.vol,
                pnl: e.pnl,
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn get_trader_profile(address: String) -> Option<TraderProfileDto> {
    tokio::task::spawn_blocking(move || {
        fetch_stats(&address).map(|s| TraderProfileDto {
            address: address.clone(),
            trade_count: s.trade_count,
            top_category: s.top_category,
            win_rate: s.win_rate,
        })
    })
    .await
    .ok()?
}

#[tauri::command]
pub fn add_trader_to_copy(state: State<AppState>, address: String) -> bool {
    state.copy_monitor.trader_list().add(address)
}
