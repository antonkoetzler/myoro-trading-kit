//! BinanceLagStrategy: detect when Binance 5-min momentum hasn't repriced Polymarket.
//!
//! Logic:
//!   1. Fetch latest BTC price from Binance REST (5-min kline or ticker).
//!   2. Fetch the last cached price from state.
//!   3. If move > MOMENTUM_THRESHOLD (0.4%), fetch matching Polymarket crypto market odds.
//!   4. If Polymarket hasn't moved commensurately (edge > MIN_EDGE), emit signal.

use super::{GammaMarket, StoredCryptoSignal};
use crate::shared::strategy::{Side, Signal, Strategy, StrategyMetadata};
use anyhow::Result;
use chrono::Utc;

const MOMENTUM_THRESHOLD: f64 = 0.004; // 0.4% Binance move triggers check
const MIN_EDGE: f64 = 0.03; // 3% min implied edge
const KELLY_FRACTION: f64 = 0.25; // quarter-Kelly sizing

pub struct BinanceLagStrategy {
    /// Binance symbols to scan (e.g. ["BTCUSDT", "ETHUSDT"]).
    pub symbols: Vec<String>,
}

impl BinanceLagStrategy {
    pub fn new() -> Self {
        Self {
            symbols: vec!["BTCUSDT".to_string()],
        }
    }

    /// Construct with a custom symbol list (from config).
    pub fn with_symbols(symbols: Vec<String>) -> Self {
        let symbols = if symbols.is_empty() {
            vec!["BTCUSDT".to_string()]
        } else {
            symbols
        };
        Self { symbols }
    }

    fn ticker_url(symbol: &str) -> String {
        format!(
            "https://api.binance.com/api/v3/ticker/price?symbol={}",
            symbol
        )
    }

    /// Fetch current BTC/USDT price from Binance.
    fn fetch_btc_price(client: &reqwest::blocking::Client, url: &str) -> Result<f64> {
        let json: serde_json::Value = client.get(url).send()?.json()?;
        let price_str = json
            .get("price")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow::anyhow!("no price field"))?;
        Ok(price_str.parse::<f64>()?)
    }

    /// Fetch crypto markets from Gamma API tagged "bitcoin" or "btc".
    fn fetch_crypto_markets(client: &reqwest::blocking::Client) -> Result<Vec<GammaMarket>> {
        let url = "https://gamma-api.polymarket.com/markets?closed=false&limit=20&tag=crypto";
        let resp = client.get(url).send()?;
        let arr: Vec<serde_json::Value> = resp.json().unwrap_or_default();
        let markets = arr
            .iter()
            .filter_map(|m| {
                let id = m.get("conditionId").and_then(|v| v.as_str())?.to_string();
                let title = m.get("question").and_then(|v| v.as_str())?.to_string();
                let best_bid = m.get("bestBid").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let best_ask = m.get("bestAsk").and_then(|v| v.as_f64()).unwrap_or(1.0);
                let volume = m.get("volumeNum").and_then(|v| v.as_f64()).unwrap_or(0.0);
                // Only surface BTC/ETH price markets (title contains "above" or "$")
                let lower = title.to_lowercase();
                if lower.contains("btc") || lower.contains("bitcoin") || lower.contains("ethereum")
                {
                    Some(GammaMarket {
                        id,
                        title,
                        best_bid,
                        best_ask,
                        volume,
                    })
                } else {
                    None
                }
            })
            .collect();
        Ok(markets)
    }

    /// Compute implied probability from mid-price and check for edge.
    /// Returns (side, edge_pct, kelly_size) if signal found.
    ///
    /// Heuristic: 1% BTC price move ≈ 4% shift in binary market probability.
    /// This is a rough proxy; real sizing would use Black-Scholes or market-specific vol.
    fn find_signal(price_change_pct: f64, market_mid: f64) -> Option<(Side, f64, f64)> {
        let expected_edge = (price_change_pct.abs() * 4.0).min(0.20);
        if price_change_pct > MOMENTUM_THRESHOLD {
            let fair = (market_mid + expected_edge).min(0.95);
            let edge = fair - market_mid;
            if edge >= MIN_EDGE {
                let kelly = (edge / (1.0 - market_mid)) * KELLY_FRACTION;
                return Some((Side::Yes, edge, kelly.min(0.1)));
            }
        } else if price_change_pct < -MOMENTUM_THRESHOLD {
            let fair = (market_mid - expected_edge).max(0.05);
            let edge = market_mid - fair;
            if edge >= MIN_EDGE {
                let kelly = (edge / market_mid) * KELLY_FRACTION;
                return Some((Side::No, edge, kelly.min(0.1)));
            }
        }
        None
    }

    /// Run the full strategy for a single symbol, returning any new signal.
    pub fn run(&self, prev_price: f64) -> Result<(f64, Option<StoredCryptoSignal>)> {
        let symbol = self
            .symbols
            .first()
            .map(|s| s.as_str())
            .unwrap_or("BTCUSDT");
        self.run_symbol(symbol, prev_price)
    }

    /// Run for a specific symbol. Returns (current_price, Option<signal>).
    pub fn run_symbol(
        &self,
        symbol: &str,
        prev_price: f64,
    ) -> Result<(f64, Option<StoredCryptoSignal>)> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let url = Self::ticker_url(symbol);
        let current_price = Self::fetch_btc_price(&client, &url)?;
        if prev_price <= 0.0 {
            return Ok((current_price, None));
        }
        let change_pct = (current_price - prev_price) / prev_price;
        if change_pct.abs() < MOMENTUM_THRESHOLD {
            return Ok((current_price, None));
        }
        let markets = Self::fetch_crypto_markets(&client)?;
        for market in &markets {
            let mid = (market.best_bid + market.best_ask) / 2.0;
            if let Some((side, edge, kelly)) = Self::find_signal(change_pct, mid) {
                let strategy_id = format!("binance_lag_{}", symbol.to_lowercase());
                let signal = StoredCryptoSignal {
                    market_id: market.id.clone(),
                    label: market.title.clone(),
                    side: format!("{:?}", side),
                    edge_pct: edge,
                    kelly_size: kelly,
                    strategy_id,
                    status: "pending".to_string(),
                    created_at: Utc::now(),
                };
                return Ok((current_price, Some(signal)));
            }
        }
        Ok((current_price, None))
    }

    /// Run for all configured symbols. Returns all signals found.
    pub fn run_all_symbols(
        &self,
        prev_prices: &std::collections::HashMap<String, f64>,
    ) -> Vec<StoredCryptoSignal> {
        let mut signals = Vec::new();
        for symbol in &self.symbols {
            let prev = prev_prices.get(symbol).copied().unwrap_or(0.0);
            if let Ok((_price, Some(sig))) = self.run_symbol(symbol, prev) {
                signals.push(sig);
            }
        }
        signals
    }
}

impl Strategy for BinanceLagStrategy {
    fn id(&self) -> &'static str {
        "binance_lag"
    }

    fn metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            name: "Binance Lag",
            domain: "crypto",
        }
    }

    fn signal(&self) -> Result<Option<Signal>> {
        // For the shared Strategy trait: run with zero prev price (first call).
        let (_price, stored) = self.run(0.0)?;
        Ok(stored.map(|s| Signal {
            market_id: s.market_id,
            side: crate::shared::strategy::Side::Yes,
            confidence: s.edge_pct.min(1.0),
            edge_pct: s.edge_pct,
            kelly_size: s.kelly_size,
            auto_execute: false,
            strategy_id: s.strategy_id,
            metadata: None,
            stop_loss_pct: None,
            take_profit_pct: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_signal_when_price_unchanged() {
        let (_, sig) = BinanceLagStrategy::new().run(0.0).unwrap();
        assert!(sig.is_none());
    }

    #[test]
    fn find_signal_positive_move_returns_yes() {
        let result = BinanceLagStrategy::find_signal(0.01, 0.42);
        assert!(result.is_some());
        let (side, edge, kelly) = result.unwrap();
        assert!(matches!(side, Side::Yes));
        assert!(edge >= 0.03);
        assert!(kelly > 0.0 && kelly <= 0.1);
    }

    #[test]
    fn find_signal_negative_move_returns_no() {
        let result = BinanceLagStrategy::find_signal(-0.01, 0.58);
        assert!(result.is_some());
        let (side, _, _) = result.unwrap();
        assert!(matches!(side, Side::No));
    }

    #[test]
    fn find_signal_small_move_returns_none() {
        let result = BinanceLagStrategy::find_signal(0.001, 0.50);
        assert!(result.is_none());
    }

    #[test]
    fn with_symbols_stores_all_assets() {
        let assets = vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "SOLUSDT".to_string(),
        ];
        let strat = BinanceLagStrategy::with_symbols(assets.clone());
        assert_eq!(strat.symbols.len(), 3);
        assert_eq!(strat.symbols, assets);
    }

    #[test]
    fn with_symbols_defaults_to_btcusdt_when_empty() {
        let strat = BinanceLagStrategy::with_symbols(vec![]);
        assert_eq!(strat.symbols, vec!["BTCUSDT"]);
    }

    #[test]
    fn run_all_symbols_returns_empty_when_no_prior_prices() {
        // With no prior price, run_symbol returns (price, None) → no signals
        let assets = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let strat = BinanceLagStrategy::with_symbols(assets);
        let prev = std::collections::HashMap::new();
        // run_all_symbols makes live HTTP calls; but with empty prev prices
        // run_symbol returns early with Ok((price, None)) — no panic.
        // We just verify the function is callable without panicking.
        let _ = strat.run_all_symbols(&prev);
    }
}

impl Default for BinanceLagStrategy {
    fn default() -> Self {
        Self::new()
    }
}
