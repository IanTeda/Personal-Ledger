//! The detail pane docked beside the Settings index at X-Large (DESKTOP-182), drawn from
//! `lib-settings`' [`PageSummary`] for the page under the index's cursor. It takes only the 400px
//! dock width and the group styling of the list pages' detail pane (Units' `pane.rs`): it has no
//! row of its own to select and never takes the keyboard, so it follows the cursor as the index
//! moves. **PAGE** names the page, its scope note, what it is for and its values read-only, with
//! **Open**, which opens the page as `enter` on the index does; **THIS DEVICE OR THIS LEDGER** says
//! where each kind of setting is kept.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_settings::{PageSummary, ScopeGroup, SettingsFrameEvent, SummaryRow};

use super::state::SettingsView;
use crate::{shell::view::page_clicks::PageClicks, theme::color};

/// `width:400px` at the View's right edge, as the list pages' detail pane.
const PANE_WIDTH: gpui::Pixels = px(400.0);
/// The **Open** button, the list pages' pane button size.
const OPEN_WIDTH: gpui::Pixels = px(112.0);
const OPEN_HEIGHT: gpui::Pixels = px(34.0);
/// A refused This Ledger page's **Open**, dimmed as the index's refused rows are (DESKTOP-179).
const DISABLED_OPACITY: f32 = 0.45;

pub fn index_pane(
    summary: &PageSummary,
    clicks: &PageClicks<SettingsView>,
    cx: &App,
) -> AnyElement {
    div()
        .debug_selector(|| "settings-index-pane".to_string())
        .id("settings-index-pane")
        .w(PANE_WIDTH)
        .flex_none()
        .h_full()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::structural_rule(cx))
        .child(group(
            &crate::msg::desktop_settings_pane_page(),
            page_block(summary, clicks, cx),
            cx,
        ))
        .child(group(
            &crate::msg::desktop_settings_pane_scope(),
            scope_block(cx),
            cx,
        ))
        .into_any_element()
}

/// A group: a caps label over a 2px rule, then the body.
fn group(label: &str, body: impl IntoElement, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(16.0))
                .pt(px(14.0))
                .pb(px(8.0))
                .border_b(px(2.0))
                .border_color(color::structural_rule(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .child(lib_locale::format::upper(label)),
        )
        .child(
            div()
                .px(px(16.0))
                .pt(px(12.0))
                .pb(px(16.0))
                .flex()
                .flex_col()
                .child(body),
        )
        .into_any_element()
}

/// The page under the cursor: its name and scope note, what it is for, its values (or why there
/// are none), then **Open**.
fn page_block(
    summary: &PageSummary,
    clicks: &PageClicks<SettingsView>,
    cx: &App,
) -> impl IntoElement {
    let page = summary.page;
    div()
        .debug_selector(move || format!("settings-index-pane-{page:?}"))
        .flex()
        .flex_col()
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(15.0))
                .line_height(gpui::relative(1.25))
                .child(summary.name.clone()),
        )
        .child(
            div()
                .mt(px(2.0))
                .mb(px(8.0))
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(summary.scope_note.clone()),
        )
        .child(
            div()
                .mb(px(10.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(summary.purpose.clone()),
        )
        .children(summary.rows.iter().map(|value| value_row(value, cx)))
        .children(summary.note.clone().map(|note| {
            div()
                .debug_selector(|| "settings-index-pane-note".to_string())
                .text_size(px(12.0))
                .text_color(color::faint_text(cx))
                .child(note)
        }))
        .child(open_button(summary, clicks, cx))
}

/// A value: the label muted, the value right-aligned and free to wrap, so a long path stays in
/// the pane.
fn value_row(value: &SummaryRow, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .justify_between()
        .gap(px(12.0))
        .py(px(5.0))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .text_size(px(12.0))
        .child(
            div()
                .flex_none()
                .text_color(color::muted(cx))
                .child(value.label.clone()),
        )
        .child(
            div()
                .min_w(px(0.0))
                .text_align(gpui::TextAlign::Right)
                .child(value.value.clone()),
        )
}

/// **Open**, which opens the page as a click on its index row does; dimmed and inert for a This
/// Ledger page with no Ledger open.
fn open_button(
    summary: &PageSummary,
    clicks: &PageClicks<SettingsView>,
    cx: &App,
) -> impl IntoElement {
    let button = div()
        .debug_selector(|| "settings-index-pane-open".to_string())
        .id("settings-index-pane-open")
        .mt(px(10.0))
        .w(OPEN_WIDTH)
        .h(OPEN_HEIGHT)
        .flex()
        .items_center()
        .px(px(10.0))
        .border_1()
        .border_color(color::border(cx))
        .bg(color::background(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .whitespace_nowrap()
        .child(crate::msg::desktop_settings_pane_open());
    if summary.open_enabled {
        button
            .cursor_pointer()
            .on_click(clicks.send(SettingsFrameEvent::ClickIndexPage(summary.page)))
    } else {
        button.opacity(DISABLED_OPACITY)
    }
}

/// The two kinds of setting, a paragraph each: where they are kept and whether they sync.
fn scope_block(cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .text_size(px(12.0))
        .text_color(color::muted(cx))
        .children(ScopeGroup::ALL.map(|group| div().child(group.pane_paragraph())))
}
