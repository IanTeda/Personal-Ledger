//! The Add tag and Edit tag Dialogs (`docs/ux/desktop-mockups/19-tags/README.md`'s 7b and 7c) on
//! the shared `crate::dialog` frame: Name with its live inline error, the shared Colour field, the
//! Edit Dialog's Active checkbox and a callout on how names are matched. The swatch row and the
//! checkbox take no text, and Backspace goes nowhere on them.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_tags::presenter::{
    ActiveField, TagField, TagFormEvent, TagFormPresenter, TagFormViewModel,
};

use super::{Clicks, TagsDialog, View};
use crate::form::viper::{enter, typed};
use crate::{
    dialog,
    form::colour_field::{self, ColourFieldProps, error_line},
    form::dialog_fields::{label, text_field},
    shell::view::dialog_host::{DialogKey, DialogOutcome},
    theme::color,
};

fn handle(presenter: &mut TagFormPresenter, event: TagFormEvent) {
    let mut router = crate::tags::router::DialogRouter::default();
    presenter.handle(event, &mut router);
}

pub(super) fn key(presenter: &mut TagFormPresenter, key: DialogKey) -> DialogOutcome {
    let view = presenter.view_model();
    match key {
        DialogKey::Tab => {
            handle(presenter, TagFormEvent::NextField);
            return DialogOutcome::Handled;
        }
        DialogKey::BackTab => {
            handle(presenter, TagFormEvent::PrevField);
            return DialogOutcome::Handled;
        }
        DialogKey::Enter => return enter(view.frame.can_confirm),
        _ => {}
    }
    match view.focused {
        TagField::Name => text_key(presenter, &view.name.text, key, TagFormEvent::Name),
        TagField::Hex => text_key(presenter, &view.colour.hex.text, key, TagFormEvent::Hex),
        TagField::Swatches => match key {
            DialogKey::Left => step(presenter, false),
            DialogKey::Right => step(presenter, true),
            DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
            _ => DialogOutcome::Ignored,
        },
        TagField::Active => match key {
            DialogKey::Char(' ') => {
                handle(presenter, TagFormEvent::ToggleActive);
                DialogOutcome::Handled
            }
            DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
            _ => DialogOutcome::Ignored,
        },
    }
}

fn step(presenter: &mut TagFormPresenter, forward: bool) -> DialogOutcome {
    handle(presenter, TagFormEvent::StepPick { forward });
    DialogOutcome::Handled
}

/// A key on a text field: typing edits it.
fn text_key(
    presenter: &mut TagFormPresenter,
    text: &str,
    key: DialogKey,
    event: fn(String) -> TagFormEvent,
) -> DialogOutcome {
    match typed(text, key) {
        Some(text) => {
            handle(presenter, event(text));
            DialogOutcome::Handled
        }
        None => DialogOutcome::Ignored,
    }
}

fn form(dialog: &mut TagsDialog) -> Option<&mut TagFormPresenter> {
    match &mut dialog.view {
        View::Form(presenter) => Some(presenter),
        View::Remove(_) | View::Merge(..) => None,
    }
}

fn send(dialog: &mut TagsDialog, events: Vec<TagFormEvent>) {
    if let Some(presenter) = form(dialog) {
        for event in events {
            handle(presenter, event);
        }
    }
}

pub(super) fn render(view: &TagFormViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    // `text_field` and the buttons take a `'static` id, so these are spelt out.
    let (id, name_id) = if view.editing {
        ("edit-tag", "edit-tag-name")
    } else {
        ("add-tag", "add-tag-name")
    };
    let click = |field: TagField| {
        actions.click(move |dialog| send(dialog, vec![TagFormEvent::Focus(field)]))
    };
    let on_pick = actions.option(|dialog, index| {
        send(
            dialog,
            vec![
                TagFormEvent::Focus(TagField::Swatches),
                TagFormEvent::Pick(index),
            ],
        );
    });
    let mut fields = vec![
        div()
            .child(text_field(
                name_id,
                label(view.name.label.clone()),
                &view.name.text,
                &view.name.placeholder,
                view.name.focused,
                click(TagField::Name),
                cx,
            ))
            .children(view.name_error.clone().map(|error| error_line(error, cx)))
            .into_any_element(),
        colour_field::render(
            ColourFieldProps {
                id,
                field: &view.colour,
                on_pick,
                on_hex_click: click(TagField::Hex),
            },
            cx,
        ),
    ];
    fields.extend(
        view.active
            .as_ref()
            .map(|active| active_checkbox(active, actions, cx)),
    );
    fields.push(callout(view.callout.clone(), cx));
    fields.extend(view.refusal.clone().map(|text| error_line(text, cx)));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button(
                    SharedString::from(format!("{id}-cancel")),
                    actions.cancel.clone(),
                    cx,
                )
                .into_any_element(),
                dialog::confirm_button(
                    SharedString::from(format!("{id}-confirm")),
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
    dialog::overlay(dialog::WIDTH, false, card, cx)
}

/// The Active checkbox, drawn like the Import footer's: a 12px box, filled with a tick when on,
/// underlined while it has keyboard focus.
fn active_checkbox(active: &ActiveField, actions: &Clicks, cx: &App) -> AnyElement {
    let on_toggle = actions.click(|dialog| {
        send(
            dialog,
            vec![
                TagFormEvent::Focus(TagField::Active),
                TagFormEvent::ToggleActive,
            ],
        );
    });
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .id("edit-tag-active")
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(8.0))
                .on_click(move |_event, window, cx| on_toggle(window, cx))
                .child(
                    div()
                        .size(px(12.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_1()
                        .border_color(color::foreground(cx))
                        .when(active.checked, |this| {
                            this.bg(color::foreground(cx))
                                .text_color(color::background(cx))
                                .text_size(px(9.0))
                                .child("\u{2713}")
                        }),
                )
                .child(
                    div()
                        .text_color(color::foreground(cx))
                        .when(active.focused, |this| this.underline())
                        .child(active.label.clone()),
                ),
        )
        .child(
            div()
                .pl(px(20.0))
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .child(active.hint.clone()),
        )
        .into_any_element()
}

/// The handoff's dark-bordered note.
fn callout(text: String, cx: &App) -> AnyElement {
    div()
        .p(px(10.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::foreground(cx))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}

/// The Name field, for a test.
pub(super) fn readback(view: &TagFormViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("name", view.name.text.clone()),
        ("hex", view.colour.hex.text.clone()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
