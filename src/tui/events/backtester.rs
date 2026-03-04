//! Key event handler for the Backtester tab.
//! Handles: r=run, Enter=edit param, ←/→=adjust param, Esc=cancel edit.
//! ↑/↓ and Tab navigation handled in router.rs.
use crossterm::event::KeyCode;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::backtester::BacktesterState;

/// Handle backtester-specific keys. Returns true if key was consumed.
#[allow(clippy::too_many_arguments)]
pub fn handle_key(
    code: KeyCode,
    backtester: &Arc<BacktesterState>,
    strategy_sel: usize,
    data_sel: usize,
    tool_sel: usize,
    param_sel: usize,
    param_editing: &mut bool,
    param_input: &mut String,
    focused_pane: usize,
) -> bool {
    match code {
        // Run analysis
        KeyCode::Char('r') if !*param_editing => {
            if !backtester.is_running.load(Ordering::SeqCst) {
                let bt = Arc::clone(backtester);
                let s_sel = strategy_sel;
                let d_sel = data_sel;
                let t_sel = tool_sel;
                std::thread::spawn(move || {
                    bt.load_trades(s_sel, d_sel);
                    bt.run_tool(t_sel);
                });
            }
            true
        }
        // Enter = start editing param (when on params pane)
        KeyCode::Enter if focused_pane == 3 && !*param_editing => {
            if let Ok(params) = backtester.tool_params.read() {
                if let Some(tool_params) = params.get(tool_sel) {
                    if let Some(p) = tool_params.get(param_sel) {
                        *param_editing = true;
                        *param_input = format_val(p.value, p.step);
                        return true;
                    }
                }
            }
            false
        }
        // Confirm edit
        KeyCode::Enter if *param_editing => {
            if let Ok(val) = param_input.parse::<f64>() {
                if let Ok(mut params) = backtester.tool_params.write() {
                    if let Some(tool_params) = params.get_mut(tool_sel) {
                        if let Some(p) = tool_params.get_mut(param_sel) {
                            p.value = val.clamp(p.min, p.max);
                        }
                    }
                }
            }
            *param_editing = false;
            param_input.clear();
            true
        }
        // Cancel edit
        KeyCode::Esc if *param_editing => {
            *param_editing = false;
            param_input.clear();
            true
        }
        // Type into param
        KeyCode::Char(c) if *param_editing => {
            if c.is_ascii_digit() || c == '.' || c == '-' {
                param_input.push(c);
            }
            true
        }
        KeyCode::Backspace if *param_editing => {
            param_input.pop();
            true
        }
        // ← decrement param, → increment param (when on params pane, not editing)
        KeyCode::Left if focused_pane == 3 && !*param_editing => {
            if let Ok(mut params) = backtester.tool_params.write() {
                if let Some(tool_params) = params.get_mut(tool_sel) {
                    if let Some(p) = tool_params.get_mut(param_sel) {
                        p.decrement();
                    }
                }
            }
            true
        }
        KeyCode::Right if focused_pane == 3 && !*param_editing => {
            if let Ok(mut params) = backtester.tool_params.write() {
                if let Some(tool_params) = params.get_mut(tool_sel) {
                    if let Some(p) = tool_params.get_mut(param_sel) {
                        p.increment();
                    }
                }
            }
            true
        }
        _ => false,
    }
}

fn format_val(val: f64, step: f64) -> String {
    if step >= 1.0 {
        format!("{:.0}", val)
    } else {
        format!("{:.2}", val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_runs_analysis() {
        let bt = BacktesterState::new();
        let mut editing = false;
        let mut input = String::new();
        let consumed = handle_key(
            KeyCode::Char('r'),
            &bt,
            0,
            1,
            3,
            0,
            &mut editing,
            &mut input,
            0,
        );
        assert!(consumed);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn enter_starts_param_edit() {
        let bt = BacktesterState::new();
        let mut editing = false;
        let mut input = String::new();
        // tool_sel=0 (PermTradeOrder) has params
        let consumed = handle_key(KeyCode::Enter, &bt, 0, 0, 0, 0, &mut editing, &mut input, 3);
        assert!(consumed);
        assert!(editing);
    }

    #[test]
    fn left_right_adjust_param() {
        let bt = BacktesterState::new();
        let mut editing = false;
        let mut input = String::new();
        let before = bt.tool_params.read().unwrap()[0][0].value;
        handle_key(KeyCode::Right, &bt, 0, 0, 0, 0, &mut editing, &mut input, 3);
        let after = bt.tool_params.read().unwrap()[0][0].value;
        assert!(after > before);
    }
}
