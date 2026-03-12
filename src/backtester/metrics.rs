//! Performance metrics computed from a trade sequence.
use crate::backtester::data::Trade;

/// Full performance metrics dashboard.
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub total_return: f64,
    pub cagr: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub max_drawdown: f64,
    pub max_dd_pct: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub trade_count: usize,
    pub calmar: f64,
    pub var_95: f64,
    pub avg_win_loss: f64,
    pub expectancy: f64,
    pub recovery_factor: f64,
    pub brier_score: Option<f64>,
    pub avg_win: f64,
    pub avg_loss: f64,
    /// Cumulative equity curve (cumulative PnL per trade).
    pub equity_curve: Vec<f64>,
}

impl PerformanceMetrics {
    /// Return key metrics as a flat vector matching the GUI results panel order.
    pub fn to_vec(&self) -> Vec<f64> {
        vec![
            self.trade_count as f64,
            self.total_return,
            self.sharpe,
            self.sortino,
            self.max_dd_pct,
            self.win_rate,
            self.profit_factor,
            self.expectancy,
            self.calmar,
            self.var_95,
            self.recovery_factor,
            self.brier_score.unwrap_or(0.0),
        ]
    }

    /// Compute all metrics from a trade sequence.
    pub fn compute(trades: &[Trade]) -> Self {
        if trades.is_empty() {
            return Self::default();
        }

        let n = trades.len();
        let pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
        let total_return: f64 = pnls.iter().sum();
        // Equity curve: cumulative sum of PnL
        let equity_curve: Vec<f64> = pnls
            .iter()
            .scan(0.0, |acc, &p| {
                *acc += p;
                Some(*acc)
            })
            .collect();

        // Win / loss splits
        let wins: Vec<f64> = pnls.iter().copied().filter(|&p| p > 0.0).collect();
        let losses: Vec<f64> = pnls.iter().copied().filter(|&p| p < 0.0).collect();
        let win_rate = wins.len() as f64 / n as f64;
        let avg_win = if wins.is_empty() {
            0.0
        } else {
            wins.iter().sum::<f64>() / wins.len() as f64
        };
        let avg_loss = if losses.is_empty() {
            0.0
        } else {
            losses.iter().sum::<f64>().abs() / losses.len() as f64
        };
        let gross_profit: f64 = wins.iter().sum();
        let gross_loss: f64 = losses.iter().sum::<f64>().abs();
        let profit_factor = if gross_loss > 0.0 {
            gross_profit / gross_loss
        } else {
            f64::INFINITY
        };
        let avg_win_loss = if avg_loss > 0.0 {
            avg_win / avg_loss
        } else {
            f64::INFINITY
        };
        let expectancy = win_rate * avg_win - (1.0 - win_rate) * avg_loss;

        // Sharpe & Sortino (annualized, assuming daily trades)
        let mean_pnl = total_return / n as f64;
        let variance = pnls.iter().map(|p| (p - mean_pnl).powi(2)).sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();
        let sharpe = if std_dev > 0.0 {
            (mean_pnl / std_dev) * (252.0_f64).sqrt()
        } else {
            0.0
        };

        let downside_var = pnls
            .iter()
            .filter(|&&p| p < 0.0)
            .map(|p| p.powi(2))
            .sum::<f64>()
            / n as f64;
        let downside_dev = downside_var.sqrt();
        let sortino = if downside_dev > 0.0 {
            (mean_pnl / downside_dev) * (252.0_f64).sqrt()
        } else {
            0.0
        };

        // Drawdown
        let (max_drawdown, max_dd_pct) = compute_max_drawdown(&pnls);

        // CAGR (assume 252 trading days)
        let years = n as f64 / 252.0;
        let start_capital = 10_000.0;
        let end_capital = start_capital + total_return;
        let cagr = if years > 0.0 && end_capital > 0.0 {
            (end_capital / start_capital).powf(1.0 / years) - 1.0
        } else {
            0.0
        };

        // Calmar = CAGR / |MaxDD|
        let calmar = if max_dd_pct.abs() > 0.0 {
            cagr / max_dd_pct.abs()
        } else {
            0.0
        };

        // VaR 95% (historical)
        let var_95 = percentile(&pnls, 5.0);

        // Recovery factor
        let recovery_factor = if max_drawdown.abs() > 0.0 {
            total_return / max_drawdown.abs()
        } else {
            0.0
        };

        // Brier score (if trades have prediction data)
        let brier = compute_brier(trades);

        Self {
            total_return,
            cagr,
            sharpe,
            sortino,
            max_drawdown,
            max_dd_pct,
            win_rate,
            profit_factor,
            trade_count: n,
            calmar,
            var_95,
            avg_win_loss,
            expectancy,
            recovery_factor,
            brier_score: brier,
            avg_win,
            avg_loss,
            equity_curve,
        }
    }
}

fn compute_max_drawdown(pnls: &[f64]) -> (f64, f64) {
    let start = 10_000.0;
    let mut peak = start;
    let mut max_dd = 0.0_f64;
    let mut equity = start;
    for &pnl in pnls {
        equity += pnl;
        peak = peak.max(equity);
        let dd = equity - peak;
        max_dd = max_dd.min(dd);
    }
    let max_dd_pct = if peak > 0.0 { max_dd / peak } else { 0.0 };
    (max_dd, max_dd_pct)
}

fn percentile(data: &[f64], pct: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f64> = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn compute_brier(trades: &[Trade]) -> Option<f64> {
    let resolved: Vec<_> = trades
        .iter()
        .filter(|t| t.entry_price > 0.0 && t.entry_price < 1.0)
        .collect();
    if resolved.len() < 2 {
        return None;
    }
    let sum: f64 = resolved
        .iter()
        .map(|t| {
            let outcome = if t.exit_price > 0.5 { 1.0 } else { 0.0 };
            (t.entry_price - outcome).powi(2)
        })
        .sum();
    Some(sum / resolved.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trades(pnls: &[f64]) -> Vec<Trade> {
        use crate::shared::strategy::Side;
        pnls.iter()
            .map(|&p| Trade {
                timestamp: 0,
                side: Side::Yes,
                size: 1.0,
                entry_price: 0.5,
                exit_price: 0.5 + p,
                pnl: p,
                strategy_id: "test".into(),
            })
            .collect()
    }

    #[test]
    fn equity_curve_is_cumulative_sum() {
        let trades = make_trades(&[1.0, -0.5, 2.0]);
        let m = PerformanceMetrics::compute(&trades);
        assert_eq!(m.equity_curve.len(), 3);
        assert!((m.equity_curve[0] - 1.0).abs() < 1e-9);
        assert!((m.equity_curve[1] - 0.5).abs() < 1e-9);
        assert!((m.equity_curve[2] - 2.5).abs() < 1e-9);
    }

    #[test]
    fn equity_curve_empty_for_no_trades() {
        let m = PerformanceMetrics::compute(&[]);
        assert!(m.equity_curve.is_empty());
    }
}

/// Compute drawdown series (underwater equity).
pub fn drawdown_series(pnls: &[f64]) -> Vec<f64> {
    let start = 10_000.0;
    let mut peak = start;
    let mut equity = start;
    pnls.iter()
        .map(|&pnl| {
            equity += pnl;
            peak = peak.max(equity);
            equity - peak
        })
        .collect()
}
