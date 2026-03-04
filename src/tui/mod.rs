//! Ratatui app: layout, screens (dashboard, crypto, sports, weather, logs).

mod app;
pub(crate) mod events;
pub mod layout;
pub(crate) mod presets;
pub mod router;
pub(crate) mod runner;
mod theme;
pub(crate) mod views;

pub use app::run;
