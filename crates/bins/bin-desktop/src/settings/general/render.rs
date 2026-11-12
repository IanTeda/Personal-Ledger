//! Settings › General drawn from its view model (DESKTOP-178): the field column (On start, Start
//! view, Toasts, the Locale and the Data folder) beside the THIS DEVICE box. Every click is a
//! Presenter event through [`PageClicks`]. Start view and the Locale are the shared dropdown, their
//! lists the Presenter's; the View keeps only each open list's scroll ([`GeneralWidgets`]).

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_settings::presenter::{DataFolderView, FieldView, ThisDeviceView};
use lib_settings::{GeneralEvent, GeneralField, GeneralViewModel};
use lib_viper::view::Hint;

use super::{GeneralPage, GeneralWidgets};
use crate::{
    form::fields::{UNIFORM_OPTION_WIDTH, field_label, field_value},
    form::select::SelectState,
    form::select_field::{self, SelectFieldProps},
    settings::controls::{focus_mark, hint, segmented_choices, shadowed},
    shell::view::page_clicks::PageClicks,
    theme::color,
};

/// The page drawn from `model`.
pub(super) fn page(
    model: &GeneralViewModel,
    clicks: &PageClicks<GeneralPage>,
    widgets: &mut GeneralWidgets,
    cx: &App,
) -> AnyElement {
    let field = |field| model.fields.view(field);
    div()
        .flex()
        .gap_x(px(40.0))
        .child(
            div()
                .debug_selector(|| "general-fields".to_string())
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(18.0))
                .children(field(GeneralField::OnStart).map(|view| {
                    segmented(
                        "general-on-start",
                        view,
                        GeneralField::OnStart,
                        None,
                        clicks,
                        None,
                        cx,
                    )
                }))
                .children(field(GeneralField::StartView).map(|view| {
                    start_view_field(
                        view,
                        model.start_view_unlisted.as_deref(),
                        clicks,
                        &mut widgets.start_view_scroll,
                        cx,
                    )
                }))
                .children(field(GeneralField::Toasts).map(|view| {
                    segmented(
                        "general-toasts",
                        view,
                        GeneralField::Toasts,
                        Some(px(UNIFORM_OPTION_WIDTH)),
                        clicks,
                        Some(test_button(&model.toasts_test, clicks, cx).into_any_element()),
                        cx,
                    )
                }))
                .children(field(GeneralField::Locale).map(|view| {
                    locale_field(
                        view,
                        model.locale_fallback.as_deref(),
                        clicks,
                        &mut widgets.locale_scroll,
                        cx,
                    )
                }))
                .children(
                    field(GeneralField::DataFolder)
                        .map(|view| data_folder_field(view, &model.data_folder, clicks, cx)),
                ),
        )
        .child(this_device_panel(&model.this_device, cx))
        .into_any_element()
}

/// A segmented field with its label, hint and shadowed note, and `beside` it 8px to its right if
/// given. A click on an option chooses it by its index.
fn segmented(
    id: &'static str,
    view: &FieldView,
    field: GeneralField,
    option_width: Option<gpui::Pixels>,
    clicks: &PageClicks<GeneralPage>,
    beside: Option<AnyElement>,
    cx: &App,
) -> impl IntoElement {
    focus_mark(view.focused, cx)
        .child(field_label(view.label.clone()))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(segmented_choices(
                    id,
                    view,
                    clicks.segment(move |index| GeneralEvent::Choose { field, index }),
                    option_width,
                    cx,
                ))
                .children(beside),
        )
        .children(view.hint.clone().map(|text| hint(text, cx)))
        .children(shadowed(view, cx))
}

/// A dropdown field as the shared select draws it, its list the Presenter's. `scroll` is the open
/// list's window, which the View keeps between draws so the window follows the highlight.
fn dropdown(
    id: &'static str,
    view: &FieldView,
    field: GeneralField,
    clicks: &PageClicks<GeneralPage>,
    scroll: &mut usize,
    cx: &App,
) -> impl IntoElement {
    let chosen = view
        .chosen
        .and_then(|index| view.options.get(index).cloned());
    let state = SelectState::drawn(chosen, view.list.highlight(), *scroll, view.options.len());
    *scroll = state.scroll();
    select_field::render(
        SelectFieldProps {
            id: id.into(),
            label: view.label.clone().into(),
            options: &view.options,
            state: &state,
            focused: view.focused,
            accent: false,
            read_only: None,
            on_field_click: clicks.click(GeneralEvent::Press(field)),
            on_option_click: clicks.option(move |index| GeneralEvent::Choose { field, index }),
        },
        cx,
    )
}

/// Toasts' **Test** button beside On/Off, in a segment's treatment (#1156): the same padding, text
/// and `divider` border, so it stands as tall as the bar, with no fill since it is never chosen.
/// Its key follows its label, faint. It is no `j`/`k` stop of its own: the key presses it from
/// Toasts.
fn test_button(test: &Hint, clicks: &PageClicks<GeneralPage>, cx: &App) -> impl IntoElement {
    div()
        .id("general-toasts-test")
        .debug_selector(|| "general-toasts-test".to_string())
        .cursor_pointer()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(6.0))
        .py(px(7.0))
        .px(px(12.0))
        .border_1()
        .border_color(color::divider(cx))
        .text_size(px(13.0))
        .whitespace_nowrap()
        .on_click(clicks.send(GeneralEvent::PressTest))
        .child(test.action.clone())
        .child(
            div()
                .text_color(color::faint_text(cx))
                .child(test.chord.clone()),
        )
}

/// The Data folder's **Change…** button: its label, then the key that presses it, faint.
fn button(
    id: &'static str,
    label: &Hint,
    event: GeneralEvent,
    clicks: &PageClicks<GeneralPage>,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .cursor_pointer()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(6.0))
        .h(px(32.0))
        .px(px(12.0))
        .border_1()
        .border_color(color::border(cx))
        .text_size(px(13.0))
        .text_color(color::foreground(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(clicks.send(event))
        .child(label.action.clone())
        .child(
            div()
                .text_color(color::faint_text(cx))
                .font_weight(gpui::FontWeight::NORMAL)
                .child(label.chord.clone()),
        )
}

/// Start view's dropdown (#1156): seven choices don't fit a segmented control in the field column.
/// A noun from the file that isn't a choice shows nothing chosen and is named under the field.
fn start_view_field(
    view: &FieldView,
    unlisted: Option<&str>,
    clicks: &PageClicks<GeneralPage>,
    scroll: &mut usize,
    cx: &App,
) -> impl IntoElement {
    let unlisted = unlisted.map(|text| {
        div()
            .debug_selector(|| "general-start-view-unlisted".to_string())
            .child(hint(text.to_string(), cx))
    });
    focus_mark(view.focused, cx)
        .child(dropdown(
            "general-start-view",
            view,
            GeneralField::StartView,
            clicks,
            scroll,
            cx,
        ))
        .children(unlisted)
        .children(view.hint.clone().map(|text| hint(text, cx)))
        .children(shadowed(view, cx))
}

/// The Locale's dropdown (#1156), each choice its label then its code (`System · now en-AU`), then
/// the note while the choice waits on the next start or a source above the file overrides it
/// (DESKTOP-185, DESKTOP-186).
fn locale_field(
    view: &FieldView,
    fallback: Option<&str>,
    clicks: &PageClicks<GeneralPage>,
    scroll: &mut usize,
    cx: &App,
) -> impl IntoElement {
    let note = view
        .own_note
        .clone()
        .map(|text| note(text, "general-locale-note", cx));
    focus_mark(view.focused, cx)
        .debug_selector(|| "general-locale".to_string())
        .child(dropdown(
            "general-locale-select",
            view,
            GeneralField::Locale,
            clicks,
            scroll,
            cx,
        ))
        .children(note)
        .children(fallback.map(|text| hint(text.to_string(), cx)))
        .children(view.hint.clone().map(|text| hint(text, cx)))
}

/// The Data folder: its path in monospace beside **Change…** (DESKTOP-187), then the note while a
/// source above the file sets it (DESKTOP-186) and the hint.
fn data_folder_field(
    view: &FieldView,
    folder: &DataFolderView,
    clicks: &PageClicks<GeneralPage>,
    cx: &App,
) -> impl IntoElement {
    focus_mark(view.focused, cx)
        .debug_selector(|| "general-data-folder".to_string())
        .child(field_label(view.label.clone()))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .debug_selector(|| "general-data-folder-path".to_string())
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .font_family("monospace")
                        .text_color(color::muted(cx))
                        .child(field_value(folder.value.clone(), false, cx)),
                )
                .child(button(
                    "general-data-folder-change",
                    &folder.change,
                    GeneralEvent::PressChange,
                    clicks,
                    cx,
                )),
        )
        .children(
            view.own_note
                .clone()
                .map(|text| note(text, "general-data-folder-note", cx)),
        )
        .children(view.hint.clone().map(|text| hint(text, cx)))
}

/// A note under a field, after a square accent marker: the Locale's and the Data folder's.
fn note(text: String, selector: &'static str, cx: &App) -> impl IntoElement {
    div()
        .debug_selector(move || selector.to_string())
        .mt(px(6.0))
        .flex()
        .items_start()
        .gap(px(8.0))
        .text_size(px(11.5))
        .text_color(color::foreground(cx))
        .child(
            div()
                .flex_none()
                .mt(px(4.0))
                .size(px(8.0))
                .bg(color::accent(cx)),
        )
        .child(div().min_w(px(0.0)).child(text))
}

/// The THIS DEVICE box, in the Ledger page's summary treatment: the Ledger On start would reopen
/// and the config file Settings writes to, then the page's footnote.
fn this_device_panel(view: &ThisDeviceView, cx: &App) -> impl IntoElement {
    div()
        .debug_selector(|| "general-this-device".to_string())
        .flex_1()
        .min_w(px(0.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .mb(px(10.0))
                .child(lib_locale::format::upper(&view.heading)),
        )
        .child(
            div()
                .border_1()
                .border_color(color::border(cx))
                .bg(color::chrome(cx))
                .flex()
                .flex_col()
                .children(view.rows.iter().enumerate().map(|(index, (label, value))| {
                    div()
                        .flex()
                        .justify_between()
                        .gap(px(12.0))
                        .px(px(12.0))
                        .py(px(10.0))
                        .text_size(px(12.0))
                        .when(index == 0, |this| {
                            this.border_b(px(1.0)).border_color(color::hairline(cx))
                        })
                        .child(
                            div()
                                .flex_none()
                                .text_color(color::muted(cx))
                                .child(label.clone()),
                        )
                        .child(
                            div()
                                .min_w(px(0.0))
                                .truncate()
                                .font_family("monospace")
                                .child(value.clone()),
                        )
                })),
        )
        .child(
            div()
                .mt(px(14.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(view.note.clone()),
        )
}
