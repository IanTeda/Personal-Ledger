//! The shell's top bar (`docs/ux/desktop/Shell & Navigation/README.md`'s "1a" spec, "Top
//! bar" component): rail toggle, brand mark, sync indicator, window controls. The rail toggle
//! is clickable (issue #152). The spec's own `:` palette-hint box was dropped from here --
//! palette entry is keyboard-only (`:`), and the status line's own hint strip already names it.
//!
//! The mockup draws a "sync" icon next to the sync indicator, but no Lucide name for it
//! appears in the handoff's own "Assets" table -- rather than invent one outside that table,
//! the sync indicator here is text-only.
//!
//! Window controls (minimize/maximize/close, issue #162) fill the handoff's own "3 window
//! controls" line -- present in the spec's text but absent from both its HTML mockup and its
//! Assets table, so there's no visual reference to match; laid out and iconed to the
//! conventional cross-platform minimize/maximize/close order and style instead. Unlike the
//! rail toggle, none of the three touch `Shell`/`NavState`, so they're wired directly here
//! (`Window::minimize_window`/`Window::zoom_window`/`App::quit`) rather than threaded through
//! a callback the way `OnRailToggle` is.

use std::rc::Rc;

use gpui::{App, ClickEvent, Window, div, prelude::*, px};
use gpui_component::Sizable;

use crate::{icon::DesktopIcon, nav::Noun, theme::color};

/// Band height: `docs/ux/desktop/Shell & Navigation/README.md`'s "Layout" table.
pub const HEIGHT: gpui::Pixels = px(48.0);

/// The rail-toggle button's own click, reported raw -- `Shell::handle_toggle_rail` owns what
/// it actually does. `Rc` since `TopBar` is rebuilt fresh every render (see the module doc).
pub type OnRailToggle = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct TopBar {
    on_rail_toggle: OnRailToggle,
    active_noun: Noun,
}

impl TopBar {
    pub fn new(on_rail_toggle: OnRailToggle, active_noun: Noun) -> Self {
        Self {
            on_rail_toggle,
            active_noun,
        }
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
            .child(rail_toggle(self.on_rail_toggle))
            .child(brand_mark(self.active_noun))
            .child(div().flex_1())
            .child(sync_indicator())
            .child(window_controls())
    }
}

fn rail_toggle(on_rail_toggle: OnRailToggle) -> impl IntoElement {
    div()
        .id("topbar-rail-toggle")
        .w(px(28.0))
        .h(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|this| this.bg(color::HOVER_TINT))
        .on_click(move |event, window, cx| on_rail_toggle(event, window, cx))
        .child(
            DesktopIcon::RailToggle
                .icon()
                .with_size(px(16.0))
                .text_color(color::INK),
        )
}

/// The brand tile names the active screen too (e.g. "Personal Ledger | Dashboard"), so it
/// changes with `NavState::noun` -- the status line's own bottom-right breadcrumb used to
/// duplicate this same fact and was dropped in favour of naming it once, here. The open
/// Ledger's own file path lives at the status line's own bottom right instead
/// (`crate::statusline`), not here.
fn brand_mark(active_noun: Noun) -> impl IntoElement {
    div()
        .flex()
        .items_baseline()
        .gap(px(6.0))
        .text_size(px(13.5))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child("Personal Ledger"),
        )
        .child(div().text_color(color::INK_TERTIARY).child("|"))
        .child(format!("{active_noun:?}"))
}

fn sync_indicator() -> impl IntoElement {
    div()
        .ml(px(6.0))
        .text_size(px(12.0))
        .text_color(color::INK_SECONDARY)
        // Representative content -- a real last-write timestamp lands with the sync ticket.
        .child("synced 14:22")
}

/// Minimize/maximize/close, in that order -- the conventional cross-platform window-control
/// layout (GNOME, Windows), close trailing at the window's own edge.
fn window_controls() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .ml(px(6.0))
        .child(window_control_button(
            "topbar-window-minimize",
            DesktopIcon::WindowMinimize,
            |_event, window, _cx| window.minimize_window(),
        ))
        .child(window_control_button(
            "topbar-window-maximize",
            DesktopIcon::WindowMaximize,
            |_event, window, _cx| window.zoom_window(),
        ))
        .child(window_control_button(
            "topbar-window-close",
            DesktopIcon::WindowClose,
            |_event, _window, cx| cx.quit(),
        ))
}

/// One window-control button, mirroring `rail_toggle`'s own size/hover/click shape. Unlike
/// `rail_toggle`, `on_click` is a plain `'static` closure rather than an `Rc`-shared one --
/// none of the three touch `Shell` state, so there's nothing for a caller to own or clone in.
fn window_control_button(
    id: &'static str,
    icon: DesktopIcon,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w(px(28.0))
        .h(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|this| this.bg(color::HOVER_TINT))
        .on_click(on_click)
        .child(icon.icon().with_size(px(14.0)).text_color(color::INK))
}
