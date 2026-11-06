//! The Shell's gpui View state, which `Root` owns as its `state` field: the hosted Dialog Entity
//! and its Subscription, the collapsed rail's hover tooltip and its generation, and whether the top
//! bar's recent-actions list is open. These are gpui View state, not a stand-in for Presenter
//! state, so they do not move into `lib_shell` and do not become Entities (ADR-0032). The status
//! message, the Toasts, focus, modes, the rail's `b` override and the Dialog keymap are the Shell's
//! Presenter's (`lib_shell`), and so is the command palette's input, selection and history (#1063):
//! only its row list's scroll handle is View state here.

use crate::{navigation::nav::Noun, shell::view::dialog_entity::DialogHandle};

pub(crate) struct ShellState {
    /// The command palette's row list scroll, kept on the selected row after each palette key
    /// and fresh each time the palette opens.
    pub(crate) palette_scroll: gpui::ScrollHandle,
    /// The collapsed primary rail's row whose hover has settled past
    /// `TOOLTIP_REVEAL_DELAY` -- `None` while nothing's hovered, the delay hasn't elapsed
    /// yet, or the rail isn't collapsed (see `shell::view::rail::primary::PrimaryRail`, which only wires
    /// hover at all in its collapsed rendering).
    pub(crate) collapsed_rail_tooltip: Option<Noun>,
    /// Bumped on every hover transition; a pending reveal timer checks this against the value
    /// it captured before applying, so hovering a second row (or leaving the rail entirely)
    /// before the first row's delay elapses can't reveal the wrong tooltip.
    pub(crate) hover_generation: u64,
    /// The open module Dialog, an Entity holding its Presenter (`crate::shell::view::dialog_entity`),
    /// and the subscription that closes it once its Presenter does. Only `Root::host_dialog` and
    /// `Root::close_dialog` change it, so `NavState::mode` is `InputMode::Dialog` for exactly as
    /// long as this is `Some`.
    pub(crate) hosted: Option<(DialogHandle, gpui::Subscription)>,
    /// The top bar's recent-actions list below Undo is open.
    pub(crate) undo_menu_open: bool,
}
