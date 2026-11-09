//! Draws the **Sync server** section (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state"): a
//! 300px column of label/value rows (Server URL, Status, Last sync), then a "Sync now" button.
//!
//! Unlike the summary rows `general::summary_panel` uses (bordered, `background:#eae9e9`,
//! 1px hairline between rows), these rows are the mockup's own plainer `gap:12px; padding:8px 0`
//! shape with no border at all -- a distinct component, not a reuse of `general`'s panel, despite
//! the visual family resemblance.
//!
//! "Sync now" is permanently out of scope for this map (its own Destination/Out-of-scope: a
//! stand-in with no real effect) rather than "not yet built" behind a future ticket -- so its
//! sample status-line message, unlike Units'/Institutions' "+ Add" buttons, names no issue.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_settings::{SyncServerEvent, SyncServerViewModel};

use super::SyncServerPage;
use crate::shell::view::page_clicks::PageClicks;
use crate::theme::color;

const COLUMN_WIDTH: gpui::Pixels = px(300.0);
/// The status dot's own `border-radius:50%` on a 6x6 box -- a one-off exception to the
/// handoff's "Radius 0 everywhere" system rule (`crate::theme`'s own doc), since the concrete
/// mockup markup draws a circular status indicator and no square rendering of "connected" reads
/// as the intended design (same "concrete mockup over general prose" precedent issue #176 used
/// for its own segmented control). Set via `Styled::style` directly, mirroring
/// `units::view::page`'s **+** buttons' `align_self` -- `Corners`/`CornersRefinement` has no dedicated `Styled`
/// builder method.
const DOT_RADIUS: gpui::Pixels = px(3.0);

/// The page drawn from `model`.
pub(super) fn page(
    model: &SyncServerViewModel,
    clicks: &PageClicks<SyncServerPage>,
    cx: &App,
) -> AnyElement {
    div()
        .w(COLUMN_WIDTH)
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(row(model.url_label.clone(), model.url.clone(), cx))
        .child(status_row(model, cx))
        .child(row(model.last_label.clone(), model.last.clone(), cx))
        .child(sync_now_button(model, clicks, cx))
        .into_any_element()
}

fn row(label: String, value: String, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .items_center()
        .py(px(8.0))
        .child(div().text_color(color::muted(cx)).child(label))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(value),
        )
}

fn status_row(model: &SyncServerViewModel, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .items_center()
        .py(px(8.0))
        .child(
            div()
                .text_color(color::muted(cx))
                .child(model.status_label.clone()),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(status_dot(cx))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::positive_text(cx))
                        .child(model.connected.clone()),
                ),
        )
}

fn status_dot(cx: &App) -> impl IntoElement {
    let mut dot = div().w(px(6.0)).h(px(6.0)).bg(color::positive(cx));
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
fn sync_now_button(
    model: &SyncServerViewModel,
    clicks: &PageClicks<SyncServerPage>,
    cx: &App,
) -> impl IntoElement {
    let mut button = div()
        .id("settings-sync-now")
        .debug_selector(|| "settings-sync-now".to_string())
        .cursor_pointer()
        .mt(px(8.0))
        .py(px(8.0))
        .px(px(16.0))
        .bg(color::chrome(cx))
        .border_1()
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .on_click(clicks.send(SyncServerEvent::SyncNow))
        .child(model.sync_now.clone());
    button.style().align_self = Some(gpui::AlignItems::FlexStart);
    button
}
