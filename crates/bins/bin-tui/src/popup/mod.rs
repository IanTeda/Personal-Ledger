//! Floating overlay windows hosted by `Shell` on top of whatever `View` is active — the
//! "centred floating overlay" treatment `docs/ux/tui/README.md` §3a specifies for the command
//! popup and `docs/ux/tui/units/README.md` "The forms" reuses verbatim for the unit add/edit/
//! delete dialogs ("same window treatment as the command palette"). Each popup gets its own
//! module here with its own state struct, the same way each `View` gets its own module under
//! `view/`.
//!
//! [`Dim`] is shared by every popup: the view behind an open overlay "stays visible and
//! heavily dimmed", never hidden or cleared, per §3a.

pub mod command;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::Widget,
};

/// A widget that dims an already-drawn area of the buffer without touching its colours, so the
/// view behind a popup stays visible and heavily dimmed, per §3a — `Buffer` has no public
/// per-cell mutation outside the `Widget`/render path, so this goes through one.
pub struct Dim;

impl Widget for Dim {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        buf.set_style(area, Style::default().add_modifier(Modifier::DIM));
    }
}
