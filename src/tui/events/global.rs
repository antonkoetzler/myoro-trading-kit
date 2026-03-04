//! Global key event handlers: theme overlay, bankroll input, currency picker, shortcuts.

use crate::config::ExecutionMode;
use crate::live::LiveState;
use crate::tui::theme::{
    self as theme_mod, add_custom_theme, export_current_theme, import_theme, set_theme_index,
    theme_count, theme_name_at, ThemePalette, COLOR_PRESETS, THEME_CREATOR_ROLES,
};
use crossterm::event::KeyCode;
use std::sync::Arc;

const CURRENCIES: &[&str] = &["USD", "EUR", "GBP", "BTC", "ETH"];

/// Handle keys when in the bankroll input prompt. Returns true if consumed.
pub fn handle_bankroll_input(
    key: KeyCode,
    s: &mut String,
    live: &LiveState,
    config: &Arc<std::sync::RwLock<crate::config::Config>>,
    bankroll_input: &mut Option<String>,
) -> bool {
    match key {
        KeyCode::Esc => *bankroll_input = None,
        KeyCode::Enter => {
            if let Ok(v) = s.trim().parse::<f64>() {
                if v >= 0.0 {
                    live.set_bankroll(Some(v));
                    if let Ok(mut c) = config.write() {
                        c.paper_bankroll = Some(v);
                        let _ = crate::config::save_config(&c);
                    }
                }
            }
            *bankroll_input = None;
        }
        KeyCode::Backspace => {
            s.pop();
        }
        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
            if c == '.' && s.contains('.') {
                // ignore second decimal point
            } else {
                s.push(c);
            }
        }
        _ => {}
    }
    true
}

/// Handle keys inside the currency picker. Returns true if consumed.
pub fn handle_currency_picker(
    key: KeyCode,
    currency_filter: &mut String,
    currency_selected: &mut usize,
    show_currency_picker: &mut bool,
    config: &Arc<std::sync::RwLock<crate::config::Config>>,
) -> bool {
    match key {
        KeyCode::Esc => {
            *show_currency_picker = false;
            currency_filter.clear();
        }
        KeyCode::Enter => {
            let filtered: Vec<&str> = CURRENCIES
                .iter()
                .filter(|c| {
                    currency_filter.is_empty()
                        || c.to_lowercase().contains(&currency_filter.to_lowercase())
                })
                .copied()
                .collect();
            if let Some(&cur) = filtered.get(*currency_selected) {
                if let Ok(mut c) = config.write() {
                    c.pnl_currency = cur.to_string();
                    let _ = crate::config::save_config(&c);
                }
            }
            *show_currency_picker = false;
            currency_filter.clear();
        }
        KeyCode::Backspace => {
            currency_filter.pop();
            *currency_selected = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let filtered_len = CURRENCIES
                .iter()
                .filter(|x| {
                    currency_filter.is_empty()
                        || x.to_lowercase().contains(&currency_filter.to_lowercase())
                })
                .count();
            *currency_selected = currency_selected
                .saturating_sub(1)
                .min(filtered_len.saturating_sub(1));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let filtered: Vec<&str> = CURRENCIES
                .iter()
                .filter(|x| {
                    currency_filter.is_empty()
                        || x.to_lowercase().contains(&currency_filter.to_lowercase())
                })
                .copied()
                .collect();
            *currency_selected = (*currency_selected + 1).min(filtered.len().saturating_sub(1));
        }
        KeyCode::Char(c) => {
            currency_filter.push(c);
            *currency_selected = 0;
        }
        _ => {}
    }
    true
}

/// Handle keys inside the theme overlay. Returns true if consumed, false to break (shouldn't happen).
pub fn handle_theme_overlay(
    key: KeyCode,
    show_theme_overlay: &mut bool,
    theme_overlay_selection: &mut usize,
    theme_in_creator: &mut bool,
    theme_creator_role: &mut usize,
    theme_creator_color_idx: &mut usize,
    theme_editor_palette: &mut Option<ThemePalette>,
) -> bool {
    let n = theme_count();
    if *theme_in_creator {
        match key {
            KeyCode::Esc => {
                *theme_in_creator = false;
                *theme_editor_palette = None;
            }
            KeyCode::Char('s') => {
                if let Some(mut p) = theme_editor_palette.take() {
                    p.name = format!("Custom {}", n);
                    let idx = add_custom_theme(p);
                    set_theme_index(idx);
                    *theme_in_creator = false;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                *theme_creator_role =
                    (*theme_creator_role + 1).min(THEME_CREATOR_ROLES.len().saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                *theme_creator_role = theme_creator_role.saturating_sub(1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                *theme_creator_color_idx =
                    (*theme_creator_color_idx + 1).min(COLOR_PRESETS.len().saturating_sub(1));
                if let Some(ref mut p) = theme_editor_palette {
                    p.set_role_color(*theme_creator_role, COLOR_PRESETS[*theme_creator_color_idx]);
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                *theme_creator_color_idx = theme_creator_color_idx.saturating_sub(1);
                if let Some(ref mut p) = theme_editor_palette {
                    p.set_role_color(*theme_creator_role, COLOR_PRESETS[*theme_creator_color_idx]);
                }
            }
            _ => {}
        }
    } else {
        let total_items = n + 3;
        match key {
            KeyCode::Char('T') | KeyCode::F(10) | KeyCode::Esc => {
                *show_theme_overlay = false;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                *theme_overlay_selection =
                    (*theme_overlay_selection + 1).min(total_items.saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                *theme_overlay_selection = theme_overlay_selection.saturating_sub(1);
            }
            KeyCode::Enter => {
                if *theme_overlay_selection < n {
                    set_theme_index(*theme_overlay_selection);
                } else if *theme_overlay_selection == n {
                    // Export option (n+0)
                } else if *theme_overlay_selection == n + 2 {
                    *theme_in_creator = true;
                    *theme_editor_palette = Some(theme_mod::current_palette());
                    *theme_creator_role = 0;
                    *theme_creator_color_idx = 0;
                }
            }
            KeyCode::Char('e') => {
                let path = std::path::Path::new("theme_export.toml");
                let _ = export_current_theme(path);
            }
            KeyCode::Char('i') => {
                let path = std::path::Path::new("theme_import.toml");
                if let Ok(idx) = import_theme(path) {
                    set_theme_index(idx);
                }
            }
            KeyCode::Char('c') => {
                *theme_in_creator = true;
                *theme_editor_palette = Some(theme_mod::current_palette());
                *theme_creator_role = 0;
                *theme_creator_color_idx = 0;
            }
            _ => {}
        }
    }
    true
}

/// Returns the theme names list for rendering.
pub fn theme_names_list() -> Vec<String> {
    let n = theme_count();
    (0..n).map(theme_name_at).collect()
}

/// Handle live-mode confirm dialog (y/n). Returns true if consumed.
pub fn handle_live_confirm(
    key: KeyCode,
    live_confirm_tab: &mut Option<usize>,
    tab_execution_mode: &mut [ExecutionMode],
) -> bool {
    match key {
        KeyCode::Char('y') => {
            if let Some(tab) = *live_confirm_tab {
                tab_execution_mode[tab] = ExecutionMode::Live;
                *live_confirm_tab = None;
            }
        }
        KeyCode::Char('n') | KeyCode::Esc => *live_confirm_tab = None,
        _ => {}
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ExecutionMode};
    use crate::live::LiveState;
    use std::sync::{Arc, RwLock};

    fn make_config() -> Arc<RwLock<Config>> {
        Arc::new(RwLock::new(Config::default()))
    }

    // ── handle_bankroll_input ─────────────────────────────────────────────────

    #[test]
    fn bankroll_digit_appends_to_string() {
        let live = LiveState::default();
        let cfg = make_config();
        let mut s = String::new();
        let mut opt: Option<String> = Some(s.clone());
        handle_bankroll_input(KeyCode::Char('5'), &mut s, &live, &cfg, &mut opt);
        assert_eq!(s, "5");
        assert!(opt.is_some(), "option stays Some on digit input");
    }

    #[test]
    fn bankroll_backspace_pops_char() {
        let live = LiveState::default();
        let cfg = make_config();
        let mut s = "42".to_string();
        let mut opt: Option<String> = Some(s.clone());
        handle_bankroll_input(KeyCode::Backspace, &mut s, &live, &cfg, &mut opt);
        assert_eq!(s, "4");
    }

    #[test]
    fn bankroll_second_decimal_is_ignored() {
        let live = LiveState::default();
        let cfg = make_config();
        let mut s = "1.2".to_string();
        let mut opt: Option<String> = Some(s.clone());
        handle_bankroll_input(KeyCode::Char('.'), &mut s, &live, &cfg, &mut opt);
        assert_eq!(s, "1.2", "second decimal must not be appended");
    }

    #[test]
    fn bankroll_esc_sets_option_to_none() {
        let live = LiveState::default();
        let cfg = make_config();
        let mut s = "100".to_string();
        let mut opt: Option<String> = Some(s.clone());
        handle_bankroll_input(KeyCode::Esc, &mut s, &live, &cfg, &mut opt);
        assert!(opt.is_none());
    }

    #[test]
    fn bankroll_enter_with_valid_value_stores_bankroll() {
        let live = LiveState::default();
        let cfg = make_config();
        let mut s = "500".to_string();
        let mut opt: Option<String> = Some(s.clone());
        handle_bankroll_input(KeyCode::Enter, &mut s, &live, &cfg, &mut opt);
        assert!(opt.is_none(), "Enter clears the prompt");
        let stored = live.global_stats.read().ok().and_then(|g| g.bankroll);
        assert_eq!(stored, Some(500.0));
    }

    #[test]
    fn bankroll_enter_with_invalid_value_does_not_panic() {
        let live = LiveState::default();
        let cfg = make_config();
        let mut s = "not_a_number".to_string();
        let mut opt: Option<String> = Some(s.clone());
        handle_bankroll_input(KeyCode::Enter, &mut s, &live, &cfg, &mut opt);
        assert!(opt.is_none(), "Enter always clears prompt");
    }

    // ── handle_live_confirm ───────────────────────────────────────────────────

    #[test]
    fn live_confirm_y_sets_live_mode() {
        let mut modes = [ExecutionMode::Paper; 7];
        let mut tab: Option<usize> = Some(1);
        let consumed = handle_live_confirm(KeyCode::Char('y'), &mut tab, &mut modes);
        assert!(consumed);
        assert!(tab.is_none());
        assert_eq!(modes[1], ExecutionMode::Live);
    }

    #[test]
    fn live_confirm_n_cancels_without_mode_change() {
        let mut modes = [ExecutionMode::Paper; 7];
        let mut tab: Option<usize> = Some(2);
        handle_live_confirm(KeyCode::Char('n'), &mut tab, &mut modes);
        assert!(tab.is_none());
        assert_eq!(modes[2], ExecutionMode::Paper);
    }

    #[test]
    fn live_confirm_always_returns_true() {
        let mut modes = [ExecutionMode::Paper; 7];
        let mut tab: Option<usize> = Some(0);
        assert!(handle_live_confirm(
            KeyCode::Char('x'),
            &mut tab,
            &mut modes
        ));
    }

    // ── handle_currency_picker ────────────────────────────────────────────────

    #[test]
    fn currency_picker_esc_closes() {
        let cfg = make_config();
        let mut filter = String::new();
        let mut sel = 0usize;
        let mut open = true;
        handle_currency_picker(KeyCode::Esc, &mut filter, &mut sel, &mut open, &cfg);
        assert!(!open);
    }

    #[test]
    fn currency_picker_char_appends_to_filter() {
        let cfg = make_config();
        let mut filter = String::new();
        let mut sel = 2usize;
        let mut open = true;
        handle_currency_picker(KeyCode::Char('U'), &mut filter, &mut sel, &mut open, &cfg);
        assert_eq!(filter, "U");
        assert_eq!(sel, 0, "selection resets on filter change");
    }

    #[test]
    fn currency_picker_down_increments_selection() {
        let cfg = make_config();
        let mut filter = String::new();
        let mut sel = 0usize;
        let mut open = true;
        handle_currency_picker(KeyCode::Down, &mut filter, &mut sel, &mut open, &cfg);
        assert_eq!(sel, 1);
    }

    #[test]
    fn currency_picker_up_saturates_at_zero() {
        let cfg = make_config();
        let mut filter = String::new();
        let mut sel = 0usize;
        let mut open = true;
        handle_currency_picker(KeyCode::Up, &mut filter, &mut sel, &mut open, &cfg);
        assert_eq!(sel, 0);
    }
}
