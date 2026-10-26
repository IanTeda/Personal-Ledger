//! Draws the **Categories** page (`docs/ux/desktop-mockups/16-settings/README.md`'s 2i) from the
//! Presenter's view model: a kicker (EXPENSE · n) with **+ Add category** over a nested tree of
//! NAME and ACTIONS only, then the Income tree under its own kicker, and the note.
//!
//! Every click is an event sent to the Presenter, so the keyboard and the mouse reach the same
//! handlers.

use gpui::{AnyElement, App, Context, Entity, Render, SharedString, Window, div, prelude::*, px};
use lib_categories::presenter::{
    CategoriesPageEvent, CategoriesPageViewModel, CategoryLine, CategoryTree,
};
use lib_core::CategoryId;

use super::CategoriesPage;
use crate::theme::color;

type OnClick = std::rc::Rc<dyn Fn(&mut Window, &mut App)>;

/// A click that sends `event` to the page.
fn on(entity: &Entity<CategoriesPage>, event: CategoriesPageEvent) -> OnClick {
    let entity = entity.clone();
    std::rc::Rc::new(move |_window, cx| {
        entity.update(cx, |page, cx| page.send(event.clone(), cx));
    })
}

/// A row click: selects it and hands focus to the page.
fn on_row(entity: &Entity<CategoriesPage>, id: CategoryId) -> OnClick {
    let entity = entity.clone();
    std::rc::Rc::new(move |_window, cx| {
        entity.update(cx, |page, cx| {
            page.click(CategoriesPageEvent::Select(id), cx)
        });
    })
}

impl Render for CategoriesPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let view = self.view_model();
        let focused = self.page_focused;
        let cx: &App = cx;
        div()
            .flex()
            .flex_col()
            .child(kicker_row(
                view.expense.kicker.clone(),
                Some((
                    view.add_button.clone(),
                    on(&entity, CategoriesPageEvent::New),
                )),
                cx,
            ))
            .child(tree_table(&view.expense, &view, focused, &entity, cx))
            .child(kicker_row(view.income.kicker.clone(), None, cx))
            .child(tree_table(&view.income, &view, focused, &entity, cx))
            .child(
                div()
                    .max_w(px(620.0))
                    .text_size(px(11.5))
                    .text_color(color::muted(cx))
                    .child(view.note.clone()),
            )
    }
}

fn kicker_row(label: String, add: Option<(String, OnClick)>, cx: &App) -> impl IntoElement {
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
        .children(add.map(|(label, on_click)| add_button(label, on_click, cx)))
}

/// `btn btn-primary` at `height:32px`.
fn add_button(label: String, on_click: OnClick, cx: &App) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .debug_selector(|| "settings-categories-add".to_string())
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
        .child(label)
}

/// The bordered table: a NAME / ACTIONS header (`padding:6px 14px; background:#eae9e9`), then a
/// tree row per Category the folds show.
fn tree_table(
    tree: &CategoryTree,
    view: &CategoriesPageViewModel,
    focused: bool,
    entity: &Entity<CategoriesPage>,
    cx: &App,
) -> impl IntoElement {
    let last = tree.rows.len().saturating_sub(1);
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
                .child(div().flex_1().child(view.columns.name.clone()))
                .child(div().child(view.columns.actions.clone())),
        )
        .children(tree.rows.iter().enumerate().map(|(position, row)| {
            tree_row(
                row,
                position == last,
                view.selected == Some(row.id),
                focused,
                view,
                entity,
                cx,
            )
        }))
}

/// `padding:3px 14px; font-size:12.5px`; the selected row is inverted while the page has focus
/// (`background:#201e1d; color:#f3f2f2`) and only tinted while the index does.
fn tree_row(
    row: &CategoryLine,
    last: bool,
    selected: bool,
    focused: bool,
    view: &CategoriesPageViewModel,
    entity: &Entity<CategoriesPage>,
    cx: &App,
) -> impl IntoElement {
    let id = row.id;
    let inverted = selected && focused;
    let hover = color::hover(cx);
    let on_row_click = on_row(entity, id);

    div()
        .debug_selector(move || format!("settings-categories-row-{id}"))
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
        .on_click(move |_event, window, cx| on_row_click(window, cx))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .pl(px(row.depth as f32 * 20.0))
                .child(disclosure(
                    row,
                    inverted,
                    on(entity, CategoriesPageEvent::Toggle(id)),
                    cx,
                ))
                .child(
                    div()
                        .truncate()
                        .when(!row.is_leaf, |this| {
                            this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        })
                        .child(row.name.clone()),
                )
                .children(row.subcount.clone().map(|subcount| {
                    div()
                        .flex_none()
                        .text_size(px(10.5))
                        .text_color(if inverted {
                            color::selection_text(cx)
                        } else {
                            color::faint_text(cx)
                        })
                        .child(subcount)
                })),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .when(row.can_add_child, |this| {
                    this.child(row_action_button(
                        SharedString::from(format!("settings-categories-sub-{id}")),
                        view.row_add_child.clone(),
                        on(entity, CategoriesPageEvent::AddChildRow(id)),
                        cx,
                    ))
                })
                .child(row_action_button(
                    SharedString::from(format!("settings-categories-edit-{id}")),
                    view.row_edit.clone(),
                    on(entity, CategoriesPageEvent::EditRow(id)),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("settings-categories-delete-{id}")),
                    view.row_delete.clone(),
                    on(entity, CategoriesPageEvent::DeleteRow(id)),
                    cx,
                )),
        )
}

/// The 10px ▾/▸ cell; a leaf keeps the width, empty, so names line up.
fn disclosure(row: &CategoryLine, inverted: bool, on_click: OnClick, cx: &App) -> AnyElement {
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
            on_click(window, cx)
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
    on_click: OnClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
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
