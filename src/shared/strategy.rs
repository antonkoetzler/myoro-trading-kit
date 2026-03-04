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
}

#[derive(Clone, Debug)]
pub struct StrategyMetadata {
    pub name: &'static str,
    pub domain: &'static str,
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
        };
        let s = serde_json::to_string(&sig).unwrap();
        assert!(s.contains("\"key\""));
        assert!(sig.auto_execute);
    }
}
