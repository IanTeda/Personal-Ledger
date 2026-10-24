//! The Shell's state the bin keeps: the command palette, the open Dialog, the rail's `b` override
//! and the collapsed rail's hover tooltip. `Root` owns one of these as its `state` field; none of
//! it is View state, so it does not become an Entity (ADR-0032). The status message, the Toasts,
//! focus and modes are the Shell's Presenter's (`lib_shell`).

use crate::{
    navigation::nav::Noun,
    shell::view::{
        dialog_entity::DialogHandle,
        dialog_host::{HostConfirm, OpenDialog},
        palette::Palette,
        rail::RailOverride,
    },
    theme::colours::ColourChange,
};

pub(crate) struct ShellState {
    /// The command palette's own input/selection state -- `Some` only while
    /// `NavState::mode` is `InputMode::Command`, mirroring `bin-tui`'s own
    /// `Shell`'s `Option<popup::command::CommandPopup>` (`docs/ux/desktop-mockups/README.md`'s Notes).
    pub(crate) palette: Option<Palette>,
    /// The collapsed primary rail's row whose hover has settled past
    /// `TOOLTIP_REVEAL_DELAY` -- `None` while nothing's hovered, the delay hasn't elapsed
    /// yet, or the rail isn't collapsed (see `shell::view::rail::primary::PrimaryRail`, which only wires
    /// hover at all in its collapsed rendering).
    pub(crate) collapsed_rail_tooltip: Option<Noun>,
    /// The rail `b` last set, with the Window Size it was set at; `Root::render` drops it once
    /// the window is drawn at another (ADR-0042).
    pub(crate) rail_override: Option<RailOverride>,
    /// Bumped on every hover transition; a pending reveal timer checks this against the value
    /// it captured before applying, so hovering a second row (or leaving the rail entirely)
    /// before the first row's delay elapses can't reveal the wrong tooltip.
    pub(crate) hover_generation: u64,
    /// The open Dialog in the Dialog host (`crate::shell::view::dialog_host`), if any. Only
    /// `Root::open_dialog`/`Root::close_dialog` change it, so `NavState::mode` is
    /// `InputMode::Dialog` for exactly as long as this is `Some`. Every feature's Dialog lives here.
    pub(crate) dialog: Option<OpenDialog>,
    /// The open Dialog's confirm, opener bound, while it is a hosted one
    /// (`dialog_host::open_dialog`). Set and cleared with `dialog`.
    pub(crate) dialog_confirm: Option<HostConfirm>,
    /// The open module Dialog, an Entity holding its Presenter (`crate::shell::view::dialog_entity`),
    /// and the subscription that closes it once its Presenter does. Opened and closed with
    /// `dialog` by the same two methods, so at most one of the two is ever `Some`.
    pub(crate) hosted: Option<(DialogHandle, gpui::Subscription)>,
    /// The keys every module Dialog shares, under `[keybindings]` (`lib_shell::dialog`).
    pub(crate) dialog_keymap: lib_viper::keymap::Keymap<lib_shell::DialogCommand>,
    /// A Colour Theme or Colour Appearance picked by a keystroke or palette command, applied by
    /// the caller once it has an `App` (the key handling runs without one).
    pub(crate) pending_colour_change: Option<ColourChange>,
    /// The top bar's recent-actions list below Undo is open.
    pub(crate) undo_menu_open: bool,
}
