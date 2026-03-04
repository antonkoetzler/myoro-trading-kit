//! Crypto tab key event handler.

use crate::live::LiveState;
use crossterm::event::KeyCode;

pub fn handle_key(key: KeyCode, strategy_sel: &mut usize, live: &LiveState) {
    let n_strats = live
        .crypto
        .read()
        .map(|c| c.strategy_configs.len())
        .unwrap_or(0);
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            *strategy_sel = (*strategy_sel + 1).min(n_strats.saturating_sub(1));
        }
        KeyCode::Char('k') | KeyCode::Up => {
            *strategy_sel = strategy_sel.saturating_sub(1);
        }
        KeyCode::Char(' ') => {
            if let Ok(mut c) = live.crypto.write() {
                if let Some(cfg) = c.strategy_configs.get_mut(*strategy_sel) {
                    cfg.enabled = !cfg.enabled;
                }
            }
        }
        KeyCode::Char('d') => {
            if let Ok(mut c) = live.crypto.write() {
                if let Some(pos) = c.signals.iter().position(|s| s.status == "pending") {
                    c.signals[pos].status = "dismissed".to_string();
                }
            }
        }
        KeyCode::Char('r') => {
            live.push_crypto_log(
                crate::live::LogLevel::Info,
                "Manual refresh triggered".to_string(),
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::LiveState;
    use crate::strategies::crypto::StoredCryptoSignal;

    fn live_with_pending_signal() -> LiveState {
        let live = LiveState::default();
        if let Ok(mut c) = live.crypto.write() {
            c.signals.push(StoredCryptoSignal {
                market_id: "m1".to_string(),
                label: "Test".to_string(),
                side: "YES".to_string(),
                edge_pct: 0.05,
                kelly_size: 0.1,
                strategy_id: "s1".to_string(),
                status: "pending".to_string(),
                created_at: chrono::Utc::now(),
            });
        }
        live
    }

    #[test]
    fn space_toggles_strategy_enabled() {
        // Default crypto state has 2 built-in strategies, both disabled
        let live = LiveState::default();
        let mut sel = 0usize;
        handle_key(KeyCode::Char(' '), &mut sel, &live);
        let enabled = live
            .crypto
            .read()
            .ok()
            .and_then(|c| c.strategy_configs.first().map(|s| s.enabled));
        assert_eq!(enabled, Some(true));
        // toggle back
        handle_key(KeyCode::Char(' '), &mut sel, &live);
        let enabled2 = live
            .crypto
            .read()
            .ok()
            .and_then(|c| c.strategy_configs.first().map(|s| s.enabled));
        assert_eq!(enabled2, Some(false));
    }

    #[test]
    fn d_dismisses_first_pending_signal() {
        let live = live_with_pending_signal();
        let mut sel = 0usize;
        handle_key(KeyCode::Char('d'), &mut sel, &live);
        let status = live
            .crypto
            .read()
            .ok()
            .and_then(|c| c.signals.first().map(|s| s.status.clone()));
        assert_eq!(status, Some("dismissed".to_string()));
    }

    #[test]
    fn r_pushes_log_entry() {
        let live = LiveState::default();
        let mut sel = 0usize;
        handle_key(KeyCode::Char('r'), &mut sel, &live);
        let logs = live.get_crypto_logs();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].1.contains("refresh"));
    }

    #[test]
    fn unknown_key_is_noop() {
        let live = LiveState::default();
        let mut sel = 2usize;
        handle_key(KeyCode::F(1), &mut sel, &live);
        assert_eq!(sel, 2);
    }
}
