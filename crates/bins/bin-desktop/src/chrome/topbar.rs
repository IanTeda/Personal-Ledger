//! The shell's top bar (`docs/ux/desktop-mockups/01-shell/README.md`'s "1a" spec, "Top
//! bar" component): rail toggle, Back/Forward, brand mark, Undo with its recent-actions list,
//! sync indicator, window controls. The rail toggle
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

use gpui::{
    App, ClickEvent, FontWeight, MouseButton, Pixels, Size, Window, div, point, prelude::*, px,
};
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
/// The caret half of the Undo split button: narrower than a full button, so it reads as Undo's.
const CARET_WIDTH: Pixels = px(16.0);
const MENU_WIDTH: Pixels = px(280.0);
const MENU_TEXT_SIZE: Pixels = px(12.0);

/// The rail-toggle button's own click, reported raw -- `Shell::handle_toggle_rail` owns what
/// it actually does. `Rc` since `TopBar` is rebuilt fresh every render (see the module doc).
pub type OnRailToggle = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// A click on one of the history buttons (Back, Forward, Undo and its list), reported for
/// `Shell::handle_topbar_action` to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBarAction {
    Back,
    Forward,
    Undo,
    ToggleUndoMenu,
    /// Undo back to the recent action at this index, `0` the newest.
    UndoTo(usize),
    /// A click outside the open recent-actions list.
    DismissUndoMenu,
}

pub type OnTopBarAction = Rc<dyn Fn(TopBarAction, &mut Window, &mut App)>;

/// How many recent actions the Undo caret lists.
pub const RECENT_ACTIONS: usize = 6;

/// The recent actions the Undo caret lists, newest first. Representative content -- the
/// Ledger keeps no Undo history yet, so these name the kind of action one would hold.
pub fn recent_actions() -> [String; RECENT_ACTIONS] {
    [
        crate::msg::desktop_topbar_undo_action_categorise(),
        crate::msg::desktop_topbar_undo_action_add_transaction(),
        crate::msg::desktop_topbar_undo_action_rename_payee(),
        crate::msg::desktop_topbar_undo_action_tag(),
        crate::msg::desktop_topbar_undo_action_delete_transaction(),
        crate::msg::desktop_topbar_undo_action_import(),
    ]
}

#[derive(IntoElement)]
pub struct TopBar {
    on_rail_toggle: OnRailToggle,
    on_action: OnTopBarAction,
    can_go_back: bool,
    can_go_forward: bool,
    undo_menu_open: bool,
}

impl TopBar {
    pub fn new(
        on_rail_toggle: OnRailToggle,
        on_action: OnTopBarAction,
        can_go_back: bool,
        can_go_forward: bool,
        undo_menu_open: bool,
    ) -> Self {
        Self {
            on_rail_toggle,
            on_action,
            can_go_back,
            can_go_forward,
            undo_menu_open,
        }
    }
}

impl RenderOnce for TopBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let viewport = window.viewport_size();
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(CONTROLS_GAP)
                    .child(history_button(
                        "topbar-back",
                        DesktopIcon::NavBack,
                        TopBarAction::Back,
                        self.can_go_back,
                        &self.on_action,
                        cx,
                    ))
                    .child(history_button(
                        "topbar-forward",
                        DesktopIcon::NavForward,
                        TopBarAction::Forward,
                        self.can_go_forward,
                        &self.on_action,
                        cx,
                    )),
            )
            .child(brand_mark())
            .child(undo_cluster(
                self.on_action,
                self.undo_menu_open,
                viewport,
                cx,
            ))
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

/// Back or Forward: drawn faint and inert when there's nowhere to go, so the pair never shifts.
fn history_button(
    id: &'static str,
    icon: DesktopIcon,
    action: TopBarAction,
    enabled: bool,
    on_action: &OnTopBarAction,
    cx: &App,
) -> impl IntoElement {
    let on_action = on_action.clone();
    let icon_color = if enabled {
        color::foreground(cx)
    } else {
        color::faint_text(cx)
    };
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .w(BUTTON)
        .h(BUTTON)
        .flex()
        .items_center()
        .justify_center()
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|this| this.bg(color::hover(cx)))
                .on_click(move |_event, window, cx| on_action(action, window, cx))
        })
        .child(icon.icon().with_size(px(16.0)).text_color(icon_color))
}

/// Undo and its caret, a split button: Undo steps back one action, the caret lists the last
/// `RECENT_ACTIONS` to undo back to. The list hangs below the pair, painted over the page.
fn undo_cluster(
    on_action: OnTopBarAction,
    menu_open: bool,
    viewport: Size<Pixels>,
    cx: &App,
) -> impl IntoElement {
    let on_undo = on_action.clone();
    let on_caret = on_action.clone();
    div()
        .relative()
        .flex()
        .items_center()
        .child(
            div()
                .id("topbar-undo")
                .debug_selector(|| "topbar-undo".to_string())
                .w(BUTTON)
                .h(BUTTON)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|this| this.bg(color::hover(cx)))
                .on_click(move |_event, window, cx| on_undo(TopBarAction::Undo, window, cx))
                .child(
                    DesktopIcon::Undo
                        .icon()
                        .with_size(px(16.0))
                        .text_color(color::foreground(cx)),
                ),
        )
        .child(
            div()
                .id("topbar-undo-menu")
                .debug_selector(|| "topbar-undo-menu".to_string())
                .w(CARET_WIDTH)
                .h(BUTTON)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .when(menu_open, |this| this.bg(color::hover(cx)))
                .hover(|this| this.bg(color::hover(cx)))
                .on_click(move |_event, window, cx| {
                    on_caret(TopBarAction::ToggleUndoMenu, window, cx)
                })
                .child(
                    DesktopIcon::UndoMenu
                        .icon()
                        .with_size(px(12.0))
                        .text_color(color::foreground(cx)),
                ),
        )
        .when(menu_open, |this| {
            this.child(undo_menu(on_action, viewport, cx))
        })
}

/// The open recent-actions list, newest first, under a window-wide layer that closes it on a
/// click anywhere else (the caret included, so that click closes rather than reopens it). Both
/// are `deferred` so the page and rails below can't paint over them; the list's higher priority
/// keeps it above its own dismiss layer.
fn undo_menu(on_action: OnTopBarAction, viewport: Size<Pixels>, cx: &App) -> impl IntoElement {
    let on_dismiss = on_action.clone();
    let rows = recent_actions()
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            let on_action = on_action.clone();
            div()
                .id(("topbar-undo-action", index))
                .debug_selector(move || format!("topbar-undo-action-{index}"))
                .px(px(12.0))
                .py(px(6.0))
                .whitespace_nowrap()
                .text_size(MENU_TEXT_SIZE)
                .cursor_pointer()
                .hover(|this| this.bg(color::hover(cx)))
                .on_click(move |_event, window, cx| {
                    on_action(TopBarAction::UndoTo(index), window, cx)
                })
                .child(label)
        });
    let card = div()
        .debug_selector(|| "topbar-undo-list".to_string())
        .occlude()
        .w(MENU_WIDTH)
        .py(px(4.0))
        .flex()
        .flex_col()
        .bg(color::chrome(cx))
        .border_1()
        .border_color(color::structural_rule(cx))
        .shadow_md()
        .child(
            div()
                .px(px(12.0))
                .py(px(4.0))
                .text_size(px(10.5))
                .font_weight(FontWeight::BOLD)
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_topbar_undo_recent()),
        )
        .children(rows);
    div()
        .absolute()
        .top(BUTTON + px(4.0))
        .left(px(0.0))
        .child(
            gpui::deferred(
                gpui::anchored().position(point(px(0.0), px(0.0))).child(
                    div()
                        .id("topbar-undo-dismiss")
                        .debug_selector(|| "topbar-undo-dismiss".to_string())
                        .w(viewport.width)
                        .h(viewport.height)
                        .occlude()
                        .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                            on_dismiss(TopBarAction::DismissUndoMenu, window, cx)
                        }),
                ),
            )
            .with_priority(1),
        )
        .child(gpui::deferred(gpui::anchored().snap_to_window().child(card)).with_priority(2))
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
