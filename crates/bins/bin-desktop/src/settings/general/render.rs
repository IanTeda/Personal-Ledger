use std::{path::Path, rc::Rc};

use gpui::{App, Context, Window, div, prelude::*, px};
use lib_config::ConfigKey;

use crate::settings::device::shadowed_note;
use lib_core::OnStart;

use super::{
    GENERAL_FIELD_ON_START, GENERAL_FIELD_TOASTS, GeneralPage, ON_START_CHOICES, TOASTS_CHOICES,
    ThisDevice,
};
use crate::{
    form::fields::{UNIFORM_OPTION_WIDTH, field_label, field_value, segmented_control_sized},
    theme::color,
};

/// The field column's width (DESKTOP-178).
const FIELD_COLUMN_WIDTH: gpui::Pixels = px(320.0);

impl Render for GeneralPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let on_start_click = {
            let entity = entity.clone();
            Rc::new(move |choice, _window: &mut Window, cx: &mut App| {
                entity.update(cx, |page, cx| page.click_on_start(choice, cx));
            })
        };
        let toasts_click = Rc::new(move |on, _window: &mut Window, cx: &mut App| {
            entity.update(cx, |page, cx| page.click_toasts(on, cx));
        });
        let focus = self.field_focus;
        let cx: &App = cx;
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
                    .child(
                        focus_mark(focus == Some(GENERAL_FIELD_ON_START), cx)
                            .child(field_label(crate::msg::desktop_settings_general_on_start()))
                            .child(segmented_control_sized(
                                "general-on-start",
                                &ON_START_CHOICES,
                                self.on_start,
                                on_start_label,
                                on_start_click,
                                None,
                                cx,
                            ))
                            .child(hint(
                                crate::msg::desktop_settings_general_on_start_hint(),
                                cx,
                            ))
                            .children(shadowed_note(ConfigKey::OnStart, cx)),
                    )
                    .child(
                        focus_mark(focus == Some(GENERAL_FIELD_TOASTS), cx)
                            .child(field_label(lib_locale::msg::toast_setting_label()))
                            .child(segmented_control_sized(
                                "general-toasts",
                                &TOASTS_CHOICES,
                                self.toasts_on,
                                toasts_label,
                                toasts_click,
                                Some(px(UNIFORM_OPTION_WIDTH)),
                                cx,
                            ))
                            .child(hint(lib_locale::msg::toast_setting_note(), cx))
                            .children(shadowed_note(ConfigKey::Toasts, cx)),
                    )
                    .child(locale_field(cx))
                    .child(
                        div()
                            .debug_selector(|| "general-ledger-folder".to_string())
                            .child(field_label(
                                crate::msg::desktop_settings_general_ledger_folder(),
                            ))
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_color(color::muted(cx))
                                    .child(field_value(
                                        self.device.ledger_folder.display().to_string(),
                                        false,
                                        cx,
                                    )),
                            )
                            .child(hint(
                                crate::msg::desktop_settings_general_ledger_folder_hint(),
                                cx,
                            )),
                    ),
            )
            .child(this_device_panel(&self.device, cx))
    }
}

fn on_start_label(choice: OnStart) -> String {
    match choice {
        OnStart::ReopenLastLedger => crate::msg::desktop_settings_general_on_start_reopen(),
        OnStart::NoLedger => crate::msg::desktop_settings_general_on_start_none(),
    }
}

fn toasts_label(on: bool) -> String {
    if on {
        lib_locale::msg::toast_setting_on()
    } else {
        lib_locale::msg::toast_setting_off()
    }
}

/// The keyboard-focus mark for one control: an accent rule down its left edge, drawn on every
/// control so the column does not shift as focus moves (the Display page's treatment).
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

fn hint(text: String, cx: &App) -> impl IntoElement {
    div()
        .mt(px(3.0))
        .text_size(px(12.0))
        .text_color(color::muted(cx))
        .child(text)
}

/// The effective Locale, read-only, with where it came from. There is no control: the Locale is
/// Configuration, changed by the `locale` setting or `--locale` and a restart (ADR-0021).
fn locale_field(cx: &App) -> impl IntoElement {
    let (line, fallback_note) = crate::locale::describe(&crate::locale::info());
    div()
        .debug_selector(|| "general-locale".to_string())
        .child(field_label(
            crate::msg::desktop_settings_general_locale_label(),
        ))
        .child(
            div()
                .text_color(color::muted(cx))
                .child(field_value(line, false, cx)),
        )
        .children(fallback_note.map(|note| hint(note, cx)))
        .child(hint(crate::msg::desktop_settings_general_locale_hint(), cx))
}

/// The THIS DEVICE box, in the Ledger page's summary treatment: the Ledger On start would reopen
/// and the config file Settings writes to, then the page's footnote.
fn this_device_panel(device: &ThisDevice, cx: &App) -> impl IntoElement {
    let path = |path: Option<&Path>| {
        path.map_or_else(crate::msg::desktop_settings_general_none, |path| {
            path.display().to_string()
        })
    };
    let rows = [
        (
            crate::msg::desktop_settings_general_last_ledger(),
            path(device.last_ledger.as_deref()),
        ),
        (
            crate::msg::desktop_settings_general_configuration(),
            path(device.configuration.as_deref()),
        ),
    ];
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
                .child(lib_locale::format::upper(
                    &crate::msg::desktop_settings_general_this_device(),
                )),
        )
        .child(
            div()
                .border_1()
                .border_color(color::border(cx))
                .bg(color::chrome(cx))
                .flex()
                .flex_col()
                .children(rows.into_iter().enumerate().map(|(index, (label, value))| {
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
                        .child(div().flex_none().text_color(color::muted(cx)).child(label))
                        .child(
                            div()
                                .min_w(px(0.0))
                                .truncate()
                                .font_family("monospace")
                                .child(value),
                        )
                })),
        )
        .child(
            div()
                .mt(px(14.0))
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_settings_general_note()),
        )
}
