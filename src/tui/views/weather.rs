//! Weather tab renderer: 3-pane (strategies | signal feed | city forecasts).

use crate::live::LiveState;
use crate::tui::theme::Theme;
use crate::tui::views::header::bordered_block;
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, live: &LiveState, scroll: usize) {
    let cols = RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(38),
            Constraint::Percentage(37),
        ])
        .split(area);

    let w = live.weather.read().ok();
    render_strategies(f, cols[0], w.as_deref());
    render_signals(f, cols[1], w.as_deref(), scroll);
    render_city_forecasts(f, cols[2], live, w.as_deref());
}

fn render_strategies(f: &mut Frame, area: Rect, w: Option<&crate::live::WeatherState>) {
    let block = bordered_block("Strategies [Space toggle]");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let configs = w.map(|s| s.strategy_configs.as_slice()).unwrap_or_default();
    if configs.is_empty() {
        f.render_widget(Paragraph::new("Loading…").style(Theme::dim()), inner);
        return;
    }

    let rows: Vec<Row> = configs
        .iter()
        .map(|cfg| {
            let status = if cfg.enabled { "ON " } else { "off" };
            let style = if cfg.enabled {
                Theme::success()
            } else {
                Theme::dim()
            };
            Row::new([
                ratatui::widgets::Cell::from(status).style(style),
                ratatui::widgets::Cell::from(cfg.name).style(Theme::body()),
            ])
        })
        .collect();
    let widths = [Constraint::Length(4), Constraint::Min(12)];
    f.render_widget(Table::new(rows, widths), inner);
}

fn render_signals(f: &mut Frame, area: Rect, w: Option<&crate::live::WeatherState>, scroll: usize) {
    let block = bordered_block("Signal Feed");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let signals = w.map(|s| s.signals.as_slice()).unwrap_or_default();
    if signals.is_empty() {
        f.render_widget(
            Paragraph::new("No signals yet.\nEnable ForecastLag to start scanning.")
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
            let side_style = if s.side == "Yes" {
                Theme::success()
            } else {
                Theme::danger()
            };
            let ts = s.created_at.format("%H:%M").to_string();
            Line::from(vec![
                Span::styled(format!("{} ", ts), Theme::dim()),
                Span::styled(
                    format!("{:<20} ", &s.label.get(..20).unwrap_or(&s.label)),
                    Theme::body(),
                ),
                Span::styled(format!("{:<3} ", s.side), side_style),
                Span::styled(
                    format!("{:+.1}%", s.edge_pct * 100.0),
                    edge_color(s.edge_pct),
                ),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(Theme::body()), inner);
}

fn render_city_forecasts(
    f: &mut Frame,
    area: Rect,
    live: &LiveState,
    w: Option<&crate::live::WeatherState>,
) {
    let block = if live.last_log_is_error(2) {
        bordered_block_error("City Forecasts")
    } else {
        bordered_block("City Forecasts")
    };
    let inner = block.inner(area);
    f.render_widget(block, area);

    let city_forecasts = w.map(|s| s.city_forecasts.as_slice()).unwrap_or_default();
    if city_forecasts.is_empty() {
        // Fallback: show plain forecast text (NYC only).
        let lines = w.map(|s| s.forecast.join("\n")).unwrap_or_default();
        let content = if lines.is_empty() {
            "⏳ Loading…".to_string()
        } else {
            format!("7-day forecast (NYC)\n\n{}", lines)
        };
        f.render_widget(
            Paragraph::new(content)
                .style(Theme::body())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let header = Row::new([
        ratatui::widgets::Cell::from("City").style(Theme::block_title()),
        ratatui::widgets::Cell::from("Max°C").style(Theme::block_title()),
        ratatui::widgets::Cell::from("Min°C").style(Theme::block_title()),
        ratatui::widgets::Cell::from("Mkt").style(Theme::block_title()),
    ]);
    let rows: Vec<Row> = city_forecasts
        .iter()
        .map(|cf| {
            let max = cf
                .today_max_c
                .map(|t| format!("{:.1}", t))
                .unwrap_or_else(|| "—".to_string());
            let min = cf
                .today_min_c
                .map(|t| format!("{:.1}", t))
                .unwrap_or_else(|| "—".to_string());
            let mkt = cf
                .market_implied
                .map(|p| format!("{:.2}", p))
                .unwrap_or_else(|| "—".to_string());
            Row::new([
                ratatui::widgets::Cell::from(cf.city.as_str()).style(Theme::body()),
                ratatui::widgets::Cell::from(max).style(Theme::body()),
                ratatui::widgets::Cell::from(min).style(Theme::body()),
                ratatui::widgets::Cell::from(mkt).style(Theme::body()),
            ])
        })
        .collect();
    let widths = [
        Constraint::Min(10),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(5),
    ];
    f.render_widget(Table::new(rows, widths).header(header), inner);
}

fn bordered_block_error(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} — Error (see logs) ", title))
        .border_style(Theme::danger())
        .title_style(Theme::block_title())
        .style(Style::default().bg(Theme::BG()))
}

fn edge_color(edge: f64) -> Style {
    if edge >= 0.08 {
        Theme::success()
    } else if edge >= 0.06 {
        Theme::neutral_pnl()
    } else {
        Theme::dim()
    }
}
