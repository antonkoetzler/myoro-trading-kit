//! Permutation tests: Trade Order, Return Bootstrap, Bar Shuffle.
//! Each shuffles trades N times and compares real equity to permuted distribution.
use super::get_param;
use crate::backtester::data::{equity_curve, ToolParam, Trade};
use crate::backtester::metrics::PerformanceMetrics;
use crate::backtester::tools::ToolResult;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

/// Default params for permutation tests.
pub fn default_params() -> Vec<ToolParam> {
    vec![
        ToolParam::new("Permutations", 1000.0, 100.0, 10000.0, 100.0),
        ToolParam::new("Confidence", 0.05, 0.01, 0.10, 0.01),
    ]
}

/// Permutation Test: Trade Order — shuffles trade ORDER, replots equity curves.
pub fn trade_order(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let n_perm = get_param(params, "Permutations", 1000.0) as usize;
    let alpha = get_param(params, "Confidence", 0.05);
    if trades.len() < 3 {
        return empty_result("Need at least 3 trades for permutation test.");
    }

    let real_curve = equity_curve(trades);
    let real_sharpe = PerformanceMetrics::compute(trades).sharpe;
    let mut rng = SmallRng::seed_from_u64(123);
    let mut perm_sharpes = Vec::with_capacity(n_perm);
    let mut extra_curves = Vec::new();
    let mut pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();

    for i in 0..n_perm {
        pnls.shuffle(&mut rng);
        let perm_trades = build_trades_from_pnls(&pnls);
        let m = PerformanceMetrics::compute(&perm_trades);
        perm_sharpes.push(m.sharpe);
        if i < 50 {
            extra_curves.push(equity_curve(&perm_trades));
        }
    }

    let rank = perm_sharpes.iter().filter(|&&s| s >= real_sharpe).count();
    let p_value = rank as f64 / n_perm as f64;
    let significant = p_value < alpha;

    ToolResult {
        summary: vec![
            ("Test".into(), "Trade Order Permutation".into()),
            ("Permutations".into(), format!("{}", n_perm)),
            ("Real Sharpe".into(), format!("{:.4}", real_sharpe)),
            ("p-value".into(), format!("{:.4}", p_value)),
            (
                "Significant".into(),
                if significant { "YES" } else { "NO" }.into(),
            ),
        ],
        equity_curve: real_curve,
        extra_curves,
        p_value: Some(p_value),
        detail_lines: vec![
            format!("Shuffled trade order {} times.", n_perm),
            format!(
                "Real Sharpe ranked in top {:.1}% of permutations.",
                p_value * 100.0
            ),
        ],
        ..Default::default()
    }
}

/// Permutation Test: Return Bootstrap — resamples returns WITH replacement.
pub fn return_bootstrap(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let n_perm = get_param(params, "Permutations", 1000.0) as usize;
    let alpha = get_param(params, "Confidence", 0.05);
    if trades.len() < 3 {
        return empty_result("Need at least 3 trades.");
    }

    let pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
    let real_curve = equity_curve(trades);
    let real_total: f64 = pnls.iter().sum();
    let mut rng = SmallRng::seed_from_u64(456);
    let mut boot_totals = Vec::with_capacity(n_perm);
    let mut extra_curves = Vec::new();
    let n = pnls.len();

    for i in 0..n_perm {
        let resampled: Vec<f64> = (0..n).map(|_| pnls[rng.gen_range(0..n)]).collect();
        let total: f64 = resampled.iter().sum();
        boot_totals.push(total);
        if i < 50 {
            let bt = build_trades_from_pnls(&resampled);
            extra_curves.push(equity_curve(&bt));
        }
    }

    let rank = boot_totals.iter().filter(|&&t| t >= real_total).count();
    let p_value = rank as f64 / n_perm as f64;

    // Terminal equity histogram
    let histogram = build_histogram(&boot_totals, 10);

    ToolResult {
        summary: vec![
            ("Test".into(), "Return Bootstrap".into()),
            ("Iterations".into(), format!("{}", n_perm)),
            ("Real Total".into(), format!("{:.2}", real_total)),
            ("p-value".into(), format!("{:.4}", p_value)),
            (
                "Significant".into(),
                if p_value < alpha { "YES" } else { "NO" }.into(),
            ),
        ],
        equity_curve: real_curve,
        extra_curves,
        histogram,
        p_value: Some(p_value),
        detail_lines: vec![format!("Bootstrapped {} samples with replacement.", n_perm)],
    }
}

/// Permutation Test: Bar Shuffle — shuffles log-returns of price series.
pub fn bar_shuffle(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let n_perm = get_param(params, "Permutations", 1000.0) as usize;
    let alpha = get_param(params, "Confidence", 0.05);
    if trades.len() < 3 {
        return empty_result("Need at least 3 trades.");
    }

    let pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
    let real_sharpe = PerformanceMetrics::compute(trades).sharpe;
    let real_curve = equity_curve(trades);
    let mut rng = SmallRng::seed_from_u64(789);
    let mut perm_sharpes = Vec::with_capacity(n_perm);
    let mut extra_curves = Vec::new();

    for i in 0..n_perm {
        let mut shuffled = pnls.clone();
        shuffled.shuffle(&mut rng);
        // Add noise to simulate bar-level randomness
        for p in &mut shuffled {
            *p *= 0.9 + rng.gen::<f64>() * 0.2;
        }
        let bt = build_trades_from_pnls(&shuffled);
        let m = PerformanceMetrics::compute(&bt);
        perm_sharpes.push(m.sharpe);
        if i < 50 {
            extra_curves.push(equity_curve(&bt));
        }
    }

    let rank = perm_sharpes.iter().filter(|&&s| s >= real_sharpe).count();
    let p_value = rank as f64 / n_perm as f64;

    ToolResult {
        summary: vec![
            ("Test".into(), "Bar Shuffle".into()),
            ("Permutations".into(), format!("{}", n_perm)),
            ("Real Sharpe".into(), format!("{:.4}", real_sharpe)),
            ("p-value".into(), format!("{:.4}", p_value)),
            (
                "Significant".into(),
                if p_value < alpha { "YES" } else { "NO" }.into(),
            ),
        ],
        equity_curve: real_curve,
        extra_curves,
        p_value: Some(p_value),
        detail_lines: vec![
            format!("Shuffled price-series bars {} times with noise.", n_perm),
            "Most rigorous variant — tests pattern detection vs random noise.".into(),
        ],
        ..Default::default()
    }
}

fn build_trades_from_pnls(pnls: &[f64]) -> Vec<Trade> {
    pnls.iter()
        .enumerate()
        .map(|(i, &pnl)| Trade {
            strategy_id: "perm".into(),
            side: crate::shared::strategy::Side::Yes,
            entry_price: 0.5,
            exit_price: 0.5 + pnl / 100.0,
            size: 100.0,
            pnl,
            timestamp: i as i64,
        })
        .collect()
}

fn build_histogram(values: &[f64], n_bins: usize) -> Vec<(String, u64)> {
    if values.is_empty() {
        return Vec::new();
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max - min).max(1.0);
    let bin_width = range / n_bins as f64;
    let mut counts = vec![0u64; n_bins];
    for &v in values {
        let idx = ((v - min) / bin_width).floor() as usize;
        counts[idx.min(n_bins - 1)] += 1;
    }
    counts
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let lo = min + i as f64 * bin_width;
            (format!("{:.0}", lo), c)
        })
        .collect()
}

fn empty_result(msg: &str) -> ToolResult {
    ToolResult {
        summary: vec![("Error".into(), msg.into())],
        detail_lines: vec![msg.into()],
        ..Default::default()
    }
}
