//! Portfolio tab commands.

use crate::app_state::AppState;
use crate::commands::dto::portfolio::*;
use tauri::State;

#[tauri::command]
pub fn get_portfolio_state(state: State<AppState>) -> PortfolioStateDto {
    state
        .live
        .portfolio
        .read()
        .map(|p| PortfolioStateDto {
            open_positions: p
                .open_positions
                .iter()
                .map(|pos| PositionDto {
                    market_id: pos.market_id.clone(),
                    outcome: format!("{:?}", pos.side),
                    size: pos.size,
                    avg_price: pos.avg_price,
                    current_value: pos.current_price * pos.size,
                })
                .collect(),
            trade_history: p
                .trade_history
                .iter()
                .map(|t| TradeRowDto {
                    timestamp: t.timestamp.clone(),
                    domain: t.domain.clone(),
                    market_id: t.market_id.clone(),
                    side: t.side.clone(),
                    size: t.size,
                    price: t.price,
                    status: t.status.clone(),
                })
                .collect(),
            domain_pnl: p
                .domain_pnl
                .iter()
                .map(|(d, today, all)| DomainPnlDto {
                    domain: d.clone(),
                    today_pnl: *today,
                    alltime_pnl: *all,
                })
                .collect(),
            total_pnl: p.total_pnl(),
        })
        .unwrap_or_else(|_| PortfolioStateDto {
            open_positions: Vec::new(),
            trade_history: Vec::new(),
            domain_pnl: Vec::new(),
            total_pnl: 0.0,
        })
}
