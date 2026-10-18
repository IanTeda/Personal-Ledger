//! Renders **Add price source** and a Custom source's **Edit price source** (16z·custom, 620px),
//! on the shared `crate::dialog` chrome. There is no built-in mode and no Source toggle: the
//! built-ins are always listed, so **+** adds a Custom source only.
//!
//! Name and Updates share a row; Address, Request and Price path are monospace; Authentication,
//! Key parameter and API key share the last row, the key fields only while API key is chosen. The
//! API key is masked and never stored, which the footer note says.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use super::{Change, SourceDialogHandlers, help, test_not_built, updates_label};
use crate::{
    dialog,
    form::fields::{field_label, segmented_control},
    settings::units::source_form::{
        AUTH_OPTIONS, AuthChoice, CustomSourceForm, SourceField, UPDATES_OPTIONS,
    },
    theme::color,
};

/// `width:620px` (16z·custom).
const WIDTH: gpui::Pixels = px(620.0);
const MONOSPACE: &str = "monospace";

/// The API key as it is drawn: one bullet per character.
pub(crate) fn masked(key: &str) -> String {
    "\u{2022}".repeat(key.chars().count())
}

fn auth_label(auth: AuthChoice) -> String {
    match auth {
        AuthChoice::None => crate::msg::desktop_settings_price_source_auth_none(),
        AuthChoice::ApiKey => crate::msg::desktop_settings_price_source_auth_api_key(),
    }
}

/// One of the form's text boxes.
struct Field<'a> {
    id: &'static str,
    label: String,
    value: &'a str,
    placeholder: String,
    field: SourceField,
    mono: bool,
}

pub fn render(
    form: &CustomSourceForm,
    editing: bool,
    handlers: SourceDialogHandlers,
    cx: &App,
) -> AnyElement {
    let (prefix, title, submit) = if editing {
        (
            "edit-price-source",
            crate::msg::desktop_settings_price_source_edit_title(form.name.text()),
            crate::msg::desktop_settings_price_source_edit_submit(),
        )
    } else {
        (
            "add-price-source",
            crate::msg::desktop_settings_price_source_add_title(),
            crate::msg::desktop_settings_price_source_add_submit(),
        )
    };
    let text = |field: Field<'_>| text_box(field, form.focused, &handlers, cx);

    let name = div()
        .flex_1()
        .min_w(px(0.0))
        .child(text(Field {
            id: "price-source-name",
            label: lib_locale::msg::column_name(),
            value: form.name.text(),
            placeholder: crate::msg::desktop_settings_price_source_name_placeholder(),
            field: SourceField::Name,
            mono: false,
        }))
        .child(help(
            crate::msg::desktop_settings_price_source_name_help(),
            cx,
        ));
    let updates = {
        let on_change = handlers.on_change.clone();
        div()
            .flex_none()
            .child(field_label(
                crate::msg::desktop_settings_price_source_field_updates(),
            ))
            .child(segmented_control(
                "price-source-updates",
                &UPDATES_OPTIONS,
                form.updates,
                updates_label,
                Rc::new(move |updates, window: &mut Window, cx: &mut App| {
                    on_change(Change::Updates(updates), window, cx);
                }),
                cx,
            ))
    };
    let auth = {
        let on_change = handlers.on_change.clone();
        div()
            .flex_none()
            .child(field_label(
                crate::msg::desktop_settings_price_source_field_auth(),
            ))
            .child(segmented_control(
                "price-source-auth",
                &AUTH_OPTIONS,
                form.auth,
                auth_label,
                Rc::new(move |auth, window: &mut Window, cx: &mut App| {
                    on_change(Change::Auth(auth), window, cx);
                }),
                cx,
            ))
    };
    let shows_key = form.shows(SourceField::KeyParam);
    let masked_key = masked(form.api_key.text());

    let fields = [
        div()
            .flex()
            .items_start()
            .gap(px(16.0))
            .child(name)
            .child(updates)
            .into_any_element(),
        text(Field {
            id: "price-source-address",
            label: crate::msg::desktop_settings_price_source_field_address(),
            value: form.address.text(),
            placeholder: crate::msg::desktop_settings_price_source_address_placeholder(),
            field: SourceField::Address,
            mono: true,
        }),
        div()
            .child(text(Field {
                id: "price-source-request",
                label: crate::msg::desktop_settings_price_source_field_request(),
                value: form.request.text(),
                placeholder: crate::msg::desktop_settings_price_source_request_placeholder(),
                field: SourceField::Request,
                mono: true,
            }))
            .child(help(
                crate::msg::desktop_settings_price_source_request_help(),
                cx,
            ))
            .into_any_element(),
        div()
            .child(text(Field {
                id: "price-source-price-path",
                label: crate::msg::desktop_settings_price_source_field_price_path(),
                value: form.price_path.text(),
                placeholder: crate::msg::desktop_settings_price_source_price_path_placeholder(),
                field: SourceField::PricePath,
                mono: true,
            }))
            .child(help(
                crate::msg::desktop_settings_price_source_price_path_help(),
                cx,
            ))
            .into_any_element(),
        div()
            .flex()
            .items_start()
            .gap(px(12.0))
            .child(auth)
            .when(shows_key, |this| {
                this.child(div().flex_1().min_w(px(0.0)).child(text(Field {
                    id: "price-source-key-param",
                    label: crate::msg::desktop_settings_price_source_field_key_param(),
                    value: form.key_param.text(),
                    placeholder: crate::msg::desktop_settings_price_source_key_param_placeholder(),
                    field: SourceField::KeyParam,
                    mono: true,
                })))
                .child(div().flex_1().min_w(px(0.0)).child(text(Field {
                    id: "price-source-api-key",
                    label: crate::msg::desktop_settings_price_source_field_api_key(),
                    value: &masked_key,
                    placeholder: String::new(),
                    field: SourceField::ApiKey,
                    mono: true,
                })))
            })
            .into_any_element(),
        test_not_built(
            "price-source-test-with-symbol",
            crate::msg::desktop_settings_price_source_test_with_symbol(),
            cx,
        )
        .into_any_element(),
        div()
            .debug_selector(|| "price-source-key-note".to_string())
            .text_size(px(11.5))
            .text_color(color::faint_text(cx))
            .child(crate::msg::desktop_settings_price_source_key_note())
            .into_any_element(),
    ];

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, false, cx))
        .child(
            div()
                .p(px(20.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .children(fields),
        )
        .child(dialog::action_row(
            [
                dialog::cancel_button(
                    SharedString::from(format!("{prefix}-cancel")),
                    handlers.on_cancel.clone(),
                    cx,
                )
                .into_any_element(),
                dialog::confirm_button(
                    SharedString::from(format!("{prefix}-confirm")),
                    submit,
                    form.is_valid(),
                    false,
                    handlers.on_confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}

/// A label above a clickable box, accent-bordered with a trailing caret while focused, as
/// `form::fields::text_field` draws one, and monospace for the endpoint's parts.
fn text_box(
    field: Field<'_>,
    focused: SourceField,
    handlers: &SourceDialogHandlers,
    cx: &App,
) -> AnyElement {
    let Field {
        id,
        label,
        value,
        placeholder,
        field,
        mono,
    } = field;
    let focused = focused == field;
    let caret = if focused { "\u{2502}" } else { "" };
    let (text, text_color) = if value.is_empty() {
        (format!("{placeholder}{caret}"), color::faint_text(cx))
    } else {
        (format!("{value}{caret}"), color::foreground(cx))
    };
    let on_click = handlers.click(Change::Focus(field));
    div()
        .child(field_label(label))
        .child(
            div()
                .id(id)
                .debug_selector(move || id.to_string())
                .cursor_pointer()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(if focused {
                    color::accent(cx)
                } else {
                    color::border(cx)
                })
                .text_size(px(13.0))
                .when(mono, |this| this.font_family(MONOSPACE))
                .text_color(text_color)
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(text),
        )
        .into_any_element()
}
