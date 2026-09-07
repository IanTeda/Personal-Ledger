//! The per-view model each `View` implements, hosted by `Shell` — the new navigation shape
//! ADR-0013 (`docs/adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md`) introduces in
//! place of the breadcrumb `App`/`Screen` stack (`app.rs`/`screen/`, left compiling but
//! disconnected from `main.rs`). `Shell` hosts exactly one active `View` at a time — no
//! navigation stack, no breadcrumb; the shell's own command window (a later ticket) is the
//! only way the design specifies for switching which `View` is active.

pub mod dashboard;

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use tokio::sync::mpsc::UnboundedSender;

/// A message `Shell` or the active `View` reacts to. Deliberately minimal for now — the full
/// action registry the command window will dispatch through (ADR-0013) is later work; this
/// only needs enough to drive the event loop and let a `View` react to a tick.
///
/// The `Palette*` variants drive `Shell`'s own command window (`crate::command_palette`)
/// rather than the active `View` — they exist here because `Shell`'s event loop only redraws
/// in response to an `Action`, and every `View::update` implementation ignores variants it
/// doesn't care about, so adding shell-chrome messages alongside `Quit`/`Tick` costs a `View`
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// A periodic tick, driving redraws even without input.
    Tick,
    /// `Ctrl+C` — the hard-quit safety net, recognised by `Shell` itself before any `View`
    /// sees the key.
    Quit,
    /// `Ctrl+;` — opens the command palette overlay.
    OpenPalette,
    /// `Esc`, or `Ctrl+;` again — closes the command palette overlay.
    ClosePalette,
    /// A printable character typed while the palette is open, appended to its input buffer.
    PaletteInput(char),
    /// `Backspace` while the palette is open — removes the last character of its input.
    PaletteBackspace,
    /// `↑` while the palette is open — moves the selection up one candidate row.
    PaletteMoveUp,
    /// `↓` while the palette is open — moves the selection down one candidate row.
    PaletteMoveDown,
}

/// The single view `Shell` hosts at a time.
pub trait View {
    /// Called once when the view becomes active, with a sender any background work (e.g. a
    /// data load) can use to report results back as an [`Action`]. Most views have no
    /// background work and can rely on this default no-op.
    fn init(&mut self, _action_tx: UnboundedSender<Action>) {}

    /// First refusal on a raw key press while this view is active: interpret it as an
    /// [`Action`], or return `None` to fall through to `Shell`'s own global keys (`Ctrl+C`).
    /// The default no-op means a view with nothing view-specific to bind just gets the
    /// global keys.
    fn handle_key(&mut self, _key: KeyEvent) -> Option<Action> {
        None
    }

    /// Reacts to an [`Action`] that wasn't handled at the `Shell` level.
    fn update(&mut self, action: &Action);

    /// Renders the view into the given area of the frame — the full-bleed view region below
    /// the status line and above the command line.
    fn view(&self, frame: &mut Frame, area: Rect);

    /// Short name for the view, shown in the shell's status line.
    fn title(&self) -> &'static str;
}
