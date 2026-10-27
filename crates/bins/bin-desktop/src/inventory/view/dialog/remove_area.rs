//! The Remove area Dialog (#487) on the shared `crate::dialog` frame: a plain confirm while the
//! Area is empty, else the red dialog with the "Move items to" select, pre-selected on the Area
//! above (below when first).
//!
//! The select takes the arrows and `Space`; `Enter` commits an open list, else removes. Typing goes
//! nowhere, rather than reaching the Shell.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_inventory::presenter::{RemoveAreaEvent, RemoveAreaPresenter, RemoveAreaViewModel};

use super::{Clicks, InventoryDialog, View};
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::form::select::SelectState;
use crate::form::select_field::{self, SelectFieldProps};
use crate::form::viper::{choose_option, enter, labels, select_key, synced, toggle_select};
use crate::inventory::router::DialogRouter;
use crate::shell::view::dialog_host::{DialogKey, DialogOutcome};

fn handle(presenter: &mut RemoveAreaPresenter, event: RemoveAreaEvent) {
    presenter.handle(event, &mut DialogRouter::default());
}

pub(super) fn key(
    presenter: &mut RemoveAreaPresenter,
    select: &mut SelectState,
    key: DialogKey,
) -> DialogOutcome {
    let view = presenter.view_model();
    let Some(destination) = view.destination.as_ref() else {
        // Nothing to choose: only confirming and cancelling are keys here.
        return match key {
            DialogKey::Enter => enter(view.frame.can_confirm),
            DialogKey::Char(_) | DialogKey::Backspace | DialogKey::Tab | DialogKey::BackTab => {
                DialogOutcome::Handled
            }
            _ => DialogOutcome::Ignored,
        };
    };
    match key {
        DialogKey::Enter if !select.is_open() => enter(view.frame.can_confirm),
        DialogKey::Enter | DialogKey::Char(' ') | DialogKey::Up | DialogKey::Down => {
            let (_, value) = select_key(select, destination, key);
            if let Some(id) = value {
                handle(presenter, RemoveAreaEvent::Destination(id));
            }
            DialogOutcome::Handled
        }
        DialogKey::Char(_) | DialogKey::Backspace | DialogKey::Tab | DialogKey::BackTab => {
            DialogOutcome::Handled
        }
        _ => DialogOutcome::Ignored,
    }
}

fn remove(dialog: &mut InventoryDialog) -> Option<(&mut RemoveAreaPresenter, &mut SelectState)> {
    match &mut dialog.view {
        View::RemoveArea(presenter, select) => Some((presenter, select)),
        _ => None,
    }
}

/// A click on the destination field toggles its list.
fn click_field(dialog: &mut InventoryDialog) {
    if let Some((presenter, select)) = remove(dialog)
        && let Some(destination) = presenter.view_model().destination
    {
        toggle_select(select, &destination);
    }
}

fn click_option(dialog: &mut InventoryDialog, index: usize) {
    if let Some((presenter, select)) = remove(dialog)
        && let Some(destination) = presenter.view_model().destination
        && let Some(id) = choose_option(select, &destination, index)
    {
        handle(presenter, RemoveAreaEvent::Destination(id));
    }
}

pub(super) fn render(
    view: &RemoveAreaViewModel,
    select: &SelectState,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let destructive = view.frame.destructive;
    let mut body = vec![
        div()
            .text_size(px(13.0))
            .child(view.warning.clone())
            .into_any_element(),
    ];
    body.extend(view.destination.as_ref().map(|destination| {
        let options = labels(destination);
        select_field::render(
            SelectFieldProps {
                id: "area-destination",
                label: destination.label.clone().into(),
                options: &options,
                state: &synced(select, destination),
                focused: destination.focused,
                accent: false,
                read_only: None,
                on_field_click: actions.click(click_field),
                on_option_click: actions.option(click_option),
            },
            cx,
        )
    }));
    body.extend(view.refusal.clone().map(|text| error_line(text, cx)));
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), destructive, cx))
        .child(dialog::body(body))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-area-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "remove-area-confirm",
                    view.frame.submit.clone(),
                    view.frame.can_confirm,
                    destructive,
                    actions.confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, destructive, card, cx)
}
