//! Weather tab key event handler.

use crate::live::LiveState;
use crossterm::event::KeyCode;

pub fn handle_key(key: KeyCode, strategy_sel: &mut usize, live: &LiveState) {
    let n_strats = live
        .weather
        .read()
        .map(|w| w.strategy_configs.len())
        .unwrap_or(0);
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            *strategy_sel = (*strategy_sel + 1).min(n_strats.saturating_sub(1));
        }
        KeyCode::Char('k') | KeyCode::Up => {
            *strategy_sel = strategy_sel.saturating_sub(1);
        }
        KeyCode::Char(' ') => {
            if let Ok(mut w) = live.weather.write() {
                if let Some(cfg) = w.strategy_configs.get_mut(*strategy_sel) {
                    cfg.enabled = !cfg.enabled;
                }
            }
        }
        KeyCode::Char('d') => {
            if let Ok(mut w) = live.weather.write() {
                if let Some(pos) = w.signals.iter().position(|s| s.status == "pending") {
                    w.signals[pos].status = "dismissed".to_string();
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::LiveState;
    use crate::strategies::weather::WeatherSignal;

    fn live_with_signal() -> LiveState {
        let live = LiveState::default();
        if let Ok(mut w) = live.weather.write() {
            w.signals.push(WeatherSignal {
                market_id: "m1".to_string(),
                city: "NYC".to_string(),
                label: "Test".to_string(),
                side: "YES".to_string(),
                edge_pct: 0.07,
                kelly_size: 0.05,
                strategy_id: "forecast_lag".to_string(),
                status: "pending".to_string(),
                created_at: chrono::Utc::now(),
            });
        }
        live
    }

    #[test]
    fn space_toggles_strategy_enabled() {
        let live = LiveState::default(); // has 1 builtin strategy, disabled
        let mut sel = 0usize;
        handle_key(KeyCode::Char(' '), &mut sel, &live);
        let enabled = live
            .weather
            .read()
            .ok()
            .and_then(|w| w.strategy_configs.first().map(|s| s.enabled));
        assert_eq!(enabled, Some(true));
    }

    #[test]
    fn d_dismisses_pending_signal() {
        let live = live_with_signal();
        let mut sel = 0usize;
        handle_key(KeyCode::Char('d'), &mut sel, &live);
        let status = live
            .weather
            .read()
            .ok()
            .and_then(|w| w.signals.first().map(|s| s.status.clone()));
        assert_eq!(status, Some("dismissed".to_string()));
    }

    #[test]
    fn unknown_key_is_noop() {
        let live = LiveState::default();
        let mut sel = 0usize;
        handle_key(KeyCode::F(5), &mut sel, &live);
        // Just verify no panic
    }

    #[test]
    fn d_on_no_pending_signal_is_noop() {
        let live = LiveState::default(); // no signals
        let mut sel = 0usize;
        handle_key(KeyCode::Char('d'), &mut sel, &live);
        // Just verify no panic
    }
}
