//! Renders the **Add account** dialog (`docs/ux/desktop/Accounts/README.md`'s 3b) on the shared
//! `crate::dialog` chrome. `Shell` owns the live form (`accounts::AccountForm`) and every
//! keystroke while it is open (`InputMode::Dialog`); this module only draws it.
//!
//! Width is 440px rather than `dialog::WIDTH`'s 420px, the README's own note: only because the
//! two-up field rows need the room. Text fields follow the Settings dialogs' append/pop-only
//! model. Institution, Type and Unit are the shared dropdown (`super::select_field`); Institution
//! reads as the read-only placeholder while Type is Cash.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use super::select_field::{self, SelectFieldProps};
use crate::{
    accounts::{AccountField, AccountForm, AccountOptions, NO_INSTITUTION},
    dialog,
    theme::color,
};

/// The dialog's width: `docs/ux/desktop/Accounts/README.md`'s "Dialog" row.
pub const WIDTH: gpui::Pixels = px(440.0);

pub type OnFieldClick = Rc<dyn Fn(AccountField, &mut Window, &mut App)>;
pub type OnOptionClick = Rc<dyn Fn(AccountField, usize, &mut Window, &mut App)>;
pub type OnCancel = dialog::OnClick;
pub type OnConfirm = dialog::OnClick;

pub fn render(
    form: &AccountForm,
    options: &AccountOptions,
    on_field_click: OnFieldClick,
    on_option_click: OnOptionClick,
    on_cancel: OnCancel,
    on_confirm: OnConfirm,
) -> AnyElement {
    let focused = |field: AccountField| form.focused == field;
    let click = |field: AccountField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let option_click = |field: AccountField| -> select_field::OnOptionClick {
        let on_option_click = on_option_click.clone();
        Rc::new(move |index: usize, window: &mut Window, cx: &mut App| {
            on_option_click(field, index, window, cx)
        })
    };

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_accounts_add_title(),
            false,
        ))
        .child(dialog::body([
            text_field(
                "add-account-name",
                label(lib_locale::msg::column_name()),
                &form.name,
                &crate::msg::desktop_accounts_name_placeholder(),
                focused(AccountField::Name),
                click(AccountField::Name),
            ),
            two_up([
                select_field::render(SelectFieldProps {
                    id: "add-account-institution",
                    label: lib_locale::msg::column_institution().into(),
                    options: &options.institutions,
                    state: &form.institution,
                    focused: focused(AccountField::Institution),
                    read_only: form.is_cash().then_some(NO_INSTITUTION),
                    on_field_click: click(AccountField::Institution),
                    on_option_click: option_click(AccountField::Institution),
                }),
                select_field::render(SelectFieldProps {
                    id: "add-account-type",
                    label: lib_locale::msg::column_type().into(),
                    options: &options.types,
                    state: &form.account_type,
                    focused: focused(AccountField::Type),
                    read_only: None,
                    on_field_click: click(AccountField::Type),
                    on_option_click: option_click(AccountField::Type),
                }),
            ]),
            two_up([
                select_field::render(SelectFieldProps {
                    id: "add-account-unit",
                    label: lib_locale::msg::column_unit().into(),
                    options: &options.units,
                    state: &form.unit,
                    focused: focused(AccountField::Unit),
                    read_only: None,
                    on_field_click: click(AccountField::Unit),
                    on_option_click: option_click(AccountField::Unit),
                }),
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(text_field(
                        "add-account-balance",
                        label(crate::msg::desktop_accounts_field_opening_balance()),
                        &form.opening_balance,
                        "0.00",
                        focused(AccountField::OpeningBalance),
                        click(AccountField::OpeningBalance),
                    ))
                    .into_any_element(),
            ]),
            text_field(
                "add-account-number",
                optional_label(crate::msg::desktop_accounts_field_number()),
                &form.account_number,
                "\u{2022}\u{2022}\u{2022}\u{2022} \u{2022}\u{2022}\u{2022}\u{2022} 1234",
                focused(AccountField::AccountNumber),
                click(AccountField::AccountNumber),
            ),
        ]))
        .child(dialog::action_row([
            dialog::cancel_button("add-account-cancel", on_cancel).into_any_element(),
            dialog::confirm_button(
                "add-account-confirm",
                crate::msg::desktop_accounts_add_submit(),
                form.is_valid(),
                false,
                on_confirm,
            )
            .into_any_element(),
        ]));

    dialog::overlay(WIDTH, false, card)
}

/// Two fields side by side: `gap:16px`, each `flex:1`.
pub(crate) fn two_up(fields: [AnyElement; 2]) -> AnyElement {
    div()
        .flex()
        .items_start()
        .gap(px(16.0))
        .children(fields)
        .into_any_element()
}

/// A field label: `font-weight:800; font-size:12px; margin-bottom:6px`.
pub(crate) fn label(text: impl Into<SharedString>) -> AnyElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .mb(px(6.0))
        .child(text.into())
        .into_any_element()
}

/// A label suffixed `(optional)` in the tertiary ink.
pub(crate) fn optional_label(text: impl Into<SharedString>) -> AnyElement {
    suffixed_label(text, crate::msg::desktop_field_optional())
}

/// A label followed by a `suffix` in the tertiary ink, at regular weight.
pub(crate) fn suffixed_label(
    text: impl Into<SharedString>,
    suffix: impl Into<SharedString>,
) -> AnyElement {
    div()
        .flex()
        .gap(px(4.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .mb(px(6.0))
        .child(text.into())
        .child(
            div()
                .font_weight(gpui::FontWeight::NORMAL)
                .text_color(color::INK_TERTIARY)
                .child(suffix.into()),
        )
        .into_any_element()
}

/// A label above a clickable text box whose border turns `ACCENT` with a trailing caret while
/// focused -- the same look as the Settings dialogs' text fields.
pub(crate) fn text_field(
    id: &'static str,
    label: AnyElement,
    value: &str,
    placeholder: &str,
    focused: bool,
    on_click: dialog::OnClick,
) -> AnyElement {
    let caret = if focused { "\u{2502}" } else { "" };
    let (text, text_color) = if value.is_empty() {
        (
            SharedString::from(format!("{placeholder}{caret}")),
            color::INK_TERTIARY,
        )
    } else {
        (SharedString::from(format!("{value}{caret}")), color::INK)
    };

    div()
        .child(label)
        .child(
            div()
                .id(id)
                .cursor_pointer()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(if focused {
                    color::ACCENT
                } else {
                    color::BORDER
                })
                .text_size(px(13.0))
                .text_color(text_color)
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(text),
        )
        .into_any_element()
}
