//! The Add and Edit area Dialogs (#495) on the shared `crate::dialog` frame: the one Name field,
//! whose error shows once it has been typed in, and the note on which Location the Area is in.

use gpui::{AnyElement, App, SharedString, div, prelude::*};
use lib_inventory::presenter::{AreaFormEvent, AreaFormPresenter, AreaFormViewModel};
use lib_shell::Hint;

use super::note;
use crate::form::fields::text_field;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{dialog, form::colour_field::error_line};

/// The open form's clicks.
type Clicks = ModuleDialogClicks<AreaFormPresenter>;

/// The `lib-inventory` Area form Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056).
impl DialogView for AreaFormPresenter {
    type Widgets = ();

    fn kind(view: &AreaFormViewModel) -> &'static str {
        if view.editing { "EditArea" } else { "AddArea" }
    }

    fn render(
        view: &AreaFormViewModel,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    fn readback(view: &AreaFormViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, vec![("name", view.name.text.clone())])
    }

    fn hints(view: &AreaFormViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(view: &AreaFormViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    // `text_field` and the buttons take a `'static` id, so these are spelt out.
    let prefix = if view.editing {
        "edit-area"
    } else {
        "add-area"
    };
    // The one field is already focused: a click only repaints.
    let on_name_click = actions.send_all(Vec::<AreaFormEvent>::new());
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
