//! Settings › General drawn from its view model (DESKTOP-178): the field column (On start, Start
//! view, Toasts, the Locale and the Data folder) beside the THIS DEVICE box. Every click is a
//! Presenter event through [`PageClicks`].

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_core::StartView;
use lib_settings::presenter::general::{
    GENERAL_FIELD_LOCALE, GENERAL_FIELD_ON_START, GENERAL_FIELD_START_VIEW, GENERAL_FIELD_TOASTS,
};
use lib_settings::{
    DeviceKey, GeneralEvent, GeneralViewModel,
    presenter::{ControlView, DataFolderView, LocaleControl, StartViewControl, ThisDeviceView},
};
use lib_viper::view::Hint;

use super::GeneralPage;
use crate::{
    form::fields::{UNIFORM_OPTION_WIDTH, field_label, field_value, radio_dot, segmented_labelled},
    settings::controls::{focus_mark, hint, shadowed},
    shell::view::page_clicks::PageClicks,
    theme::color,
};

/// The page drawn from `model`.
pub(super) fn page(
    model: &GeneralViewModel,
    clicks: &PageClicks<GeneralPage>,
    cx: &App,
) -> AnyElement {
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
                .child(segmented(
                    "general-on-start",
                    &model.on_start,
                    DeviceKey::OnStart,
                    model.field == Some(GENERAL_FIELD_ON_START),
                    None,
                    clicks.segment(GeneralEvent::ChooseOnStart),
                    None,
                    cx,
                ))
                .child(start_view_field(
                    &model.start_view,
                    model.field == Some(GENERAL_FIELD_START_VIEW),
                    clicks,
                    cx,
                ))
                .child(segmented(
                    "general-toasts",
                    &model.toasts,
                    DeviceKey::Toasts,
                    model.field == Some(GENERAL_FIELD_TOASTS),
                    Some(px(UNIFORM_OPTION_WIDTH)),
                    clicks.segment(GeneralEvent::ChooseToasts),
                    Some(test_button(&model.toasts_test, clicks, cx).into_any_element()),
                    cx,
                ))
                .child(locale_field(
                    &model.locale,
                    model.field == Some(GENERAL_FIELD_LOCALE),
                    clicks,
                    cx,
                ))
                .child(data_folder_field(&model.data_folder, clicks, cx)),
        )
        .child(this_device_panel(&model.this_device, cx))
        .into_any_element()
}

/// A segmented control with its label, hint and shadowed note, and `beside` it 8px to its right if
/// given. `focused` is the keyboard's mark, drawn as the Display page draws it.
#[expect(
    clippy::too_many_arguments,
    reason = "one control's parts, each drawn once; a struct would only rename them"
)]
fn segmented<T: Copy + PartialEq + 'static>(
    id: &'static str,
    control: &ControlView<T>,
    key: DeviceKey,
    focused: bool,
    option_width: Option<gpui::Pixels>,
    on_click: crate::form::fields::OnSegmentClick<T>,
    beside: Option<AnyElement>,
    cx: &App,
) -> impl IntoElement {
    let select = &control.select;
    let options = select
        .options
        .iter()
        .map(|choice| (choice.value, SharedString::from(choice.label.clone())))
        .collect();
    let chosen = select.chosen.unwrap_or(select.options[0].value);
    focus_mark(focused, cx)
        .child(field_label(select.label.clone()))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(segmented_labelled(
                    id,
                    options,
                    chosen,
                    on_click,
                    option_width,
                    cx,
                ))
                .children(beside),
        )
        .children(control.hint.clone().map(|text| hint(text, cx)))
        .children(shadowed(key, control.shadowed.as_deref(), cx))
}

/// Toasts' secondary **Test** button, its key after its label. It is no `j`/`k` stop of its own:
/// the key presses it from Toasts.
fn test_button(test: &Hint, clicks: &PageClicks<GeneralPage>, cx: &App) -> impl IntoElement {
    button(
        "general-toasts-test",
        test,
        GeneralEvent::PressTest,
        clicks,
        cx,
    )
}

/// A secondary button: its label, then the key that presses it, faint.
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

/// Start view's choices as a radio group, one per line: seven don't fit a segmented control in the
/// field column. A noun from the file that isn't a choice checks none and is named in the hint.
fn start_view_field(
    view: &StartViewControl,
    focused: bool,
    clicks: &PageClicks<GeneralPage>,
    cx: &App,
) -> impl IntoElement {
    let control = &view.control;
    let unlisted = view.unlisted.clone().map(|text| {
        div()
            .debug_selector(|| "general-start-view-unlisted".to_string())
            .child(hint(text, cx))
    });
    focus_mark(focused, cx)
        .child(field_label(control.select.label.clone()))
        .child(
            div().flex().flex_col().gap(px(6.0)).children(
                control
                    .select
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, choice)| {
                        let id = format!("general-start-view-{index}");
                        let checked = control.select.chosen.as_ref() == Some(&choice.value);
                        let value: StartView = choice.value.clone();
                        div()
                            .debug_selector({
                                let id = id.clone();
                                move || id
                            })
                            .id(SharedString::from(id))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .on_click(clicks.send(GeneralEvent::ChooseStartView(value)))
                            .child(radio_dot(checked, cx))
                            .child(div().text_size(px(13.0)).child(choice.label.clone()))
                    }),
            ),
        )
        .children(unlisted)
        .children(control.hint.clone().map(|text| hint(text, cx)))
        .children(shadowed(
            DeviceKey::StartView,
            control.shadowed.as_deref(),
            cx,
        ))
}

/// The Locale's choices as a radio group, one per line, each label followed by its code (`now
/// en-AU` after **System**), then the note while the choice waits on the next start or a source
/// above the file overrides it (DESKTOP-185, DESKTOP-186).
fn locale_field(
    view: &LocaleControl,
    focused: bool,
    clicks: &PageClicks<GeneralPage>,
    cx: &App,
) -> impl IntoElement {
    let select = &view.control.select;
    let note = view
        .note
        .clone()
        .map(|text| note(text, "general-locale-note", cx));
    focus_mark(focused, cx)
        .debug_selector(|| "general-locale".to_string())
        .child(field_label(select.label.clone()))
        .child(
            div().flex().flex_col().gap(px(6.0)).children(
                select.options.iter().zip(&view.codes).enumerate().map(
                    |(index, (choice, code))| {
                        let id = format!("general-locale-{index}");
                        let checked = select.chosen == Some(choice.value);
                        div()
                            .debug_selector({
                                let id = id.clone();
                                move || id
                            })
                            .id(SharedString::from(id))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .on_click(clicks.send(GeneralEvent::ChooseLocale(choice.value)))
                            .child(radio_dot(checked, cx))
                            .child(div().text_size(px(13.0)).child(choice.label.clone()))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_family("monospace")
                                    .text_color(color::faint_text(cx))
                                    .child(code.clone()),
                            )
                    },
                ),
            ),
        )
        .children(note)
        .children(view.fallback.clone().map(|text| hint(text, cx)))
        .children(view.control.hint.clone().map(|text| hint(text, cx)))
}

/// The Data folder: its path in monospace beside **Change…** (DESKTOP-187), then the note while a
/// source above the file sets it (DESKTOP-186) and the hint.
fn data_folder_field(
    view: &DataFolderView,
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
                        .child(field_value(view.value.clone(), false, cx)),
                )
                .child(button(
                    "general-data-folder-change",
                    &view.change,
                    GeneralEvent::PressChange,
                    clicks,
                    cx,
                )),
        )
        .children(
            view.note
                .clone()
                .map(|text| note(text, "general-data-folder-note", cx)),
        )
        .child(hint(view.hint.clone(), cx))
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
