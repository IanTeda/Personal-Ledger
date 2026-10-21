//! Add and Edit price source for a Custom source (16z·custom, 620px) and Edit price source for a
//! built-in (16z·edit, 480px): Name and Updates, then a Custom source's endpoint and key fields,
//! or a built-in's endpoint as read-only facts. Updates and Authentication are segmented controls
//! that `Left`/`Right` step while focused.
//!
//! The typed API key is kept here only to append to; the Presenter takes it as a secret and the
//! form draws it masked. It is never stored (ADR-0043).

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_units::Updates;
use lib_units::presenter::{
    AuthChoice, Choice, PriceSourceField, PriceSourceFormEvent, PriceSourceFormPresenter,
    PriceSourceFormViewModel, SelectView, TextFieldView,
};
use secrecy::{ExposeSecret as _, SecretString};

use super::{Shared, UnitsDialog, View, enter, help, refusal, send, test_not_built, typed};
use crate::{
    chrome::dialog_host::{DialogActions, DialogKey, DialogOutcome},
    dialog,
    form::fields::{field_label, segmented_labelled, text_field},
    theme::color,
};

/// `width:620px` (16z·custom).
const CUSTOM_WIDTH: gpui::Pixels = px(620.0);
/// `width:480px` (16z·edit).
const BUILT_IN_WIDTH: gpui::Pixels = px(480.0);
const MONOSPACE: &str = "monospace";

/// The API key as typed so far.
#[derive(Clone, Default)]
pub(super) struct ApiKeyInput(SecretString);

pub(super) fn open(presenter: PriceSourceFormPresenter) -> View {
    View::PriceSourceForm(Rc::new(RefCell::new(presenter)), ApiKeyInput::default())
}

fn handle(presenter: &Shared<PriceSourceFormPresenter>, event: PriceSourceFormEvent) {
    send(presenter, event, PriceSourceFormPresenter::handle);
}

/// The field with focus, as the view model draws it.
fn focused(view: &PriceSourceFormViewModel) -> Option<PriceSourceField> {
    let text_focus = |field: &TextFieldView| field.focused;
    if view.name.focused {
        return Some(PriceSourceField::Name);
    }
    if view.updates.focused {
        return Some(PriceSourceField::Updates);
    }
    let endpoint = view.endpoint.as_ref()?;
    [
        (text_focus(&endpoint.address), PriceSourceField::Address),
        (text_focus(&endpoint.request), PriceSourceField::Request),
        (
            text_focus(&endpoint.price_path),
            PriceSourceField::PricePath,
        ),
        (endpoint.auth.focused, PriceSourceField::Auth),
        (
            endpoint.key_param.as_ref().is_some_and(text_focus),
            PriceSourceField::KeyParam,
        ),
        (
            endpoint.api_key.as_ref().is_some_and(text_focus),
            PriceSourceField::ApiKey,
        ),
    ]
    .into_iter()
    .find_map(|(is_focused, field)| is_focused.then_some(field))
}

/// The text a text field holds; the API key is read from the input, never the masked view.
fn text_of(view: &PriceSourceFormViewModel, field: PriceSourceField) -> Option<String> {
    let endpoint = view.endpoint.as_ref();
    Some(match field {
        PriceSourceField::Name => view.name.text.clone(),
        PriceSourceField::Address => endpoint?.address.text.clone(),
        PriceSourceField::Request => endpoint?.request.text.clone(),
        PriceSourceField::PricePath => endpoint?.price_path.text.clone(),
        PriceSourceField::KeyParam => endpoint?.key_param.as_ref()?.text.clone(),
        PriceSourceField::Updates | PriceSourceField::Auth | PriceSourceField::ApiKey => {
            return None;
        }
    })
}

pub(super) fn key(
    presenter: &Shared<PriceSourceFormPresenter>,
    api_key: &mut ApiKeyInput,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.borrow().view_model();
    let event = match (key, focused(&view)) {
        (DialogKey::Tab, _) => PriceSourceFormEvent::NextField,
        (DialogKey::BackTab, _) => PriceSourceFormEvent::PrevField,
        (DialogKey::Enter | DialogKey::CtrlEnter, _) => return enter(view.frame.can_confirm),
        (
            DialogKey::Left | DialogKey::Right,
            Some(PriceSourceField::Updates | PriceSourceField::Auth),
        ) => PriceSourceFormEvent::Step(if key == DialogKey::Left { -1 } else { 1 }),
        (DialogKey::Char(_), Some(PriceSourceField::Updates | PriceSourceField::Auth)) => {
            return DialogOutcome::Handled;
        }
        (_, Some(PriceSourceField::ApiKey)) => {
            let Some(text) = typed(api_key.0.expose_secret(), key) else {
                return DialogOutcome::Ignored;
            };
            api_key.0 = SecretString::from(text);
            PriceSourceFormEvent::ApiKey(api_key.0.clone())
        }
        (_, Some(field)) => {
            let Some(text) = text_of(&view, field).and_then(|text| typed(&text, key)) else {
                return DialogOutcome::Ignored;
            };
            PriceSourceFormEvent::Text(field, text)
        }
        (_, None) => return DialogOutcome::Ignored,
    };
    handle(presenter, event);
    DialogOutcome::Handled
}

/// A click that sends `event` to the open form.
fn click(actions: &DialogActions, event: PriceSourceFormEvent) -> dialog::OnClick {
    UnitsDialog::on_click(actions, move |dialog| {
        if let View::PriceSourceForm(presenter, _) = &dialog.view {
            handle(presenter, event.clone());
        }
    })
}

/// A segmented control over a select's choices, each click sending `event` of the choice.
fn segmented<T: Copy + PartialEq + 'static>(
    id_prefix: &'static str,
    select: &SelectView<T>,
    actions: &DialogActions,
    event: fn(T) -> PriceSourceFormEvent,
    cx: &App,
) -> AnyElement {
    let Some(current) = select.chosen else {
        return div().into_any_element();
    };
    let options = select
        .options
        .iter()
        .map(|Choice { value, label }| (*value, SharedString::from(label.clone())))
        .collect();
    let actions = actions.clone();
    div()
        .flex_none()
        .child(field_label(select.label.clone()))
        .child(segmented_labelled(
            id_prefix,
            options,
            current,
            Rc::new(move |value: T, window: &mut Window, cx: &mut App| {
                click(&actions, event(value))(window, cx);
            }),
            None,
            cx,
        ))
        .into_any_element()
}

/// A label above a clickable box, accent-bordered with a trailing caret while focused, and
/// monospace for the endpoint's parts.
fn text_box(
    id: &'static str,
    field: &TextFieldView,
    which: PriceSourceField,
    mono: bool,
    actions: &DialogActions,
    cx: &App,
) -> AnyElement {
    let caret = if field.focused { "\u{2502}" } else { "" };
    let (text, text_color) = if field.text.is_empty() {
        (
            format!("{}{caret}", field.placeholder),
            color::faint_text(cx),
        )
    } else {
        (format!("{}{caret}", field.text), color::foreground(cx))
    };
    let on_click = click(actions, PriceSourceFormEvent::Focus(which));
    div()
        .child(field_label(field.label.clone()))
        .child(
            div()
                .id(id)
                .debug_selector(move || id.to_string())
                .cursor_pointer()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(if field.focused {
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

pub(super) fn render(
    view: &PriceSourceFormViewModel,
    actions: &DialogActions,
    cx: &App,
) -> AnyElement {
    match &view.endpoint {
        Some(_) => render_custom(view, actions, cx),
        None => render_built_in(view, actions, cx),
    }
}

fn render_custom(view: &PriceSourceFormViewModel, actions: &DialogActions, cx: &App) -> AnyElement {
    let prefix = if view.usage.is_some() {
        "edit-price-source"
    } else {
        "add-price-source"
    };
    let Some(endpoint) = &view.endpoint else {
        return div().into_any_element();
    };
    let text =
        |id, field: &TextFieldView, which, mono| text_box(id, field, which, mono, actions, cx);
    let mut fields = vec![
        div()
            .flex()
            .items_start()
            .gap(px(16.0))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(text(
                        "price-source-name",
                        &view.name,
                        PriceSourceField::Name,
                        false,
                    ))
                    .child(help(view.name_help.clone(), cx)),
            )
            .child(segmented(
                "price-source-updates",
                &view.updates,
                actions,
                PriceSourceFormEvent::Updates as fn(Updates) -> PriceSourceFormEvent,
                cx,
            ))
            .into_any_element(),
        text(
            "price-source-address",
            &endpoint.address,
            PriceSourceField::Address,
            true,
        ),
        div()
            .child(text(
                "price-source-request",
                &endpoint.request,
                PriceSourceField::Request,
                true,
            ))
            .child(help(endpoint.request_help.clone(), cx))
            .into_any_element(),
        div()
            .child(text(
                "price-source-price-path",
                &endpoint.price_path,
                PriceSourceField::PricePath,
                true,
            ))
            .child(help(endpoint.price_path_help.clone(), cx))
            .into_any_element(),
        div()
            .flex()
            .items_start()
            .gap(px(12.0))
            .child(segmented(
                "price-source-auth",
                &endpoint.auth,
                actions,
                PriceSourceFormEvent::Auth as fn(AuthChoice) -> PriceSourceFormEvent,
                cx,
            ))
            .children(endpoint.key_param.as_ref().map(|key_param| {
                div().flex_1().min_w(px(0.0)).child(text(
                    "price-source-key-param",
                    key_param,
                    PriceSourceField::KeyParam,
                    true,
                ))
            }))
            .children(endpoint.api_key.as_ref().map(|api_key| {
                div().flex_1().min_w(px(0.0)).child(text(
                    "price-source-api-key",
                    api_key,
                    PriceSourceField::ApiKey,
                    true,
                ))
            }))
            .into_any_element(),
        test_not_built(
            "price-source-test-with-symbol",
            view.test.clone(),
            view.test_note.clone(),
            cx,
        ),
    ];
    if let Some(note) = &endpoint.key_note {
        fields.push(
            div()
                .debug_selector(|| "price-source-key-note".to_string())
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child(note.clone())
                .into_any_element(),
        );
    }
    fields.extend(view.usage.clone().map(|usage| {
        div()
            .debug_selector(|| "edit-price-source-usage".to_string())
            .child(dialog::info_panel(usage, cx))
            .into_any_element()
    }));
    fields.extend(refusal(view.refusal.clone(), cx));
    frame(view, prefix, CUSTOM_WIDTH, fields, actions, cx)
}

fn render_built_in(
    view: &PriceSourceFormViewModel,
    actions: &DialogActions,
    cx: &App,
) -> AnyElement {
    let mut fields = vec![
        div()
            .child(text_field(
                "edit-built-in-source-name",
                view.name.label.clone(),
                &view.name.text,
                &view.name.placeholder,
                view.name.focused,
                click(actions, PriceSourceFormEvent::Focus(PriceSourceField::Name)),
                cx,
            ))
            .child(help(view.name_help.clone(), cx))
            .into_any_element(),
        fact_block(view, cx),
        segmented(
            "edit-built-in-source-updates",
            &view.updates,
            actions,
            PriceSourceFormEvent::Updates as fn(Updates) -> PriceSourceFormEvent,
            cx,
        ),
    ];
    fields.extend(view.usage.clone().map(|usage| {
        div()
            .debug_selector(|| "edit-built-in-source-usage".to_string())
            .child(dialog::info_panel(usage, cx))
            .into_any_element()
    }));
    fields.push(test_not_built(
        "edit-built-in-source-test",
        view.test.clone(),
        view.test_note.clone(),
        cx,
    ));
    fields.extend(refusal(view.refusal.clone(), cx));
    frame(
        view,
        "edit-built-in-source",
        BUILT_IN_WIDTH,
        fields,
        actions,
        cx,
    )
}

/// The card: title, the fields, Cancel and the submit.
fn frame(
    view: &PriceSourceFormViewModel,
    prefix: &'static str,
    width: gpui::Pixels,
    fields: Vec<AnyElement>,
    actions: &DialogActions,
    cx: &App,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
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
                    actions.cancel.clone(),
                    cx,
                )
                .into_any_element(),
                dialog::confirm_button(
                    SharedString::from(format!("{prefix}-confirm")),
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    false,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(width, false, card, cx)
}

/// A built-in's endpoint, read-only: fact rows as the X-Large pane's, the URL parts monospace,
/// then the note on why they aren't edited here.
fn fact_block(view: &PriceSourceFormViewModel, cx: &App) -> AnyElement {
    div()
        .debug_selector(|| "edit-built-in-source-facts".to_string())
        .flex()
        .flex_col()
        .px(px(12.0))
        .py(px(8.0))
        .bg(color::chrome(cx))
        .children(
            view.built_in_facts
                .iter()
                .enumerate()
                .map(|(index, (label, value))| {
                    // The first fact names the source; the rest are its URL parts.
                    let mono = index > 0;
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
                                .child(label.clone()),
                        )
                        .child(
                            div()
                                .min_w(px(0.0))
                                .text_align(gpui::TextAlign::Right)
                                .when(mono, |this| this.font_family(MONOSPACE))
                                .child(value.clone()),
                        )
                }),
        )
        .children(view.built_in_help.clone().map(|text| help(text, cx)))
        .into_any_element()
}

pub(super) fn readback(view: &PriceSourceFormViewModel) -> Vec<(&'static str, String)> {
    let endpoint = view.endpoint.as_ref();
    let text =
        |field: Option<&TextFieldView>| field.map(|field| field.text.clone()).unwrap_or_default();
    vec![
        ("name", view.name.text.clone()),
        (
            "updates",
            super::chosen_label(&view.updates).unwrap_or_default(),
        ),
        ("address", text(endpoint.map(|e| &e.address))),
        ("request", text(endpoint.map(|e| &e.request))),
        ("price_path", text(endpoint.map(|e| &e.price_path))),
        (
            "auth",
            endpoint
                .and_then(|e| super::chosen_label(&e.auth))
                .unwrap_or_default(),
        ),
        (
            "key_param",
            text(endpoint.and_then(|e| e.key_param.as_ref())),
        ),
        ("api_key", text(endpoint.and_then(|e| e.api_key.as_ref()))),
        (
            "field",
            focused(view)
                .map(|field| format!("{field:?}"))
                .unwrap_or_default(),
        ),
        ("usage", view.usage.clone().unwrap_or_default()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
