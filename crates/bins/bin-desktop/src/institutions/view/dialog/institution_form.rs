//! The Add institution Dialog (issue #187's "2e") on the shared `crate::dialog` frame, the one
//! dialog in this map whose own raw markup sets `width:460px` instead of the shared
//! [`dialog::WIDTH`] (see that constant's own doc).
//!
//! Three field shapes, none reused verbatim from the Unit dialogs:
//! - **Institution name**: the dialog's only text field, so it holds the typing whenever it has
//!   focus; a click into it focuses it and closes the Usual unit list.
//! - **Products**: multi-select chips (`docs/ux/desktop-mockups/styles.css`'s `.chip` role), the
//!   first *multi*-select control in this crate; every other "pick one" control (segmented
//!   controls, dot radios) only ever holds one selection at a time.
//! - **Usual unit**: the shared dropdown (`crate::form::select_field`), optional, so its first
//!   option is None. It takes the arrows, `Enter` and `Space` while focused; `Tab` moves between it
//!   and the name.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_institutions::presenter::{
    InstitutionField, InstitutionFormEvent, InstitutionFormViewModel, InstitutionProduct,
};

use super::{Clicks, InstitutionsDialog};
use crate::form::viper::{
    choose_option, chosen_label, enter, labels, select_key, synced, toggle_select, typed,
};
use crate::{
    dialog,
    form::colour_field::error_line,
    form::fields::{field_label, text_field},
    form::select::SelectState,
    form::select_field::{self, SelectFieldProps},
    shell::view::dialog_host::{DialogKey, DialogOutcome},
    theme::color,
};

const WIDTH: gpui::Pixels = px(460.0);

/// A key on the Dialog: `Tab` commits an open list's highlight and moves field; the Usual unit
/// select takes the arrows, `Enter` and `Space` and swallows other typing; on the name, typing edits
/// it, `Enter` confirms a valid form and the arrows do nothing.
pub(super) fn key(dialog: &mut InstitutionsDialog, key: DialogKey) -> DialogOutcome {
    let view = dialog.presenter.view_model();
    if matches!(key, DialogKey::Tab | DialogKey::BackTab) {
        if let Some(event) = select_event(dialog, &view, DialogKey::Enter, true) {
            dialog.send(event);
        }
        let event = if key == DialogKey::Tab {
            InstitutionFormEvent::NextField
        } else {
            InstitutionFormEvent::PrevField
        };
        dialog.send(event);
        dialog.usual_unit.cancel();
        return DialogOutcome::Handled;
    }
    match view.focused {
        InstitutionField::UsualUnit => {
            let taken = matches!(
                key,
                DialogKey::Up | DialogKey::Down | DialogKey::Enter | DialogKey::Char(' ')
            );
            if let Some(event) = select_event(dialog, &view, key, false) {
                dialog.send(event);
            }
            match key {
                // A select swallows typing rather than letting it fall through to the Shell.
                DialogKey::Char(_) | DialogKey::Backspace => DialogOutcome::Handled,
                _ if taken => DialogOutcome::Handled,
                _ => DialogOutcome::Ignored,
            }
        }
        InstitutionField::Name => match key {
            DialogKey::Enter => enter(view.frame.can_confirm),
            DialogKey::Up | DialogKey::Down => DialogOutcome::Handled,
            _ => match typed(&view.name.text, key) {
                Some(text) => {
                    dialog.send(InstitutionFormEvent::Name(text));
                    DialogOutcome::Handled
                }
                None => DialogOutcome::Ignored,
            },
        },
    }
}

/// Runs `key` through the Usual unit select, returning the event for a changed choice. With
/// `only_open`, only an open list takes the key (a `Tab` commits its highlight).
fn select_event(
    dialog: &mut InstitutionsDialog,
    view: &InstitutionFormViewModel,
    key: DialogKey,
    only_open: bool,
) -> Option<InstitutionFormEvent> {
    if view.focused != InstitutionField::UsualUnit || (only_open && !dialog.usual_unit.is_open()) {
        return None;
    }
    select_key(&mut dialog.usual_unit, &view.usual_unit, key)
        .1
        .map(InstitutionFormEvent::UsualUnit)
}

/// A click on the name focuses it, closing the list.
fn click_name(dialog: &mut InstitutionsDialog) {
    dialog.usual_unit.cancel();
    dialog.send(InstitutionFormEvent::Focus(InstitutionField::Name));
}

/// A click on the Usual unit's closed field focuses it and toggles its list.
fn click_usual_unit(dialog: &mut InstitutionsDialog) {
    dialog.send(InstitutionFormEvent::Focus(InstitutionField::UsualUnit));
    let view = dialog.presenter.view_model();
    toggle_select(&mut dialog.usual_unit, &view.usual_unit);
}

/// A click on row `index` of the open list.
fn click_option(dialog: &mut InstitutionsDialog, index: usize) {
    let view = dialog.presenter.view_model();
    if let Some(unit) = choose_option(&mut dialog.usual_unit, &view.usual_unit, index) {
        dialog.send(InstitutionFormEvent::UsualUnit(unit));
    }
}

fn click_product(dialog: &mut InstitutionsDialog, product: InstitutionProduct) {
    dialog.send(InstitutionFormEvent::ToggleProduct(product));
}

pub(super) fn render(
    view: &InstitutionFormViewModel,
    usual_unit: &SelectState,
    actions: &Clicks,
    cx: &App,
) -> AnyElement {
    let unit_options = labels(&view.usual_unit);
    let unit_state = synced(usual_unit, &view.usual_unit);
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.frame.title.clone(), false, cx))
        .child(dialog::body([
            div()
                .child(
                    text_field(
                        "add-institution-name",
                        view.name.label.clone(),
                        &view.name.text,
                        &view.name.placeholder,
                        view.name.focused,
                        actions.click(click_name),
                        cx,
                    )
                    .into_any_element(),
                )
                .children(view.name_error.clone().map(|error| error_line(error, cx)))
                .into_any_element(),
            products_field(view, actions.choose(click_product), cx).into_any_element(),
            select_field::render(
                SelectFieldProps {
                    id: "institution-usual-unit",
                    label: view.usual_unit.label.clone().into(),
                    options: &unit_options,
                    state: &unit_state,
                    focused: view.usual_unit.focused,
                    accent: false,
                    read_only: None,
                    on_field_click: actions.click(click_usual_unit),
                    on_option_click: actions.option(click_option),
                },
                cx,
            ),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("add-institution-cancel", actions.cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "add-institution-confirm",
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

/// The `.chip` multi-select row (`docs/ux/desktop-mockups/16-settings/README.md`'s Dialog
/// components: `display:flex; align-items:center; gap:6px; padding:6px 10px; border:1px solid
/// rgba(32,30,29,.30); font-size:12px; cursor:pointer`, selected takes the dark treatment).
fn products_field(
    view: &InstitutionFormViewModel,
    on_click: crate::shell::view::dialog_entity::OnChoose<InstitutionProduct>,
    cx: &App,
) -> impl IntoElement {
    div().child(field_label(view.products_label.clone())).child(
        div()
            .flex()
            .flex_wrap()
            .gap(px(8.0))
            .mt(px(4.0))
            .children(view.products.iter().map(|chip| {
                let product = chip.product;
                let id = SharedString::from(format!(
                    "add-institution-chip-{}",
                    format!("{product:?}").to_lowercase()
                ));
                let on_click = on_click.clone();
                div()
                    .debug_selector({
                        let id = id.clone();
                        move || id.to_string()
                    })
                    .id(id)
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .py(px(6.0))
                    .px(px(10.0))
                    .border_1()
                    .border_color(color::border(cx))
                    .text_size(px(12.0))
                    .when(chip.checked, |this| {
                        this.bg(color::selection_background(cx))
                            .text_color(color::selection_text(cx))
                    })
                    .on_click(move |_event, window, cx| on_click(product, window, cx))
                    .child(chip.label.clone())
            })),
    )
}

/// The form's fields, for a test: the Name draft and the checked products (comma-joined variant
/// names).
pub(super) fn readback(view: &InstitutionFormViewModel) -> Vec<(&'static str, String)> {
    vec![
        ("name", view.name.text.clone()),
        (
            "types",
            view.products
                .iter()
                .filter(|chip| chip.checked)
                .map(|chip| format!("{:?}", chip.product))
                .collect::<Vec<_>>()
                .join(","),
        ),
        (
            "usual_unit",
            chosen_label(&view.usual_unit).unwrap_or_default(),
        ),
        ("focused", format!("{:?}", view.focused)),
        ("error", view.name_error.clone().unwrap_or_default()),
    ]
}
