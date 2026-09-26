//! Renders the **Delete category** destructive-confirm dialog (`docs/ux/desktop/Categories/README.md`'s
//! 5d) on the shared `crate::dialog` chrome's destructive variant: accent border and title, copy
//! naming the real Split count and Budget references, the Uncategorised re-pointing notice,
//! typed-name confirm (exact, case-sensitive) gating **Delete category**. A parent is refused
//! (status message, dialog not opened) with "delete or move its children first".

use gpui::{AnyElement, SharedString, div, prelude::*, px};

use super::add_dialog::WIDTH;
use crate::{
    categories::{self, DeleteCategoryForm},
    dialog,
    theme::color,
};

pub type OnCancel = dialog::OnClick;
pub type OnConfirm = dialog::OnClick;

pub fn render(
    category: &categories::Category,
    form: &DeleteCategoryForm,
    split_count: usize,
    budget_count: usize,
    on_cancel: OnCancel,
    on_confirm: OnConfirm,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_categories_delete_title(&category.name),
            true,
        ))
        .child(dialog::body([
            warning_copy(split_count),
            dialog::info_panel(reference_notice(budget_count)).into_any_element(),
            confirm_input_field(
                &category.name,
                &form.confirmation_name,
            ),
        ]))
        .child(dialog::action_row([
            dialog::cancel_button("delete-category-cancel", on_cancel).into_any_element(),
            dialog::confirm_button(
                "delete-category-confirm",
                crate::msg::desktop_categories_delete_submit(),
                form.matches(&category.name),
                true,
                on_confirm,
            )
            .into_any_element(),
        ]));

    dialog::overlay(WIDTH, true, card)
}

fn reference_notice(budget_count: usize) -> String {
    crate::msg::desktop_categories_delete_reference(i64::try_from(budget_count).unwrap_or(0))
}

fn warning_copy(split_count: usize) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .text_size(px(13.0))
        .children(
            crate::msg::desktop_categories_delete_warning(i64::try_from(split_count).unwrap_or(0))
                .into_iter()
                .map(|segment| match segment.tag.as_deref() {
                    Some("strong") => div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child(segment.text)
                        .into_any_element(),
                    _ => div().child(segment.text).into_any_element(),
                }),
        )
        .into_any_element()
}

fn confirm_input_field(name: &str, input_value: &str) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(crate::msg::desktop_categories_delete_confirm_label(name)),
        )
        .child(
            div()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(color::BORDER)
                .text_size(px(13.0))
                .text_color(color::INK)
                .child(SharedString::from(input_value.to_string())),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_notice_names_budgets() {
        crate::locale::init_for_tests();
        assert_eq!(
            reference_notice(2),
            "Splits will be moved to Uncategorised · 2 budgets attached"
        );
    }

    #[test]
    fn the_reference_notice_handles_singular_and_zero() {
        crate::locale::init_for_tests();
        assert_eq!(
            reference_notice(1),
            "Splits will be moved to Uncategorised · 1 budget attached"
        );
        assert_eq!(
            reference_notice(0),
            "Splits will be moved to Uncategorised · no budget attached"
        );
    }

    #[test]
    fn the_warning_emphasises_transaction_count() {
        crate::locale::init_for_tests();
        let strong: Vec<String> = crate::msg::desktop_categories_delete_warning(3)
            .into_iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .map(|segment| segment.text)
            .collect();
        assert!(!strong.is_empty());
        assert!(strong[0].contains("3 transactions"));
    }
}
