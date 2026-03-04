//! Transaction Cost Analysis: models spread, slippage, and commission erosion.
use super::get_param;
use crate::backtester::data::{ToolParam, Trade};
use crate::backtester::metrics::PerformanceMetrics;
use crate::backtester::tools::ToolResult;

pub fn default_params() -> Vec<ToolParam> {
    vec![
        ToolParam::new("Commission %", 0.10, 0.0, 10.0, 0.05),
        ToolParam::new("Slippage bps", 5.0, 0.0, 50.0, 1.0),
        ToolParam::new("Spread bps", 10.0, 0.0, 100.0, 5.0),
    ]
}

pub fn analyze(trades: &[Trade], params: &[ToolParam]) -> ToolResult {
    let commission_pct = get_param(params, "Commission %", 0.10) / 100.0;
    let slippage_bps = get_param(params, "Slippage bps", 5.0) / 10_000.0;
    let spread_bps = get_param(params, "Spread bps", 10.0) / 10_000.0;

    if trades.is_empty() {
        return ToolResult {
            summary: vec![("Error".into(), "No trades to analyze.".into())],
            ..Default::default()
        };
    }

    let gross = PerformanceMetrics::compute(trades);

    // Apply costs to each trade
    let net_trades: Vec<Trade> = trades
        .iter()
        .map(|t| {
            let cost = t.size * (commission_pct + slippage_bps + spread_bps);
            let net_pnl = t.pnl - cost;
            Trade {
                pnl: net_pnl,
                exit_price: t.entry_price + net_pnl / t.size.max(1.0),
                ..t.clone()
            }
        })
        .collect();

    let net = PerformanceMetrics::compute(&net_trades);
    let total_cost: f64 = trades
        .iter()
        .map(|t| t.size * (commission_pct + slippage_bps + spread_bps))
        .sum();

    let erosion = if gross.total_return.abs() > 0.0 {
        (1.0 - net.total_return / gross.total_return) * 100.0
    } else {
        0.0
    };

    let curve_gross = crate::backtester::data::equity_curve(trades);
    let curve_net = crate::backtester::data::equity_curve(&net_trades);

    ToolResult {
        summary: vec![
            ("Gross Return".into(), format!("{:.2}", gross.total_return)),
            ("Net Return".into(), format!("{:.2}", net.total_return)),
            ("Total Costs".into(), format!("{:.2}", total_cost)),
            ("Return Erosion".into(), format!("{:.1}%", erosion)),
            ("Gross Sharpe".into(), format!("{:.3}", gross.sharpe)),
            ("Net Sharpe".into(), format!("{:.3}", net.sharpe)),
            (
                "Gross Win Rate".into(),
                format!("{:.1}%", gross.win_rate * 100.0),
            ),
            (
                "Net Win Rate".into(),
                format!("{:.1}%", net.win_rate * 100.0),
            ),
        ],
        equity_curve: curve_gross,
        extra_curves: vec![curve_net],
        detail_lines: vec![
            format!(
                "Costs: {:.2}% commission + {:.1}bps slippage + {:.1}bps spread",
                commission_pct * 100.0,
                slippage_bps * 10_000.0,
                spread_bps * 10_000.0
            ),
            format!("Gross→Net erosion: {:.1}%", erosion),
        ],
        ..Default::default()
    }
}
