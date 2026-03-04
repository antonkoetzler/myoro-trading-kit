//! Crypto tab renderer: 3-pane (strategies | signal feed | active markets).

use crate::live::LiveState;
use crate::tui::theme::Theme;
use crate::tui::views::header::bordered_block;
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, live: &LiveState, scroll_offsets: &[usize]) {
    let cols = RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(36),
            Constraint::Percentage(36),
        ])
        .split(area);

    let c = live.crypto.read().ok();
    render_strategies(f, cols[0], c.as_deref());
    render_signals(
        f,
        cols[1],
        c.as_deref(),
        scroll_offsets.first().copied().unwrap_or(0),
    );
    render_markets(f, cols[2], live, c.as_deref());
}

fn render_strategies(f: &mut Frame, area: Rect, c: Option<&crate::live::CryptoState>) {
    let block = bordered_block("Strategies [Space toggle]");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let configs = c.map(|s| s.strategy_configs.as_slice()).unwrap_or_default();
    if configs.is_empty() {
        f.render_widget(Paragraph::new("Loading…").style(Theme::dim()), inner);
        return;
    }

    let rows: Vec<Row> = configs
        .iter()
        .map(|cfg| {
            let status = if cfg.enabled { "ON " } else { "off" };
            let status_style = if cfg.enabled {
                Theme::success()
            } else {
                Theme::dim()
            };
            Row::new([
                Cell::from(status).style(status_style),
                Cell::from(cfg.name).style(Theme::body()),
            ])
        })
        .collect();
    let widths = [Constraint::Length(4), Constraint::Min(12)];
    let table = Table::new(rows, widths);
    f.render_widget(table, inner);
}

fn render_signals(f: &mut Frame, area: Rect, c: Option<&crate::live::CryptoState>, scroll: usize) {
    let block = bordered_block("Signal Feed");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let signals = c.map(|s| s.signals.as_slice()).unwrap_or_default();
    if signals.is_empty() {
        f.render_widget(
            Paragraph::new("No signals yet.\nEnable a strategy to start scanning.")
                .style(Theme::dim())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let visible_h = inner.height as usize;
    let offset = scroll.min(signals.len().saturating_sub(visible_h));
    let lines: Vec<Line> = signals
        .iter()
        .rev()
        .skip(offset)
        .take(visible_h)
        .map(|s| {
            let pnl_style = if s.side == "Yes" {
                Theme::success()
            } else {
                Theme::danger()
            };
            let ts = s.created_at.format("%H:%M:%S").to_string();
            let label_short = s.label.get(..20).unwrap_or(&s.label);
            Line::from(vec![
                Span::styled(format!("{} ", ts), Theme::dim()),
                Span::styled(format!("{:<20} ", label_short), Theme::body()),
                Span::styled(format!("{:<3} ", s.side), pnl_style),
                Span::styled(
                    format!("{:+.1}%", s.edge_pct * 100.0),
                    edge_color(s.edge_pct),
                ),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(Theme::body()), inner);
}

fn render_markets(
    f: &mut Frame,
    area: Rect,
    live: &LiveState,
    c: Option<&crate::live::CryptoState>,
) {
    let block = bordered_block("Crypto Markets");
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Show BTC price in header, then market list or Gamma events fallback.
    let btc_line = c
        .map(|s| s.btc_usdt.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("BTC/USDT —");

    let markets = c.map(|s| s.markets.as_slice()).unwrap_or_default();
    if markets.is_empty() {
        // Fallback to Gamma events text
        let events = c.map(|s| s.events.join("\n")).unwrap_or_default();
        let content = if events.is_empty() {
            format!("{}\n\n⏳ Loading markets…", btc_line)
        } else {
            format!("{}\n\n{}", btc_line, events)
        };
        f.render_widget(
            Paragraph::new(content)
                .style(Theme::body())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let _ = live; // reserved for future live price overlay
    let header = Row::new([
        Cell::from("Market").style(Theme::block_title()),
        Cell::from("Bid").style(Theme::block_title()),
        Cell::from("Ask").style(Theme::block_title()),
    ]);
    let rows: Vec<Row> = markets
        .iter()
        .take(inner.height as usize)
        .map(|m| {
            let title_short = m.title.get(..22).unwrap_or(&m.title);
            Row::new([
                Cell::from(title_short).style(Theme::body()),
                Cell::from(format!("{:.3}", m.best_bid)).style(Theme::body()),
                Cell::from(format!("{:.3}", m.best_ask)).style(Theme::body()),
            ])
        })
        .collect();
    let widths = [
        Constraint::Min(10),
        Constraint::Length(5),
        Constraint::Length(5),
    ];
    let table = Table::new(rows, widths).header(header);
    f.render_widget(table, inner);
}

fn edge_color(edge: f64) -> Style {
    if edge >= 0.05 {
        Theme::success()
    } else if edge >= 0.03 {
        Theme::neutral_pnl()
    } else {
        Theme::dim()
    }
}
