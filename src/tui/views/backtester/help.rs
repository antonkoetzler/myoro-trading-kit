//! Backtester-specific help overlay (H key).
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout as RLayout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame) {
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::tab_selected())
        .title(Span::styled(
            " Backtester Help  [H or Esc to close] ",
            Theme::tab_selected().add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(Theme::BG()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split into two columns
    let cols = RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let left_lines = vec![
        Line::from(Span::styled(
            "WORKFLOW",
            Theme::metrics_label().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("1. Strategy pane", Theme::tab_selected())),
        Line::from(Span::styled(
            "   ↑/↓  Move cursor through strategies",
            Theme::dim(),
        )),
        Line::from(Span::styled(
            "   Enter  Confirm selection (shows ✓)",
            Theme::dim(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "2. Data Source pane  (appears after step 1)",
            Theme::tab_selected(),
        )),
        Line::from(Span::styled(
            "   ↑/↓  Browse compatible data sources",
            Theme::dim(),
        )),
        Line::from(Span::styled(
            "   Enter  Open data config dialog",
            Theme::dim(),
        )),
        Line::from(Span::styled(
            "   [s]  Save config and advance to Tools",
            Theme::dim(),
        )),
        Line::from(""),
        Line::from(Span::styled("3. Tools pane", Theme::tab_selected())),
        Line::from(Span::styled(
            "   ↑/↓  Browse 19 analysis tools",
            Theme::dim(),
        )),
        Line::from(Span::styled("   Enter  Go to Params pane", Theme::dim())),
        Line::from(""),
        Line::from(Span::styled("4. Params pane", Theme::tab_selected())),
        Line::from(Span::styled("   ↑/↓  Select parameter", Theme::dim())),
        Line::from(Span::styled("   ←/→  Adjust value by step", Theme::dim())),
        Line::from(Span::styled(
            "   Enter  Type a value directly",
            Theme::dim(),
        )),
        Line::from(Span::styled("   Esc  Cancel edit", Theme::dim())),
        Line::from(""),
        Line::from(Span::styled("5. Run", Theme::tab_selected())),
        Line::from(Span::styled(
            "   r  Run the backtest (works from any pane)",
            Theme::dim(),
        )),
    ];

    let right_lines = vec![
        Line::from(Span::styled(
            "NAVIGATION",
            Theme::metrics_label().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Tab / Shift+Tab  Cycle panes", Theme::dim())),
        Line::from(Span::styled("Esc  Quit app / Close dialog", Theme::dim())),
        Line::from(Span::styled("g  Toggle full-screen graph", Theme::dim())),
        Line::from(Span::styled("H  This help screen", Theme::dim())),
        Line::from(Span::styled("?  Global shortcuts", Theme::dim())),
        Line::from(""),
        Line::from(Span::styled(
            "TOOLS (19 total)",
            Theme::metrics_label().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Tier 0 — Permutation Tests",
            Theme::tab_selected(),
        )),
        Line::from(Span::styled(
            "  Perm: Trade Order, Return Bootstrap, Bar Shuffle",
            Theme::dim(),
        )),
        Line::from(Span::styled(
            "  → Gold standard for strategy significance",
            Theme::dim(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Tier 1 — Core Backtesting",
            Theme::tab_selected(),
        )),
        Line::from(Span::styled(
            "  Historical Replay, Walk-Forward, Bootstrap CI, TCA",
            Theme::dim(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Tier 2 — Overfitting Detection",
            Theme::tab_selected(),
        )),
        Line::from(Span::styled(
            "  Sensitivity, Deflated Sharpe, PBO, Min Backtest Length",
            Theme::dim(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Tier 3 — Risk & Stress",
            Theme::tab_selected(),
        )),
        Line::from(Span::styled(
            "  Stress Test, Regime Detection, Risk of Ruin, Drawdown",
            Theme::dim(),
        )),
        Line::from(""),
        Line::from(Span::styled("Tier 4 — Simulation", Theme::tab_selected())),
        Line::from(Span::styled(
            "  Monte Carlo, Monte Carlo+IS, Copula, Agent-Based",
            Theme::dim(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Custom strategies: docs/CUSTOM_STRATEGIES.md",
            Theme::dim(),
        )),
    ];

    f.render_widget(
        Paragraph::new(left_lines)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        cols[0],
    );
    f.render_widget(
        Paragraph::new(right_lines)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true }),
        cols[1],
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert = RLayout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}
