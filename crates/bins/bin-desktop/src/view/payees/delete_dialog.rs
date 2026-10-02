//! Renders the **Delete payee** dialog (`docs/ux/desktop/handoff_payees/README.md`'s 6d) on the shared
//! `crate::dialog` chrome's destructive variant at the handoff's 440px. #283 overrode the handoff's
//! always-hard-delete, so the one dialog serves three [`DeleteAction`]s: an unreferenced Payee is
//! deleted, a referenced one is deactivated, and a referenced inactive one is offered
//! reactivation. The first two sit behind the typed-name confirm (exact, case-sensitive);
//! reactivating loses nothing, so it drops the gate and the red chrome.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};

use crate::{
    dialog,
    payees::{DeleteAction, DeletePayeeForm, Payee},
    theme::color,
};

/// The dialog's width: the handoff's 6d.
pub const WIDTH: gpui::Pixels = px(440.0);

pub struct DeletePayeeProps<'a> {
    pub payee: &'a Payee,
    pub action: DeleteAction,
    pub form: &'a DeletePayeeForm,
    /// Splits carrying the Payee.
    pub splits: usize,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub fn render(props: DeletePayeeProps<'_>, cx: &App) -> AnyElement {
    let DeletePayeeProps {
        payee,
        action,
        form,
        splits,
        on_cancel,
        on_confirm,
    } = props;
    let destructive = action.is_destructive();
    let (title, submit) = match action {
        DeleteAction::Delete => (
            crate::msg::desktop_payees_delete_title(&payee.name),
            crate::msg::desktop_payees_delete_submit(),
        ),
        DeleteAction::Deactivate => (
            crate::msg::desktop_payees_deactivate_title(&payee.name),
            crate::msg::desktop_payees_deactivate_submit(),
        ),
        DeleteAction::Reactivate => (
            crate::msg::desktop_payees_reactivate_title(&payee.name),
            crate::msg::desktop_payees_reactivate_submit(),
        ),
    };

    let mut fields = vec![
        warning_copy(action, splits),
        callout(action, payee.aliases.len(), cx),
    ];
    if destructive {
        fields.push(confirm_input_field(
            &payee.name,
            &form.confirmation_name,
            cx,
        ));
    }

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(title, destructive, cx))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("delete-payee-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "delete-payee-confirm",
                    submit,
                    form.allows(action, &payee.name),
                    destructive,
                    on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(WIDTH, destructive, card, cx)
}

fn warning_copy(action: DeleteAction, splits: usize) -> AnyElement {
    let count = i64::try_from(splits).unwrap_or(i64::MAX);
    let segments = match action {
        DeleteAction::Delete => crate::msg::desktop_payees_delete_warning(),
        DeleteAction::Deactivate => crate::msg::desktop_payees_deactivate_warning(count),
        DeleteAction::Reactivate => crate::msg::desktop_payees_reactivate_warning(count),
    };
    div()
        .flex()
        .flex_wrap()
        .text_size(px(13.0))
        .children(segments.into_iter().map(|segment| {
            let text = div().child(segment.text);
            match segment.tag.as_deref() {
                Some("strong") => text.font_weight(gpui::FontWeight::EXTRA_BOLD),
                _ => text,
            }
        }))
        .into_any_element()
}

fn callout_text(action: DeleteAction, rules: usize) -> String {
    match action {
        DeleteAction::Delete => {
            crate::msg::desktop_payees_delete_callout(i64::try_from(rules).unwrap_or(i64::MAX))
        }
        DeleteAction::Deactivate => crate::msg::desktop_payees_deactivate_callout(),
        DeleteAction::Reactivate => crate::msg::desktop_payees_reactivate_callout(),
    }
}

/// The handoff's red-bordered callout on a destructive action; reactivation's is the neutral
/// `info_panel`.
fn callout(action: DeleteAction, rules: usize, cx: &App) -> AnyElement {
    let text = callout_text(action, rules);
    if !action.is_destructive() {
        return dialog::info_panel(text, cx).into_any_element();
    }
    div()
        .p(px(10.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::negative(cx))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}

fn confirm_input_field(name: &str, input_value: &str, cx: &App) -> AnyElement {
    let (text, text_color) = if input_value.is_empty() {
        (name.to_string(), color::muted(cx))
    } else {
        (input_value.to_string(), color::foreground(cx))
    };
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(crate::msg::desktop_payees_delete_confirm_label(name)),
        )
        .child(
            div()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(color::border(cx))
                .text_size(px(13.0))
                .text_color(text_color)
                .child(SharedString::from(text)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_delete_callout_counts_the_rules_that_go_with_it() {
        crate::locale::init_for_tests();
        assert_eq!(
            callout_text(DeleteAction::Delete, 2),
            "Its 2 match rules are also removed."
        );
        assert_eq!(
            callout_text(DeleteAction::Delete, 1),
            "Its 1 match rule is also removed."
        );
        assert_eq!(
            callout_text(DeleteAction::Delete, 0),
            "It has no match rules."
        );
    }

    #[test]
    fn the_deactivate_warning_emphasises_the_transaction_count() {
        crate::locale::init_for_tests();
        let strong: Vec<String> = crate::msg::desktop_payees_deactivate_warning(4)
            .into_iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .map(|segment| segment.text)
            .collect();
        assert_eq!(strong, ["4 transactions"]);
    }
}
