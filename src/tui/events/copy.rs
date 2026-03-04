//! Copy tab key event handler + add-trader dialog helpers.

use crate::copy_trading::TraderList;
use crate::discover::LeaderboardEntry;
use crate::live::LiveState;
use crossterm::event::KeyCode;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// ── CopyAddOption ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum CopyAddOption {
    PasteAddress(String),
    Profile { addr: String, name: String },
}

impl CopyAddOption {
    pub fn address(&self) -> &str {
        match self {
            CopyAddOption::PasteAddress(a) => a.as_str(),
            CopyAddOption::Profile { addr, .. } => addr.as_str(),
        }
    }

    pub fn display_line(&self) -> String {
        match self {
            CopyAddOption::PasteAddress(a) => {
                let short = a
                    .get(..14)
                    .map(|s| format!("{}…", s))
                    .unwrap_or_else(|| a.clone());
                format!("Add pasted address {}", short)
            }
            CopyAddOption::Profile { addr, name } => {
                let label = if name.is_empty() || name == "—" {
                    addr.get(..14)
                        .map(|s| format!("{}…", s))
                        .unwrap_or_else(|| addr.clone())
                } else {
                    name.clone()
                };
                format!("{} ({})", label, addr.get(..10).unwrap_or(addr))
            }
        }
    }
}

fn looks_like_address(s: &str) -> bool {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("0x") {
        rest.len() == 40 && rest.chars().all(|c| c.is_ascii_hexdigit())
    } else {
        s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
    }
}

fn normalize_address(s: &str) -> String {
    let s = s.trim();
    if s.starts_with("0x") {
        s.to_string()
    } else {
        format!("0x{}", s)
    }
}

pub fn build_add_options(search: &str, entries: &[LeaderboardEntry]) -> Vec<CopyAddOption> {
    let q = search.trim();
    if q.len() >= 40 && looks_like_address(q) {
        return vec![CopyAddOption::PasteAddress(normalize_address(q))];
    }
    if q.is_empty() {
        return entries
            .iter()
            .take(25)
            .map(|e| CopyAddOption::Profile {
                addr: e.proxy_wallet.clone(),
                name: e.user_name.clone(),
            })
            .collect();
    }
    let ql = q.to_lowercase();
    entries
        .iter()
        .filter(|e| {
            e.user_name.to_lowercase().contains(&ql) || e.proxy_wallet.to_lowercase().contains(&ql)
        })
        .take(25)
        .map(|e| CopyAddOption::Profile {
            addr: e.proxy_wallet.clone(),
            name: e.user_name.clone(),
        })
        .collect()
}

/// Handle keys while the add-trader dialog is open. Returns true if consumed.
pub fn handle_add_dialog_key(
    key: KeyCode,
    copy_add_dialog: &mut Option<(String, usize)>,
    entries: &[LeaderboardEntry],
    trader_list: &Arc<TraderList>,
    live: &LiveState,
) -> bool {
    let (ref mut search, ref mut sel_idx) = match copy_add_dialog {
        Some(d) => d,
        None => return false,
    };
    match key {
        KeyCode::Esc => *copy_add_dialog = None,
        KeyCode::Backspace => {
            search.pop();
            *sel_idx = 0;
        }
        KeyCode::Up => {
            let n = build_add_options(search, entries).len();
            if n > 0 {
                *sel_idx = sel_idx.saturating_sub(1).min(n - 1);
            }
        }
        KeyCode::Down => {
            let n = build_add_options(search, entries).len();
            if n > 0 {
                *sel_idx = (*sel_idx + 1).min(n - 1);
            }
        }
        KeyCode::Enter => {
            let opts = build_add_options(search, entries);
            let si = (*sel_idx).min(opts.len().saturating_sub(1));
            if let Some(opt) = opts.get(si) {
                if trader_list.add(opt.address().to_string()) {
                    live.push_copy_log(
                        crate::live::LogLevel::Success,
                        format!("Added {} to copy list", opt.address()),
                    );
                }
            }
            *copy_add_dialog = None;
        }
        KeyCode::Char(c) => {
            search.push(c);
            *sel_idx = 0;
        }
        _ => {}
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::LeaderboardEntry;

    fn entry(addr: &str, name: &str) -> LeaderboardEntry {
        LeaderboardEntry {
            rank: "1".into(),
            proxy_wallet: addr.into(),
            user_name: name.into(),
            vol: 0.0,
            pnl: 0.0,
        }
    }

    #[test]
    fn looks_like_address_valid_hex40() {
        assert!(looks_like_address(
            "0xabcdef1234567890abcdef1234567890abcdef12"
        ));
        assert!(looks_like_address(
            "abcdef1234567890abcdef1234567890abcdef12"
        ));
    }

    #[test]
    fn looks_like_address_invalid() {
        assert!(!looks_like_address("0xabc")); // too short
        assert!(!looks_like_address("not-an-address"));
        assert!(!looks_like_address("")); // empty
        assert!(!looks_like_address(
            "0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG"
        )); // non-hex
    }

    #[test]
    fn normalize_address_prepends_0x_when_missing() {
        let raw = "abcdef1234567890abcdef1234567890abcdef12";
        assert_eq!(normalize_address(raw), format!("0x{}", raw));
    }

    #[test]
    fn normalize_address_leaves_0x_intact() {
        let addr = "0xabcdef1234567890abcdef1234567890abcdef12";
        assert_eq!(normalize_address(addr), addr);
    }

    #[test]
    fn build_add_options_paste_on_valid_address() {
        let addr = "0xabcdef1234567890abcdef1234567890abcdef12";
        let opts = build_add_options(addr, &[]);
        assert_eq!(opts.len(), 1);
        assert!(matches!(&opts[0], CopyAddOption::PasteAddress(a) if a == addr));
    }

    #[test]
    fn build_add_options_empty_query_returns_entries() {
        let entries = vec![entry("0xaaa", "Alice"), entry("0xbbb", "Bob")];
        let opts = build_add_options("", &entries);
        assert_eq!(opts.len(), 2);
        assert!(matches!(&opts[0], CopyAddOption::Profile { name, .. } if name == "Alice"));
    }

    #[test]
    fn build_add_options_filters_by_name() {
        let entries = vec![
            entry("0xaaa", "Alice"),
            entry("0xbbb", "Bob"),
            entry("0xccc", "Charlie"),
        ];
        let opts = build_add_options("ali", &entries);
        assert_eq!(opts.len(), 1);
        assert!(matches!(&opts[0], CopyAddOption::Profile { name, .. } if name == "Alice"));
    }

    #[test]
    fn copy_add_option_address_extracts_correctly() {
        let paste = CopyAddOption::PasteAddress("0xabc".into());
        assert_eq!(paste.address(), "0xabc");
        let profile = CopyAddOption::Profile {
            addr: "0xdef".into(),
            name: "Bob".into(),
        };
        assert_eq!(profile.address(), "0xdef");
    }

    // ── handle_add_dialog_key tests ───────────────────────────────────────────

    use crate::config::Config;
    use crate::copy_trading::TraderList;
    use crate::live::LiveState;
    use std::sync::{Arc, RwLock};

    fn make_trader_list() -> Arc<TraderList> {
        let cfg = Arc::new(RwLock::new(Config::default()));
        Arc::new(TraderList::new(cfg))
    }

    #[test]
    fn add_dialog_esc_closes() {
        let list = make_trader_list();
        let live = LiveState::default();
        let mut dialog: Option<(String, usize)> = Some((String::new(), 0));
        handle_add_dialog_key(KeyCode::Esc, &mut dialog, &[], &list, &live);
        assert!(dialog.is_none());
    }

    #[test]
    fn add_dialog_char_appends_to_search() {
        let list = make_trader_list();
        let live = LiveState::default();
        let mut dialog: Option<(String, usize)> = Some((String::new(), 0));
        handle_add_dialog_key(KeyCode::Char('a'), &mut dialog, &[], &list, &live);
        assert_eq!(dialog.as_ref().unwrap().0, "a");
    }

    #[test]
    fn add_dialog_backspace_pops_search() {
        let list = make_trader_list();
        let live = LiveState::default();
        let mut dialog: Option<(String, usize)> = Some(("ab".to_string(), 0));
        handle_add_dialog_key(KeyCode::Backspace, &mut dialog, &[], &list, &live);
        assert_eq!(dialog.as_ref().unwrap().0, "a");
    }

    #[test]
    fn add_dialog_down_increments_sel() {
        let list = make_trader_list();
        let live = LiveState::default();
        let entries = vec![
            entry("0xaaa", "Alice"),
            entry("0xbbb", "Bob"),
            entry("0xccc", "Carol"),
        ];
        let mut dialog: Option<(String, usize)> = Some((String::new(), 0));
        handle_add_dialog_key(KeyCode::Down, &mut dialog, &entries, &list, &live);
        assert_eq!(dialog.as_ref().unwrap().1, 1);
    }

    #[test]
    fn add_dialog_up_saturates_at_zero() {
        let list = make_trader_list();
        let live = LiveState::default();
        let entries = vec![entry("0xaaa", "Alice"), entry("0xbbb", "Bob")];
        let mut dialog: Option<(String, usize)> = Some((String::new(), 0));
        handle_add_dialog_key(KeyCode::Up, &mut dialog, &entries, &list, &live);
        assert_eq!(dialog.as_ref().unwrap().1, 0);
    }

    #[test]
    fn add_dialog_returns_false_when_none() {
        let list = make_trader_list();
        let live = LiveState::default();
        let mut dialog: Option<(String, usize)> = None;
        let consumed = handle_add_dialog_key(KeyCode::Char('x'), &mut dialog, &[], &list, &live);
        assert!(!consumed);
    }
}

// ── Tab key handler ───────────────────────────────────────────────────────────

pub fn handle_key(
    key: KeyCode,
    copy_running: &AtomicBool,
    copy_selected: &mut Option<usize>,
    copy_add_dialog: &mut Option<(String, usize)>,
    trader_list: &Arc<TraderList>,
    n_addr: usize,
    live: &LiveState,
) {
    match key {
        KeyCode::Char('s') => {
            let v = copy_running.load(Ordering::SeqCst);
            copy_running.store(!v, Ordering::SeqCst);
        }
        KeyCode::Char('a') | KeyCode::Enter => {
            *copy_add_dialog = Some((String::new(), 0));
        }
        KeyCode::Char('d') => {
            let idx = copy_selected.unwrap_or(0);
            if idx < n_addr {
                trader_list.remove_at(idx);
                live.push_copy_log(
                    crate::live::LogLevel::Info,
                    format!("Removed trader at index {}", idx),
                );
                *copy_selected = if n_addr <= 1 {
                    None
                } else {
                    Some(idx.saturating_sub(1).min(n_addr.saturating_sub(2)))
                };
            }
        }
        _ => {}
    }
}
