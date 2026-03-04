//! Header, metrics bar, and tab strip renderers.

use crate::live::LiveState;
use crate::tui::layout::TABS;
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

pub fn render_title(f: &mut Frame, area: Rect) {
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::block_border())
        .style(Style::default().bg(Theme::BG()));
    let inner = header_block.inner(area);
    f.render_widget(header_block, area);
    let title_center = " Myoro Polymarket Terminal ";
    let w = inner.width;
    let n = title_center.len() as u16;
    let left = (w.saturating_sub(n)) / 2;
    let right = w.saturating_sub(n).saturating_sub(left);
    let line_str = format!(
        "{}{}{}",
        "─".repeat(left as usize),
        title_center,
        "─".repeat(right as usize)
    );
    let header_para = Paragraph::new(Line::from(Span::styled(line_str, Theme::block_title())));
    f.render_widget(header_para, inner);
}

pub fn render_metrics(
    f: &mut Frame,
    area: Rect,
    live: &LiveState,
    pnl_currency: &str,
    tab_mode: Option<&str>,
    copy_status: Option<&str>,
    copy_status_style: Option<Style>,
) {
    let stats = live.global_stats.read().ok();
    let pnl_prefix = match pnl_currency.to_uppercase().as_str() {
        "USD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "BTC" => "BTC ",
        "ETH" => "ETH ",
        _ => "$",
    };
    let bankroll = stats
        .as_ref()
        .and_then(|s| s.bankroll)
        .map(|b| format!("{}{:.2}", pnl_prefix, b))
        .unwrap_or_else(|| "—".to_string());
    let pnl = stats.as_ref().map(|s| s.pnl).unwrap_or(0.0);
    let open_t = stats.as_ref().map(|s| s.open_trades).unwrap_or(0);
    let closed_t = stats.as_ref().map(|s| s.closed_trades).unwrap_or(0);
    let pnl_str = format!("{}{:.2}", pnl_prefix, pnl);
    let pnl_style = if pnl > 0.0 {
        Theme::success()
    } else if pnl < 0.0 {
        Theme::danger()
    } else {
        Theme::neutral_pnl()
    };
    let mode_str = tab_mode.unwrap_or("—");
    let copy_str = copy_status.unwrap_or("—");
    let copy_style = copy_status_style.unwrap_or_else(Theme::body);

    let metrics_block = bordered_block("Metrics");
    let metrics_inner = metrics_block.inner(area);
    f.render_widget(metrics_block, area);

    // Check MM state for status display.
    let mm = live.mm.read().ok();
    let mm_running = mm.as_ref().map(|m| m.running).unwrap_or(false);
    let mm_quotes = mm.as_ref().map(|m| m.active_quotes.len()).unwrap_or(0);
    let mm_pnl = mm.as_ref().map(|m| m.total_realized_pnl).unwrap_or(0.0);
    let mm_str = if mm_running {
        format!("ON {q}q {p:+.2}", q = mm_quotes, p = mm_pnl)
    } else {
        "off".to_string()
    };
    let mm_style = if mm_running {
        Theme::success()
    } else {
        Theme::dim()
    };

    let segs = RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Min(0),
            Constraint::Min(0),
            Constraint::Min(0),
            Constraint::Min(0),
            Constraint::Min(0),
            Constraint::Min(0),
        ])
        .split(metrics_inner);
    let open_s = open_t.to_string();
    let closed_s = closed_t.to_string();
    let groups: [(&str, &str, Style); 7] = [
        ("P&L:", pnl_str.as_str(), pnl_style),
        ("Bankroll:", bankroll.as_str(), Theme::body()),
        ("Open:", open_s.as_str(), Theme::body()),
        ("Closed:", closed_s.as_str(), Theme::body()),
        ("Mode:", mode_str, Theme::body()),
        ("Copy:", copy_str, copy_style),
        ("MM:", mm_str.as_str(), mm_style),
    ];
    for (rect, (label, value, value_style)) in segs.iter().zip(groups.iter()) {
        let line = Line::from(vec![
            Span::styled(format!("{} ", label), Theme::metrics_label()),
            Span::styled(*value, *value_style),
        ]);
        f.render_widget(Paragraph::new(line), *rect);
    }
}

pub fn render_tabs(f: &mut Frame, area: Rect, selected_tab: usize) {
    let tab_titles = TABS.iter().map(|t| Line::from(*t)).collect::<Vec<_>>();
    let tabs = Tabs::new(tab_titles)
        .block(bordered_block("Tabs"))
        .style(Theme::tab_default())
        .highlight_style(Theme::tab_selected())
        .select(selected_tab.min(TABS.len().saturating_sub(1)));
    f.render_widget(tabs, area);
}

pub fn render_hint_bar(f: &mut Frame, area: Rect, scan_note: Option<&str>) {
    let left_str = scan_note.unwrap_or("");
    let right_len = 14u16;
    let pad = area
        .width
        .saturating_sub(left_str.len() as u16)
        .saturating_sub(right_len);
    let hint_line = Line::from(vec![
        Span::styled(left_str, Theme::dim()),
        Span::raw(" ".repeat(pad as usize)),
        Span::styled("[?] ", Theme::key()),
        Span::styled("Shortcuts", Theme::body()),
    ]);
    let hint_block = Block::default()
        .style(Style::default().bg(Theme::BG()))
        .borders(Borders::NONE);
    f.render_widget(Paragraph::new(hint_line).block(hint_block), area);
}

pub fn bordered_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Theme::block_border())
        .title_style(Theme::block_title())
        .style(Style::default().bg(Theme::BG()))
}

/// Like `bordered_block` but with accent border+title when `focused` is true.
pub fn focused_block(title: &str, focused: bool) -> Block<'_> {
    if focused {
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", title))
            .border_style(Theme::tab_selected())
            .title_style(Theme::tab_selected().add_modifier(Modifier::BOLD))
            .style(Style::default().bg(Theme::BG()))
    } else {
        bordered_block(title)
    }
}
