//! Renders **Manage budgets** (`docs/ux/desktop/design_handoff_budgets_v2/README.md`'s 11f) on the shared
//! `crate::dialog` chrome: the header counts, one row per Budget with its actions, and the three
//! notes. An archived row is dimmed and offers only `restore` and `duplicate`; `set default` is
//! left off the default and `archive` is refused on it (#400). `Shell` owns the row cursor and
//! every keystroke.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_locale::format::upper;

use crate::{
    budgets::{BudgetError, Method},
    dialog,
    theme::color,
};

use super::limit_dialog::{error_line, error_text};

/// Wide enough for the table's seven columns.
pub const WIDTH: gpui::Pixels = px(880.0);

const METHOD_WIDTH: gpui::Pixels = px(80.0);
const PERIOD_WIDTH: gpui::Pixels = px(80.0);
const ACCOUNTS_WIDTH: gpui::Pixels = px(90.0);
const SCOPE_WIDTH: gpui::Pixels = px(110.0);
const DEFAULT_WIDTH: gpui::Pixels = px(60.0);
const ACTIONS_WIDTH: gpui::Pixels = px(250.0);

/// What a row's action link does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageAction {
    Open,
    Edit,
    Duplicate,
    SetDefault,
    Archive,
    Restore,
}

/// Called with the Budget's id and the action clicked.
pub type OnActionClick = Rc<dyn Fn(u32, ManageAction, &mut Window, &mut App)>;
/// Called with a row's position.
pub type OnRowClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// One Budget, already worded.
pub struct ManageRow {
    pub id: u32,
    pub name: String,
    pub method: Method,
    pub accounts: String,
    pub scope: String,
    pub is_default: bool,
    pub archived: bool,
}

pub struct ManageDialogProps {
    /// The header's "4 active · 1 archived · opens to Household".
    pub counts: Vec<lib_locale::Segment>,
    pub rows: Vec<ManageRow>,
    pub selected: usize,
    /// A refused action, shown until the next one.
    pub error: Option<BudgetError>,
    pub on_row_click: OnRowClick,
    pub on_action_click: OnActionClick,
    pub on_new: dialog::OnClick,
    pub on_close: dialog::OnClick,
}

fn action_label(action: ManageAction) -> String {
    match action {
        ManageAction::Open => crate::msg::desktop_budgets_manage_action_open(),
        ManageAction::Edit => crate::msg::desktop_budgets_manage_action_edit(),
        ManageAction::Duplicate => crate::msg::desktop_budgets_manage_action_duplicate(),
        ManageAction::SetDefault => crate::msg::desktop_budgets_manage_action_default(),
        ManageAction::Archive => crate::msg::desktop_budgets_manage_action_archive(),
        ManageAction::Restore => crate::msg::desktop_budgets_manage_action_restore(),
    }
}

/// The actions a row offers, in the order drawn.
pub fn actions(is_default: bool, archived: bool) -> Vec<ManageAction> {
    if archived {
        return vec![
            ManageAction::Open,
            ManageAction::Restore,
            ManageAction::Duplicate,
        ];
    }
    let mut all = vec![
        ManageAction::Open,
        ManageAction::Edit,
        ManageAction::Duplicate,
    ];
    if !is_default {
        all.push(ManageAction::SetDefault);
        all.push(ManageAction::Archive);
    }
    all
}

fn table_header(cx: &App) -> impl IntoElement {
    let cell = |width, label: String| div().w(width).flex_none().child(upper(&label));
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
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(upper(&crate::msg::desktop_budgets_manage_column_budget())),
        )
        .child(cell(
            METHOD_WIDTH,
            crate::msg::desktop_budgets_manage_column_method(),
        ))
        .child(cell(
            PERIOD_WIDTH,
            crate::msg::desktop_budgets_manage_column_period(),
        ))
        .child(cell(
            ACCOUNTS_WIDTH,
            crate::msg::desktop_budgets_manage_column_accounts(),
        ))
        .child(cell(
            SCOPE_WIDTH,
            crate::msg::desktop_budgets_manage_column_scope(),
        ))
        .child(div().w(DEFAULT_WIDTH).flex_none())
        .child(cell(
            ACTIONS_WIDTH,
            crate::msg::desktop_budgets_manage_column_actions(),
        ))
}

fn table_row(
    index: usize,
    row: ManageRow,
    selected: bool,
    last: bool,
    on_row_click: &OnRowClick,
    on_action_click: &OnActionClick,
    cx: &App,
) -> AnyElement {
    let (primary, secondary) = if selected {
        (color::selection_text(cx), color::selection_muted(cx))
    } else {
        (color::foreground(cx), color::muted(cx))
    };
    let on_row_click = on_row_click.clone();
    let id = row.id;
    let links = actions(row.is_default, row.archived)
        .into_iter()
        .map(|action| {
            let on_click = on_action_click.clone();
            div()
                .id(SharedString::from(format!(
                    "budgets-manage-{id}-{action:?}"
                )))
                .cursor_pointer()
                .text_size(px(11.5))
                .text_color(secondary)
                .hover(move |style| style.text_color(primary))
                .on_click(move |_event, window, cx| {
                    // The row's own click would only move the cursor.
                    cx.stop_propagation();
                    on_click(id, action, window, cx);
                })
                .child(action_label(action))
        });
    div()
        .id(SharedString::from(format!("budgets-manage-row-{index}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(14.0))
        .py(px(9.0))
        .text_size(px(13.0))
        .text_color(primary)
        .when(row.archived && !selected, |this| this.opacity(0.55))
        .when(selected, |this| this.bg(color::selection_background(cx)))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .on_click(move |_event, window, cx| on_row_click(index, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(row.name),
        )
        .child(
            div()
                .w(METHOD_WIDTH)
                .flex_none()
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(upper(&super::method_label(row.method))),
        )
        .child(
            div()
                .w(PERIOD_WIDTH)
                .flex_none()
                .text_color(secondary)
                .child(crate::msg::desktop_budgets_manage_period_monthly()),
        )
        .child(
            div()
                .w(ACCOUNTS_WIDTH)
                .flex_none()
                .text_color(secondary)
                .child(row.accounts),
        )
        .child(
            div()
                .w(SCOPE_WIDTH)
                .flex_none()
                .text_color(secondary)
                .child(row.scope),
        )
        .child(
            div()
                .w(DEFAULT_WIDTH)
                .flex_none()
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(if row.is_default {
                    upper(&crate::msg::desktop_budgets_switcher_default())
                } else {
                    String::new()
                }),
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

pub fn render(props: ManageDialogProps, cx: &App) -> AnyElement {
    let ManageDialogProps {
        counts,
        rows,
        selected,
        error,
        on_row_click,
        on_action_click,
        on_new,
        on_close,
    } = props;
    let count = rows.len();
    let selected = selected.min(count.saturating_sub(1));

    let table = div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(cx))
        .child(table_header(cx))
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            table_row(
                index,
                row,
                index == selected,
                index + 1 == count,
                &on_row_click,
                &on_action_click,
                cx,
            )
        }));
    let notes = div()
        .flex()
        .gap(px(20.0))
        .child(note(
            crate::msg::desktop_budgets_manage_note_views_title(),
            crate::msg::desktop_budgets_manage_note_views(),
            cx,
        ))
        .child(note(
            crate::msg::desktop_budgets_manage_note_switching_title(),
            crate::msg::desktop_budgets_manage_note_switching(),
            cx,
        ))
        .child(note(
            crate::msg::desktop_budgets_manage_note_archive_title(),
            crate::msg::desktop_budgets_manage_note_archive(),
            cx,
        ));

    let mut fields = vec![
        div()
            .text_size(px(12.5))
            .text_color(color::muted(cx))
            .child(dialog::rich_text(
                counts,
                Some(color::foreground(cx).into()),
            ))
            .into_any_element(),
        table.into_any_element(),
    ];
    if let Some(error) = error.as_ref() {
        fields.push(error_line(
            match error {
                BudgetError::DefaultCannotBeArchived => {
                    crate::msg::desktop_budgets_manage_error_default()
                }
                other => error_text(other),
            },
            cx,
        ));
    }
    fields.push(notes.into_any_element());

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_budgets_manage_title(),
            false,
            cx,
        ))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("budgets-manage-close", on_close, cx).into_any_element(),
                dialog::confirm_button(
                    "budgets-manage-new",
                    crate::msg::desktop_budgets_switcher_new(),
                    true,
                    false,
                    on_new,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));
    dialog::overlay(WIDTH, false, card, cx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_offers_no_set_default_or_archive_and_an_archived_row_only_restores() {
        assert_eq!(
            actions(true, false),
            [
                ManageAction::Open,
                ManageAction::Edit,
                ManageAction::Duplicate
            ]
        );
        assert_eq!(
            actions(false, false),
            [
                ManageAction::Open,
                ManageAction::Edit,
                ManageAction::Duplicate,
                ManageAction::SetDefault,
                ManageAction::Archive
            ]
        );
        assert_eq!(
            actions(false, true),
            [
                ManageAction::Open,
                ManageAction::Restore,
                ManageAction::Duplicate
            ]
        );
    }
}
