//! The Add and Edit area Dialogs (#495) on the shared `crate::dialog` frame: the one Name field,
//! whose error shows once it has been typed in, and the note on which Location the Area is in.

use gpui::{AnyElement, App, SharedString, div, prelude::*};
use lib_inventory::presenter::{AreaFormEvent, AreaFormPresenter, AreaFormViewModel};

use super::{Clicks, note};
use crate::form::fields::text_field;
use crate::form::viper::{enter, typed};
use crate::inventory::router::DialogRouter;
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};
use crate::{dialog, form::colour_field::error_line};

pub(super) fn key(presenter: &mut AreaFormPresenter, key: DialogKey) -> DialogOutcome {
    let view = presenter.view_model();
    match key {
        DialogKey::Enter => enter(view.frame.can_confirm),
        // The Name is the only field.
        DialogKey::Tab | DialogKey::BackTab => DialogOutcome::Handled,
        _ => match typed(&view.name.text, key) {
            Some(text) => {
                presenter.handle(AreaFormEvent::Name(text), &mut DialogRouter::default());
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
    }
}

pub(super) fn render(view: &AreaFormViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    // `text_field` and the buttons take a `'static` id, so these are spelt out.
    let prefix = if view.editing {
        "edit-area"
    } else {
        "add-area"
    };
    // The one field is already focused: a click only repaints.
    let on_name_click = actions.click(|_| {});
    let shown = match &view.name_error {
        Some(error) => note(error.clone(), true, cx).into_any_element(),
        None => note(view.note.clone(), false, cx).into_any_element(),
    };
    let body = div()
        .child(text_field(
            "area-name",
            view.name.label.clone(),
            &view.name.text,
            &view.name.placeholder,
            view.name.focused,
            on_name_click,
            cx,
        ))
        .child(shown)
        .children(view.refusal.clone().map(|text| error_line(text, cx)))
        .into_any_element();
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body([body]))
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
    dialog::overlay(dialog::WIDTH, false, card, cx)
}
