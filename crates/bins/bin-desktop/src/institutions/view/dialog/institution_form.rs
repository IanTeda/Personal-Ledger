//! The Add institution Dialog (issue #187's "2e") on the shared `crate::dialog` frame, the one
//! dialog in this map whose own raw markup sets `width:460px` instead of the shared
//! [`dialog::WIDTH`] (see that constant's own doc).
//!
//! Two field shapes, neither reused verbatim from the Unit dialogs:
//! - **Institution name**: the dialog's only text field, so it always holds the typing and a click
//!   into it does nothing.
//! - **Products**: multi-select chips (`docs/ux/desktop-mockups/styles.css`'s `.chip` role), the
//!   first *multi*-select control in this crate; every other "pick one" control (segmented
//!   controls, dot radios) only ever holds one selection at a time.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_institutions::presenter::{
    InstitutionFormEvent, InstitutionFormViewModel, InstitutionProduct,
};

use super::{Clicks, InstitutionsDialog};
use crate::form::viper::{enter, typed};
use crate::{
    dialog,
    form::colour_field::error_line,
    form::fields::{field_label, text_field},
    shell::view::dialog_host::{DialogKey, DialogOutcome},
    theme::color,
};

const WIDTH: gpui::Pixels = px(460.0);

/// A key on the Dialog: typing edits the name, `Enter` confirms a valid form, `Tab` has no other
/// field to go to, and the arrows do nothing.
pub(super) fn key(dialog: &mut InstitutionsDialog, key: DialogKey) -> DialogOutcome {
    let view = dialog.presenter.view_model();
    match key {
        DialogKey::Enter => enter(view.frame.can_confirm),
        DialogKey::Tab | DialogKey::BackTab | DialogKey::Up | DialogKey::Down => {
            DialogOutcome::Handled
        }
        _ => match typed(&view.name.text, key) {
            Some(text) => {
                dialog.send(InstitutionFormEvent::Name(text));
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
    }
}

fn click_product(dialog: &mut InstitutionsDialog, product: InstitutionProduct) {
    dialog.send(InstitutionFormEvent::ToggleProduct(product));
}

pub(super) fn render(view: &InstitutionFormViewModel, actions: &Clicks, cx: &App) -> AnyElement {
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
                        Rc::new(|_window: &mut Window, _cx: &mut App| {}),
                        cx,
                    )
                    .into_any_element(),
                )
                .children(view.name_error.clone().map(|error| error_line(error, cx)))
                .into_any_element(),
            products_field(view, actions.choose(click_product), cx).into_any_element(),
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
        ("error", view.name_error.clone().unwrap_or_default()),
    ]
}
