#![allow(dead_code)]

mod config;
mod copy_trading;
mod discover;
mod live;
mod trader_stats;
mod pm;
mod shared;
mod strategies;
mod tui;

fn main() -> anyhow::Result<()> {
    let _config = config::load()?;
    tui::run()
}
