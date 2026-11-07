//! The Set price Dialog: Week, with the Monday it snaps to under it, then Close in the quote Unit.
//! `Tab` moves between them, and `Enter` confirms once both read: the Presenter reads every key.

use gpui::{AnyElement, App, div, prelude::*};
use lib_shell::Hint;
use lib_units::presenter::{SetPriceEvent, SetPriceField, SetPricePresenter, SetPriceViewModel};
use lib_viper::view::TextFieldView;

use super::{help, refusal};
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{
    dialog,
    form::dialog_fields::{label, text_field},
};

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<SetPricePresenter>;

/// The `lib-units` Set price Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056).
impl DialogView for SetPricePresenter {
    type Widgets = ();

    fn kind(_view: &SetPriceViewModel) -> &'static str {
        "SetPrice"
    }

    fn render(
        view: &SetPriceViewModel,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    fn readback(view: &SetPriceViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &SetPriceViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(view: &SetPriceViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let text = |id: &'static str, field: &TextFieldView, which: SetPriceField| {
        text_field(
            id,
            label(field.label.clone()),
            &field.text,
            &field.placeholder,
            field.focused,
            actions.send(SetPriceEvent::Focus(which)),
            cx,
        )
    };
    let mut fields = vec![
        div()
            .child(text("set-price-week", &view.week, SetPriceField::Week))
            .child(
                div()
                    .debug_selector(|| "set-price-week-help".to_string())
                    .child(help(view.week_help.clone(), cx)),
            )
            .into_any_element(),
        div()
            .child(text("set-price-close", &view.close, SetPriceField::Close))
            .child(help(view.close_help.clone(), cx))
            .into_any_element(),
    ];
    fields.extend(refusal(view.refusal.clone(), cx));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("set-price-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "set-price-confirm",
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

fn readback(view: &SetPriceViewModel) -> Vec<(&'static str, String)> {
    let field = if view.week.focused { "Week" } else { "Close" };
    vec![
        ("week", view.week.text.clone()),
        ("week_help", view.week_help.clone()),
        ("close", view.close.text.clone()),
        ("close_help", view.close_help.clone()),
        ("field", field.to_string()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
