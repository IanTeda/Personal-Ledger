//! The **Display** section (`docs/ux/desktop/16-settings/README.md`'s "2a resting state", moved
//! directly after General by issue #189's own reorder): a 300px column of three segmented
//! controls (Date format, Row density) plus a read-only Locale and a dot-style Status
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
//! Configuration, not Preferences (`docs/ux/desktop/16-settings/README.md`'s Overview) -- like
//! every other click in this map, a selection is "saved" only in `Shell`-owned in-memory state,
//! not written to `personal-ledger.conf`.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_core::DateStyle;

use crate::{
    settings::{
        DATE_STYLE_CHOICES, DEFAULT_DISPLAY_PREVIEW_ROWS, DISPLAY_FIELD_SIDEBAR, DisplayPreviewRow,
        PreviewStatus, RowDensity, StatusGlyphs, date_style_label, format_preview_amount,
        format_preview_date,
    },
    theme::color,
};

use super::{
    add_unit_dialog::{UNIFORM_OPTION_WIDTH, segmented_control_sized},
    colour_theme,
    tracing::radio_dot,
};

pub type OnDateStyleClick = Rc<dyn Fn(Option<DateStyle>, &mut Window, &mut App)>;
pub type OnRowDensityClick = Rc<dyn Fn(RowDensity, &mut Window, &mut App)>;
pub type OnStatusGlyphsClick = Rc<dyn Fn(StatusGlyphs, &mut Window, &mut App)>;
pub type OnToastsClick = Rc<dyn Fn(bool, &mut Window, &mut App)>;
/// A curried, option-less click handler -- what a [`status_glyphs_option`] is bound to after its
/// own option has already been curried in (mirrors `units::OnPlainClick`).
pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

const FIELD_COLUMN_WIDTH: gpui::Pixels = px(380.0);
const PREVIEW_GLYPH_WIDTH: gpui::Pixels = px(18.0);
const PREVIEW_DATE_WIDTH: gpui::Pixels = px(92.0);
const PREVIEW_AMOUNT_WIDTH: gpui::Pixels = px(96.0);

#[expect(
    clippy::too_many_arguments,
    reason = "a stateless GPUI render fn takes each value and handler it wires; a props struct would only rename the list"
)]
pub fn render(
    date_style: Option<DateStyle>,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
    start_sidebar_minimised: bool,
    on_start_sidebar_minimised_click: OnPlainClick,
    on_date_style_click: OnDateStyleClick,
    on_row_density_click: OnRowDensityClick,
    on_status_glyphs_click: OnStatusGlyphsClick,
    toasts_on: bool,
    on_toasts_click: OnToastsClick,
    colour_theme_focus: Option<usize>,
    field_focus: Option<usize>,
    on_colour_theme_click: colour_theme::OnColourThemeClick,
    cx: &App,
) -> AnyElement {
    let columns = div()
        .flex()
        .gap(px(40.0))
        .child(field_column(
            date_style,
            row_density,
            status_glyphs,
            start_sidebar_minimised,
            on_start_sidebar_minimised_click,
            on_date_style_click,
            on_row_density_click,
            on_status_glyphs_click,
            field_focus,
            cx,
        ))
        .child(preview_column(date_style, row_density, status_glyphs, cx));
    div()
        .flex()
        .flex_col()
        .child(columns)
        .child(toasts_field(
            toasts_on,
            on_toasts_click,
            field_focus == Some(4),
            cx,
        ))
        .child(colour_theme::render(
            colour_theme_focus,
            on_colour_theme_click,
            cx,
        ))
        .into_any_element()
}

#[expect(
    clippy::too_many_arguments,
    reason = "a stateless GPUI render fn takes each value and handler it wires; a props struct would only rename the list"
)]
fn field_column(
    date_style: Option<DateStyle>,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
    start_sidebar_minimised: bool,
    on_start_sidebar_minimised_click: OnPlainClick,
    on_date_style_click: OnDateStyleClick,
    on_row_density_click: OnRowDensityClick,
    on_status_glyphs_click: OnStatusGlyphsClick,
    field_focus: Option<usize>,
    cx: &App,
) -> impl IntoElement {
    div()
        .w(FIELD_COLUMN_WIDTH)
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(18.0))
        .child(locale_field(cx))
        .child(
            focus_mark(field_focus == Some(0), cx)
                .child(field_label(
                    crate::msg::desktop_display_date_style_label(),
                    cx,
                ))
                .child(segmented_control_sized(
                    "display-date-style",
                    &DATE_STYLE_CHOICES,
                    date_style,
                    date_style_label,
                    on_date_style_click,
                    Some(px(UNIFORM_OPTION_WIDTH)),
                    cx,
                )),
        )
        .child(
            focus_mark(field_focus == Some(1), cx)
                .child(field_label(
                    crate::msg::desktop_settings_display_row_density(),
                    cx,
                ))
                .child(segmented_control_sized(
                    "display-row-density",
                    &RowDensity::ALL,
                    row_density,
                    RowDensity::label,
                    on_row_density_click,
                    Some(px(UNIFORM_OPTION_WIDTH)),
                    cx,
                )),
        )
        .child(status_glyphs_field(
            status_glyphs,
            on_status_glyphs_click,
            field_focus == Some(2),
            cx,
        ))
        .child(
            focus_mark(field_focus == Some(DISPLAY_FIELD_SIDEBAR), cx).child(
                start_sidebar_minimised_toggle(
                    start_sidebar_minimised,
                    on_start_sidebar_minimised_click,
                    cx,
                ),
            ),
        )
}

/// The keyboard-focus mark for one control: an accent rule down its left edge, drawn on every
/// control so the column does not shift as focus moves.
fn focus_mark(focused: bool, cx: &App) -> gpui::Div {
    div()
        .pl(px(8.0))
        .ml(px(-10.0))
        .border_l(px(2.0))
        .border_color(if focused {
            gpui::Hsla::from(color::accent(cx))
        } else {
            gpui::transparent_black()
        })
}

/// The "Toasts" On/Off row (ADR-0027), above the Colour Theme group, noting that Errors still
/// show when off.
fn toasts_field(
    toasts_on: bool,
    on_click: OnToastsClick,
    focused: bool,
    cx: &App,
) -> impl IntoElement {
    fn label(on: bool) -> String {
        if on {
            lib_locale::msg::toast_setting_on()
        } else {
            lib_locale::msg::toast_setting_off()
        }
    }
    focus_mark(focused, cx)
        .id("display-toasts")
        .mt(px(24.0))
        .w(FIELD_COLUMN_WIDTH)
        .child(field_label(lib_locale::msg::toast_setting_label(), cx))
        .child(segmented_control_sized(
            "display-toasts",
            &[true, false],
            toasts_on,
            label,
            on_click,
            Some(px(UNIFORM_OPTION_WIDTH)),
            cx,
        ))
        .child(
            div()
                .mt(px(3.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(lib_locale::msg::toast_setting_note()),
        )
}

/// The effective Locale, read-only, with where it came from. There is no control: the Locale is
/// Configuration, changed by the `locale` setting or `--locale` and a restart.
fn locale_field(cx: &App) -> impl IntoElement {
    let (line, fallback_note) = crate::locale::describe(&crate::locale::info());
    let note = |text: String| {
        div()
            .mt(px(3.0))
            .text_size(px(12.0))
            .text_color(color::muted(cx))
            .child(text)
    };
    div()
        .id("display-locale")
        .child(field_label(crate::msg::desktop_display_locale_label(), cx))
        .child(div().text_size(px(13.0)).child(line))
        .children(fallback_note.map(note))
        .child(note(crate::msg::desktop_display_locale_hint()))
}

fn start_sidebar_minimised_toggle(
    checked: bool,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    div()
        .debug_selector(|| "display-start-sidebar-minimised".to_string())
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
        .child(
            div()
                .text_size(px(12.0))
                .child(crate::msg::desktop_settings_display_start_minimised()),
        )
}

/// `.field > label`: `display:block; font-size:12px; margin-bottom:5px; color: color-mix(text
/// 70%, transparent)` -- the same style the now-removed `ledger_units.rs`'s own field label used
/// (issue #189), distinct from General's bold `super::field_label`
/// (`font-weight:800; margin-bottom:6px`), which only sits over `Input`/`select` pairs.
fn field_label(label: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    div()
        .text_size(px(12.0))
        .mb(px(5.0))
        .text_color(color::muted(cx))
        .child(label.into())
}

fn status_glyphs_field(
    selected: StatusGlyphs,
    on_click: OnStatusGlyphsClick,
    focused: bool,
    cx: &App,
) -> impl IntoElement {
    focus_mark(focused, cx)
        .child(field_label(
            crate::msg::desktop_settings_display_status_glyphs(),
            cx,
        ))
        .child(
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
                            cx,
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
    cx: &App,
) -> impl IntoElement {
    div()
        .debug_selector(move || format!("display-status-glyphs-{index}"))
        .id(SharedString::from(format!("display-status-glyphs-{index}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(radio_dot(checked, cx))
        .child(div().text_size(px(12.0)).child(option.label()))
}

fn preview_column(
    date_style: Option<DateStyle>,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .max_w(px(460.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .mb(px(10.0))
                .child(lib_locale::format::upper(
                    &crate::msg::desktop_settings_display_preview(),
                )),
        )
        .child(preview_table(date_style, row_density, status_glyphs, cx))
        .child(
            div()
                .mt(px(14.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_settings_display_note()),
        )
}

fn preview_table(
    date_style: Option<DateStyle>,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
    cx: &App,
) -> impl IntoElement {
    let last_index = DEFAULT_DISPLAY_PREVIEW_ROWS.len().saturating_sub(1);
    div()
        .border_1()
        .border_color(color::border(cx))
        .bg(color::chrome(cx))
        .flex()
        .flex_col()
        .child(preview_table_header(cx))
        .children(
            DEFAULT_DISPLAY_PREVIEW_ROWS
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    preview_row(
                        row,
                        index == last_index,
                        date_style,
                        row_density,
                        status_glyphs,
                        cx,
                    )
                }),
        )
}

fn preview_table_header(cx: &App) -> impl IntoElement {
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
                .child(lib_locale::format::upper(&lib_locale::msg::column_date())),
        )
        .child(
            div()
                .flex_1()
                .child(lib_locale::format::upper(&lib_locale::msg::column_payee())),
        )
        .child(
            div()
                .w(PREVIEW_AMOUNT_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child(lib_locale::format::upper(&lib_locale::msg::column_amount())),
        )
}

/// One PREVIEW row -- the glyph column takes `color::accent(cx)` only for a flagged row, matching
/// the mockup's own `<span style="color:#ec3013">\u{2691}</span>` (the only coloured glyph among
/// the three seeded rows).
fn preview_row(
    row: &DisplayPreviewRow,
    last: bool,
    date_style: Option<DateStyle>,
    row_density: RowDensity,
    status_glyphs: StatusGlyphs,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .px(px(12.0))
        .py(px(row_density.preview_row_padding_y()))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .child(
            div()
                .w(PREVIEW_GLYPH_WIDTH)
                .when(row.status == PreviewStatus::Flagged, |this| {
                    this.text_color(color::accent_text(cx))
                })
                .child(row.status.glyph(status_glyphs)),
        )
        .child(div().w(PREVIEW_DATE_WIDTH).child(format_preview_date(
            row.year, row.month, row.day, date_style,
        )))
        .child(div().flex_1().child(row.payee))
        .child(
            div()
                .w(PREVIEW_AMOUNT_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child(format_preview_amount(row.amount_cents)),
        )
}
