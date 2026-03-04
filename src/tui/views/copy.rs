//! Copy tab renderer and content builders.

use crate::copy_trading::Monitor;
use crate::tui::theme::Theme;
use crate::tui::views::header::focused_block;
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    widgets::{Paragraph, Wrap},
    Frame,
};

pub fn render(
    f: &mut Frame,
    area: Rect,
    copy_list_content: &str,
    copy_trades_content: &str,
    scroll0: u16,
    scroll1: u16,
    focused_section: usize,
) {
    let split = RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    f.render_widget(
        Paragraph::new(copy_list_content)
            .style(Theme::body())
            .wrap(Wrap { trim: true })
            .scroll((scroll0, 0))
            .block(focused_block("Copy List", focused_section == 0)),
        split[0],
    );
    f.render_widget(
        Paragraph::new(copy_trades_content)
            .style(Theme::body())
            .wrap(Wrap { trim: true })
            .scroll((scroll1, 0))
            .block(focused_block("Recent Trades", focused_section == 1)),
        split[1],
    );
}

pub fn build_list_content(
    addresses: Vec<String>,
    selected_index: Option<usize>,
    discover_entries: &[crate::discover::LeaderboardEntry],
) -> String {
    if addresses.is_empty() {
        return "No profiles. Add from Discover (a/Enter) or see Shortcuts [?].".to_string();
    }
    addresses
        .iter()
        .enumerate()
        .map(|(i, addr)| {
            let mark = if Some(i) == selected_index {
                "► "
            } else {
                "  "
            };
            let short = addr
                .get(..10)
                .map(|s| format!("{}…", s))
                .unwrap_or_else(|| addr.clone());
            let username = discover_entries
                .iter()
                .find(|e| e.proxy_wallet == *addr)
                .map(|e| e.user_name.as_str())
                .filter(|s| !s.is_empty() && *s != "—");
            match username {
                Some(u) => format!("{}{} ({})", mark, u, short),
                None => format!("{}{}", mark, short),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build_trades_content(monitor: &Monitor) -> String {
    let rows = monitor.recent_trades(20);
    if rows.is_empty() {
        return "No copied trades yet. Start monitor (s).".to_string();
    }
    rows.into_iter()
        .map(|r| {
            let side = r.side.get(..4).map(|s| s.to_string()).unwrap_or(r.side);
            format!(
                "{} {:>8.4} @ {:>5.3} | {} | {}",
                side, r.size, r.price, r.outcome, r.title
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build_status_line(
    copy_running: bool,
    copy_auto_execute: bool,
    sizing_label: &str,
    bankroll: &str,
) -> String {
    format!(
        "Monitor: {} | Auto-execute: {} | Sizing: {} | Bankroll: {}",
        if copy_running { "Running" } else { "Stopped" },
        if copy_auto_execute { "on" } else { "off" },
        sizing_label,
        bankroll,
    )
}
