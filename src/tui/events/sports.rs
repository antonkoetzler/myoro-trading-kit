//! Sports tab key event handler.

use crate::live::LiveState;
use crate::tui::events::SportsUiState;
use crossterm::event::KeyCode;

pub fn handle_key(key: KeyCode, state: &mut SportsUiState, live: &LiveState, scroll: &mut usize) {
    if state.show_league_picker {
        handle_league_picker(key, state, live);
        return;
    }
    if state.show_team_picker {
        handle_team_picker(key, state, live);
        return;
    }
    handle_normal(key, state, live, scroll);
}

fn handle_league_picker(key: KeyCode, state: &mut SportsUiState, live: &LiveState) {
    let n_leagues = live.sports.read().map(|s| s.leagues.len()).unwrap_or(0);
    match key {
        KeyCode::Esc => state.show_league_picker = false,
        KeyCode::Up | KeyCode::Char('k') => {
            state.league_picker_sel = state.league_picker_sel.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.league_picker_sel =
                (state.league_picker_sel + 1).min(n_leagues.saturating_sub(1));
        }
        KeyCode::Enter => {
            let chosen = live.sports.read().ok().and_then(|s| {
                s.leagues
                    .get(state.league_picker_sel)
                    .map(|l| l.short.clone())
            });
            state.league_filter = chosen;
            state.team_filter = None;
            state.show_league_picker = false;
        }
        _ => {}
    }
}

fn handle_team_picker(key: KeyCode, state: &mut SportsUiState, live: &LiveState) {
    match key {
        KeyCode::Esc => state.show_team_picker = false,
        KeyCode::Up | KeyCode::Char('k') => {
            state.team_picker_sel = state.team_picker_sel.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.team_picker_sel += 1;
        }
        KeyCode::Enter => {
            let chosen = live.sports.read().ok().and_then(|s| {
                let teams: Vec<String> = if let Some(lf) = state.league_filter.as_deref() {
                    s.leagues
                        .iter()
                        .find(|l| l.short == lf)
                        .map(|l| l.teams.clone())
                        .unwrap_or_default()
                } else {
                    let mut t: Vec<String> = s
                        .leagues
                        .iter()
                        .flat_map(|l| l.teams.iter().cloned())
                        .collect();
                    t.sort();
                    t.dedup();
                    t
                };
                teams.into_iter().nth(state.team_picker_sel)
            });
            state.team_filter = chosen;
            state.show_team_picker = false;
        }
        _ => {}
    }
}

pub(crate) fn handle_normal(
    key: KeyCode,
    state: &mut SportsUiState,
    live: &LiveState,
    scroll: &mut usize,
) {
    match key {
        KeyCode::Char(' ') if state.pane == 0 => {
            let id = live
                .sports
                .read()
                .ok()
                .and_then(|s| s.strategy_configs.get(state.strategy_sel).map(|c| c.id));
            if let Some(id) = id {
                if let Ok(mut s) = live.sports.write() {
                    if let Some(cfg) = s.strategy_configs.iter_mut().find(|c| c.id == id) {
                        cfg.enabled = !cfg.enabled;
                    }
                }
            }
        }
        KeyCode::Enter if state.pane == 1 => {
            if let Ok(mut s) = live.sports.write() {
                if let Some(sig) = s.signals.get_mut(state.signal_sel) {
                    if sig.status == "pending" {
                        sig.status = "done".to_string();
                        live.push_sports_log(
                            crate::live::LogLevel::Success,
                            format!("Executed {} {} on {}", sig.side, sig.strategy_id, sig.home),
                        );
                    }
                }
            }
        }
        KeyCode::Char('d') if state.pane == 1 => {
            if let Ok(mut s) = live.sports.write() {
                if let Some(sig) = s.signals.get_mut(state.signal_sel) {
                    if sig.status == "pending" {
                        sig.status = "dismissed".to_string();
                    }
                }
            }
        }
        KeyCode::Char('L') if state.pane == 2 => {
            state.show_league_picker = true;
            state.league_picker_sel = 0;
        }
        KeyCode::Char('T') if state.pane == 2 => {
            state.show_team_picker = true;
            state.team_picker_sel = 0;
        }
        KeyCode::Char('r') => {
            *scroll = 0;
            state.fixture_sel = 0;
            state.signal_sel = 0;
            live.push_sports_log(
                crate::live::LogLevel::Info,
                "Manual refresh triggered".into(),
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::{sports::StoredSignal, LiveState};

    fn live_with_signal() -> LiveState {
        let live = LiveState::default();
        if let Ok(mut s) = live.sports.write() {
            s.signals.push(StoredSignal {
                market_id: "m1".to_string(),
                home: "Home FC".to_string(),
                away: "Away FC".to_string(),
                date: "2026-03-01".to_string(),
                side: "YES".to_string(),
                edge_pct: 0.06,
                kelly_size: 0.1,
                strategy_id: "poisson".to_string(),
                status: "pending".to_string(),
                created_at: chrono::Utc::now(),
            });
        }
        live
    }

    #[test]
    fn space_on_pane_0_toggles_strategy_enabled() {
        let live = LiveState::default(); // 5 built-in strategies
        let mut ui = SportsUiState {
            pane: 0,
            strategy_sel: 0,
            ..Default::default()
        };
        let mut scroll = 0usize;
        handle_normal(KeyCode::Char(' '), &mut ui, &live, &mut scroll);
        let enabled = live
            .sports
            .read()
            .ok()
            .and_then(|s| s.strategy_configs.first().map(|c| c.enabled));
        assert_eq!(enabled, Some(true));
    }

    #[test]
    fn enter_on_pane_1_marks_signal_done() {
        let live = live_with_signal();
        let mut ui = SportsUiState {
            pane: 1,
            signal_sel: 0,
            ..Default::default()
        };
        let mut scroll = 0usize;
        handle_normal(KeyCode::Enter, &mut ui, &live, &mut scroll);
        let status = live
            .sports
            .read()
            .ok()
            .and_then(|s| s.signals.first().map(|sg| sg.status.clone()));
        assert_eq!(status, Some("done".to_string()));
    }

    #[test]
    fn d_on_pane_1_dismisses_signal() {
        let live = live_with_signal();
        let mut ui = SportsUiState {
            pane: 1,
            signal_sel: 0,
            ..Default::default()
        };
        let mut scroll = 0usize;
        handle_normal(KeyCode::Char('d'), &mut ui, &live, &mut scroll);
        let status = live
            .sports
            .read()
            .ok()
            .and_then(|s| s.signals.first().map(|sg| sg.status.clone()));
        assert_eq!(status, Some("dismissed".to_string()));
    }

    #[test]
    fn r_resets_scroll_and_pushes_log() {
        let live = LiveState::default();
        let mut ui = SportsUiState::default();
        let mut scroll = 10usize;
        handle_normal(KeyCode::Char('r'), &mut ui, &live, &mut scroll);
        assert_eq!(scroll, 0);
        let logs = live.get_sports_logs();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].1.contains("refresh"));
    }

    #[test]
    fn big_l_on_pane_2_opens_league_picker() {
        let live = LiveState::default();
        let mut ui = SportsUiState {
            pane: 2,
            ..Default::default()
        };
        let mut scroll = 0usize;
        handle_normal(KeyCode::Char('L'), &mut ui, &live, &mut scroll);
        assert!(ui.show_league_picker);
    }

    #[test]
    fn big_t_on_pane_2_opens_team_picker() {
        let live = LiveState::default();
        let mut ui = SportsUiState {
            pane: 2,
            ..Default::default()
        };
        let mut scroll = 0usize;
        handle_normal(KeyCode::Char('T'), &mut ui, &live, &mut scroll);
        assert!(ui.show_team_picker);
    }

    #[test]
    fn league_picker_esc_closes() {
        let live = LiveState::default();
        let mut ui = SportsUiState {
            show_league_picker: true,
            ..Default::default()
        };
        handle_league_picker(KeyCode::Esc, &mut ui, &live);
        assert!(!ui.show_league_picker);
    }

    #[test]
    fn team_picker_esc_closes() {
        let live = LiveState::default();
        let mut ui = SportsUiState {
            show_team_picker: true,
            ..Default::default()
        };
        handle_team_picker(KeyCode::Esc, &mut ui, &live);
        assert!(!ui.show_team_picker);
    }

    #[test]
    fn league_picker_up_down_navigate() {
        let live = LiveState::default();
        let mut ui = SportsUiState {
            show_league_picker: true,
            league_picker_sel: 1,
            ..Default::default()
        };
        handle_league_picker(KeyCode::Down, &mut ui, &live);
        let after_down = ui.league_picker_sel;
        handle_league_picker(KeyCode::Up, &mut ui, &live);
        assert!(ui.league_picker_sel < after_down || ui.league_picker_sel == 0);
    }
}
