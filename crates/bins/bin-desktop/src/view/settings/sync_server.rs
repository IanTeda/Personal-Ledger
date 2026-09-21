//! The **Sync server** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state"): a
//! 300px column of label/value rows (Server URL, Status, Last sync), then a "Sync now" button.
//!
//! Unlike the summary rows `general::summary_panel` uses (bordered, `background:#eae9e9`,
//! 1px hairline between rows), these rows are the mockup's own plainer `gap:12px; padding:8px 0`
//! shape with no border at all -- a distinct component, not a reuse of `general`'s panel, despite
//! the visual family resemblance.
//!
//! "Sync now" is permanently out of scope for this map (its own Destination/Out-of-scope: a
//! stand-in with no real effect) rather than "not yet built" behind a future ticket -- so its
//! stub status-line message, unlike Units'/Institutions' "+ Add" buttons, names no issue.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use crate::theme::color;

pub type OnSyncNowClick = Rc<dyn Fn(&mut Window, &mut App)>;

const COLUMN_WIDTH: gpui::Pixels = px(300.0);
/// The status dot's own `border-radius:50%` on a 6x6 box -- a one-off exception to the
/// handoff's "Radius 0 everywhere" system rule (`crate::theme`'s own doc), since the concrete
/// mockup markup draws a circular status indicator and no square rendering of "connected" reads
/// as the intended design (same "concrete mockup over general prose" precedent issue #176 used
/// for its own segmented control). Set via `Styled::style` directly, mirroring
/// `units::add_button`'s `align_self` -- `Corners`/`CornersRefinement` has no dedicated `Styled`
/// builder method.
const DOT_RADIUS: gpui::Pixels = px(3.0);

pub fn render(on_sync_now_click: OnSyncNowClick) -> AnyElement {
    div()
        .w(COLUMN_WIDTH)
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(row(
            crate::msg::desktop_settings_sync_url(),
            "sync.ledger.localhost",
        ))
        .child(status_row())
        .child(row(
            crate::msg::desktop_settings_sync_last(),
            "14 sep 2026 \u{b7} 09:14",
        ))
        .child(sync_now_button(on_sync_now_click))
        .into_any_element()
}

fn row(label: String, value: &'static str) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .items_center()
        .py(px(8.0))
        .child(div().text_color(color::INK_SECONDARY).child(label))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(value),
        )
}

fn status_row() -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .items_center()
        .py(px(8.0))
        .child(
            div()
                .text_color(color::INK_SECONDARY)
                .child(crate::msg::desktop_settings_sync_status()),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(status_dot())
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::POSITIVE)
                        .child(crate::msg::desktop_settings_sync_connected()),
                ),
        )
}

fn status_dot() -> impl IntoElement {
    let mut dot = div().w(px(6.0)).h(px(6.0)).bg(color::POSITIVE);
    let radius: gpui::AbsoluteLength = DOT_RADIUS.into();
    let corner_radii = &mut dot.style().corner_radii;
    corner_radii.top_left = Some(radius);
    corner_radii.top_right = Some(radius);
    corner_radii.bottom_right = Some(radius);
    corner_radii.bottom_left = Some(radius);
    dot
}

/// The "Sync now" button: `padding:8px 16px; border:1px solid rgba(32,30,29,.30);
/// background:#eae9e9; font-weight:800; margin-top:8px` -- narrower top gap than the "+ Add
/// unit"/"+ Add institution" buttons' own `16px`, matching the mockup's own markup here.
fn sync_now_button(on_click: OnSyncNowClick) -> impl IntoElement {
    let mut button = div()
        .id("settings-sync-now")
        .cursor_pointer()
        .mt(px(8.0))
        .py(px(8.0))
        .px(px(16.0))
        .bg(color::CHROME)
        .border_1()
        .border_color(color::BORDER)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(crate::msg::desktop_settings_sync_now());
    button.style().align_self = Some(gpui::AlignItems::FlexStart);
    button
}
