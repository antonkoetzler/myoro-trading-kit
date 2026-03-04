//! Polymarket client: CLOB orders, data API, WebSocket subscriptions.

pub mod clob;
pub mod data;
pub mod ws;

#[allow(unused_imports)]
pub use clob::{ClobAuth, ClobClient, Fill, Order, OrderType, Side as ClobSide};
#[allow(unused_imports)]
pub use data::{DataClient, Position};
#[allow(unused_imports)]
pub use ws::{OrderbookSnapshot, WsClient};

/// Unified Polymarket client wrapping all sub-clients.
pub struct PmClient {
    pub clob: ClobClient,
    pub data: DataClient,
    pub ws: WsClient,
}

impl PmClient {
    pub fn new(clob_host: &str, data_host: &str, ws_endpoint: &str) -> Self {
        Self {
            clob: ClobClient::new(clob_host),
            data: DataClient::new(data_host),
            ws: WsClient::new(ws_endpoint),
        }
    }

    /// Create client from config — wires CLOB credentials when present.
    pub fn from_config(config: &crate::config::Config) -> Self {
        let mut pm = Self::default_endpoints();
        let pm_cfg = &config.polymarket;
        // Require at minimum funder_address + API key to attempt live CLOB.
        if let (Some(addr), Some(key), Some(pass)) = (
            pm_cfg.funder_address.as_deref(),
            pm_cfg.api_key.as_deref(),
            pm_cfg.api_passphrase.as_deref(),
        ) {
            pm.clob = pm.clob.with_auth(ClobAuth {
                api_key: key.to_string(),
                api_secret: pm_cfg.api_secret.clone().unwrap_or_default(),
                api_passphrase: pass.to_string(),
                funder_address: addr.to_string(),
            });
        }
        pm
    }

    /// Create client with default Polymarket endpoints.
    pub fn default_endpoints() -> Self {
        Self::new(
            "https://clob.polymarket.com",
            "https://data-api.polymarket.com",
            "wss://ws-subscriptions-clob.polymarket.com/ws/market",
        )
    }
}
