//! Risk of Ruin and Drawdown Analysis tools.
use super::get_param;
use crate::backtester::data::{equity_curve, ToolParam, Trade};
use crate::backtester::metrics::{drawdown_series, PerformanceMetrics};
use crate::backtester::tools::ToolResult;

pub fn ruin_params() -> Vec<ToolParam> {
    vec![
        ToolParam::new("Ruin Threshold %", 50.0, 10.0, 90.0, 5.0),
        ToolParam::new("Position Size %", 5.0, 1.0, 25.0, 1.0),
    ]
}

/// Risk of Ruin: probability of losing X% of capital given win rate and sizing.
pub fn risk_of_ruin(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let ruin_pct = get_param(params, "Ruin Threshold %", 50.0) / 100.0;
    let pos_size = get_param(params, "Position Size %", 5.0) / 100.0;
    if trades.is_empty() {
        return ToolResult {
            summary: vec![("Error".into(), "No trades.".into())],
            ..Default::default()
        };
    }

    let m = PerformanceMetrics::compute(trades);
    let win_rate = m.win_rate;
    let avg_win = m.avg_win;
    let avg_loss = m.avg_loss;

    // Classic risk of ruin formula for binary outcomes
    let edge = win_rate * avg_win - (1.0 - win_rate) * avg_loss;
    let units_to_ruin = (ruin_pct / pos_size).ceil() as usize;

    let ror = if edge <= 0.0 {
        1.0 // Negative edge = certain ruin
    } else if win_rate >= 1.0 {
        0.0
    } else {
        let q = 1.0 - win_rate;
        let ratio = q / win_rate;
        ratio.powi(units_to_ruin as i32).min(1.0)
    };

    // Expected trades to ruin (simplified geometric)
    let trades_to_ruin = if edge > 0.0 {
        (ruin_pct * 10_000.0) / (edge * pos_size)
    } else {
        0.0
    };

    ToolResult {
        summary: vec![
            ("Ruin Threshold".into(), format!("{:.0}%", ruin_pct * 100.0)),
            ("Position Size".into(), format!("{:.1}%", pos_size * 100.0)),
            ("Win Rate".into(), format!("{:.1}%", win_rate * 100.0)),
            ("Edge per Trade".into(), format!("{:.2}", edge)),
            ("Risk of Ruin".into(), format!("{:.4}%", ror * 100.0)),
            (
                "Trades to Ruin".into(),
                if edge > 0.0 {
                    format!("{:.0}", trades_to_ruin)
                } else {
                    "N/A".into()
                },
            ),
        ],
        equity_curve: equity_curve(trades),
        detail_lines: vec![format!(
            "With {:.1}% sizing and {:.1}% win rate, probability of losing {:.0}% = {:.4}%",
            pos_size * 100.0,
            win_rate * 100.0,
            ruin_pct * 100.0,
            ror * 100.0,
        )],
        ..Default::default()
    }
}

/// Drawdown Analysis: underwater equity curve, depth/duration/recovery stats.
pub fn drawdown_analysis(trades: &[Trade]) -> ToolResult {
    if trades.is_empty() {
        return ToolResult {
            summary: vec![("Error".into(), "No trades.".into())],
            ..Default::default()
        };
    }

    let pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
    let dd_series = drawdown_series(&pnls);
    let curve = equity_curve(trades);

    // Find all drawdown events
    let mut events: Vec<DrawdownEvent> = Vec::new();
    let mut in_dd = false;
    let mut dd_start = 0;
    let mut dd_depth = 0.0_f64;

    for (i, &dd) in dd_series.iter().enumerate() {
        if dd < 0.0 {
            if !in_dd {
                dd_start = i;
                in_dd = true;
            }
            dd_depth = dd_depth.min(dd);
        } else if in_dd {
            events.push(DrawdownEvent {
                start: dd_start,
                end: i,
                depth: dd_depth,
                duration: i - dd_start,
            });
            in_dd = false;
            dd_depth = 0.0;
        }
    }
    if in_dd {
        events.push(DrawdownEvent {
            start: dd_start,
            end: dd_series.len(),
            depth: dd_depth,
            duration: dd_series.len() - dd_start,
        });
    }

    let max_dd = events.iter().map(|e| e.depth).fold(0.0_f64, f64::min);
    let max_dur = events.iter().map(|e| e.duration).max().unwrap_or(0);
    let avg_dd = if events.is_empty() {
        0.0
    } else {
        events.iter().map(|e| e.depth).sum::<f64>() / events.len() as f64
    };
    let avg_dur = if events.is_empty() {
        0.0
    } else {
        events.iter().map(|e| e.duration).sum::<usize>() as f64 / events.len() as f64
    };

    let detail: Vec<String> = events
        .iter()
        .take(5)
        .enumerate()
        .map(|(i, e)| {
            format!(
                "DD #{}: depth={:.2} duration={} trades ({}→{})",
                i + 1,
                e.depth,
                e.duration,
                e.start,
                e.end,
            )
        })
        .collect();

    ToolResult {
        summary: vec![
            ("Drawdown Events".into(), format!("{}", events.len())),
            ("Max Depth".into(), format!("{:.2}", max_dd)),
            ("Max Duration".into(), format!("{} trades", max_dur)),
            ("Avg Depth".into(), format!("{:.2}", avg_dd)),
            ("Avg Duration".into(), format!("{:.1} trades", avg_dur)),
        ],
        equity_curve: curve,
        extra_curves: vec![dd_series],
        detail_lines: detail,
        ..Default::default()
    }
}

struct DrawdownEvent {
    start: usize,
    end: usize,
    depth: f64,
    duration: usize,
}
