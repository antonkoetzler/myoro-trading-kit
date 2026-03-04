//! Polymarket WebSocket subscriptions: orderbook and price feed.
//! Stub implementation — real wiring requires the SDK `ws` feature.

use anyhow::Result;

/// Orderbook snapshot for a market.
#[derive(Clone, Debug)]
pub struct OrderbookSnapshot {
    pub market_id: String,
    /// Best bid price (YES side).
    pub best_bid: f64,
    /// Best ask price (YES side).
    pub best_ask: f64,
    /// Mid price.
    pub mid: f64,
}

impl OrderbookSnapshot {
    pub fn spread(&self) -> f64 {
        self.best_ask - self.best_bid
    }
}

/// Stub WS client. Real implementation subscribes via SDK.
pub struct WsClient {
    _endpoint: String,
}

impl WsClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            _endpoint: endpoint.to_string(),
        }
    }

    /// Subscribe to orderbook updates for a market.
    /// Sends snapshots to the provided channel.
    pub async fn subscribe_orderbook(
        &self,
        _market_id: &str,
        _tx: tokio::sync::mpsc::Sender<OrderbookSnapshot>,
    ) -> Result<()> {
        // TODO: wire polymarket-client-sdk WS subscribe_market when credentials available.
        Ok(())
    }

    /// Subscribe to price feed for a market.
    pub async fn subscribe_price(
        &self,
        _market_id: &str,
        _tx: tokio::sync::mpsc::Sender<f64>,
    ) -> Result<()> {
        // TODO: wire polymarket-client-sdk WS subscribe_price when credentials available.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orderbook_spread_calculation() {
        let snap = OrderbookSnapshot {
            market_id: "m1".into(),
            best_bid: 0.48,
            best_ask: 0.52,
            mid: 0.50,
        };
        assert!((snap.spread() - 0.04).abs() < 1e-9);
    }

    #[tokio::test]
    async fn subscribe_orderbook_returns_ok() {
        let client = WsClient::new("wss://ws-subscriptions-clob.polymarket.com/ws/market");
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        assert!(client.subscribe_orderbook("m1", tx).await.is_ok());
    }
}
