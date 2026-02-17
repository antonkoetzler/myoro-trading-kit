//! Strategy trait and registry; domains implement this.

use anyhow::Result;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Signal {
    pub market_id: String,
    pub side: Side,
    pub confidence: f64,
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
