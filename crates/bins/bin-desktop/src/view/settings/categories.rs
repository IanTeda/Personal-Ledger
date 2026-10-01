//! The **Categories** page (`docs/ux/desktop/Settings/README.md`'s 2i): a kicker (EXPENSE · n)
//! with **+ Add category** over a nested tree of NAME and ACTIONS only, then the Income tree under
//! its own kicker. Budget and spend figures live in Budgets and Reports; the Monthly budget field
//! stays in the edit dialog. It reuses the Categories model and dialogs.
//!
//! Every action is a callback into `Shell`, so the keyboard (`n`/`N`/`e`/`d`/`→`/`←`) and the
//! mouse reach the same handlers.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_core::CategoryTypes;
use lib_locale::format::upper;

use crate::{
    categories::{self, Category, TreeNode},
    theme::color,
    view::categories::{
        OnAddClick, OnAddSubClick, OnDeleteClick, OnDisclosureClick, OnEditClick, OnRowClick,
    },
};

pub struct CategoriesPageProps<'a> {
    pub categories: &'a [Category],
    /// The ids of the parents shown open.
    pub expanded: &'a [u32],
    /// The selected row's Category id.
    pub selected: Option<u32>,
    pub on_add_click: OnAddClick,
    pub on_add_sub_click: OnAddSubClick,
    pub on_edit_click: OnEditClick,
    pub on_delete_click: OnDeleteClick,
    pub on_disclosure_click: OnDisclosureClick,
    pub on_row_click: OnRowClick,
}

type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// A row count as the number a plural selector takes.
fn count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

/// The section heading's meta: `12 categories · 3 levels deep`.
pub fn scope_note(categories: &[Category]) -> String {
    crate::msg::desktop_settings_categories_summary(
        count(categories.len()),
        i64::from(categories::levels_deep(categories)),
    )
}

pub fn render(props: &CategoriesPageProps<'_>, focused: bool, cx: &App) -> AnyElement {
    let rows = categories::settings_rows(props.categories, props.expanded);
    let of_type = |wanted: &CategoryTypes| -> Vec<&TreeNode> {
        rows.iter()
            .filter(|row| {
                props
                    .categories
                    .iter()
                    .any(|c| c.id == row.id && &c.category_type == wanted)
            })
            .collect()
    };
    let total = |wanted: &CategoryTypes| {
        count(
            props
                .categories
                .iter()
                .filter(|c| &c.category_type == wanted)
                .count(),
        )
    };

    div()
        .flex()
        .flex_col()
        .child(kicker_row(
            upper(&crate::msg::desktop_settings_categories_kicker_expense(
                &total(&CategoryTypes::Expense).to_string(),
            )),
            Some(props.on_add_click.clone()),
            cx,
        ))
        .child(tree_table(
            &of_type(&CategoryTypes::Expense),
            focused,
            props,
            cx,
        ))
        .child(kicker_row(
            upper(&crate::msg::desktop_settings_categories_kicker_income(
                &total(&CategoryTypes::Income).to_string(),
            )),
            None,
            cx,
        ))
        .child(tree_table(
            &of_type(&CategoryTypes::Income),
            focused,
            props,
            cx,
        ))
        .child(
            div()
                .max_w(px(620.0))
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_settings_categories_note()),
        )
        .into_any_element()
}

fn kicker_row(label: String, on_add_click: Option<OnAddClick>, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .mb(px(10.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(label),
        )
        .children(on_add_click.map(|on_click| add_button(on_click, cx)))
}

/// `btn btn-primary` at `height:32px`.
fn add_button(on_click: OnAddClick, cx: &App) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .id("settings-categories-add")
        .cursor_pointer()
        .flex_none()
        .py(px(8.0))
        .px(px(14.0))
        .bg(color::foreground(cx))
        .text_color(color::selection_text(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(crate::msg::desktop_settings_categories_add())
}

/// The bordered table: a NAME / ACTIONS header (`padding:6px 14px; background:#eae9e9`), then a
/// tree row per visible Category.
fn tree_table(
    rows: &[&TreeNode],
    focused: bool,
    props: &CategoriesPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let last = rows.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(cx))
        .mb(px(18.0))
        .child(
            div()
                .flex()
                .px(px(14.0))
                .py(px(6.0))
                .bg(color::chrome(cx))
                .border_b(px(1.0))
                .border_color(color::border(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::muted(cx))
                .child(div().flex_1().child(upper(&lib_locale::msg::column_name())))
                .child(div().child(upper(&lib_locale::msg::column_actions()))),
        )
        .children(rows.iter().enumerate().map(|(position, row)| {
            tree_row(
                row,
                position == last,
                props.selected == Some(row.id),
                focused,
                props,
                cx,
            )
        }))
}

/// `padding:3px 14px; font-size:12.5px`; the selected row is inverted while the page has focus
/// (`background:#201e1d; color:#f3f2f2`) and only tinted while the index does.
fn tree_row(
    row: &TreeNode,
    last: bool,
    selected: bool,
    focused: bool,
    props: &CategoriesPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let id = row.id;
    let inverted = selected && focused;
    let hover = color::hover(cx);
    let on_row_click = props.on_row_click.clone();
    let on_disclosure_click = props.on_disclosure_click.clone();
    let can_add_sub = row.depth < 2;
    let subcategories = categories::child_count(props.categories, id);

    div()
        .id(SharedString::from(format!("settings-categories-row-{id}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(14.0))
        .py(px(3.0))
        .text_size(px(12.5))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected && !focused, |this| this.bg(color::chrome(cx)))
        .when(inverted, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        .on_click(move |_event, window, cx| on_row_click(id, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .pl(px(row.depth as f32 * 20.0))
                .child(disclosure(row, inverted, on_disclosure_click, cx))
                .child(
                    div()
                        .truncate()
                        .when(!row.is_leaf, |this| {
                            this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        })
                        .child(row.name.clone()),
                )
                .when(!row.is_leaf, |this| {
                    this.child(
                        div()
                            .flex_none()
                            .text_size(px(10.5))
                            .text_color(if inverted {
                                color::selection_text(cx)
                            } else {
                                color::faint_text(cx)
                            })
                            .child(crate::msg::desktop_settings_categories_subcount(count(
                                subcategories,
                            ))),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .when(can_add_sub, |this| {
                    let on_click = props.on_add_sub_click.clone();
                    this.child(row_action_button(
                        SharedString::from(format!("settings-categories-sub-{id}")),
                        crate::msg::desktop_settings_categories_row_sub(),
                        Rc::new(move |window: &mut Window, cx: &mut App| on_click(id, window, cx)),
                        cx,
                    ))
                })
                .child(row_action_button(
                    SharedString::from(format!("settings-categories-edit-{id}")),
                    crate::msg::desktop_accounts_row_edit(),
                    {
                        let on_click = props.on_edit_click.clone();
                        Rc::new(move |window: &mut Window, cx: &mut App| on_click(id, window, cx))
                    },
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("settings-categories-delete-{id}")),
                    crate::msg::desktop_accounts_row_delete(),
                    {
                        let on_click = props.on_delete_click.clone();
                        Rc::new(move |window: &mut Window, cx: &mut App| on_click(id, window, cx))
                    },
                    cx,
                )),
        )
}

/// The 10px ▾/▸ cell; a leaf keeps the width, empty, so names line up.
fn disclosure(
    row: &TreeNode,
    inverted: bool,
    on_click: OnDisclosureClick,
    cx: &App,
) -> impl IntoElement {
    let id = row.id;
    let cell = div().w(px(10.0)).flex_none().text_color(if inverted {
        color::selection_text(cx)
    } else {
        color::muted(cx)
    });
    if row.is_leaf {
        return cell.into_any_element();
    }
    cell.id(SharedString::from(format!("settings-categories-fold-{id}")))
        .cursor_pointer()
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(id, window, cx)
        })
        .child(if row.is_expanded {
            "\u{25be}"
        } else {
            "\u{25b8}"
        })
        .into_any_element()
}

/// `padding:2px 6px; font-size:11px; border:1px solid rgba(32,30,29,.30); background:transparent`.
/// Stops the click reaching the row's own handler.
fn row_action_button(
    id: SharedString,
    label: String,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .id(id)
        .cursor_pointer()
        .py(px(2.0))
        .px(px(6.0))
        .border_1()
        .border_color(color::border(cx))
        .text_size(px(11.0))
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_note_reads_count_and_levels_deep() {
        crate::locale::init_for_tests();
        assert_eq!(
            scope_note(&categories::default_categories()),
            "12 categories \u{b7} 3 levels deep"
        );
    }
}
