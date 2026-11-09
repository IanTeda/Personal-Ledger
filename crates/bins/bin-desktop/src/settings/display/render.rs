//! Settings › Display drawn from its view model (`docs/ux/desktop-mockups/16-settings/README.md`'s
//! "2a resting state"): a column of segmented controls (Date format, Row density), a Status glyphs
//! radio group and the Start Sidebar minimised checkbox, beside a PREVIEW of three seeded rows
//! under the choices made, then the Colour Theme grid (`colour_theme.rs`). Every click is a
//! Presenter event through [`PageClicks`].

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_settings::presenter::display::{DATE_STYLE_CHOICES, DISPLAY_FIELD_SIDEBAR};
use lib_settings::presenter::{CheckboxView, PreviewView};
use lib_settings::{DeviceKey, DisplayEvent, DisplayViewModel};
use lib_viper::display::{RowDensity, StatusGlyphs};
use lib_viper::view::SelectView;

use super::{DisplayPage, colour_theme};
use crate::{
    form::fields::{OnSegmentClick, UNIFORM_OPTION_WIDTH, radio_dot, segmented_labelled},
    settings::controls::{focus_mark, shadowed},
    shell::view::page_clicks::PageClicks,
    theme::color,
};

/// As wide as the widest control, Date format's uniform bar (one option per choice plus its 1px
/// border each side): any narrower and the bar overflows the `flex_none` column into the
/// PREVIEW beside it.
const FIELD_COLUMN_WIDTH: gpui::Pixels =
    px(DATE_STYLE_CHOICES.len() as f32 * UNIFORM_OPTION_WIDTH + 2.0);
/// The narrowest the PREVIEW draws beside the field column: its fixed glyph, date and amount
/// columns plus a readable payee. Below it, as at the 720px floor with the icon rail, the PREVIEW
/// wraps under the field column instead of crushing the payee.
const PREVIEW_MIN_WIDTH: gpui::Pixels = px(300.0);
const PREVIEW_GLYPH_WIDTH: gpui::Pixels = px(18.0);
const PREVIEW_DATE_WIDTH: gpui::Pixels = px(92.0);
const PREVIEW_AMOUNT_WIDTH: gpui::Pixels = px(96.0);

/// A PREVIEW row's vertical padding under `density`: the one place in this concept that Row
/// density visibly changes, on the 4px rhythm either side of Regular.
pub fn preview_row_padding_y(density: RowDensity) -> f32 {
    match density {
        RowDensity::Compact => 4.0,
        RowDensity::Regular => 8.0,
        RowDensity::Roomy => 12.0,
    }
}

/// The page: the controls beside the PREVIEW, then the Colour Theme group.
pub(super) fn page(
    model: &DisplayViewModel,
    clicks: &PageClicks<DisplayPage>,
    cx: &App,
) -> AnyElement {
    let columns = div()
        .flex()
        .flex_wrap()
        .gap_x(px(40.0))
        .gap_y(px(24.0))
        .child(
            div()
                .debug_selector(|| "display-fields".to_string())
                .w(FIELD_COLUMN_WIDTH)
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(18.0))
                .child(
                    segmented(
                        "display-date-style",
                        &model.date_style.select,
                        clicks.segment(DisplayEvent::ChooseDateStyle),
                        cx,
                    )
                    .children(shadowed(
                        DeviceKey::DateStyle,
                        model.date_style.shadowed.as_deref(),
                        cx,
                    )),
                )
                .child(
                    segmented(
                        "display-row-density",
                        &model.row_density.select,
                        clicks.segment(DisplayEvent::ChooseRowDensity),
                        cx,
                    )
                    .children(shadowed(
                        DeviceKey::RowDensity,
                        model.row_density.shadowed.as_deref(),
                        cx,
                    )),
                )
                .child(
                    status_glyphs_field(&model.status_glyphs.select, clicks, cx).children(
                        shadowed(
                            DeviceKey::StatusGlyphs,
                            model.status_glyphs.shadowed.as_deref(),
                            cx,
                        ),
                    ),
                )
                .child(
                    focus_mark(model.field == Some(DISPLAY_FIELD_SIDEBAR), cx).child(
                        start_sidebar_minimised_toggle(&model.start_sidebar_minimised, clicks, cx),
                    ),
                ),
        )
        .child(preview_column(&model.preview, cx));
    div()
        .flex()
        .flex_col()
        .child(columns)
        .child(colour_theme::render(
            &model.colour_theme,
            &model.appearance,
            clicks,
            cx,
        ))
        .into_any_element()
}

/// A uniform segmented control with its label, the keyboard's mark on it while focused.
fn segmented<T: Copy + PartialEq + 'static>(
    id: &'static str,
    select: &SelectView<T>,
    on_click: OnSegmentClick<T>,
    cx: &App,
) -> gpui::Div {
    let options = select
        .options
        .iter()
        .map(|choice| (choice.value, SharedString::from(choice.label.clone())))
        .collect();
    let chosen = select.chosen.unwrap_or(select.options[0].value);
    focus_mark(select.focused, cx)
        .child(field_label(select.label.clone(), cx))
        .child(segmented_labelled(
            id,
            options,
            chosen,
            on_click,
            Some(px(UNIFORM_OPTION_WIDTH)),
            cx,
        ))
}

fn start_sidebar_minimised_toggle(
    view: &CheckboxView,
    clicks: &PageClicks<DisplayPage>,
    cx: &App,
) -> impl IntoElement {
    let checked = view.checked;
    div()
        .debug_selector(|| "display-start-sidebar-minimised".to_string())
        .id("display-start-sidebar-minimised")
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
        .on_click(clicks.send(DisplayEvent::ToggleStartSidebarMinimised))
        .child(
            div()
                .w(px(14.0))
                .h(px(14.0))
                .flex_none()
                .border_1()
                .border_color(if checked {
                    color::accent(cx)
                } else {
                    color::divider(cx)
                })
                .bg(if checked {
                    color::accent(cx)
                } else {
                    color::background(cx)
                }),
        )
        .child(div().text_size(px(12.0)).child(view.label.clone()))
}

/// Display's own field label, lighter than the form fields' bold one, which sits over
/// `Input`/`select` pairs.
fn field_label(label: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .mb(px(5.0))
        .text_color(color::muted(cx))
        .child(label.into())
}

fn status_glyphs_field(
    select: &SelectView<StatusGlyphs>,
    clicks: &PageClicks<DisplayPage>,
    cx: &App,
) -> gpui::Div {
    focus_mark(select.focused, cx)
        .child(field_label(select.label.clone(), cx))
        .child(div().flex().flex_col().gap(px(7.0)).mt(px(2.0)).children(
            select.options.iter().enumerate().map(|(index, choice)| {
                let checked = select.chosen == Some(choice.value);
                div()
                    .debug_selector(move || format!("display-status-glyphs-{index}"))
                    .id(SharedString::from(format!("display-status-glyphs-{index}")))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .on_click(clicks.send(DisplayEvent::ChooseStatusGlyphs(choice.value)))
                    .child(radio_dot(checked, cx))
                    .child(div().text_size(px(12.0)).child(choice.label.clone()))
            }),
        ))
}

fn preview_column(preview: &PreviewView, cx: &App) -> impl IntoElement {
    div()
        .debug_selector(|| "display-preview".to_string())
        .flex_1()
        .min_w(PREVIEW_MIN_WIDTH)
        .max_w(px(460.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .mb(px(10.0))
                .child(lib_locale::format::upper(&preview.heading)),
        )
        .child(preview_table(preview, cx))
        .child(
            div()
                .mt(px(14.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(preview.note.clone()),
        )
}

fn preview_table(preview: &PreviewView, cx: &App) -> impl IntoElement {
    let last_index = preview.rows.len().saturating_sub(1);
    let padding_y = px(preview_row_padding_y(preview.row_density));
    div()
        .border_1()
        .border_color(color::border(cx))
        .bg(color::chrome(cx))
        .flex()
        .flex_col()
        .child(preview_table_header(preview, cx))
        .children(preview.rows.iter().enumerate().map(|(index, row)| {
            // The glyph column takes the accent only for the Flagged row, as the mockup colours
            // only that one of its three seeded rows.
            div()
                .flex()
                .items_center()
                .px(px(12.0))
                .py(padding_y)
                .when(index != last_index, |this| {
                    this.border_b(px(1.0)).border_color(color::hairline(cx))
                })
                .child(
                    div()
                        .w(PREVIEW_GLYPH_WIDTH)
                        .when(row.flagged, |this| this.text_color(color::accent_text(cx)))
                        .child(row.glyph.clone()),
                )
                .child(div().w(PREVIEW_DATE_WIDTH).child(row.date.clone()))
                .child(div().flex_1().child(row.payee.clone()))
                .child(
                    div()
                        .w(PREVIEW_AMOUNT_WIDTH)
                        .text_align(gpui::TextAlign::Right)
                        .child(row.amount.clone()),
                )
        }))
}

fn preview_table_header(preview: &PreviewView, cx: &App) -> impl IntoElement {
    let upper = lib_locale::format::upper;
    div()
        .flex()
        .px(px(12.0))
        .py(px(7.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .child(div().w(PREVIEW_GLYPH_WIDTH))
        .child(
            div()
                .w(PREVIEW_DATE_WIDTH)
                .child(upper(&preview.date_column)),
        )
        .child(div().flex_1().child(upper(&preview.payee_column)))
        .child(
            div()
                .w(PREVIEW_AMOUNT_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child(upper(&preview.amount_column)),
        )
}
