//! Per-tab key event handlers. Each module handles key events for one tab.
//! All handlers take mutable references to the relevant state slices.

pub mod backtester;
pub mod copy;
pub mod crypto;
pub mod discover;
pub mod global;
pub mod sports;
pub mod weather;

/// Sports tab UI state, grouped for clean handler signatures.
#[derive(Default)]
pub struct SportsUiState {
    pub pane: usize,
    pub strategy_sel: usize,
    pub signal_sel: usize,
    pub fixture_sel: usize,
    pub league_filter: Option<String>,
    pub team_filter: Option<String>,
    pub show_league_picker: bool,
    pub show_team_picker: bool,
    pub league_picker_sel: usize,
    pub team_picker_sel: usize,
}
