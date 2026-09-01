//! The per-screen model each demo implements — the "Component" half of ADR-0003's hybrid
//! Elm/Component architecture (`docs/adr/0003-hybrid-tea-component-tui-architecture.md`).
//! `App` owns which screen is active; each screen owns its own state and renders itself.

pub mod candlestick_chart;
pub mod categories;
pub mod divergent_chart;
pub mod doughnut_chart;
pub mod line_chart;
pub mod table;

use ratatui::{Frame, layout::Rect};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;

/// A single screen of the application (one chart/table demo).
pub trait Screen {
    /// Called once when the app starts, with a sender any background work (e.g. loading real
    /// data) can use to report results back as an [`Action`]. Most screens have no background
    /// work and can rely on this default no-op.
    fn init(&mut self, _action_tx: UnboundedSender<Action>) {}

    /// Reacts to an [`Action`] that wasn't handled at the `App` level.
    fn update(&mut self, action: &Action);

    /// Renders the screen into the given area of the frame.
    fn view(&self, frame: &mut Frame, area: Rect);

    /// Short name shown in the app's tab bar.
    fn title(&self) -> &'static str;
}
