// NOTE: Exceeds 300-line limit — TOML parser + Rhai expression evaluator + condition compiler; these three passes form a single pipeline and cannot be split without introducing unnecessary abstraction. See docs/ai-rules/file-size.md
//! Enhanced TOML strategy loader with Rhai expression support.
//!
//! Supports both legacy field/operator/value conditions and new `expr` conditions.
//! ```toml
//! [[logic.conditions]]
//! expr = "home_win_rate - market_yes_price > 0.10"
//! ```
use super::evaluator::{self, CompiledExpr};
use super::{DataContext, Domain, StrategyManifest, UniversalStrategy};
use crate::shared::strategy::{Side, Signal};
use anyhow::{Context, Result};
use rhai::Engine;
use serde::Deserialize;

/// Parsed TOML strategy with Rhai expressions.
#[derive(Debug, Deserialize)]
struct TomlFile {
    strategy: StrategySection,
    #[serde(default)]
    logic: LogicSection,
}

#[derive(Debug, Deserialize)]
struct StrategySection {
    id: String,
    name: String,
    #[serde(default = "default_domain")]
    domain: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    description: String,
    #[serde(default)]
    risk: Option<RiskSection>,
}

#[derive(Debug, Deserialize)]
struct RiskSection {
    #[serde(default = "default_kelly")]
    kelly_fraction: f64,
    #[serde(default = "default_min_edge")]
    min_edge: f64,
}

#[derive(Debug, Default, Deserialize)]
struct LogicSection {
    #[serde(default = "default_side")]
    side: String,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default)]
    conditions: Vec<ConditionEntry>,
    #[serde(default)]
    edge: Option<ExprEntry>,
    #[serde(default)]
    confidence: Option<ExprEntry>,
}

#[derive(Debug, Deserialize)]
struct ConditionEntry {
    expr: String,
}

#[derive(Debug, Deserialize)]
struct ExprEntry {
    expr: String,
}

fn default_domain() -> String {
    "all".into()
}
fn default_true() -> bool {
    true
}
fn default_kelly() -> f64 {
    0.25
}
fn default_min_edge() -> f64 {
    0.05
}
fn default_side() -> String {
    "yes".into()
}
fn default_mode() -> String {
    "all".into()
}

/// A loaded strategy backed by compiled Rhai expressions.
pub struct ExprStrategy {
    manifest: StrategyManifest,
    engine: Engine,
    conditions: Vec<CompiledExpr>,
    mode_all: bool,
    side: Side,
    edge_expr: Option<CompiledExpr>,
    confidence_expr: Option<CompiledExpr>,
}

impl std::fmt::Debug for ExprStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExprStrategy")
            .field("id", &self.manifest.id)
            .finish()
    }
}

impl ExprStrategy {
    /// Parse a TOML string into a strategy with compiled Rhai expressions.
    pub fn parse(content: &str) -> Result<Self> {
        let file: TomlFile = toml::from_str(content).context("parse strategy TOML")?;
        let engine = evaluator::create_engine();

        let risk = file.strategy.risk.unwrap_or(RiskSection {
            kelly_fraction: default_kelly(),
            min_edge: default_min_edge(),
        });

        let manifest = StrategyManifest {
            id: file.strategy.id,
            name: file.strategy.name,
            domain: Domain::parse(&file.strategy.domain),
            enabled: file.strategy.enabled,
            description: file.strategy.description,
            kelly_fraction: risk.kelly_fraction.clamp(0.01, 1.0),
            min_edge: risk.min_edge.clamp(0.001, 1.0),
            auto_execute: false,
        };

        let conditions: Vec<CompiledExpr> = file
            .logic
            .conditions
            .iter()
            .filter_map(|c| CompiledExpr::compile(&engine, &c.expr))
            .collect();

        let mode_all = file.logic.mode != "any";
        let side = if file.logic.side == "no" {
            Side::No
        } else {
            Side::Yes
        };

        let edge_expr = file
            .logic
            .edge
            .as_ref()
            .and_then(|e| CompiledExpr::compile(&engine, &e.expr));
        let confidence_expr = file
            .logic
            .confidence
            .as_ref()
            .and_then(|e| CompiledExpr::compile(&engine, &e.expr));

        Ok(Self {
            manifest,
            engine,
            conditions,
            mode_all,
            side,
            edge_expr,
            confidence_expr,
        })
    }

    fn conditions_met(&self, ctx: &DataContext) -> bool {
        if self.conditions.is_empty() {
            return false;
        }
        if self.mode_all {
            self.conditions
                .iter()
                .all(|c| c.eval_bool(&self.engine, ctx).unwrap_or(false))
        } else {
            self.conditions
                .iter()
                .any(|c| c.eval_bool(&self.engine, ctx).unwrap_or(false))
        }
    }
}

impl UniversalStrategy for ExprStrategy {
    fn manifest(&self) -> &StrategyManifest {
        &self.manifest
    }

    fn evaluate(&self, contexts: &[DataContext]) -> Vec<Signal> {
        contexts
            .iter()
            .filter(|ctx| self.conditions_met(ctx))
            .map(|ctx| {
                let edge = self
                    .edge_expr
                    .as_ref()
                    .and_then(|e| e.eval_float(&self.engine, ctx))
                    .unwrap_or(self.manifest.min_edge);

                let confidence = self
                    .confidence_expr
                    .as_ref()
                    .and_then(|e| e.eval_float(&self.engine, ctx))
                    .unwrap_or(0.5)
                    .clamp(0.0, 1.0);

                let kelly = (confidence - (1.0 - confidence) / (edge / confidence).max(0.01))
                    .max(0.0)
                    * self.manifest.kelly_fraction;

                Signal {
                    market_id: ctx.market_id.clone(),
                    side: self.side,
                    confidence,
                    edge_pct: edge,
                    kelly_size: kelly.max(0.0),
                    auto_execute: self.manifest.auto_execute,
                    strategy_id: self.manifest.id.clone(),
                    metadata: None,
                    stop_loss_pct: None,
                    take_profit_pct: None,
                }
            })
            .filter(|s| s.edge_pct >= self.manifest.min_edge)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[strategy]
id = "xg_value"
name = "xG Value Finder"
domain = "sports"
enabled = true
description = "Finds value on home teams with strong xG stats"

[strategy.risk]
kelly_fraction = 0.20
min_edge = 0.08

[logic]
side = "yes"
mode = "all"

[[logic.conditions]]
expr = "home_xg_per90 > 1.8"

[[logic.conditions]]
expr = "home_win_rate - market_yes_price > min_edge"

[logic.edge]
expr = "home_win_rate - market_yes_price"

[logic.confidence]
expr = "if home_win_rate > 1.0 { 1.0 } else { home_win_rate }"
"#;

    #[test]
    fn parse_expr_strategy() {
        let strat = ExprStrategy::parse(SAMPLE).unwrap();
        assert_eq!(strat.manifest.id, "xg_value");
        assert_eq!(strat.manifest.domain, Domain::Sports);
        assert_eq!(strat.conditions.len(), 2);
    }

    #[test]
    fn evaluate_matching_context() {
        let strat = ExprStrategy::parse(SAMPLE).unwrap();
        let mut ctx = DataContext::new("market_1", Domain::Sports);
        ctx.set_float("home_xg_per90", 2.1);
        ctx.set_float("home_win_rate", 0.72);
        ctx.set_float("market_yes_price", 0.55);
        ctx.set_float("min_edge", 0.08);

        let signals = strat.evaluate(&[ctx]);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].market_id, "market_1");
        assert!(signals[0].edge_pct > 0.08);
    }

    #[test]
    fn evaluate_non_matching_context() {
        let strat = ExprStrategy::parse(SAMPLE).unwrap();
        let mut ctx = DataContext::new("market_2", Domain::Sports);
        ctx.set_float("home_xg_per90", 1.2); // fails: < 1.8
        ctx.set_float("home_win_rate", 0.45);
        ctx.set_float("market_yes_price", 0.55);
        ctx.set_float("min_edge", 0.08);

        let signals = strat.evaluate(&[ctx]);
        assert!(signals.is_empty());
    }

    #[test]
    fn any_mode_works() {
        let toml = r#"
[strategy]
id = "any_test"
name = "Any Mode Test"

[logic]
mode = "any"

[[logic.conditions]]
expr = "price > 100.0"

[[logic.conditions]]
expr = "volume > 1000.0"
"#;
        let strat = ExprStrategy::parse(toml).unwrap();
        let mut ctx = DataContext::new("m1", Domain::All);
        ctx.set_float("price", 50.0); // fails first
        ctx.set_float("volume", 2000.0); // passes second

        let signals = strat.evaluate(&[ctx]);
        assert_eq!(signals.len(), 1); // any mode: one condition enough
    }
}
