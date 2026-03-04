//! Discover tab key event handler + filter dialog state.

use crate::copy_trading::TraderList;
use crate::discover::DiscoverState;
use crate::live::LiveState;
use crossterm::event::KeyCode;
use std::sync::Arc;

// ── Filter dialog ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum DiscoverFilterDialog {
    Category(usize),
    Period(usize),
    Order(usize),
}

impl DiscoverFilterDialog {
    pub fn options_len(self) -> usize {
        match self {
            DiscoverFilterDialog::Category(_) => 9,
            DiscoverFilterDialog::Period(_) => 4,
            DiscoverFilterDialog::Order(_) => 2,
        }
    }

    pub fn next(self) -> Self {
        let len = self.options_len();
        match self {
            DiscoverFilterDialog::Category(i) => DiscoverFilterDialog::Category((i + 1) % len),
            DiscoverFilterDialog::Period(i) => DiscoverFilterDialog::Period((i + 1) % len),
            DiscoverFilterDialog::Order(i) => DiscoverFilterDialog::Order((i + 1) % len),
        }
    }

    pub fn prev(self) -> Self {
        let len = self.options_len();
        match self {
            DiscoverFilterDialog::Category(i) => {
                DiscoverFilterDialog::Category((i + len - 1) % len)
            }
            DiscoverFilterDialog::Period(i) => DiscoverFilterDialog::Period((i + len - 1) % len),
            DiscoverFilterDialog::Order(i) => DiscoverFilterDialog::Order((i + 1) % 2),
        }
    }

    pub fn index(self) -> usize {
        match self {
            DiscoverFilterDialog::Category(i)
            | DiscoverFilterDialog::Period(i)
            | DiscoverFilterDialog::Order(i) => i,
        }
    }
}

/// Handle keys when the filter dialog is open. Returns true if the key was consumed.
pub fn handle_filter_dialog_key(
    key: KeyCode,
    dialog: &mut Option<DiscoverFilterDialog>,
    discover: &DiscoverState,
) -> bool {
    let d = match *dialog {
        Some(d) => d,
        None => return false,
    };
    match key {
        KeyCode::Esc => *dialog = None,
        KeyCode::Enter => {
            match d {
                DiscoverFilterDialog::Category(i) => discover.set_category_by_index(i),
                DiscoverFilterDialog::Period(i) => discover.set_time_period_by_index(i),
                DiscoverFilterDialog::Order(i) => discover.set_order_by_index(i),
            }
            *dialog = None;
        }
        KeyCode::Up | KeyCode::Char('k') => *dialog = Some(d.prev()),
        KeyCode::Down | KeyCode::Char('j') => *dialog = Some(d.next()),
        _ => {}
    }
    true
}

// ── Tab key handler ───────────────────────────────────────────────────────────

pub fn handle_key(
    key: KeyCode,
    discover: &Arc<DiscoverState>,
    discover_selected: Option<usize>,
    trader_list: &Arc<TraderList>,
    discover_entries: &[crate::discover::LeaderboardEntry],
    live: &LiveState,
    filter_dialog: &mut Option<DiscoverFilterDialog>,
) {
    match key {
        KeyCode::Char('s') => {
            discover.toggle_screener_mode();
            if discover.is_screener_mode() {
                let d = Arc::clone(discover);
                std::thread::spawn(move || d.fetch_screener());
            }
        }
        KeyCode::Char('r') => {
            let d = Arc::clone(discover);
            if discover.is_screener_mode() {
                std::thread::spawn(move || d.fetch_screener());
            } else {
                std::thread::spawn(move || d.fetch());
            }
        }
        KeyCode::Char('c') => {
            *filter_dialog = Some(DiscoverFilterDialog::Category(discover.category_index()));
        }
        KeyCode::Char('t') => {
            *filter_dialog = Some(DiscoverFilterDialog::Period(discover.time_period_index()));
        }
        KeyCode::Char('o') => {
            *filter_dialog = Some(DiscoverFilterDialog::Order(discover.order_by_index()));
        }
        KeyCode::Char('a') | KeyCode::Enter => {
            if let Some(i) = discover_selected {
                if let Some(e) = discover_entries.get(i) {
                    let addrs = trader_list.get_addresses();
                    if let Some(pos) = addrs.iter().position(|a| a == &e.proxy_wallet) {
                        trader_list.remove_at(pos);
                        live.push_copy_log(
                            crate::live::LogLevel::Info,
                            format!("Removed {} from copy list", e.proxy_wallet),
                        );
                    } else if trader_list.add(e.proxy_wallet.clone()) {
                        live.push_copy_log(
                            crate::live::LogLevel::Success,
                            format!("Added {} to copy list", e.proxy_wallet),
                        );
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::DiscoverState;

    #[test]
    fn filter_dialog_key_returns_false_when_none() {
        let discover = DiscoverState::new();
        let mut dialog: Option<DiscoverFilterDialog> = None;
        let consumed = handle_filter_dialog_key(KeyCode::Up, &mut dialog, &discover);
        assert!(!consumed);
    }

    #[test]
    fn filter_dialog_esc_closes() {
        let discover = DiscoverState::new();
        let mut dialog = Some(DiscoverFilterDialog::Category(2));
        handle_filter_dialog_key(KeyCode::Esc, &mut dialog, &discover);
        assert!(dialog.is_none());
    }

    #[test]
    fn filter_dialog_down_increments_category_index() {
        let discover = DiscoverState::new();
        let mut dialog = Some(DiscoverFilterDialog::Category(0));
        handle_filter_dialog_key(KeyCode::Down, &mut dialog, &discover);
        assert_eq!(dialog.unwrap().index(), 1);
    }

    #[test]
    fn filter_dialog_up_on_category_wraps() {
        let discover = DiscoverState::new();
        let mut dialog = Some(DiscoverFilterDialog::Category(0));
        handle_filter_dialog_key(KeyCode::Up, &mut dialog, &discover);
        assert_eq!(dialog.unwrap().index(), 8); // (0 + 9 - 1) % 9
    }

    #[test]
    fn filter_dialog_enter_applies_and_closes() {
        let discover = DiscoverState::new();
        let mut dialog = Some(DiscoverFilterDialog::Category(1));
        handle_filter_dialog_key(KeyCode::Enter, &mut dialog, &discover);
        assert!(dialog.is_none());
        assert_eq!(discover.category_label(), "CRYPTO");
    }

    #[test]
    fn period_dialog_wraps_on_down() {
        let discover = DiscoverState::new();
        let mut dialog = Some(DiscoverFilterDialog::Period(0));
        for _ in 0..4 {
            handle_filter_dialog_key(KeyCode::Down, &mut dialog, &discover);
        }
        assert_eq!(dialog.unwrap().index(), 0); // wraps around
    }
}
