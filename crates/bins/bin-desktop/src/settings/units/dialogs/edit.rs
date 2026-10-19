//! Renders the **Edit unit** dialog (issue #185's own "2c", issue #715's PRICE SOURCE group), on
//! the shared `crate::dialog` chrome. Same form and fields as [`super::add`] (both wrap the same
//! `UnitForm`), so the only real differences here are the dynamic title, the pre-filled starting
//! values (`UnitsPage::click_edit`, via `UnitForm::from_row`), the usage-notice info panel, and
//! the "Save" button label.
//!
//! The usage notice ("Used by 4 accounts · 604 transactions...") is the ticket's own fixed
//! mockup example, shown identically regardless of which row is being edited -- there's no real
//! accounts/transactions data behind *any* row in this map (see the map's own Destination), so a
//! per-row count would be inventing a fact this map has no data to back. Renamed to `save`
//! semantics: per the ticket's own body, a changed code just relabels the row here -- rewriting
//! real references is real-persistence work, out of scope.

use gpui::{AnyElement, App, div, prelude::*};

use super::{FieldIds, UnitsDialogHandlers, form_fields};
use crate::{dialog, settings::units::form::UnitForm};

const IDS: FieldIds = FieldIds {
    code: "edit-unit-code",
    kind: "edit-unit-type",
    name: "edit-unit-name",
    source: "edit-unit-source",
    symbol: "edit-unit-symbol",
    quote: "edit-unit-quote",
    test: "edit-unit-test",
};

pub fn render(form: &UnitForm, handlers: UnitsDialogHandlers, cx: &App) -> AnyElement {
    let mut fields = form_fields(IDS, form, &handlers, cx);
    fields.push(
        dialog::info_panel(crate::msg::desktop_settings_units_usage_notice(4, 604), cx)
            .into_any_element(),
    );
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_settings_units_edit_title(form.code.text()),
            false,
            cx,
        ))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("edit-unit-cancel", handlers.on_cancel.clone(), cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "edit-unit-confirm",
                    crate::msg::desktop_settings_units_edit_submit(),
                    form.is_valid(),
                    false,
                    handlers.on_confirm.clone(),
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(dialog::WIDTH, false, card, cx)
}
