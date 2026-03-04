//! Portfolio tab renderer: 3-pane (open positions | trade history | P&L breakdown).

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

pub fn render(f: &mut Frame, area: Rect, live: &LiveState, scroll: usize) {
    let cols = RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(area);

    render_open_positions(f, cols[0], live);
    render_trade_history(f, cols[1], live, scroll);
    render_pnl_breakdown(f, cols[2], live);
}

fn render_open_positions(f: &mut Frame, area: Rect, live: &LiveState) {
    let block = bordered_block("Open Positions");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let portfolio = live.portfolio.read().ok();
    let positions = portfolio
        .as_ref()
        .map(|p| p.open_positions.as_slice())
        .unwrap_or_default();

    if positions.is_empty() {
        f.render_widget(
            Paragraph::new("No open positions.")
                .style(Theme::dim())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let widths = [
        Constraint::Min(10),
        Constraint::Length(4),
        Constraint::Length(7),
        Constraint::Length(7),
    ];
    let header = Row::new([
        Cell::from("Market").style(Theme::block_title()),
        Cell::from("Side").style(Theme::block_title()),
        Cell::from("Size").style(Theme::block_title()),
        Cell::from("P&L").style(Theme::block_title()),
    ]);
    let rows: Vec<Row> = positions
        .iter()
        .map(|p| {
            let pnl = p.total_pnl();
            let pnl_style = pnl_color(pnl);
            let market_short = p.market_id.get(..10).unwrap_or(&p.market_id);
            Row::new([
                Cell::from(market_short).style(Theme::body()),
                Cell::from(format!("{:?}", p.side)).style(Theme::body()),
                Cell::from(format!("{:.2}", p.size)).style(Theme::body()),
                Cell::from(format!("{:+.2}", pnl)).style(pnl_style),
            ])
        })
        .collect();
    let table = Table::new(rows, widths)
        .header(header)
        .block(bordered_block(""));
    f.render_widget(table, inner);
}

fn render_trade_history(f: &mut Frame, area: Rect, live: &LiveState, scroll: usize) {
    let block = bordered_block("Trade History");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let portfolio = live.portfolio.read().ok();
    let trades = portfolio
        .as_ref()
        .map(|p| p.trade_history.as_slice())
        .unwrap_or_default();

    if trades.is_empty() {
        f.render_widget(
            Paragraph::new("No trades yet.\nPaper trade: enable a strategy and execute a signal.")
                .style(Theme::dim())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let visible_h = inner.height as usize;
    let offset = scroll.min(trades.len().saturating_sub(visible_h));
    let lines: Vec<Line> = trades
        .iter()
        .skip(offset)
        .take(visible_h)
        .map(|t| {
            let ts_short = t.timestamp.get(..16).unwrap_or(&t.timestamp);
            Line::from(Span::styled(
                format!(
                    "{} {:>8} {:3} {:.2}×{:.3}",
                    ts_short,
                    truncate(&t.domain, 8),
                    t.side,
                    t.size,
                    t.price,
                ),
                Theme::body(),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(Theme::body()), inner);
}

fn render_pnl_breakdown(f: &mut Frame, area: Rect, live: &LiveState) {
    let block = bordered_block("P&L Breakdown");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let portfolio = live.portfolio.read().ok();
    let domain_pnl = portfolio
        .as_ref()
        .map(|p| p.domain_pnl.clone())
        .unwrap_or_default();
    let total_pnl = portfolio.as_ref().map(|p| p.total_pnl()).unwrap_or(0.0);

    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        "Domain P&L (paper)",
        Theme::block_title(),
    ))];
    lines.push(Line::from(""));

    for (domain, _today, all_time) in &domain_pnl {
        let pnl_style = pnl_color(*all_time);
        lines.push(Line::from(vec![
            Span::styled(format!("{:<10}", domain), Theme::body()),
            Span::styled(format!("{:+8.2}", all_time), pnl_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("─────────────", Theme::dim())));
    lines.push(Line::from(vec![
        Span::styled("Total     ", Theme::block_title()),
        Span::styled(format!("{:+8.2}", total_pnl), pnl_color(total_pnl)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "(Trade to JSONL files in paper mode)",
        Theme::dim(),
    )));

    f.render_widget(Paragraph::new(lines).style(Theme::body()), inner);
}

fn pnl_color(pnl: f64) -> Style {
    if pnl > 0.0 {
        Theme::success()
    } else if pnl < 0.0 {
        Theme::danger()
    } else {
        Theme::neutral_pnl()
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
