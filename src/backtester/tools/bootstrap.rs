//! Bootstrap Confidence Intervals for every performance metric.
use super::get_param;
use crate::backtester::data::{ToolParam, Trade};
use crate::backtester::metrics::PerformanceMetrics;
use crate::backtester::tools::ToolResult;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

pub fn default_params() -> Vec<ToolParam> {
    vec![
        ToolParam::new("Iterations", 1000.0, 100.0, 10000.0, 100.0),
        ToolParam::new("Confidence %", 95.0, 90.0, 99.0, 1.0),
    ]
}

pub fn confidence_intervals(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let n_iter = get_param(params, "Iterations", 1000.0) as usize;
    let conf = get_param(params, "Confidence %", 95.0) / 100.0;
    let n = trades.len();
    if n < 5 {
        return ToolResult {
            summary: vec![("Error".into(), "Need at least 5 trades.".into())],
            ..Default::default()
        };
    }

    let real = PerformanceMetrics::compute(trades);
    let mut rng = SmallRng::seed_from_u64(333);
    let mut sharpes = Vec::with_capacity(n_iter);
    let mut sortinos = Vec::with_capacity(n_iter);
    let mut drawdowns = Vec::with_capacity(n_iter);
    let mut win_rates = Vec::with_capacity(n_iter);
    let mut returns = Vec::with_capacity(n_iter);

    for _ in 0..n_iter {
        let sample: Vec<Trade> = (0..n)
            .map(|_| trades[rng.gen_range(0..n)].clone())
            .collect();
        let m = PerformanceMetrics::compute(&sample);
        sharpes.push(m.sharpe);
        sortinos.push(m.sortino);
        drawdowns.push(m.max_dd_pct);
        win_rates.push(m.win_rate);
        returns.push(m.total_return);
    }

    let alpha = (1.0 - conf) / 2.0;
    let fmt_ci = |vals: &mut Vec<f64>, point: f64, label: &str| -> (String, String) {
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let lo_idx = (alpha * vals.len() as f64) as usize;
        let hi_idx = ((1.0 - alpha) * vals.len() as f64) as usize;
        let lo = vals.get(lo_idx).copied().unwrap_or(point);
        let hi = vals
            .get(hi_idx.min(vals.len() - 1))
            .copied()
            .unwrap_or(point);
        (label.into(), format!("{:.3} [{:.3}, {:.3}]", point, lo, hi))
    };

    let summary = vec![
        fmt_ci(&mut sharpes, real.sharpe, "Sharpe"),
        fmt_ci(&mut sortinos, real.sortino, "Sortino"),
        fmt_ci(&mut drawdowns, real.max_dd_pct, "Max DD %"),
        fmt_ci(&mut win_rates, real.win_rate, "Win Rate"),
        fmt_ci(&mut returns, real.total_return, "Total Return"),
    ];

    // Histogram of bootstrapped Sharpes
    let histogram = build_hist(&sharpes, 10);

    ToolResult {
        summary,
        histogram,
        detail_lines: vec![
            format!(
                "{} bootstrap resamples, {:.0}% confidence.",
                n_iter,
                conf * 100.0
            ),
            "Point estimate [lower bound, upper bound].".into(),
        ],
        ..Default::default()
    }
}

fn build_hist(vals: &[f64], bins: usize) -> Vec<(String, u64)> {
    if vals.is_empty() {
        return Vec::new();
    }
    let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(0.001);
    let bw = range / bins as f64;
    let mut counts = vec![0u64; bins];
    for &v in vals {
        let idx = ((v - min) / bw).floor() as usize;
        counts[idx.min(bins - 1)] += 1;
    }
    counts
        .into_iter()
        .enumerate()
        .map(|(i, c)| (format!("{:.2}", min + i as f64 * bw), c))
        .collect()
}
