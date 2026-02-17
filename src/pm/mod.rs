//! Polymarket client wrapper (CLOB + Gamma + Data + WS via polymarket-client-sdk).
//! Init and optional WS subscription for orderbook/prices; thin wrapper for strategies and TUI.

#![allow(dead_code)]

use anyhow::Result;

/// Polymarket client wrapper. Holds SDK clients when configured; used by strategies and TUI.
pub struct PmClient {
    _host: String,
}

impl PmClient {
    pub fn new(host: &str) -> Result<Self> {
        Ok(Self {
            _host: host.to_string(),
        })
    }

    /// Placeholder: later will subscribe to orderbook/price streams via SDK WS.
    pub async fn connect_ws(&self) -> Result<()> {
        Ok(())
    }
}
