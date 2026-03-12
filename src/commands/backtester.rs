//! Backtester tab commands.

use crate::app_state::AppState;
use crate::commands::dto::backtester::*;
use myoro_trading_kit::backtester::BacktestTool;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tauri::State;

#[tauri::command]
pub fn get_backtester_state(state: State<AppState>) -> BacktesterStateDto {
    let bt = &state.backtester;
    let params_guard = bt.tool_params.read().unwrap_or_else(|e| e.into_inner());
    let tools = BacktestTool::all()
        .iter()
        .enumerate()
        .map(|(i, tool)| {
            let params = params_guard
                .get(i)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|p| ToolParamDto {
                    name: p.name.clone(),
                    value: p.value,
                    min: p.min,
                    max: p.max,
                    step: p.step,
                })
                .collect();
            ToolDto {
                idx: i,
                name: tool.name().to_string(),
                tooltip: tool.tooltip().to_string(),
                params,
            }
        })
        .collect();
    drop(params_guard);

    BacktesterStateDto {
        strategies: bt
            .strategies
            .iter()
            .map(|s| StrategyEntryDto {
                id: s.id.clone(),
                name: s.name.clone(),
                domain: s.domain.clone(),
            })
            .collect(),
        data_sources: bt
            .data_sources
            .iter()
            .map(|s| DataSourceDto {
                id: s.id.clone(),
                name: s.name.clone(),
                domain: s.domain.clone(),
            })
            .collect(),
        tools,
        is_running: bt.is_running.load(Ordering::SeqCst),
    }
}

#[tauri::command]
pub fn backtester_load_trades(state: State<AppState>, strategy_idx: usize, data_source_idx: usize) {
    state.backtester.load_trades(strategy_idx, data_source_idx);
}

#[tauri::command]
pub fn backtester_run_tool(state: State<AppState>, tool_idx: usize) {
    let bt = state.backtester.clone();
    std::thread::spawn(move || {
        bt.run_tool(tool_idx);
    });
}

#[tauri::command]
pub fn backtester_set_param(state: State<AppState>, tool_idx: usize, param_idx: usize, value: f64) {
    if let Ok(mut params) = state.backtester.tool_params.write() {
        if let Some(tool_params) = params.get_mut(tool_idx) {
            if let Some(p) = tool_params.get_mut(param_idx) {
                p.value = value.clamp(p.min, p.max);
            }
        }
    }
}

#[tauri::command]
pub fn get_backtester_results(state: State<AppState>) -> BacktesterResultsDto {
    let bt = &state.backtester;
    let is_running = bt.is_running.load(Ordering::SeqCst);

    let metrics_map: HashMap<String, f64> = bt
        .current_metrics
        .read()
        .ok()
        .and_then(|m| m.clone())
        .map(|m| {
            let mut map = HashMap::new();
            map.insert("total_return".into(), m.total_return);
            map.insert("sharpe".into(), m.sharpe);
            map.insert("sortino".into(), m.sortino);
            map.insert("max_drawdown_pct".into(), m.max_dd_pct);
            map.insert("win_rate".into(), m.win_rate);
            map.insert("profit_factor".into(), m.profit_factor);
            map.insert("trade_count".into(), m.trade_count as f64);
            map.insert("calmar".into(), m.calmar);
            map.insert("var_95".into(), m.var_95);
            map.insert("expectancy".into(), m.expectancy);
            map.insert("recovery_factor".into(), m.recovery_factor);
            map.insert("avg_win".into(), m.avg_win);
            map.insert("avg_loss".into(), m.avg_loss);
            map
        })
        .unwrap_or_default();

    let equity_curve = bt
        .current_metrics
        .read()
        .ok()
        .and_then(|m| m.clone())
        .map(|m| m.equity_curve)
        .unwrap_or_default();

    let drawdown_curve = compute_drawdown(&equity_curve);

    let trades = bt.trades.read().ok().map(|t| t.clone()).unwrap_or_default();
    let pnl_buckets = compute_pnl_buckets(&trades.iter().map(|t| t.pnl).collect::<Vec<_>>(), 20);

    let trade_list = trades
        .iter()
        .map(|t| BacktestTradeRowDto {
            strategy_id: t.strategy_id.clone(),
            side: match t.side {
                myoro_trading_kit::shared::strategy::Side::Yes => "YES".to_string(),
                myoro_trading_kit::shared::strategy::Side::No => "NO".to_string(),
            },
            entry_price: t.entry_price,
            exit_price: t.exit_price,
            size: t.size,
            pnl: t.pnl,
            timestamp: t.timestamp,
        })
        .collect();

    // mc_paths: use extra_curves if present (permutation/MC tools store paths there)
    let tool_result = bt.tool_result.read().ok().and_then(|r| r.clone());
    let mc_paths: Option<Vec<Vec<f64>>> = tool_result.as_ref().and_then(|r| {
        if r.extra_curves.is_empty() {
            None
        } else {
            Some(r.extra_curves.clone())
        }
    });
    let last_error: Option<String> = None; // errors are logged, not stored in ToolResult

    let tool_extra = tool_result.map(|r| r.summary).unwrap_or_default();

    BacktesterResultsDto {
        equity_curve,
        drawdown_curve,
        pnl_buckets,
        mc_paths,
        metrics: metrics_map,
        trade_list,
        is_running,
        last_error,
        tool_extra,
    }
}

fn compute_drawdown(equity: &[f64]) -> Vec<f64> {
    let mut peak = 0.0_f64;
    equity
        .iter()
        .map(|&v| {
            peak = peak.max(v);
            if peak == 0.0 {
                0.0
            } else {
                (peak - v) / peak * 100.0
            }
        })
        .collect()
}

fn compute_pnl_buckets(pnls: &[f64], bins: usize) -> Vec<(f64, u32)> {
    if pnls.is_empty() || bins == 0 {
        return Vec::new();
    }
    let min = pnls.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = pnls.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if (max - min).abs() < 1e-10 {
        return vec![(min, pnls.len() as u32)];
    }
    let width = (max - min) / bins as f64;
    let mut counts = vec![0u32; bins];
    for &p in pnls {
        let idx = ((p - min) / width).floor() as usize;
        let idx = idx.min(bins - 1);
        counts[idx] += 1;
    }
    counts
        .into_iter()
        .enumerate()
        .map(|(i, c)| (min + (i as f64 + 0.5) * width, c))
        .collect()
}
