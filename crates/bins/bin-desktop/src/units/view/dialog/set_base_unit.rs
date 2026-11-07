//! The Set base unit Dialog: one select over the active Units, whose keys and list the Presenter
//! holds. `Up`/`Down` step it, `Space` opens its list, and `Enter` commits an open list or
//! confirms.

use gpui::{AnyElement, App, div, prelude::*};
use lib_shell::Hint;
use lib_units::presenter::{SetBaseUnitEvent, SetBaseUnitPresenter, SetBaseUnitViewModel};

use super::{drawn, refusal};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{
    dialog,
    form::select_field::{self, SelectFieldProps},
};

/// The `lib-units` Set base unit Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056). Its widget state is the open list's scroll window.
impl DialogView for SetBaseUnitPresenter {
    type Widgets = usize;

    fn kind(_view: &SetBaseUnitViewModel) -> &'static str {
        "SetBaseUnit"
    }

    fn render(
        view: &SetBaseUnitViewModel,
        scroll: &mut usize,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, scroll, clicks, cx)
    }

    fn readback(view: &SetBaseUnitViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &SetBaseUnitViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<SetBaseUnitPresenter>;

fn render(
    view: &SetBaseUnitViewModel,
    scroll: &mut usize,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let options = super::labels(&view.base);
    let ids: Vec<_> = view
        .base
        .options
        .iter()
        .map(|choice| choice.value)
        .collect();
    let mut fields = vec![select_field::render(
        SelectFieldProps {
            id: "set-base-unit-select".into(),
            label: view.base.label.clone().into(),
            options: &options,
            state: &drawn(&view.base, view.list, scroll),
            focused: view.base.focused,
            accent: false,
            read_only: None,
            on_field_click: actions.send(SetBaseUnitEvent::ToggleList),
            on_option_click: actions.option(move |row| {
                ids.get(row)
                    .map(|id| SetBaseUnitEvent::Choose(*id))
                    .into_iter()
                    .collect()
            }),
        },
        cx,
    )];
    fields.extend(refusal(view.refusal.clone(), cx));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("set-base-unit-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "set-base-unit-confirm",
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

fn readback(view: &SetBaseUnitViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("base", super::chosen_label(&view.base).unwrap_or_default()),
        ("options", super::labels(&view.base).join("|")),
        ("select_open", view.list.is_open().to_string()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
