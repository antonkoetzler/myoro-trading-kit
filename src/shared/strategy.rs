//! Strategy trait and registry; domains implement this.

use anyhow::Result;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Signal {
    pub market_id: String,
    pub side: Side,
    pub confidence: f64,
    /// Edge percentage e.g. 0.12 = 12% edge over market price.
    pub edge_pct: f64,
    /// Fractional Kelly stake (0.0–1.0); multiply by bankroll for dollar amount.
    pub kelly_size: f64,
    /// Auto-execute immediately if true; otherwise queue for manual confirmation.
    pub auto_execute: bool,
    /// Strategy that generated this signal.
    pub strategy_id: String,
    pub metadata: Option<serde_json::Value>,
    /// Auto-close position when unrealized loss exceeds this fraction (e.g. 0.10 = 10%).
    pub stop_loss_pct: Option<f64>,
    /// Auto-close position when unrealized gain exceeds this fraction (e.g. 0.20 = 20%).
    pub take_profit_pct: Option<f64>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum Side {
    Yes,
    No,
}

pub trait Strategy: Send + Sync {
    fn id(&self) -> &'static str;
    fn metadata(&self) -> StrategyMetadata;
    fn signal(&self) -> Result<Option<Signal>>;
    /// All signals from this strategy; defaults to wrapping `signal()`.
    fn signals(&self) -> Result<Vec<Signal>> {
        Ok(self.signal()?.into_iter().collect())
    }
}

/// Health status of a strategy instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrategyHealth {
    /// Recently produced a signal or is actively scanning.
    Healthy,
    /// No signal in >5 min; may be stale data or thin market.
    Stale,
    /// Error rate high or no signal in >15 min; needs attention.
    Critical,
}

impl StrategyHealth {
    /// Status indicator character for TUI display.
    pub fn indicator(self) -> &'static str {
        match self {
            StrategyHealth::Healthy => "●",
            StrategyHealth::Stale => "◐",
            StrategyHealth::Critical => "○",
        }
    }
}

#[derive(Clone, Debug)]
pub struct StrategyMetadata {
    pub name: &'static str,
    pub domain: &'static str,
}

/// Render a sparkline from a slice of 0.0–1.0 values into block characters.
/// Output is always `width` chars wide. Uses ▁▂▃▄▅▆▇█.
pub fn sparkline(values: &[f64], width: usize) -> String {
    const BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if values.is_empty() || width == 0 {
        return " ".repeat(width);
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(1e-9);
    // Sample or stretch to `width` columns.
    (0..width)
        .map(|i| {
            let idx = (i * values.len()) / width;
            let v = values.get(idx).copied().unwrap_or(0.0);
            let normalized = ((v - min) / range).clamp(0.0, 1.0);
            let block_idx = (normalized * (BLOCKS.len() - 1) as f64).round() as usize;
            BLOCKS[block_idx.min(BLOCKS.len() - 1)]
        })
        .collect()
}

/// Lightweight health tracker: tracks last-signal timestamp and error count.
/// Intended to be stored per-strategy in LiveState.
#[derive(Clone, Debug, Default)]
pub struct StrategyHealthTracker {
    pub last_signal_at: Option<std::time::Instant>,
    /// Consecutive cycles with no signal.
    pub consecutive_no_signal: u32,
    /// Total error count in last 100 calls.
    pub error_count: u32,
}

impl StrategyHealthTracker {
    /// Record a successful signal emission.
    pub fn record_signal(&mut self) {
        self.last_signal_at = Some(std::time::Instant::now());
        self.consecutive_no_signal = 0;
    }

    /// Record a cycle with no signal.
    pub fn record_no_signal(&mut self) {
        self.consecutive_no_signal = self.consecutive_no_signal.saturating_add(1);
    }

    /// Record a strategy error.
    pub fn record_error(&mut self) {
        self.error_count = self.error_count.saturating_add(1);
    }

    /// Compute current health status.
    pub fn health(&self) -> StrategyHealth {
        if self.error_count >= 3 {
            return StrategyHealth::Critical;
        }
        let elapsed_secs = self
            .last_signal_at
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(u64::MAX);
        if elapsed_secs > 900 || self.consecutive_no_signal > 10 {
            StrategyHealth::Critical
        } else if elapsed_secs > 300 || self.consecutive_no_signal > 5 {
            StrategyHealth::Stale
        } else {
            StrategyHealth::Healthy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_fields_set_correctly() {
        let sig = Signal {
            market_id: "market-abc".into(),
            side: Side::Yes,
            confidence: 0.75,
            edge_pct: 0.12,
            kelly_size: 0.05,
            auto_execute: false,
            strategy_id: "poisson-v1".into(),
            metadata: None,
            stop_loss_pct: None,
            take_profit_pct: None,
        };
        assert_eq!(sig.market_id, "market-abc");
        assert!((sig.edge_pct - 0.12).abs() < 1e-9);
        assert!((sig.kelly_size - 0.05).abs() < 1e-9);
        assert!(!sig.auto_execute);
        assert_eq!(sig.strategy_id, "poisson-v1");
    }

    #[test]
    fn side_serializes_as_variant_name() {
        let yes = serde_json::to_string(&Side::Yes).unwrap();
        let no = serde_json::to_string(&Side::No).unwrap();
        assert_eq!(yes, "\"Yes\"");
        assert_eq!(no, "\"No\"");
    }

    #[test]
    fn sparkline_produces_correct_length() {
        let vals = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let s = sparkline(&vals, 10);
        assert_eq!(s.chars().count(), 10);
    }

    #[test]
    fn sparkline_only_block_chars() {
        let vals: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let s = sparkline(&vals, 20);
        const CHARS: &str = "▁▂▃▄▅▆▇█";
        for ch in s.chars() {
            assert!(CHARS.contains(ch), "unexpected char: {ch}");
        }
    }

    #[test]
    fn sparkline_flat_is_all_same() {
        let vals = vec![5.0; 8];
        let s = sparkline(&vals, 8);
        let chars: Vec<char> = s.chars().collect();
        assert!(chars.windows(2).all(|w| w[0] == w[1]));
    }

    #[test]
    fn health_tracker_healthy_after_signal() {
        let mut t = StrategyHealthTracker::default();
        t.record_signal();
        assert_eq!(t.health(), StrategyHealth::Healthy);
    }

    #[test]
    fn health_tracker_stale_after_no_signals() {
        let mut t = StrategyHealthTracker::default();
        t.record_signal(); // set last_signal_at to now, so elapsed is small
        for _ in 0..6 {
            t.record_no_signal(); // consecutive_no_signal = 6 > 5 → Stale
        }
        assert_eq!(t.health(), StrategyHealth::Stale);
    }

    #[test]
    fn health_tracker_critical_after_many_errors() {
        let mut t = StrategyHealthTracker::default();
        for _ in 0..10 {
            t.record_error();
        }
        assert_eq!(t.health(), StrategyHealth::Critical);
    }

    #[test]
    fn health_indicator_chars() {
        assert_eq!(StrategyHealth::Healthy.indicator(), "●");
        assert_eq!(StrategyHealth::Stale.indicator(), "◐");
        assert_eq!(StrategyHealth::Critical.indicator(), "○");
    }

    #[test]
    fn signal_stop_loss_and_take_profit() {
        let sig = Signal {
            market_id: "m".into(),
            side: Side::Yes,
            confidence: 0.8,
            edge_pct: 0.15,
            kelly_size: 0.05,
            auto_execute: false,
            strategy_id: "test".into(),
            metadata: None,
            stop_loss_pct: Some(0.10),
            take_profit_pct: Some(0.25),
        };
        assert_eq!(sig.stop_loss_pct, Some(0.10));
        assert_eq!(sig.take_profit_pct, Some(0.25));
    }

    #[test]
    fn signal_with_metadata_serializes() {
        let sig = Signal {
            market_id: "m".into(),
            side: Side::No,
            confidence: 0.6,
            edge_pct: 0.08,
            kelly_size: 0.02,
            auto_execute: true,
            strategy_id: "arb".into(),
            metadata: Some(serde_json::json!({"key": "value"})),
            stop_loss_pct: None,
            take_profit_pct: None,
        };
        let s = serde_json::to_string(&sig).unwrap();
        assert!(s.contains("\"key\""));
        assert!(sig.auto_execute);
    }
}
