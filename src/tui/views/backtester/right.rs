//! Right column: performance metrics (top) + graph (bottom).
use crate::backtester::BacktesterState;
use crate::tui::theme::Theme;
use crate::tui::views::header::focused_block;
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, state: &BacktesterState, focused: bool) {
    let chunks = RLayout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_metrics(f, chunks[0], state, focused);
    super::graph::render(f, chunks[1], state);
}

fn render_metrics(f: &mut Frame, area: Rect, state: &BacktesterState, focused: bool) {
    let block = focused_block("Results", focused);

    if state.is_running.load(std::sync::atomic::Ordering::Relaxed) {
        let para = Paragraph::new("  Running analysis…")
            .style(Theme::dim())
            .block(block);
        f.render_widget(para, area);
        return;
    }

    // Try tool result first, then fall back to base metrics
    let tool_result = state.tool_result.read().ok();
    let tool_ref = tool_result.as_ref().and_then(|r| r.as_ref());

    if let Some(tr) = tool_ref {
        let mut lines = Vec::new();
        for (k, v) in &tr.summary {
            let val_style = if v.contains("YES") {
                Theme::tab_selected().add_modifier(Modifier::BOLD)
            } else {
                Theme::body()
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<20}", k), Theme::dim()),
                Span::styled(v.clone(), val_style),
            ]));
        }

        if !tr.detail_lines.is_empty() {
            lines.push(Line::from(""));
            for d in &tr.detail_lines {
                lines.push(Line::from(Span::styled(format!("  {}", d), Theme::dim())));
            }
        }

        // Append base metrics if available
        if let Ok(guard) = state.current_metrics.read() {
            if let Some(m) = guard.as_ref() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  ── Base Metrics ──",
                    Theme::dim(),
                )));
                append_base_metrics(&mut lines, m);
            }
        }

        let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
        f.render_widget(para, area);
    } else if let Ok(guard) = state.current_metrics.read() {
        if let Some(m) = guard.as_ref() {
            let mut lines = Vec::new();
            append_base_metrics(&mut lines, m);
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Press r to run analysis tool.",
                Theme::dim(),
            )));
            let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
            f.render_widget(para, area);
        } else {
            let para = Paragraph::new("  Press r to load trades and run analysis.")
                .style(Theme::dim())
                .block(block);
            f.render_widget(para, area);
        }
    } else {
        let para = Paragraph::new("  Press r to run.")
            .style(Theme::dim())
            .block(block);
        f.render_widget(para, area);
    }
}

fn append_base_metrics(
    lines: &mut Vec<Line<'_>>,
    m: &crate::backtester::metrics::PerformanceMetrics,
) {
    let row = |label: &str, val: String| -> Line<'_> {
        Line::from(vec![
            Span::styled(format!("  {:<20}", label), Theme::dim()),
            Span::styled(val, Theme::body()),
        ])
    };
    lines.push(row("Trades", format!("{}", m.trade_count)));
    lines.push(row("Total Return", format!("{:.2}", m.total_return)));
    lines.push(row("Sharpe", format!("{:.3}", m.sharpe)));
    lines.push(row("Sortino", format!("{:.3}", m.sortino)));
    lines.push(row("Max Drawdown", format!("{:.2}%", m.max_dd_pct * 100.0)));
    lines.push(row("Win Rate", format!("{:.1}%", m.win_rate * 100.0)));
    lines.push(row("Profit Factor", format!("{:.2}", m.profit_factor)));
    lines.push(row("Expectancy", format!("{:.2}", m.expectancy)));
    lines.push(row("Calmar", format!("{:.3}", m.calmar)));
    lines.push(row("VaR (95%)", format!("{:.2}", m.var_95)));
    lines.push(row("Recovery Factor", format!("{:.2}", m.recovery_factor)));
    if let Some(bs) = m.brier_score {
        lines.push(row("Brier Score", format!("{:.4}", bs)));
    }
}
