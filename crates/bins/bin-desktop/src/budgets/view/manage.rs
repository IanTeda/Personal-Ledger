//! Draws **Manage budgets** (`docs/ux/desktop-mockups/14-budgets-v2/README.md`'s 11f) from its view
//! model, on the shared `crate::dialog` frame: the header counts, one row per Budget with its
//! actions, a refused action's error and the three notes. An archived row is dimmed.

use gpui::{AnyElement, App, Context, SharedString, div, prelude::*, px};
use lib_budgets::presenter::{ManageEvent, ManageRow, ManageViewModel};
use lib_locale::format::upper;

use super::dialog::ListDialog;
use crate::dialog;
use crate::form::colour_field::error_line;
use crate::theme::color;

/// Wide enough for the table's seven columns.
const WIDTH: gpui::Pixels = px(880.0);

const METHOD_WIDTH: gpui::Pixels = px(80.0);
const PERIOD_WIDTH: gpui::Pixels = px(80.0);
const ACCOUNTS_WIDTH: gpui::Pixels = px(90.0);
const SCOPE_WIDTH: gpui::Pixels = px(110.0);
const DEFAULT_WIDTH: gpui::Pixels = px(60.0);
const ACTIONS_WIDTH: gpui::Pixels = px(250.0);

fn table_header(view: &ManageViewModel, cx: &App) -> impl IntoElement {
    let columns = &view.columns;
    let cell = |width, label: &str| div().w(width).flex_none().child(upper(label));
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(14.0))
        .py(px(9.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        .child(div().flex_1().min_w(px(0.0)).child(upper(&columns.budget)))
        .child(cell(METHOD_WIDTH, &columns.method))
        .child(cell(PERIOD_WIDTH, &columns.period))
        .child(cell(ACCOUNTS_WIDTH, &columns.accounts))
        .child(cell(SCOPE_WIDTH, &columns.scope))
        .child(div().w(DEFAULT_WIDTH).flex_none())
        .child(cell(ACTIONS_WIDTH, &columns.actions))
}

fn table_row(
    index: usize,
    row: &ManageRow,
    selected: bool,
    last: bool,
    cx: &Context<'_, ListDialog>,
) -> AnyElement {
    let app: &App = cx;
    let (primary, secondary) = if selected {
        (color::selection_text(app), color::selection_muted(app))
    } else {
        (color::foreground(app), color::muted(app))
    };
    let on_row_click = ListDialog::on_manage(cx, ManageEvent::Select(index));
    let id = row.id;
    let links = row.actions.iter().map(|choice| {
        let action = choice.value;
        let on_click = ListDialog::on_manage(cx, ManageEvent::ActOn(id, action));
        let selector = format!("budgets-manage-{id}-{action:?}");
        div()
            .id(SharedString::from(selector.clone()))
            .debug_selector(move || selector.clone())
            .cursor_pointer()
            .text_size(px(11.5))
            .text_color(secondary)
            .hover(move |style| style.text_color(primary))
            .on_click(move |_event, window, cx| {
                // The row's own click would only move the cursor.
                cx.stop_propagation();
                on_click(window, cx);
            })
            .child(choice.label.clone())
    });
    div()
        .id(SharedString::from(format!("budgets-manage-row-{index}")))
        .debug_selector(move || format!("budgets-manage-row-{index}"))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(14.0))
        .py(px(9.0))
        .text_size(px(13.0))
        .text_color(primary)
        .when(row.archived && !selected, |this| this.opacity(0.55))
        .when(selected, |this| this.bg(color::selection_background(app)))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(app))
        })
        .on_click(move |_event, window, cx| on_row_click(window, cx))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(row.name.clone()),
        )
        .child(
            div()
                .w(METHOD_WIDTH)
                .flex_none()
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(upper(&row.method)),
        )
        .child(
            div()
                .w(PERIOD_WIDTH)
                .flex_none()
                .text_color(secondary)
                .child(row.period.clone()),
        )
        .child(
            div()
                .w(ACCOUNTS_WIDTH)
                .flex_none()
                .text_color(secondary)
                .child(row.accounts.clone()),
        )
        .child(
            div()
                .w(SCOPE_WIDTH)
                .flex_none()
                .text_color(secondary)
                .child(row.scope.clone()),
        )
        .child(
            div()
                .w(DEFAULT_WIDTH)
                .flex_none()
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(row.default.as_deref().map(upper).unwrap_or_default()),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex_none()
                .flex()
                .gap(px(12.0))
                .children(links),
        )
        .into_any_element()
}

fn note(title: String, body: String, cx: &App) -> impl IntoElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(title),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(body),
        )
}

pub(super) fn render(view: &ManageViewModel, cx: &Context<'_, ListDialog>) -> AnyElement {
    let on_close = ListDialog::on_manage(cx, ManageEvent::Cancel);
    let on_new = ListDialog::on_manage(cx, ManageEvent::New);
    let count = view.rows.len();
    let selected = view.selected.min(count.saturating_sub(1));
    let rows: Vec<AnyElement> = view
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| table_row(index, row, index == selected, index + 1 == count, cx))
        .collect();
    let app: &App = cx;
    let table = div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(app))
        .child(table_header(view, app))
        .children(rows);
    let notes = div().flex().gap(px(20.0)).children(
        view.notes
            .iter()
            .map(|(title, body)| note(title.clone(), body.clone(), app)),
    );

    let mut fields = vec![
        div()
            .text_size(px(12.5))
            .text_color(color::muted(app))
            .child(dialog::rich_text(
                view.counts.clone(),
                Some(color::foreground(app).into()),
            ))
            .into_any_element(),
        table.into_any_element(),
    ];
    if let Some(error) = &view.error {
        fields.push(error_line(error.clone(), app));
    }
    fields.push(notes.into_any_element());

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(view.title.clone(), false, app))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("budgets-manage-close", on_close, app).into_any_element(),
                dialog::confirm_button(
                    "budgets-manage-new",
                    view.new_label.clone(),
                    true,
                    false,
                    on_new,
                    app,
                )
                .into_any_element(),
            ],
            app,
        ));
    dialog::overlay(WIDTH, false, card, app)
}
