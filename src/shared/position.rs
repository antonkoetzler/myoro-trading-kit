//! PositionTracker: in-memory store of open/closed positions updated from paper fills and CLOB.

use crate::pm::clob::Side;
use crate::pm::data::Position;
use std::collections::HashMap;

/// Tracks open and closed positions, computes aggregate P&L.
#[derive(Default)]
pub struct PositionTracker {
    open: HashMap<String, Position>,
    closed: Vec<Position>,
}

impl PositionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a fill: open a new position or update an existing one.
    pub fn apply_fill(
        &mut self,
        market_id: &str,
        side: Side,
        size: f64,
        price: f64,
        current_price: f64,
    ) {
        let entry = self
            .open
            .entry(market_id.to_string())
            .or_insert_with(|| Position {
                market_id: market_id.to_string(),
                side,
                size: 0.0,
                avg_price: 0.0,
                current_price,
                unrealized_pnl: 0.0,
                realized_pnl: 0.0,
            });
        // Update average price (VWAP).
        let total_cost = entry.avg_price * entry.size + price * size;
        entry.size += size;
        entry.avg_price = if entry.size > 0.0 {
            total_cost / entry.size
        } else {
            0.0
        };
        entry.current_price = current_price;
        entry.unrealized_pnl = (current_price - entry.avg_price) * entry.size;
    }

    /// Mark a position as closed at the given exit price.
    pub fn close_position(&mut self, market_id: &str, exit_price: f64) {
        if let Some(mut pos) = self.open.remove(market_id) {
            pos.realized_pnl = (exit_price - pos.avg_price) * pos.size;
            pos.unrealized_pnl = 0.0;
            pos.current_price = exit_price;
            self.closed.push(pos);
        }
    }

    /// Update current prices for unrealized P&L recalculation.
    pub fn update_prices(&mut self, prices: &HashMap<String, f64>) {
        for (market_id, pos) in &mut self.open {
            if let Some(&price) = prices.get(market_id) {
                pos.current_price = price;
                pos.unrealized_pnl = (price - pos.avg_price) * pos.size;
            }
        }
    }

    pub fn open_positions(&self) -> Vec<&Position> {
        self.open.values().collect()
    }

    pub fn closed_positions(&self) -> &[Position] {
        &self.closed
    }

    /// Total unrealized P&L across all open positions.
    pub fn total_unrealized_pnl(&self) -> f64 {
        self.open.values().map(|p| p.unrealized_pnl).sum()
    }

    /// Total realized P&L across all closed positions.
    pub fn total_realized_pnl(&self) -> f64 {
        self.closed.iter().map(|p| p.realized_pnl).sum()
    }

    pub fn total_pnl(&self) -> f64 {
        self.total_realized_pnl() + self.total_unrealized_pnl()
    }

    pub fn open_count(&self) -> usize {
        self.open.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_fill_creates_position() {
        let mut tracker = PositionTracker::new();
        tracker.apply_fill("market1", Side::Yes, 10.0, 0.5, 0.55);
        assert_eq!(tracker.open_count(), 1);
        let positions = tracker.open_positions();
        assert!((positions[0].avg_price - 0.5).abs() < 1e-9);
        assert!((positions[0].unrealized_pnl - 0.5).abs() < 1e-9);
    }

    #[test]
    fn close_position_moves_to_closed() {
        let mut tracker = PositionTracker::new();
        tracker.apply_fill("market1", Side::Yes, 10.0, 0.5, 0.5);
        tracker.close_position("market1", 0.8);
        assert_eq!(tracker.open_count(), 0);
        assert_eq!(tracker.closed_positions().len(), 1);
        assert!((tracker.total_realized_pnl() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn vwap_updates_on_second_fill() {
        let mut tracker = PositionTracker::new();
        tracker.apply_fill("m1", Side::Yes, 10.0, 0.4, 0.5);
        tracker.apply_fill("m1", Side::Yes, 10.0, 0.6, 0.5);
        let pos = &tracker.open_positions()[0];
        // VWAP should be 0.5
        assert!((pos.avg_price - 0.5).abs() < 1e-9);
        assert!((pos.size - 20.0).abs() < 1e-9);
    }
}
