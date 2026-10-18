//! Renders **Delete price source** (16z·del, 420px) in 16d's red treatment: how many Units the
//! source values, which of them fall back to Manual entry keeping their last price, and the source's
//! name typed back before the confirm enables. Only a Custom source opens it.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_units::UnitRow;

use super::{SourceDialogHandlers, codes};
use crate::{dialog, form::fields::text_field, settings::units::source_form::DeleteSourceForm};

pub fn render(
    form: &DeleteSourceForm,
    units: &[&UnitRow],
    handlers: SourceDialogHandlers,
    cx: &App,
) -> AnyElement {
    let name = form.name.as_str();
    let count = i64::try_from(units.len()).unwrap_or(i64::MAX);
    let mut fields = vec![
        div()
            .text_size(px(13.0))
            .child(crate::msg::desktop_settings_price_source_delete_warning(
                count,
            ))
            .into_any_element(),
    ];
    if !units.is_empty() {
        fields.push(
            div()
                .debug_selector(|| "delete-price-source-fallback".to_string())
                .child(dialog::info_panel(
                    crate::msg::desktop_settings_price_source_delete_fallback(count, &codes(units)),
                    cx,
                ))
                .into_any_element(),
        );
    }
    fields.push(
        text_field(
            "delete-price-source-confirm-input",
            crate::msg::desktop_settings_price_source_delete_confirm_label(name),
            form.confirm_input.text(),
            name,
            true,
            Rc::new(|_window: &mut Window, _cx: &mut App| {}),
            cx,
        )
        .into_any_element(),
    );
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_settings_price_source_delete_title(name),
            true,
            cx,
        ))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-price-source-cancel", handlers.on_cancel, cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "delete-price-source-confirm",
                    crate::msg::desktop_settings_price_source_delete_submit(),
                    form.is_valid(),
                    true,
                    handlers.on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(dialog::WIDTH, true, card, cx)
}
