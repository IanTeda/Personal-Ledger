//! Renders the **Edit category** dialog (`docs/ux/desktop/Categories/README.md`'s 5c) on the shared
//! `crate::dialog` chrome. Form fields: Name, Type (editable only on top-level, locked for children),
//! Parent category (tree select, excluding depth-3, the category itself and its descendants),
//! Monthly budget (optional, disabled on parents showing rollup). Warning notice with Split count.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_core::CategoryTypes;

use crate::{
    categories::{self, CategoryField, CategoryForm},
    dialog,
    theme::color,
};

pub const WIDTH: gpui::Pixels = px(440.0);

pub type OnFieldClick = Rc<dyn Fn(CategoryField, &mut Window, &mut App)>;
pub type OnParentChange = Rc<dyn Fn(Option<u32>, &mut Window, &mut App)>;
pub type OnTypeChange = Rc<dyn Fn(CategoryTypes, &mut Window, &mut App)>;
pub type OnCancel = dialog::OnClick;
pub type OnConfirm = dialog::OnClick;

#[derive(Clone)]
pub struct ParentOption {
    pub id: Option<u32>,
    pub label: String,
    pub is_available: bool,
}

pub fn render(
    category_id: u32,
    form: &CategoryForm,
    parent_options: &[ParentOption],
    all_categories: &[categories::Category],
    split_count: usize,
    is_parent: bool,
    on_field_click: OnFieldClick,
    on_parent_change: OnParentChange,
    on_type_change: OnTypeChange,
    on_cancel: OnCancel,
    on_confirm: OnConfirm,
) -> AnyElement {
    let type_locked = form.parent_id.is_some();
    let parent_category = form
        .parent_id
        .and_then(|id| all_categories.iter().find(|c| c.id == id));

    let category = all_categories.iter().find(|c| c.id == category_id);
    let is_top_level = category.map(|c| c.parent.is_none()).unwrap_or(false);

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_categories_edit_title(),
            false,
        ))
        .child(dialog::body([
            name_field(
                &form.name,
                form.focused == CategoryField::Name,
                on_field_click.clone(),
            ),
            type_field(
                form.category_type
                    .as_ref()
                    .unwrap_or(&CategoryTypes::Expense),
                type_locked || !is_top_level,
                on_type_change,
            ),
            parent_field(
                form.parent_id,
                parent_options,
                form.focused == CategoryField::Parent,
                on_field_click.clone(),
                on_parent_change,
            ),
            budget_field(
                &form.budget,
                form.focused == CategoryField::Budget,
                is_parent,
                on_field_click.clone(),
            ),
            edit_notice(split_count, is_parent, type_locked, parent_category),
        ]))
        .child(dialog::action_row([
            dialog::cancel_button("edit-category-cancel", on_cancel).into_any_element(),
            dialog::confirm_button(
                "edit-category-confirm",
                crate::msg::desktop_categories_edit_submit(),
                is_valid(form, all_categories, category_id),
                false,
                on_confirm,
            )
            .into_any_element(),
        ]));

    dialog::overlay(WIDTH, false, card)
}

fn name_field(value: &str, focused: bool, on_field_click: OnFieldClick) -> AnyElement {
    div()
        .child(label(lib_locale::msg::column_name()))
        .child(
            div()
                .id("edit-category-name")
                .cursor_pointer()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(if focused { color::INK } else { color::BORDER })
                .text_size(px(13.0))
                .text_color(if value.is_empty() {
                    color::INK_TERTIARY
                } else {
                    color::INK
                })
                .on_click(move |_event, window, cx| on_field_click(CategoryField::Name, window, cx))
                .child(if value.is_empty() {
                    SharedString::from(crate::msg::desktop_categories_name_placeholder())
                } else {
                    SharedString::from(value.to_string())
                }),
        )
        .into_any_element()
}

fn type_field(category_type: &CategoryTypes, locked: bool, on_change: OnTypeChange) -> AnyElement {
    div()
        .child(label(lib_locale::msg::column_type()))
        .child(
            div()
                .flex()
                .gap(px(8.0))
                .child({
                    let cat_type = category_type.clone();
                    let on_change = on_change.clone();
                    div()
                        .id("edit-type-expense")
                        .cursor_pointer()
                        .px(px(12.0))
                        .py(px(6.0))
                        .border_1()
                        .border_color(if cat_type == CategoryTypes::Expense {
                            color::INK
                        } else {
                            color::BORDER
                        })
                        .bg(if cat_type == CategoryTypes::Expense {
                            color::INK
                        } else {
                            color::GROUND
                        })
                        .text_color(if cat_type == CategoryTypes::Expense {
                            color::GROUND
                        } else {
                            color::INK
                        })
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .when(!locked, |this| {
                            this.on_click(move |_event, window, cx| {
                                on_change(CategoryTypes::Expense, window, cx)
                            })
                        })
                        .when(locked, |this| this.opacity(0.6))
                        .child("expense")
                })
                .child({
                    let cat_type = category_type.clone();
                    let on_change = on_change.clone();
                    div()
                        .id("edit-type-income")
                        .cursor_pointer()
                        .px(px(12.0))
                        .py(px(6.0))
                        .border_1()
                        .border_color(if cat_type == CategoryTypes::Income {
                            color::INK
                        } else {
                            color::BORDER
                        })
                        .bg(if cat_type == CategoryTypes::Income {
                            color::INK
                        } else {
                            color::GROUND
                        })
                        .text_color(if cat_type == CategoryTypes::Income {
                            color::GROUND
                        } else {
                            color::INK
                        })
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .when(!locked, |this| {
                            this.on_click(move |_event, window, cx| {
                                on_change(CategoryTypes::Income, window, cx)
                            })
                        })
                        .when(locked, |this| this.opacity(0.6))
                        .child("income")
                }),
        )
        .into_any_element()
}

fn parent_field(
    selected_parent: Option<u32>,
    options: &[ParentOption],
    focused: bool,
    on_field_click: OnFieldClick,
    on_change: OnParentChange,
) -> AnyElement {
    let _on_change = on_change.clone();
    div()
        .child(suffixed_label(
            crate::msg::desktop_categories_field_parent(),
            crate::msg::desktop_field_optional(),
        ))
        .child(
            div()
                .id("edit-category-parent")
                .cursor_pointer()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(if focused { color::INK } else { color::BORDER })
                .bg(color::CHROME)
                .text_size(px(13.0))
                .text_color(color::INK)
                .on_click(move |_event, window, cx| {
                    on_field_click(CategoryField::Parent, window, cx)
                })
                .child({
                    if let Some(opt) = options.iter().find(|opt| opt.id == selected_parent) {
                        SharedString::from(opt.label.clone())
                    } else {
                        SharedString::from(crate::msg::desktop_categories_parent_none())
                    }
                }),
        )
        .into_any_element()
}

fn budget_field(
    value: &str,
    focused: bool,
    is_parent: bool,
    on_field_click: OnFieldClick,
) -> AnyElement {
    div()
        .child(suffixed_label(
            crate::msg::desktop_categories_field_monthly_budget(),
            crate::msg::desktop_field_optional(),
        ))
        .child(
            div()
                .id("edit-category-budget")
                .cursor_pointer()
                .when(!is_parent, |this| {
                    this.w_full()
                        .py(px(8.0))
                        .px(px(10.0))
                        .border_1()
                        .border_color(if focused { color::INK } else { color::BORDER })
                        .text_size(px(13.0))
                        .text_color(if value.is_empty() {
                            color::INK_TERTIARY
                        } else {
                            color::INK
                        })
                        .on_click(move |_event, window, cx| {
                            on_field_click(CategoryField::Budget, window, cx)
                        })
                        .child(if value.is_empty() {
                            SharedString::from(crate::msg::desktop_categories_budget_placeholder())
                        } else {
                            SharedString::from(value.to_string())
                        })
                })
                .when(is_parent, |this| {
                    this.w_full()
                        .py(px(8.0))
                        .px(px(10.0))
                        .border_1()
                        .border_color(color::BORDER)
                        .bg(color::CHROME)
                        .text_size(px(13.0))
                        .text_color(color::INK_SECONDARY)
                        .opacity(0.6)
                        .child(crate::msg::desktop_categories_parent_budget_rollup())
                }),
        )
        .into_any_element()
}

fn edit_notice(
    split_count: usize,
    is_parent: bool,
    type_locked: bool,
    parent_category: Option<&categories::Category>,
) -> AnyElement {
    if split_count > 0 {
        // Warning notice: accent border with split count
        let split_text = if split_count == 1 {
            "1 transaction".to_string()
        } else {
            format!("{} transactions", split_count)
        };

        div()
            .border_l(px(2.0))
            .border_color(color::INK)
            .bg(color::CHROME)
            .px(px(10.0))
            .py(px(10.0))
            .text_size(px(11.5))
            .text_color(color::INK_SECONDARY)
            .child(format!(
                "This category has {}. Changing its type will affect these transactions.",
                split_text
            ))
            .into_any_element()
    } else if type_locked {
        // Type is locked due to parent
        let parent_name = parent_category
            .map(|c| c.name.as_str())
            .unwrap_or("Unknown");
        let category_type = parent_category
            .map(|c| {
                if c.category_type == CategoryTypes::Expense {
                    "expense"
                } else {
                    "income"
                }
            })
            .unwrap_or("unknown");

        div()
            .border_l(px(2.0))
            .border_color(color::INK)
            .bg(color::CHROME)
            .px(px(10.0))
            .py(px(10.0))
            .text_size(px(11.5))
            .text_color(color::INK_SECONDARY)
            .child(format!(
                "Nesting under {} inherits its type ({}) — the type control locks once a parent is picked.",
                parent_name, category_type
            ))
            .into_any_element()
    } else if is_parent {
        // Neutral notice for parent
        div()
            .border_l(px(2.0))
            .border_color(color::INK)
            .bg(color::CHROME)
            .px(px(10.0))
            .py(px(10.0))
            .text_size(px(11.5))
            .text_color(color::INK_SECONDARY)
            .child(crate::msg::desktop_categories_edit_parent_notice())
            .into_any_element()
    } else {
        // Default notice
        div()
            .border_l(px(2.0))
            .border_color(color::INK)
            .bg(color::CHROME)
            .px(px(10.0))
            .py(px(10.0))
            .text_size(px(11.5))
            .text_color(color::INK_SECONDARY)
            .child(crate::msg::desktop_categories_edit_notice())
            .into_any_element()
    }
}

fn label(text: impl Into<SharedString>) -> AnyElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .mb(px(6.0))
        .child(text.into())
        .into_any_element()
}

fn suffixed_label(text: impl Into<SharedString>, suffix: impl Into<SharedString>) -> AnyElement {
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

fn is_valid(
    form: &CategoryForm,
    all_categories: &[categories::Category],
    category_id: u32,
) -> bool {
    // Name must not be empty
    if form.name.trim().is_empty() {
        return false;
    }

    // Check for sibling name clash (same parent and same name, excluding this category)
    let has_sibling_with_same_name = all_categories.iter().any(|c| {
        c.id != category_id
            && c.parent == form.parent_id
            && c.name.eq_ignore_ascii_case(form.name.trim())
    });

    !has_sibling_with_same_name
}
