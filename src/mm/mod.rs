//! Market Making module: scanner, quoter, risk, state.
//!
//! Paper mode: simulate fills at quote price when market crosses.
//! Live mode: post GTC limit orders via CLOB.

pub mod quoter;
pub mod risk;
pub mod scanner;
pub mod state;

pub use quoter::MmQuoter;
pub use risk::MmRisk;
#[allow(unused_imports)]
pub use scanner::{MmCandidate, MmScanner};
#[allow(unused_imports)]
pub use state::{ActiveQuote, MmState, QuoteSide};

use crate::config::Config;
use crate::live::global::{push_log_to, LogLevel};
use std::sync::RwLock;

/// Run one market-making cycle: scan for candidates, post/update quotes.
/// Called from background thread every N seconds when `mm_enabled = true`.
pub fn run_mm_cycle(
    config: &Config,
    mm_state: &RwLock<MmState>,
    logs: &RwLock<Vec<(LogLevel, String)>>,
) {
    if !config.mm_enabled {
        return;
    }

    push_log_to(logs, LogLevel::Info, "MM: scanning for candidates…".into());

    let scanner = MmScanner::new(
        config.mm_half_spread * 2.0, // min spread = 2x half-spread
        config.mm_min_volume_usd,
        config.mm_max_markets as usize,
    );

    let candidates = match scanner.scan() {
        Ok(c) => c,
        Err(e) => {
            push_log_to(logs, LogLevel::Warning, format!("MM scan error: {}", e));
            return;
        }
    };

    push_log_to(
        logs,
        LogLevel::Info,
        format!("MM: {} candidates found", candidates.len()),
    );

    let quoter = MmQuoter::new(config.mm_half_spread);
    let risk = MmRisk::from_config(config);
    let quote_size = 10.0_f64; // $10 per side in paper mode

    for candidate in &candidates {
        let (inv, total_exp) = mm_state
            .read()
            .map(|s| {
                let inv = s.inventory_for(&candidate.market_id);
                let total: f64 = s.inventory.values().map(|i| i.net_yes.abs()).sum();
                (inv, total)
            })
            .unwrap_or((0.0, 0.0));

        if risk.at_limit(inv) {
            push_log_to(
                logs,
                LogLevel::Info,
                format!(
                    "MM: {} at inventory limit, skipping",
                    &candidate.market_id[..8.min(candidate.market_id.len())]
                ),
            );
            continue;
        }

        if risk.check_quote(inv, total_exp, quote_size).is_err() {
            push_log_to(
                logs,
                LogLevel::Warning,
                "MM: total exposure limit reached".into(),
            );
            break;
        }

        let (bid, ask) = quoter.generate_quotes(candidate, quote_size);

        if let Ok(mut s) = mm_state.write() {
            // Remove stale quotes for this market.
            s.active_quotes
                .retain(|q| q.market_id != candidate.market_id);
            // In paper mode: post both sides (simulate).
            s.active_quotes.push(bid);
            s.active_quotes.push(ask);
        }
    }

    if let Ok(s) = mm_state.read() {
        push_log_to(
            logs,
            LogLevel::Success,
            format!(
                "MM: {} active quotes | PnL {:+.2}",
                s.active_quotes.len(),
                s.total_realized_pnl
            ),
        );
    }
}
