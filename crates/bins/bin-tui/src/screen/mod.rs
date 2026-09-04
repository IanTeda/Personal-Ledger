//! The per-screen model each screen implements — the "Component" half of ADR-0003's hybrid
//! Elm/Component architecture (`docs/adr/0003-hybrid-tea-component-tui-architecture.md`).
//! `App` owns the navigation stack ("Decide TUI screen map and navigation shape"); each
//! screen owns its own state, renders itself, and gets first refusal on raw key input
//! ("Decide keybinding and navigation/workflow scheme") before `App` falls back to the
//! small, truly-global key set (`Ctrl+C`, `Esc`).

pub mod candlestick_chart;
pub mod categories;
pub mod dashboard;
pub mod divergent_chart;
pub mod doughnut_chart;
pub mod help;
pub mod line_chart;
pub mod settings;
pub mod table;

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::{Action, InputMode};

/// A single screen on the navigation stack.
pub trait Screen {
    /// Called once when the screen is pushed onto the stack, with a sender any background
    /// work (e.g. loading real data) can use to report results back as an [`Action`]. Most
    /// screens have no background work and can rely on this default no-op.
    fn init(&mut self, _action_tx: UnboundedSender<Action>) {}

    /// First refusal on a raw key press while this screen is at the top of the stack:
    /// interpret it as a screen-local [`Action`], or return `None` to fall through to the
    /// small global key set (`Ctrl+C`, `Esc`) `App` handles itself. The default no-op means
    /// a screen with nothing screen-specific to bind just gets the global keys.
    fn handle_key(&mut self, _key: KeyEvent, _mode: InputMode) -> Option<Action> {
        None
    }

    /// Reacts to an [`Action`] that wasn't handled at the `App` level.
    fn update(&mut self, action: &Action);

    /// Renders the screen into the given area of the frame.
    fn view(&self, frame: &mut Frame, area: Rect);

    /// Short name shown in the app's breadcrumb/footer.
    fn title(&self) -> &'static str;
}
