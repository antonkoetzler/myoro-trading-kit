//! Execution: paper (JSONL fill simulation) or live (CLOB GTC limit order).

use crate::config::ExecutionMode;
use crate::pm::clob::{ClobClient, Fill, Order, OrderType, Side};
use anyhow::Result;
use std::io::Write;
use std::sync::Arc;

pub struct Executor {
    mode: ExecutionMode,
    /// Path to the JSONL file for paper trade recording.
    paper_trades_file: String,
    /// Live CLOB client — None in paper mode or when credentials absent.
    clob: Option<Arc<ClobClient>>,
}

impl Executor {
    pub fn new(mode: ExecutionMode, paper_trades_file: &str) -> Self {
        Self {
            mode,
            paper_trades_file: paper_trades_file.to_string(),
            clob: None,
        }
    }

    pub fn with_clob(mut self, clob: Arc<ClobClient>) -> Self {
        self.clob = Some(clob);
        self
    }

    /// Execute at best available price. Paper: simulate fill and write to JSONL.
    /// Live: place a GTC limit order via CLOB (post_only=true).
    pub fn execute(&self, market_id: &str, side: super::strategy::Side, amount: f64) -> Result<()> {
        self.execute_with_price(market_id, side, amount, 0.5)
    }

    /// Execute with a specific price. Returns Ok after paper fill or live order.
    pub fn execute_with_price(
        &self,
        market_id: &str,
        side: super::strategy::Side,
        amount: f64,
        price: f64,
    ) -> Result<()> {
        match self.mode {
            ExecutionMode::Paper => {
                let fill = self.simulate_fill(market_id, side, amount, price);
                self.write_paper_fill(&fill)?;
            }
            ExecutionMode::Live => {
                let clob_side = match side {
                    super::strategy::Side::Yes => Side::Yes,
                    super::strategy::Side::No => Side::No,
                };
                let order = Order {
                    market_id: market_id.to_string(),
                    side: clob_side,
                    price,
                    size: amount,
                    order_type: OrderType::Limit,
                    post_only: true,
                };
                let clob = self
                    .clob
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Live mode: ClobClient not configured"))?;
                let order_id = clob.place_limit_order(&order)?;
                eprintln!(
                    "[Executor] Live order placed: {} {:?} × {:.2} @ {:.4} → {}",
                    market_id, side, amount, price, order_id
                );
            }
        }
        Ok(())
    }

    pub fn is_live(&self) -> bool {
        self.mode == ExecutionMode::Live
    }

    fn simulate_fill(
        &self,
        market_id: &str,
        side: super::strategy::Side,
        amount: f64,
        price: f64,
    ) -> Fill {
        Fill {
            order_id: format!("paper-{}", chrono::Utc::now().timestamp_millis()),
            market_id: market_id.to_string(),
            side: match side {
                super::strategy::Side::Yes => Side::Yes,
                super::strategy::Side::No => Side::No,
            },
            price,
            size: amount,
            timestamp: chrono::Utc::now(),
        }
    }

    fn write_paper_fill(&self, fill: &Fill) -> Result<()> {
        // Ensure parent directory exists.
        if let Some(parent) = std::path::Path::new(&self.paper_trades_file).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.paper_trades_file)?;
        let line = serde_json::to_string(fill)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::strategy::Side;

    #[test]
    fn paper_executor_returns_ok_and_does_not_send() {
        let exec = Executor::new(ExecutionMode::Paper, "data/test_paper_trades.jsonl");
        assert!(!exec.is_live());
        assert!(exec.execute("market1", Side::Yes, 1.0).is_ok());
        let _ = std::fs::remove_file("data/test_paper_trades.jsonl");
    }

    #[test]
    fn live_executor_without_clob_returns_err() {
        let exec = Executor::new(ExecutionMode::Live, "data/paper.jsonl");
        assert!(exec.is_live());
        let result = exec.execute("market1", Side::No, 1.0);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("ClobClient not configured"));
    }

    #[test]
    fn simulate_fill_has_correct_fields() {
        let exec = Executor::new(ExecutionMode::Paper, "data/test.jsonl");
        let fill = exec.simulate_fill("mkt-1", Side::Yes, 25.0, 0.55);
        assert_eq!(fill.market_id, "mkt-1");
        assert!((fill.size - 25.0).abs() < 1e-9);
        assert!((fill.price - 0.55).abs() < 1e-9);
        assert!(fill.order_id.starts_with("paper-"));
    }
}
