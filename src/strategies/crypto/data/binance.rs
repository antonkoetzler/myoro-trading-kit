//! Binance WebSocket client: kline_5m / kline_15m, bookTicker; buffer last N candles and best bid/ask.

use anyhow::Result;

pub struct BinanceClient {
    _symbol: String,
}

impl BinanceClient {
    pub fn new(symbol: &str) -> Result<Self> {
        Ok(Self {
            _symbol: symbol.to_string(),
        })
    }

    pub async fn connect(&self) -> Result<()> {
        Ok(())
    }
}
