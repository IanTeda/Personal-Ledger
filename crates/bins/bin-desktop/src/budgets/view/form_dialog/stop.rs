//! Stop budgeting (9g) on the shared `crate::dialog` frame at the handoff's 420px. A Stop deletes
//! nothing (a later Onward amount resumes the Category), so there's no typed confirmation and the
//! confirm button keeps the neutral dark fill rather than the destructive one.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_budgets::presenter::{StopEvent, StopPresenter, StopViewModel};
use lib_shell::Hint;

use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{drawn, labels};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};

/// The dialog's width: the handoff's 9g.
const WIDTH: gpui::Pixels = px(420.0);

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<StopPresenter>;

/// The `lib-budgets` Stop budgeting Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056). What each key means is the Presenter's; this View draws the Dialog and turns its
/// clicks into Presenter events. Its widget state is the top row of the From list's window.
impl DialogView for StopPresenter {
    type Widgets = usize;

    fn kind(_view: &StopViewModel) -> &'static str {
        "Stop"
    }

    fn render(view: &StopViewModel, scroll: &mut usize, clicks: &Clicks, cx: &App) -> AnyElement {
        render(view, scroll, clicks, cx)
    }

    /// The Category the Stop is for, by id.
    fn readback(view: &StopViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, vec![("category", view.category.to_string())])
    }

    fn hints(view: &StopViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(view: &StopViewModel, scroll: &mut usize, actions: &Clicks, cx: &App) -> AnyElement {
    let options = labels(&view.from);
    let mut fields = vec![
        select_field::render(
            SelectFieldProps {
                id: "budgets-stop-from".into(),
                label: view.from.label.clone().into(),
                options: &options,
                state: &drawn(&view.from, view.from_list, scroll),
                focused: true,
                accent: false,
                read_only: None,
                on_field_click: actions.send(StopEvent::ToggleList),
                on_option_click: {
                    let months: Vec<_> = view
                        .from
                        .options
                        .iter()
                        .map(|choice| choice.value)
                        .collect();
                    actions.option(move |index| {
                        months
                            .get(index)
                            .map(|month| StopEvent::From(*month))
                            .into_iter()
                            .collect()
                    })
                },
            },
            cx,
        ),
        div()
            .text_size(px(13.0))
            .child(view.body.clone())
            .into_any_element(),
    ];
    if let Some(bills) = &view.bills {
        fields.push(
            div()
                .text_size(px(13.0))
                .child(bills.clone())
                .into_any_element(),
        );
    }
    if let Some(error) = &view.error {
        fields.push(error_line(error.clone(), cx));
    }
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("budgets-stop-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "budgets-stop-confirm",
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
    dialog::overlay(WIDTH, false, card, cx)
}
