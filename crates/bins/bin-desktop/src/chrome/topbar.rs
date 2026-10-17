//! The shell's top bar (`docs/ux/desktop-mockups/01-shell/README.md`'s "1a" spec, "Top
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

use gpui::{App, ClickEvent, FontWeight, Pixels, Window, div, prelude::*, px};
use gpui_component::Sizable;

use crate::{assets::icon::DesktopIcon, theme::color};

/// Band height: `docs/ux/desktop-mockups/01-shell/README.md`'s "Layout" table.
pub const HEIGHT: Pixels = px(48.0);

const PADDING_LEFT: Pixels = px(12.0);
const PADDING_RIGHT: Pixels = px(10.0);
const GAP: Pixels = px(12.0);
const BUTTON: Pixels = px(28.0);
const CONTROLS_GAP: Pixels = px(4.0);
const CLUSTER_MARGIN: Pixels = px(6.0);
const WORDMARK_SIZE: Pixels = px(13.5);
const SYNC_SIZE: Pixels = px(12.0);

/// The rail-toggle button's own click, reported raw -- `Shell::handle_toggle_rail` owns what
/// it actually does. `Rc` since `TopBar` is rebuilt fresh every render (see the module doc).
pub type OnRailToggle = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct TopBar {
    on_rail_toggle: OnRailToggle,
}

impl TopBar {
    pub fn new(on_rail_toggle: OnRailToggle) -> Self {
        Self { on_rail_toggle }
    }
}

impl RenderOnce for TopBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .h(HEIGHT)
            .flex_none()
            .flex()
            .items_center()
            .gap(GAP)
            .pl(PADDING_LEFT)
            .pr(PADDING_RIGHT)
            .bg(color::chrome(cx))
            .border_b(px(2.0))
            .border_color(color::structural_rule(cx))
            .child(rail_toggle(self.on_rail_toggle, cx))
            .child(brand_mark())
            .child(div().flex_1())
            .child(sync_indicator(cx))
            .child(window_controls(cx))
    }
}

fn rail_toggle(on_rail_toggle: OnRailToggle, cx: &App) -> impl IntoElement {
    div()
        .id("topbar-rail-toggle")
        .debug_selector(|| "topbar-rail-toggle".to_string())
        .w(BUTTON)
        .h(BUTTON)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|this| this.bg(color::hover(cx)))
        .on_click(move |event, window, cx| on_rail_toggle(event, window, cx))
        .child(
            DesktopIcon::RailToggle
                .icon()
                .with_size(px(16.0))
                .text_color(color::foreground(cx)),
        )
}

/// The wordmark alone, with no trail naming the screen ("Personal Ledger", not "Personal Ledger |
/// Dashboard"): every View's own heading already says where the user is. The open Ledger's own
/// file path lives at the status line's own bottom right instead (`crate::chrome::statusline`).
fn brand_mark() -> impl IntoElement {
    div()
        .debug_selector(|| "topbar-brand".to_string())
        .whitespace_nowrap()
        .text_size(WORDMARK_SIZE)
        .font_weight(FontWeight::EXTRA_BOLD)
        .child(lib_locale::msg::app_name())
}

fn sync_indicator(cx: &App) -> impl IntoElement {
    div()
        .ml(CLUSTER_MARGIN)
        .whitespace_nowrap()
        .text_size(SYNC_SIZE)
        .text_color(color::muted(cx))
        .child(sync_text())
}

// Representative content -- a real last-write timestamp lands with the sync ticket.
fn sync_text() -> String {
    crate::msg::desktop_topbar_synced("14:22")
}

/// Minimize/maximize/close, in that order -- the conventional cross-platform window-control
/// layout (GNOME, Windows), close trailing at the window's own edge.
fn window_controls(cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(CONTROLS_GAP)
        .ml(CLUSTER_MARGIN)
        .child(window_control_button(
            "topbar-window-minimize",
            DesktopIcon::WindowMinimize,
            |_event, window, _cx| window.minimize_window(),
            cx,
        ))
        .child(window_control_button(
            "topbar-window-maximize",
            DesktopIcon::WindowMaximize,
            |_event, window, _cx| window.zoom_window(),
            cx,
        ))
        .child(window_control_button(
            "topbar-window-close",
            DesktopIcon::WindowClose,
            |_event, _window, cx| cx.quit(),
            cx,
        ))
}

/// One window-control button, mirroring `rail_toggle`'s own size/hover/click shape. Unlike
/// `rail_toggle`, `on_click` is a plain `'static` closure rather than an `Rc`-shared one --
/// none of the three touch `Shell` state, so there's nothing for a caller to own or clone in.
fn window_control_button(
    id: &'static str,
    icon: DesktopIcon,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    div()
        .id(id)
        .w(BUTTON)
        .h(BUTTON)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|this| this.bg(color::hover(cx)))
        .on_click(on_click)
        .child(
            icon.icon()
                .with_size(px(14.0))
                .text_color(color::foreground(cx)),
        )
}
