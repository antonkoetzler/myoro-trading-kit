#![allow(dead_code)]
// Deny unwrap/expect in production code; tests may still use them freely.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod backtester;
pub mod config;
pub mod copy_trading;
pub mod data_providers;
pub mod discover;
pub mod live;
pub mod mm;
pub mod pm;
pub mod shared;
pub mod sports;
pub mod strategies;
pub mod strategy_engine;
pub mod weather;
