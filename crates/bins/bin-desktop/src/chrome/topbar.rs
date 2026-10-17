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

use crate::{
    assets::icon::DesktopIcon,
    chrome::text_fit::{text_width, truncate_start},
    theme::color,
};

/// Band height: `docs/ux/desktop-mockups/01-shell/README.md`'s "Layout" table.
pub const HEIGHT: Pixels = px(48.0);

const PADDING_LEFT: Pixels = px(12.0);
const PADDING_RIGHT: Pixels = px(10.0);
const GAP: Pixels = px(12.0);
const BUTTON: Pixels = px(28.0);
const CONTROLS_GAP: Pixels = px(4.0);
const CLUSTER_MARGIN: Pixels = px(6.0);
const BRAND_GAP: Pixels = px(6.0);
const WORDMARK_SIZE: Pixels = px(13.5);
const CONTEXT_SIZE: Pixels = px(12.0);

/// The rail-toggle button's own click, reported raw -- `Shell::handle_toggle_rail` owns what
/// it actually does. `Rc` since `TopBar` is rebuilt fresh every render (see the module doc).
pub type OnRailToggle = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct TopBar {
    on_rail_toggle: OnRailToggle,
    /// The active View's title, from `ActiveView::title`.
    title: String,
    /// Extra context after the noun (6e's `import statement.csv`).
    context: Option<String>,
    /// Draw the wordmark alone: Settings at Medium, whose page heading is the trail already.
    hide_trail: bool,
}

impl TopBar {
    pub fn new(on_rail_toggle: OnRailToggle, title: String) -> Self {
        Self {
            on_rail_toggle,
            title,
            context: None,
            hide_trail: false,
        }
    }

    pub fn hide_trail(mut self, hide_trail: bool) -> Self {
        self.hide_trail = hide_trail;
        self
    }

    pub fn context(mut self, context: Option<String>) -> Self {
        self.context = context;
        self
    }
}

impl RenderOnce for TopBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let trail = (!self.hide_trail).then(|| fit_trail(self.title, self.context, window));
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
            .child(brand_mark(trail, cx))
            .child(div().flex_1())
            .child(sync_indicator(cx))
            .child(window_controls(cx))
    }
}

/// The breadcrumb, `title` then `› context`, shortened from its start to the room the rest of the
/// bar leaves it, so its leaf always shows (DESKTOP-116). The wordmark, sync status and window
/// controls never shrink.
fn fit_trail(title: String, context: Option<String>, window: &Window) -> (String, Option<String>) {
    let measure = |text: &str, size: Pixels| text_width(window, text, size, FontWeight::NORMAL);
    let fixed = PADDING_LEFT
        + PADDING_RIGHT
        + BUTTON
        + GAP * 4.0
        + text_width(
            window,
            &lib_locale::msg::app_name(),
            WORDMARK_SIZE,
            FontWeight::EXTRA_BOLD,
        )
        + BRAND_GAP * 2.0
        + measure("|", WORDMARK_SIZE)
        + CLUSTER_MARGIN
        + measure(&sync_text(), CONTEXT_SIZE)
        + CLUSTER_MARGIN
        + BUTTON * 3.0
        + CONTROLS_GAP * 2.0;
    let room = window.viewport_size().width - fixed;
    let ellipsis = measure("\u{2026}", WORDMARK_SIZE);
    let Some(context) = context else {
        return (
            truncate_start(&title, room, |text| measure(text, WORDMARK_SIZE)),
            None,
        );
    };
    let context = format!("\u{203a} {context}");
    let context_room = room - BRAND_GAP;
    let title_room = context_room - measure(&context, CONTEXT_SIZE);
    if title_room >= ellipsis {
        let title = truncate_start(&title, title_room, |text| measure(text, WORDMARK_SIZE));
        return (title, Some(context));
    }
    let context = truncate_start(&context, context_room - ellipsis, |text| {
        measure(text, CONTEXT_SIZE)
    });
    ("\u{2026}".to_string(), Some(context))
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

/// The brand tile names the active screen too (e.g. "Personal Ledger | Dashboard"), so it
/// changes with the active View (`ActiveView::title`) -- the status line's own bottom-right
/// breadcrumb used to duplicate this same fact and was dropped in favour of naming it once, here.
/// The open Ledger's own file path lives at the status line's own bottom right instead
/// (`crate::chrome::statusline`), not here.
fn brand_mark(trail: Option<(String, Option<String>)>, cx: &App) -> impl IntoElement {
    let selector = if trail.is_some() {
        "topbar-brand"
    } else {
        "topbar-brand-alone"
    };
    div()
        .debug_selector(move || selector.to_string())
        .flex()
        .items_baseline()
        .gap(BRAND_GAP)
        .min_w(px(0.0))
        .whitespace_nowrap()
        .text_size(WORDMARK_SIZE)
        .child(
            div()
                .font_weight(FontWeight::EXTRA_BOLD)
                .child(lib_locale::msg::app_name()),
        )
        .when_some(trail, |this, (title, context)| {
            this.child(div().text_color(color::faint_text(cx)).child("|"))
                .child(
                    div()
                        .debug_selector(|| "topbar-trail".to_string())
                        .child(title),
                )
                .children(context.map(|context| {
                    div()
                        .text_size(CONTEXT_SIZE)
                        .text_color(color::faint_text(cx))
                        .child(context)
                }))
        })
}

fn sync_indicator(cx: &App) -> impl IntoElement {
    div()
        .ml(CLUSTER_MARGIN)
        .whitespace_nowrap()
        .text_size(CONTEXT_SIZE)
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
