//! Renders a built-in's **Edit price source** (16z·edit, 480px): Name and Updates editable, the
//! endpoint as a read-only fact block (it is code, ADR-0043), the usage notice naming the Units it
//! values, and Test price source, not yet built. Cancel / Save.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_units::{BuiltInSource, UnitRow};

use super::{Change, SourceDialogHandlers, codes, help, test_not_built, updates_label};
use crate::{
    dialog,
    form::fields::{field_label, segmented_control, text_field},
    settings::units::source_form::{BuiltInSourceForm, SourceField, UPDATES_OPTIONS},
    theme::color,
};

/// `width:480px` (16z·edit).
const WIDTH: gpui::Pixels = px(480.0);

pub fn render(
    source: &BuiltInSource,
    form: &BuiltInSourceForm,
    units: &[&UnitRow],
    handlers: SourceDialogHandlers,
    cx: &App,
) -> AnyElement {
    let on_change = handlers.on_change.clone();
    let count = i64::try_from(units.len()).unwrap_or(i64::MAX);
    let fields = [
        div()
            .child(text_field(
                "edit-built-in-source-name",
                lib_locale::msg::column_name(),
                form.name.text(),
                source.name,
                form.focused == SourceField::Name,
                handlers.click(Change::Focus(SourceField::Name)),
                cx,
            ))
            .child(help(
                crate::msg::desktop_settings_price_source_name_help(),
                cx,
            ))
            .into_any_element(),
        fact_block(source, cx).into_any_element(),
        div()
            .child(field_label(
                crate::msg::desktop_settings_price_source_field_updates(),
            ))
            .child(segmented_control(
                "edit-built-in-source-updates",
                &UPDATES_OPTIONS,
                form.updates,
                updates_label,
                Rc::new(move |updates, window: &mut Window, cx: &mut App| {
                    on_change(Change::Updates(updates), window, cx);
                }),
                cx,
            ))
            .into_any_element(),
        div()
            .debug_selector(|| "edit-built-in-source-usage".to_string())
            .child(dialog::info_panel(
                crate::msg::desktop_settings_price_source_usage(count, &codes(units)),
                cx,
            ))
            .into_any_element(),
        test_not_built(
            "edit-built-in-source-test",
            crate::msg::desktop_settings_price_source_test(),
            cx,
        )
        .into_any_element(),
    ];
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_settings_price_source_edit_title(form.name.text()),
            false,
            cx,
        ))
        .child(
            div()
                .p(px(20.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .children(fields),
        )
        .child(dialog::action_row(
            [
                dialog::cancel_button("edit-built-in-source-cancel", handlers.on_cancel, cx)
                    .into_any_element(),
                dialog::confirm_button(
                    "edit-built-in-source-confirm",
                    crate::msg::desktop_settings_price_source_edit_submit(),
                    form.is_valid(),
                    false,
                    handlers.on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}

/// The read-only endpoint (`#eae9e9`, fact rows as the X-Large pane's): Source, Address, Request
/// and Price path, the URL parts monospace, then the note on why they aren't edited here.
fn fact_block(source: &BuiltInSource, cx: &App) -> impl IntoElement {
    let endpoint = &source.endpoint;
    let rows = [
        (
            crate::msg::desktop_settings_units_column_source(),
            crate::msg::desktop_settings_price_source_built_in(source.name),
            false,
        ),
        (
            crate::msg::desktop_settings_price_source_field_address(),
            endpoint.address.to_string(),
            true,
        ),
        (
            crate::msg::desktop_settings_price_source_field_request(),
            endpoint.request.to_string(),
            true,
        ),
        (
            crate::msg::desktop_settings_price_source_field_price_path(),
            endpoint.price_path.to_string(),
            true,
        ),
    ];
    div()
        .debug_selector(|| "edit-built-in-source-facts".to_string())
        .flex()
        .flex_col()
        .px(px(12.0))
        .py(px(8.0))
        .bg(color::chrome(cx))
        .children(rows.into_iter().map(|(label, value, mono)| {
            div()
                .flex()
                .justify_between()
                .gap(px(12.0))
                .py(px(5.0))
                .border_b(px(1.0))
                .border_color(color::hairline(cx))
                .text_size(px(12.0))
                .child(div().flex_none().text_color(color::muted(cx)).child(label))
                .child(
                    div()
                        .min_w(px(0.0))
                        .text_align(gpui::TextAlign::Right)
                        .when(mono, |this| this.font_family("monospace"))
                        .child(value),
                )
        }))
        .child(help(
            crate::msg::desktop_settings_price_source_built_in_help(),
            cx,
        ))
}
