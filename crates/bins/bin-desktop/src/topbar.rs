//! The shell's top bar (`docs/ux/desktop/Shell & Navigation/README.md`'s "1a" spec, "Top
//! bar" component): rail toggle, brand mark, palette hint, sync indicator. Static for now --
//! the rail toggle and palette hint become clickable once mode transitions and the rail
//! toggle action land (issues #149/#151).
//!
//! The mockup draws a "sync" icon next to the sync indicator, but no Lucide name for it
//! appears in the handoff's own "Assets" table -- rather than invent one outside that table,
//! the sync indicator here is text-only.

use gpui::{App, Window, div, prelude::*, px};
use gpui_component::Sizable;

use crate::{icon::DesktopIcon, theme::color};

/// Band height: `docs/ux/desktop/Shell & Navigation/README.md`'s "Layout" table.
pub const HEIGHT: gpui::Pixels = px(48.0);

#[derive(IntoElement)]
pub struct TopBar;

impl TopBar {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TopBar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for TopBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .h(HEIGHT)
            .flex_none()
            .flex()
            .items_center()
            .gap(px(12.0))
            .pl(px(12.0))
            .pr(px(10.0))
            .bg(color::CHROME)
            .border_b(px(2.0))
            .border_color(color::STRUCTURAL_RULE)
            .child(rail_toggle())
            .child(brand_mark())
            .child(div().flex_1())
            .child(palette_hint())
            .child(sync_indicator())
    }
}

fn rail_toggle() -> impl IntoElement {
    div()
        .w(px(28.0))
        .h(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            DesktopIcon::RailToggle
                .icon()
                .with_size(px(16.0))
                .text_color(color::INK),
        )
}

fn brand_mark() -> impl IntoElement {
    div()
        .flex()
        .items_baseline()
        .gap(px(8.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(13.5))
                .child("Personal Ledger"),
        )
        // Representative content -- the open Ledger's own name/base Unit, not a real file yet.
        .child(
            div()
                .text_size(px(12.0))
                .text_color(color::INK_TERTIARY)
                .child("teda.ledger · aud"),
        )
}

fn palette_hint() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .py(px(4.0))
        .px(px(9.0))
        .border(px(1.0))
        .border_color(gpui::rgba(0x201e1d4d)) // rgba(32,30,29,.30)
        .text_size(px(12.0))
        .text_color(color::INK_SECONDARY)
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::INK)
                .child(":"),
        )
        .child("run a command")
}

fn sync_indicator() -> impl IntoElement {
    div()
        .ml(px(6.0))
        .text_size(px(12.0))
        .text_color(color::INK_SECONDARY)
        // Representative content -- a real last-write timestamp lands with the sync ticket.
        .child("synced 14:22")
}
