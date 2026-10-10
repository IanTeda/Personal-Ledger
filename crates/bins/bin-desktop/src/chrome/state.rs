//! Chrome's own state: the frame's status message, the Toast stack and its keybindings, the
//! command palette, the open Dialog and the collapsed rail's hover tooltip. `Shell` owns one of
//! these as its `chrome` field; none of it is View state, so Chrome does not become an Entity
//! (ADR-0032).

use chrono::{DateTime, Local};
use lib_toast::Toasts;

use crate::{
    chrome::{dialog_host::OpenDialog, palette::Palette},
    navigation::nav::Noun,
    theme::colours::ColourChange,
};

pub(crate) struct ChromeState {
    /// Replaces the status line's hint strip until the next keypress -- the `g`-prefix's own
    /// "flash the hint strip" abort message, and the command palette's "not yet built" message
    /// once it closes back to `Normal` (see `Shell::run_command`).
    pub(crate) status_message: Option<String>,
    /// The Toasts raised this session (`crate::chrome::toast` draws them), stamped with the local time
    /// for the history. Advanced by `Shell::start_toast_clock`'s timer.
    pub(crate) toasts: Toasts<DateTime<Local>>,
    /// The pointer is over the Toast stack, which pauses the timers.
    pub(crate) toasts_hovered: bool,
    /// The `[keybindings] dismiss_toasts` spec, `ctrl+l` by default.
    pub(crate) dismiss_toasts_binding: String,
    /// The `[keybindings] toast_history` spec; unbound by default.
    pub(crate) toast_history_binding: Option<String>,
    /// Debug builds only: the Kind `F9` raises next, so each can be eyeballed.
    #[cfg(debug_assertions)]
    pub(crate) debug_toast_kind: usize,
    /// The command palette's own input/selection state -- `Some` only while
    /// `NavState::mode` is `InputMode::Command`, mirroring `bin-tui`'s own
    /// `Shell`'s `Option<popup::command::CommandPopup>` (`docs/ux/desktop-mockups/README.md`'s Notes).
    pub(crate) palette: Option<Palette>,
    /// The collapsed primary rail's row whose hover has settled past
    /// `TOOLTIP_REVEAL_DELAY` -- `None` while nothing's hovered, the delay hasn't elapsed
    /// yet, or the rail isn't collapsed (see `chrome::rail::primary::PrimaryRail`, which only wires
    /// hover at all in its collapsed rendering).
    pub(crate) collapsed_rail_tooltip: Option<Noun>,
    /// Bumped on every hover transition; a pending reveal timer checks this against the value
    /// it captured before applying, so hovering a second row (or leaving the rail entirely)
    /// before the first row's delay elapses can't reveal the wrong tooltip.
    pub(crate) hover_generation: u64,
    /// The open Dialog in the Dialog host (`crate::chrome::dialog_host`), if any. Only
    /// `Shell::open_dialog`/`Shell::close_dialog` change it, so `NavState::mode` is
    /// `InputMode::Dialog` for exactly as long as this is `Some`. Every feature's Dialog lives here.
    pub(crate) dialog: Option<OpenDialog>,
    /// A Colour Theme or Colour Appearance picked by a keystroke or palette command, applied by
    /// the caller once it has an `App` (the key handling runs without one).
    pub(crate) pending_colour_change: Option<ColourChange>,
}
