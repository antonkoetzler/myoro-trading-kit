//! Walk-Forward Analysis: rolling in-sample/out-of-sample windows.
use super::get_param;
use crate::backtester::data::{equity_curve, ToolParam, Trade};
use crate::backtester::metrics::PerformanceMetrics;
use crate::backtester::tools::ToolResult;

pub fn default_params() -> Vec<ToolParam> {
    vec![
        ToolParam::new("IS Window %", 70.0, 50.0, 90.0, 5.0),
        ToolParam::new("Windows", 5.0, 2.0, 20.0, 1.0),
    ]
}

pub fn run(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let is_pct = get_param(params, "IS Window %", 70.0) / 100.0;
    let n_windows = get_param(params, "Windows", 5.0) as usize;
    let n = trades.len();
    if n < 10 || n_windows < 2 {
        return ToolResult {
            summary: vec![(
                "Error".into(),
                "Need at least 10 trades and 2 windows.".into(),
            )],
            ..Default::default()
        };
    }

    let window_size = n / n_windows;
    let is_size = (window_size as f64 * is_pct) as usize;
    let mut is_sharpes = Vec::new();
    let mut oos_sharpes = Vec::new();
    let mut oos_trades: Vec<Trade> = Vec::new();
    let mut detail = Vec::new();

    for w in 0..n_windows {
        let start = w * window_size;
        let end = ((w + 1) * window_size).min(n);
        let is_end = (start + is_size).min(end);
        if is_end >= end || start >= n {
            continue;
        }

        let is_slice = &trades[start..is_end];
        let oos_slice = &trades[is_end..end];
        let is_m = PerformanceMetrics::compute(is_slice);
        let oos_m = PerformanceMetrics::compute(oos_slice);
        is_sharpes.push(is_m.sharpe);
        oos_sharpes.push(oos_m.sharpe);
        oos_trades.extend_from_slice(oos_slice);

        detail.push(format!(
            "W{}: IS Sharpe={:.3} OOS Sharpe={:.3} (IS={} OOS={})",
            w + 1,
            is_m.sharpe,
            oos_m.sharpe,
            is_slice.len(),
            oos_slice.len(),
        ));
    }

    let is_mean = mean(&is_sharpes);
    let oos_mean = mean(&oos_sharpes);
    let wfe = if is_mean.abs() > 0.0 {
        oos_mean / is_mean
    } else {
        0.0
    };

    let curve = equity_curve(&oos_trades);

    ToolResult {
        summary: vec![
            ("Windows".into(), format!("{}", n_windows)),
            (
                "IS/OOS Split".into(),
                format!("{:.0}%/{:.0}%", is_pct * 100.0, (1.0 - is_pct) * 100.0),
            ),
            ("Avg IS Sharpe".into(), format!("{:.4}", is_mean)),
            ("Avg OOS Sharpe".into(), format!("{:.4}", oos_mean)),
            ("WFE".into(), format!("{:.3}", wfe)),
            ("Robust".into(), if wfe > 0.5 { "YES" } else { "NO" }.into()),
        ],
        equity_curve: curve,
        detail_lines: detail,
        ..Default::default()
    }
}

fn mean(vals: &[f64]) -> f64 {
    if vals.is_empty() {
        0.0
    } else {
        vals.iter().sum::<f64>() / vals.len() as f64
    }
}
