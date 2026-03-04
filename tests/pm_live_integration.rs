//! Live Polymarket integration tests.
//! Skipped by default — require real credentials and network access.
//! Run with: cargo test --test pm_live_integration -- --ignored

use myoro_polymarket_terminal::config::{Config, ExecutionMode};
use myoro_polymarket_terminal::pm::clob::Side;
use myoro_polymarket_terminal::pm::{ClobAuth, ClobClient, DataClient, Order, OrderType};

/// Helper: build ClobClient from environment or skip test.
fn clob_from_env() -> Option<ClobClient> {
    let key = std::env::var("API_KEY").ok()?;
    let secret = std::env::var("API_SECRET").ok()?;
    let pass = std::env::var("API_PASSPHRASE").ok()?;
    let addr = std::env::var("FUNDER_ADDRESS").ok()?;
    Some(
        ClobClient::new("https://clob.polymarket.com").with_auth(ClobAuth {
            api_key: key,
            api_secret: secret,
            api_passphrase: pass,
            funder_address: addr,
        }),
    )
}

fn data_client() -> DataClient {
    DataClient::new("https://data-api.polymarket.com")
}

#[test]
#[ignore = "requires live CLOB credentials (API_KEY, API_PASSPHRASE, FUNDER_ADDRESS)"]
fn place_limit_and_cancel() {
    let clob = clob_from_env().expect("set API_KEY, API_PASSPHRASE, FUNDER_ADDRESS to run");
    let order = Order {
        market_id: "21742633143463906290569050155826241533067272736897614950488156847949938836455"
            .into(), // BTC above 100k (example)
        side: Side::Yes,
        price: 0.01, // low price unlikely to fill
        size: 1.0,
        order_type: OrderType::Limit,
        post_only: true,
    };
    let order_id = clob
        .place_limit_order(&order)
        .expect("place_limit_order should succeed");
    assert!(!order_id.is_empty(), "order_id should be non-empty");

    let cancel = clob.cancel_order(&order_id);
    assert!(cancel.is_ok(), "cancel should succeed: {:?}", cancel);
}

#[test]
#[ignore = "requires live FUNDER_ADDRESS env var"]
fn get_positions_returns_result() {
    let wallet = std::env::var("FUNDER_ADDRESS").expect("set FUNDER_ADDRESS to run this test");
    let data = data_client();
    let positions = data.get_positions(&wallet);
    assert!(
        positions.is_ok(),
        "get_positions should return Ok: {:?}",
        positions
    );
}

#[test]
#[ignore = "requires live FUNDER_ADDRESS env var"]
fn get_trades_returns_result() {
    let wallet = std::env::var("FUNDER_ADDRESS").expect("set FUNDER_ADDRESS to run this test");
    let data = data_client();
    let trades = data.get_trades(&wallet, 10);
    assert!(trades.is_ok(), "get_trades should return Ok: {:?}", trades);
}
