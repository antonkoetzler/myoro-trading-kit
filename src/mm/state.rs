//! MmState: active quotes, fills, inventory, P&L per market.

use std::collections::HashMap;

/// A live quote (bid or ask) posted by the market maker.
#[derive(Clone, Debug)]
pub struct ActiveQuote {
    pub market_id: String,
    pub side: QuoteSide,
    pub price: f64,
    pub size: f64,
    /// Simulated order ID (paper) or CLOB order ID (live).
    pub order_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QuoteSide {
    Bid,
    Ask,
}

impl std::fmt::Display for QuoteSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuoteSide::Bid => write!(f, "BID"),
            QuoteSide::Ask => write!(f, "ASK"),
        }
    }
}

/// Per-market inventory and P&L tracking.
#[derive(Clone, Debug, Default)]
pub struct MarketInventory {
    pub market_id: String,
    /// Net YES position (positive = long YES, negative = long NO).
    pub net_yes: f64,
    /// Realized P&L in USD.
    pub realized_pnl: f64,
    /// Total volume traded (gross).
    pub volume: f64,
}

/// Full MM state, updated on each fill and quote cycle.
#[derive(Default)]
pub struct MmState {
    pub active_quotes: Vec<ActiveQuote>,
    pub inventory: HashMap<String, MarketInventory>,
    pub total_realized_pnl: f64,
    pub fill_count: u32,
    pub running: bool,
}

impl MmState {
    pub fn apply_fill(&mut self, market_id: &str, side: &QuoteSide, price: f64, size: f64) {
        let inv = self.inventory.entry(market_id.to_string()).or_default();
        inv.market_id = market_id.to_string();
        inv.volume += size;
        match side {
            QuoteSide::Bid => {
                // We bought YES at price.
                inv.net_yes += size;
                inv.realized_pnl -= price * size;
            }
            QuoteSide::Ask => {
                // We sold YES at price.
                inv.net_yes -= size;
                inv.realized_pnl += price * size;
            }
        }
        self.fill_count += 1;
        self.total_realized_pnl = self.inventory.values().map(|i| i.realized_pnl).sum();
    }

    pub fn remove_quote(&mut self, order_id: &str) {
        self.active_quotes.retain(|q| q.order_id != order_id);
    }

    pub fn inventory_for(&self, market_id: &str) -> f64 {
        self.inventory
            .get(market_id)
            .map(|i| i.net_yes.abs())
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_fill_bid_increases_net_yes() {
        let mut state = MmState::default();
        state.apply_fill("mkt1", &QuoteSide::Bid, 0.45, 10.0);
        assert_eq!(state.inventory["mkt1"].net_yes, 10.0);
        assert_eq!(state.fill_count, 1);
    }

    #[test]
    fn apply_fill_ask_decreases_net_yes() {
        let mut state = MmState::default();
        state.apply_fill("mkt1", &QuoteSide::Bid, 0.45, 10.0);
        state.apply_fill("mkt1", &QuoteSide::Ask, 0.55, 5.0);
        assert_eq!(state.inventory["mkt1"].net_yes, 5.0);
        // PnL: -4.5 + 2.75 = -1.75 — partial fill, still in-flight inventory
        assert!((state.inventory["mkt1"].realized_pnl - (-4.5 + 2.75)).abs() < 0.001);
    }

    #[test]
    fn inventory_for_absent_market_returns_zero() {
        let state = MmState::default();
        assert_eq!(state.inventory_for("nonexistent"), 0.0);
    }
}
