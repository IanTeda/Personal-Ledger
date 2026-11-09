//! Settings › General drawn from its view model (DESKTOP-178): the field column (On start, Start
//! view, Toasts, the read-only Locale and Ledger folder) beside the THIS DEVICE box. Every click is
//! a Presenter event through [`PageClicks`].

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_core::StartView;
use lib_settings::presenter::general::{
    GENERAL_FIELD_ON_START, GENERAL_FIELD_START_VIEW, GENERAL_FIELD_TOASTS,
};
use lib_settings::{
    DeviceKey, GeneralEvent, GeneralViewModel,
    presenter::{ControlView, ReadOnlyView, StartViewControl, ThisDeviceView},
};

use super::GeneralPage;
use crate::{
    form::fields::{UNIFORM_OPTION_WIDTH, field_label, field_value, radio_dot, segmented_labelled},
    settings::controls::{focus_mark, hint, shadowed},
    shell::view::page_clicks::PageClicks,
    theme::color,
};

/// The field column's width (DESKTOP-178).
const FIELD_COLUMN_WIDTH: gpui::Pixels = px(320.0);

/// The page drawn from `model`.
pub(super) fn page(
    model: &GeneralViewModel,
    clicks: &PageClicks<GeneralPage>,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .gap_x(px(40.0))
        .gap_y(px(24.0))
        .child(
            div()
                .debug_selector(|| "general-fields".to_string())
                .w(FIELD_COLUMN_WIDTH)
                .flex_none()
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
                    cx,
                ))
                .child(
                    read_only("general-locale", &model.locale, false, cx)
                        .children(model.locale_fallback.clone().map(|note| hint(note, cx)))
                        .child(hint(model.locale.hint.clone(), cx)),
                )
                .child(
                    read_only("general-ledger-folder", &model.ledger_folder, true, cx)
                        .child(hint(model.ledger_folder.hint.clone(), cx)),
                ),
        )
        .child(this_device_panel(&model.this_device, cx))
        .into_any_element()
}

/// A segmented control with its label, hint and shadowed note. `focused` is the keyboard's mark,
/// drawn as the Display page draws it.
fn segmented<T: Copy + PartialEq + 'static>(
    id: &'static str,
    control: &ControlView<T>,
    key: DeviceKey,
    focused: bool,
    option_width: Option<gpui::Pixels>,
    on_click: crate::form::fields::OnSegmentClick<T>,
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
        .child(segmented_labelled(
            id,
            options,
            chosen,
            on_click,
            option_width,
            cx,
        ))
        .children(control.hint.clone().map(|text| hint(text, cx)))
        .children(shadowed(key, control.shadowed.as_deref(), cx))
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

/// A read-only line: its label over its value, the Ledger folder's in monospace.
fn read_only(selector: &'static str, view: &ReadOnlyView, monospace: bool, cx: &App) -> gpui::Div {
    div()
        .debug_selector(move || selector.to_string())
        .child(field_label(view.label.clone()))
        .child(
            div()
                .when(monospace, |this| this.font_family("monospace"))
                .text_color(color::muted(cx))
                .child(field_value(view.value.clone(), false, cx)),
        )
}

/// The THIS DEVICE box, in the Ledger page's summary treatment: the Ledger On start would reopen
/// and the config file Settings writes to, then the page's footnote.
fn this_device_panel(view: &ThisDeviceView, cx: &App) -> impl IntoElement {
    div()
        .debug_selector(|| "general-this-device".to_string())
        .flex_1()
        .min_w(px(0.0))
        .max_w(px(400.0))
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
