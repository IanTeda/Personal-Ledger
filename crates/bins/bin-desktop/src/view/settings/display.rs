//! The **Display** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state", moved
//! directly after General by issue #189's own reorder): a 300px column of three segmented
//! controls (Date format, Decimal & thousands separator, Row density) plus a dot-style Status
//! glyphs radio group, beside a live **PREVIEW** table that re-renders the mockup's own three
//! seeded transaction rows under whichever combination is currently selected.
//!
//! Unlike every other section built so far, this one is genuinely reactive across *all* its
//! fields at once -- General's/Units' own interactive bits (issues #175/#177) are either static
//! or scoped to one table, but here every field click re-formats the PREVIEW table's date,
//! amount, and glyph columns, and shifts its row padding (see
//! `crate::settings::RowDensity::preview_row_padding_y`'s own doc for why row density gets a
//! real visual effect here despite this map's data otherwise being static/dummy).
//!
//! Configuration, not Preferences (`docs/ux/desktop/Settings/README.md`'s Overview) -- like
//! every other click in this map, a selection is "saved" only in `Shell`-owned in-memory state,
//! not written to `personal-ledger.conf`.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{
    settings::{
        DEFAULT_DISPLAY_PREVIEW_ROWS, DateFormat, DecimalSeparator, DisplayPreviewRow,
        PreviewStatus, RowDensity, StatusGlyphs, format_preview_amount, format_preview_date,
    },
    theme::color,
};

use super::{add_unit_dialog::segmented_control, tracing::radio_dot};

pub type OnDateFormatClick = Rc<dyn Fn(DateFormat, &mut Window, &mut App)>;
pub type OnDecimalSeparatorClick = Rc<dyn Fn(DecimalSeparator, &mut Window, &mut App)>;
pub type OnRowDensityClick = Rc<dyn Fn(RowDensity, &mut Window, &mut App)>;
pub type OnStatusGlyphsClick = Rc<dyn Fn(StatusGlyphs, &mut Window, &mut App)>;
/// A curried, option-less click handler -- what a [`status_glyphs_option`] is bound to after its
/// own option has already been curried in (mirrors `units::OnPlainClick`).
pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

const FIELD_COLUMN_WIDTH: gpui::Pixels = px(300.0);
const PREVIEW_GLYPH_WIDTH: gpui::Pixels = px(18.0);
const PREVIEW_DATE_WIDTH: gpui::Pixels = px(92.0);
const PREVIEW_AMOUNT_WIDTH: gpui::Pixels = px(96.0);

#[allow(clippy::too_many_arguments)]
pub fn render(
    date_format: DateFormat,
    decimal_separator: DecimalSeparator,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
    start_sidebar_minimised: bool,
    on_start_sidebar_minimised_click: OnPlainClick,
    on_date_format_click: OnDateFormatClick,
    on_decimal_separator_click: OnDecimalSeparatorClick,
    on_row_density_click: OnRowDensityClick,
    on_status_glyphs_click: OnStatusGlyphsClick,
) -> AnyElement {
    div()
        .flex()
        .gap(px(40.0))
        .child(field_column(
            date_format,
            decimal_separator,
            row_density,
            status_glyphs,
            start_sidebar_minimised,
            on_start_sidebar_minimised_click,
            on_date_format_click,
            on_decimal_separator_click,
            on_row_density_click,
            on_status_glyphs_click,
        ))
        .child(preview_column(
            date_format,
            decimal_separator,
            row_density,
            status_glyphs,
        ))
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn field_column(
    date_format: DateFormat,
    decimal_separator: DecimalSeparator,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
    start_sidebar_minimised: bool,
    on_start_sidebar_minimised_click: OnPlainClick,
    on_date_format_click: OnDateFormatClick,
    on_decimal_separator_click: OnDecimalSeparatorClick,
    on_row_density_click: OnRowDensityClick,
    on_status_glyphs_click: OnStatusGlyphsClick,
) -> impl IntoElement {
    div()
        .w(FIELD_COLUMN_WIDTH)
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(18.0))
        .child(
            div()
                .child(field_label("Date format"))
                .child(segmented_control(
                    "display-date-format",
                    &DateFormat::ALL,
                    date_format,
                    DateFormat::label,
                    on_date_format_click,
                )),
        )
        .child(
            div()
                .child(field_label("Decimal & thousands separator"))
                .child(segmented_control(
                    "display-decimal-separator",
                    &DecimalSeparator::ALL,
                    decimal_separator,
                    DecimalSeparator::label,
                    on_decimal_separator_click,
                )),
        )
        .child(
            div()
                .child(field_label("Row density"))
                .child(segmented_control(
                    "display-row-density",
                    &RowDensity::ALL,
                    row_density,
                    RowDensity::label,
                    on_row_density_click,
                )),
        )
        .child(status_glyphs_field(status_glyphs, on_status_glyphs_click))
        .child(start_sidebar_minimised_toggle(
            start_sidebar_minimised,
            on_start_sidebar_minimised_click,
        ))
}

fn start_sidebar_minimised_toggle(checked: bool, on_click: OnPlainClick) -> impl IntoElement {
    div()
        .id("display-start-sidebar-minimised")
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(
            div()
                .w(px(14.0))
                .h(px(14.0))
                .flex_none()
                .border_1()
                .border_color(if checked {
                    color::ACCENT
                } else {
                    color::DIVIDER
                })
                .bg(if checked {
                    color::ACCENT
                } else {
                    color::GROUND
                }),
        )
        .child(div().text_size(px(12.0)).child("Start Sidebar minimised"))
}

/// `.field > label`: `display:block; font-size:12px; margin-bottom:5px; color: color-mix(text
/// 70%, transparent)` -- the same style the now-removed `ledger_units.rs`'s own field label used
/// (issue #189), distinct from General's bold `super::field_label`
/// (`font-weight:800; margin-bottom:6px`), which only sits over `Input`/`select` pairs.
fn field_label(label: &'static str) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .mb(px(5.0))
        .text_color(color::INK_SECONDARY)
        .child(label)
}

fn status_glyphs_field(selected: StatusGlyphs, on_click: OnStatusGlyphsClick) -> impl IntoElement {
    div().child(field_label("Status glyphs")).child(
        div().flex().flex_col().gap(px(7.0)).mt(px(2.0)).children(
            StatusGlyphs::ALL
                .into_iter()
                .enumerate()
                .map(|(index, option)| {
                    let checked = option == selected;
                    let on_click = on_click.clone();
                    status_glyphs_option(
                        index,
                        option,
                        checked,
                        Rc::new(move |window: &mut Window, cx: &mut App| {
                            on_click(option, window, cx)
                        }),
                    )
                }),
        ),
    )
}

fn status_glyphs_option(
    index: usize,
    option: StatusGlyphs,
    checked: bool,
    on_click: OnPlainClick,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("display-status-glyphs-{index}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(radio_dot(checked))
        .child(div().text_size(px(12.0)).child(option.label()))
}

fn preview_column(
    date_format: DateFormat,
    decimal_separator: DecimalSeparator,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .max_w(px(460.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::INK_TERTIARY)
                .mb(px(10.0))
                .child("PREVIEW"),
        )
        .child(preview_table(
            date_format,
            decimal_separator,
            row_density,
            status_glyphs,
        ))
        .child(
            div()
                .mt(px(14.0))
                .text_size(px(12.0))
                .text_color(color::INK_SECONDARY)
                .child(
                    "Display settings are Configuration, not Preferences \u{2014} they are \
                     read from personal-ledger.conf at start-up and written back here. \
                     Ledger-scoped Preferences (like the default Unit for new entries) live \
                     under Units and sync as Change Sets.",
                ),
        )
}

fn preview_table(
    date_format: DateFormat,
    decimal_separator: DecimalSeparator,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
) -> impl IntoElement {
    let last_index = DEFAULT_DISPLAY_PREVIEW_ROWS.len().saturating_sub(1);
    div()
        .border_1()
        .border_color(color::BORDER)
        .bg(color::CHROME)
        .flex()
        .flex_col()
        .child(preview_table_header())
        .children(
            DEFAULT_DISPLAY_PREVIEW_ROWS
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    preview_row(
                        row,
                        index == last_index,
                        date_format,
                        decimal_separator,
                        row_density,
                        status_glyphs,
                    )
                }),
        )
}

fn preview_table_header() -> impl IntoElement {
    div()
        .flex()
        .px(px(12.0))
        .py(px(7.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_SECONDARY)
        .border_b(px(1.0))
        .border_color(color::HAIRLINE)
        .child(div().w(PREVIEW_GLYPH_WIDTH))
        .child(div().w(PREVIEW_DATE_WIDTH).child("DATE"))
        .child(div().flex_1().child("PAYEE"))
        .child(
            div()
                .w(PREVIEW_AMOUNT_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child("AMOUNT"),
        )
}

/// One PREVIEW row -- the glyph column takes `color::ACCENT` only for a flagged row, matching
/// the mockup's own `<span style="color:#ec3013">\u{2691}</span>` (the only coloured glyph among
/// the three seeded rows).
fn preview_row(
    row: &DisplayPreviewRow,
    last: bool,
    date_format: DateFormat,
    decimal_separator: DecimalSeparator,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .px(px(12.0))
        .py(px(row_density.preview_row_padding_y()))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::HAIRLINE)
        })
        .child(
            div()
                .w(PREVIEW_GLYPH_WIDTH)
                .when(row.status == PreviewStatus::Flagged, |this| {
                    this.text_color(color::ACCENT)
                })
                .child(row.status.glyph(status_glyphs)),
        )
        .child(div().w(PREVIEW_DATE_WIDTH).child(format_preview_date(
            row.year,
            row.month,
            row.day,
            date_format,
        )))
        .child(div().flex_1().child(row.payee))
        .child(
            div()
                .w(PREVIEW_AMOUNT_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child(format_preview_amount(row.amount_cents, decimal_separator)),
        )
}
