//! Delete price source (16z·del, 420px) in 16d's red treatment: how many Units the source values,
//! which fall back to Manual entry, and the source's name typed back before the confirm enables.
//! The Presenter reads every key.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_shell::Hint;
use lib_units::presenter::{DeletePriceSourcePresenter, DeletePriceSourceViewModel};

use super::refusal;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks, framed};
use crate::{dialog, form::fields::text_field};

/// The open Dialog's clicks.
type Clicks = ModuleDialogClicks<DeletePriceSourcePresenter>;

/// The `lib-units` Delete price source Presenter's [`DialogView`], hosted in a `ModuleDialog`
/// (ADR-0056).
impl DialogView for DeletePriceSourcePresenter {
    type Widgets = ();

    fn kind(_view: &DeletePriceSourceViewModel) -> &'static str {
        "DeletePriceSource"
    }

    fn render(
        view: &DeletePriceSourceViewModel,
        _widgets: &mut (),
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, clicks, cx)
    }

    fn readback(view: &DeletePriceSourceViewModel) -> Vec<(&'static str, String)> {
        framed(&view.frame, readback(view))
    }

    fn hints(view: &DeletePriceSourceViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

fn render(view: &DeletePriceSourceViewModel, actions: &Clicks, cx: &App) -> AnyElement {
    let mut fields = vec![
        div()
            .text_size(px(13.0))
            .child(view.warning.clone())
            .into_any_element(),
    ];
    fields.extend(view.fallback.clone().map(|fallback| {
        div()
            .debug_selector(|| "delete-price-source-fallback".to_string())
            .child(dialog::info_panel(fallback, cx))
            .into_any_element()
    }));
    fields.push(
        text_field(
            "delete-price-source-confirm-input",
            view.confirm.label.clone(),
            &view.confirm.text,
            &view.confirm.placeholder,
            view.confirm.focused,
            Rc::new(|_window: &mut Window, _cx: &mut App| {}),
            cx,
        )
        .into_any_element(),
    );
    fields.extend(refusal(view.refusal.clone(), cx));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), true, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-price-source-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-price-source-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    true,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, true, card, cx)
}

fn readback(view: &DeletePriceSourceViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("confirm", view.confirm.text.clone()),
        ("warning", view.warning.clone()),
        ("fallback", view.fallback.clone().unwrap_or_default()),
        ("refusal", view.refusal.clone().unwrap_or_default()),
    ]
}
