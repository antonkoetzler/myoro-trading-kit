//! Cross-domain strategy engine: DataContext, UniversalStrategy, Rhai evaluation, TOML loading.
pub mod evaluator;
pub mod registry;
pub mod toml_loader;

use crate::shared::strategy::Signal;
use std::collections::HashMap;

/// Domain for a strategy or data context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    All,
    Crypto,
    Sports,
    Weather,
}

impl Domain {
    pub fn parse(s: &str) -> Self {
        match s {
            "crypto" => Self::Crypto,
            "sports" => Self::Sports,
            "weather" => Self::Weather,
            _ => Self::All,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Crypto => "crypto",
            Self::Sports => "sports",
            Self::Weather => "weather",
        }
    }
}

/// A typed value in a DataContext.
#[derive(Debug, Clone)]
pub enum Value {
    Float(f64),
    Int(i64),
    Str(String),
    Bool(bool),
}

impl Value {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// Universal key-value context populated by any domain.
/// Strategies evaluate expressions against these fields.
#[derive(Debug, Clone)]
pub struct DataContext {
    pub market_id: String,
    pub domain: Domain,
    pub values: HashMap<String, Value>,
}

impl DataContext {
    pub fn new(market_id: &str, domain: Domain) -> Self {
        Self {
            market_id: market_id.to_string(),
            domain,
            values: HashMap::new(),
        }
    }

    pub fn set_float(&mut self, key: &str, val: f64) {
        self.values.insert(key.to_string(), Value::Float(val));
    }

    pub fn set_int(&mut self, key: &str, val: i64) {
        self.values.insert(key.to_string(), Value::Int(val));
    }

    pub fn set_str(&mut self, key: &str, val: &str) {
        self.values
            .insert(key.to_string(), Value::Str(val.to_string()));
    }

    pub fn set_bool(&mut self, key: &str, val: bool) {
        self.values.insert(key.to_string(), Value::Bool(val));
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.values.get(key).and_then(|v| v.as_f64())
    }
}

/// Manifest describing a strategy's metadata.
#[derive(Debug, Clone)]
pub struct StrategyManifest {
    pub id: String,
    pub name: String,
    pub domain: Domain,
    pub enabled: bool,
    pub description: String,
    pub kelly_fraction: f64,
    pub min_edge: f64,
    pub auto_execute: bool,
}

/// Universal strategy trait — any domain, any source (Rust, TOML, Rhai).
pub trait UniversalStrategy: Send + Sync {
    fn manifest(&self) -> &StrategyManifest;
    fn evaluate(&self, contexts: &[DataContext]) -> Vec<Signal>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_context_set_and_get() {
        let mut ctx = DataContext::new("market_1", Domain::Crypto);
        ctx.set_float("price", 42.0);
        ctx.set_str("symbol", "BTC");
        ctx.set_bool("active", true);
        ctx.set_int("volume", 1000);

        assert_eq!(ctx.get_f64("price"), Some(42.0));
        assert_eq!(ctx.get_f64("volume"), Some(1000.0));
        assert_eq!(
            ctx.values.get("symbol").and_then(|v| v.as_str()),
            Some("BTC")
        );
        assert_eq!(
            ctx.values.get("active").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn domain_roundtrip() {
        assert_eq!(Domain::parse("crypto"), Domain::Crypto);
        assert_eq!(Domain::parse("sports"), Domain::Sports);
        assert_eq!(Domain::parse("weather"), Domain::Weather);
        assert_eq!(Domain::parse("unknown"), Domain::All);
        assert_eq!(Domain::Crypto.as_str(), "crypto");
    }
}
