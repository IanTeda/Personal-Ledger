//! The **5a** Categories page (`docs/ux/desktop/Categories/README.md`): a full-width management page
//! with header (title, computed meta line `N categories, D levels deep · K over budget this month`,
//! `+ Add category`), 2px rule, then Expense and Income sections as bordered management tables
//! with tree rows: NAME cell (computed indent, ▾/▸ disclosure, bold parent + `rollup` tag),
//! BUDGET (or `no budget`), SPENT THIS MONTH / RECEIVED THIS MONTH with 4px progress bar (red
//! only for Expense over budget), and ACTIONS micro-buttons (`+ sub` where depth allows, `edit`,
//! `delete`). Selection with `j`/`k`, `→`/`←` expand/collapse, disclosure click.
//!
//! Like `view::settings` and `view::accounts`, the page owns its scroll container directly:
//! `Shell` scrolls a section into view with `ScrollHandle::scroll_to_item`. Context rail is hidden.

pub mod add_dialog;
pub mod delete_dialog;
pub mod edit_dialog;

use std::rc::Rc;

use gpui::{AnyElement, App, ScrollHandle, SharedString, Window, div, prelude::*, px};
use lib_core::{CategoryTypes, Money};
use lib_locale::format::upper;

use crate::{budgets, categories, categories::TreeNode, theme::color, transactions::Transaction};

pub type OnAddClick = Rc<dyn Fn(&mut Window, &mut App)>;
pub type OnAddSubClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnEditClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnDeleteClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnDisclosureClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnRowClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
/// Any per-row handler above, as an [`action_button`] takes it.
type OnCategoryIdClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;

/// The handlers the Add and Edit dialogs both wire, grouped so each `render` stays a short list
/// of what differs between them.
pub struct DialogHandlers {
    pub on_field_click: add_dialog::OnFieldClick,
    pub on_parent_change: add_dialog::OnParentChange,
    pub on_type_change: add_dialog::OnTypeChange,
    pub on_cancel: add_dialog::OnCancel,
    pub on_confirm: add_dialog::OnConfirm,
}

pub struct CategoriesPageProps<'a> {
    pub categories: &'a [categories::Category],
    pub budgets: &'a [budgets::Budget],
    pub transactions: &'a [Transaction],
    pub selected_index: Option<usize>,
    pub expanded: &'a [u32],
    pub base_unit_id: u32,
    pub today: chrono::NaiveDate,
    pub on_add_click: OnAddClick,
    pub on_add_sub_click: OnAddSubClick,
    pub on_edit_click: OnEditClick,
    pub on_delete_click: OnDeleteClick,
    pub on_disclosure_click: OnDisclosureClick,
    pub on_row_click: OnRowClick,
}

pub fn render(
    focused: bool,
    scroll_handle: &ScrollHandle,
    props: CategoriesPageProps<'_>,
) -> AnyElement {
    let tree_rows = categories::tree_rows(props.categories, props.expanded);

    let over_budget_count = count_over_budget(
        props.categories,
        props.budgets,
        props.transactions,
        props.base_unit_id,
        props.today,
    );

    div()
        .id("categories")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::INK)
        })
        .px(px(28.0))
        .py(px(22.0))
        .child(page_header(
            props.categories,
            over_budget_count,
            &props.on_add_click,
        ))
        .child(
            div()
                .h(px(2.0))
                .flex_none()
                .bg(color::STRUCTURAL_RULE)
                .mt(px(14.0))
                .mb(px(24.0)),
        )
        .children(
            [CategoryTypes::Expense, CategoryTypes::Income]
                .iter()
                .map(|cat_type| section_block(cat_type, &tree_rows, &props)),
        )
        .into_any_element()
}

fn page_header(
    categories: &[categories::Category],
    over_budget_count: usize,
    on_add_click: &OnAddClick,
) -> impl IntoElement {
    let max_depth = categories
        .iter()
        .map(|c| categories::depth(categories, c.id))
        .max()
        .unwrap_or(0);

    div()
        .flex()
        .items_end()
        .justify_between()
        .gap(px(16.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(28.0))
                        .text_color(color::INK)
                        .child(crate::nav::Noun::Categories.label()),
                )
                .child(summary_line(categories.len(), max_depth, over_budget_count)),
        )
        .child(add_button(on_add_click.clone()))
}

fn summary_line(
    category_count: usize,
    max_depth: u32,
    over_budget_count: usize,
) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap(px(4.0))
        .text_size(px(11.5))
        .text_color(color::INK_TERTIARY)
        .child(format!(
            "{} {}, {} {} {}",
            category_count,
            if category_count == 1 {
                "category"
            } else {
                "categories"
            },
            max_depth,
            if max_depth == 1 { "level" } else { "levels" },
            "deep"
        ))
        .child("·")
        .child(format!("{} over budget this month", over_budget_count))
}

fn add_button(on_add_click: OnAddClick) -> impl IntoElement {
    div()
        .id("categories-add")
        .cursor_pointer()
        .flex_none()
        .py(px(10.0))
        .px(px(16.0))
        .bg(color::INK)
        .text_color(color::INK_ON_DARK)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .hover(|this| this.bg(color::INK_SECONDARY))
        .on_click(move |_event, window, cx| on_add_click(window, cx))
        .child("+ Add category")
}

fn section_block(
    category_type: &CategoryTypes,
    tree_rows: &[TreeNode],
    props: &CategoriesPageProps<'_>,
) -> impl IntoElement {
    // Pair each row with its Category here, so `table_row` never has to look it up again.
    let filtered_rows: Vec<_> = tree_rows
        .iter()
        .filter_map(|row| {
            props
                .categories
                .iter()
                .find(|c| c.id == row.id && &c.category_type == category_type)
                .map(|category| (row, category))
        })
        .collect();

    div()
        .mb(px(24.0))
        .child(
            div()
                .border(px(1.0))
                .border_color(color::BORDER)
                .overflow_hidden()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .child(table_header(category_type))
                        .children(
                            filtered_rows.iter().enumerate().map(|(idx, (row, category))| {
                                table_row(row, category, Some(idx) == props.selected_index, props)
                            }),
                        ),
                )
        )
        .when(*category_type == CategoryTypes::Expense, |this| {
            this.child(
                div()
                    .text_size(px(11.0))
                    .text_color(color::INK_TERTIARY)
                    .mt(px(6.0))
                    .child("Budget figures are monthly targets. Progress bars show spending in the current month."),
            )
        })
}

fn table_header(category_type: &CategoryTypes) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .bg(color::CHROME)
        .border_b(px(1.0))
        .border_color(color::BORDER)
        .px(px(12.0))
        .py(px(8.0))
        .gap(px(12.0))
        .text_size(px(10.0))
        .text_color(color::INK_TERTIARY)
        .font_weight(gpui::FontWeight::BOLD)
        .child(
            div()
                .flex_1()
                .min_w(px(200.0))
                .child(upper(&lib_locale::msg::column_name())),
        )
        .child(
            div()
                .flex_none()
                .w(px(100.0))
                .text_align(gpui::TextAlign::Right)
                .child(if *category_type == CategoryTypes::Expense {
                    "BUDGET"
                } else {
                    "TARGET"
                }),
        )
        .child(
            div()
                .flex_none()
                .w(px(120.0))
                .text_align(gpui::TextAlign::Right)
                .child(if *category_type == CategoryTypes::Expense {
                    "SPENT THIS MONTH"
                } else {
                    "RECEIVED THIS MONTH"
                }),
        )
        .child(
            div()
                .flex_none()
                .w(px(80.0))
                .text_align(gpui::TextAlign::Center)
                .child(upper(&lib_locale::msg::column_actions())),
        )
}

fn table_row(
    row: &TreeNode,
    category: &categories::Category,
    selected: bool,
    props: &CategoriesPageProps<'_>,
) -> impl IntoElement {
    let spent = categories::month_to_date_spent(
        props.categories,
        props.transactions,
        &[],
        row.id,
        props.today,
    );
    let budget = budgets::find_by_category_and_unit(props.budgets, row.id, props.base_unit_id)
        .map(|b| b.monthly_amount.clone());
    let is_over = budget
        .as_ref()
        .map(|b| budgets::is_over_budget(&spent, b))
        .unwrap_or(false);

    let indent_px = (row.depth as f32) * 16.0;
    let disclosure_on_click = props.on_disclosure_click.clone();
    let row_click = props.on_row_click.clone();
    let row_id = row.id;

    div()
        .w_full()
        .flex()
        .cursor_pointer()
        .border_b(px(1.0))
        .border_color(color::HAIRLINE)
        .on_mouse_down(gpui::MouseButton::Left, move |_event, window, cx| {
            row_click(row_id, window, cx)
        })
        .when(selected, |this| this.bg(color::CHROME))
        .hover(|this| this.bg(color::HOVER_TINT))
        .px(px(12.0))
        .py(px(10.0))
        .gap(px(12.0))
        .child(
            div()
                .flex_1()
                .min_w(px(200.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().w(px(indent_px)).flex_none())
                .child(if !row.is_leaf {
                    disclosure_toggle(row_id, row.is_expanded, disclosure_on_click.clone())
                        .into_any_element()
                } else {
                    div().w(px(12.0)).into_any_element()
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .when(row.is_leaf, |this| this)
                                .when(!row.is_leaf, |this| {
                                    this.font_weight(gpui::FontWeight::BOLD)
                                })
                                .text_color(color::INK)
                                .child(row.name.clone()),
                        )
                        .when(!row.is_leaf, |this| {
                            this.child(
                                div()
                                    .text_size(px(9.0))
                                    .text_color(color::INK_TERTIARY)
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .bg(color::CHROME)
                                    .child("rollup"),
                            )
                        }),
                ),
        )
        .child(
            div()
                .flex_none()
                .w(px(100.0))
                .text_align(gpui::TextAlign::Right)
                .text_color(color::INK)
                .text_size(px(12.0))
                .child(if let Some(ref b) = budget {
                    let (_, formatted) = crate::format::amount(b);
                    formatted
                } else {
                    "no budget".to_string()
                }),
        )
        .child(
            div()
                .flex_none()
                .w(px(120.0))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .text_align(gpui::TextAlign::Right)
                        .text_color(color::INK)
                        .text_size(px(12.0))
                        .child({
                            let (_, formatted) = crate::format::amount(&spent);
                            formatted
                        }),
                )
                .child(progress_bar(
                    &spent,
                    budget.as_ref(),
                    category.category_type.clone(),
                    is_over,
                )),
        )
        .child(
            div()
                .flex_none()
                .w(px(80.0))
                .flex()
                .justify_center()
                .gap(px(4.0))
                .child(when_can_add_sub(row, props.on_add_sub_click.clone()))
                .child(action_button(
                    SharedString::from(format!("category-edit-{}", row.id)),
                    "✎",
                    row.id,
                    props.on_edit_click.clone(),
                ))
                .child(action_button(
                    SharedString::from(format!("category-delete-{}", row.id)),
                    "✕",
                    row.id,
                    props.on_delete_click.clone(),
                )),
        )
}

fn disclosure_toggle(id: u32, is_expanded: bool, on_click: OnDisclosureClick) -> impl IntoElement {
    div()
        .id("disclosure-toggle")
        .cursor_pointer()
        .text_color(color::INK)
        .on_click(move |_event, window, cx| on_click(id, window, cx))
        .child(if is_expanded { "▾" } else { "▸" })
}

fn when_can_add_sub(row: &TreeNode, on_add_sub_click: OnAddSubClick) -> AnyElement {
    let can_add = (row.is_leaf && row.depth < 2) || !row.is_leaf;

    if can_add {
        let row_id = row.id;
        div()
            .id("category-add-sub")
            .cursor_pointer()
            .px(px(4.0))
            .py(px(2.0))
            .text_color(color::INK_SECONDARY)
            .hover(|this| this.text_color(color::INK))
            .text_size(px(10.0))
            .on_click(move |_event, window, cx| on_add_sub_click(row_id, window, cx))
            .child("+")
            .into_any_element()
    } else {
        div().into_any_element()
    }
}

fn action_button(
    id: SharedString,
    symbol: &'static str,
    category_id: u32,
    on_click: OnCategoryIdClick,
) -> impl IntoElement {
    div()
        .id(id)
        .cursor_pointer()
        .px(px(4.0))
        .py(px(2.0))
        .text_color(color::INK_SECONDARY)
        .hover(|this| this.text_color(color::INK))
        .text_size(px(10.0))
        .on_click(move |_event, window, cx| on_click(category_id, window, cx))
        .child(symbol)
}

fn progress_bar(
    spent: &Money,
    budget: Option<&Money>,
    category_type: CategoryTypes,
    is_over: bool,
) -> impl IntoElement {
    let percent = if let Some(Money(budget_amount)) = budget {
        let spent_abs = spent.0.abs();
        let spent_f: f64 = spent_abs.to_string().parse().unwrap_or(0.0);
        let budget_f: f64 = budget_amount.to_string().parse().unwrap_or(1.0);
        let ratio_f = (spent_f / budget_f * 100.0).abs();
        ratio_f.clamp(0.0, 100.0)
    } else {
        0.0
    };

    // ACCENT is reserved for over-budget state (theme.rs), matching the Dashboard's bars.
    let bar_color = if is_over && category_type == CategoryTypes::Expense {
        color::ACCENT
    } else {
        color::INK_SECONDARY
    };

    div()
        .h(px(4.0))
        .w_full()
        .bg(color::CHROME)
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .w(px((percent / 100.0 * 120.0) as f32))
                .bg(bar_color),
        )
}

fn count_over_budget(
    categories: &[categories::Category],
    budgets: &[budgets::Budget],
    transactions: &[Transaction],
    base_unit_id: u32,
    today: chrono::NaiveDate,
) -> usize {
    categories
        .iter()
        .filter(|c| c.category_type == CategoryTypes::Expense)
        .filter(|c| {
            let spent = categories::month_to_date_spent(categories, transactions, &[], c.id, today);
            if let Some(budget) = budgets::find_by_category_and_unit(budgets, c.id, base_unit_id) {
                budgets::is_over_budget(&spent, &budget.monthly_amount)
            } else {
                false
            }
        })
        .count()
}
