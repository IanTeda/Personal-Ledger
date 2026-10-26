//! Draws the Accounts page's view model (`docs/ux/desktop-mockups/16-settings/README.md`'s 2o): a
//! kicker (BY TYPE) with **+ Add account** over one bordered table per account type
//! (NAME / INSTITUTION / UNIT / ACTIONS). Every click sends the Presenter the event its key does:
//! a row opens the Account's ledger, as `enter` does.

use std::rc::Rc;

use gpui::{App, Context, Entity, SharedString, Window, div, prelude::*, px};
use lib_accounts::presenter::{
    AccountGroupView, AccountLine, AccountsPageEvent, AccountsPageViewModel,
};
use lib_core::AccountId;
use lib_locale::format::upper;

use super::AccountsPage;
use crate::theme::color;

type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

impl Render for AccountsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let view = self.view_model(cx);
        let focused = self.page_focused;
        div()
            .flex()
            .flex_col()
            .child(kicker_row(&view, &entity, cx))
            .children(
                view.groups
                    .iter()
                    .map(|group| group_block(group, &view, focused, &entity, cx)),
            )
            .children(
                view.empty
                    .clone()
                    .map(|empty| div().text_color(color::muted(cx)).child(empty)),
            )
    }
}

/// A click handler that sends the page `event`, as the key that does the same.
fn on_event(entity: &Entity<AccountsPage>, event: AccountsPageEvent) -> OnPlainClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| {
        let event = event.clone();
        entity.update(cx, |page, cx| page.send(event, cx));
    })
}

fn kicker_row(
    view: &AccountsPageViewModel,
    entity: &Entity<AccountsPage>,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .mb(px(14.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::muted(cx))
                .child(upper(&view.kicker)),
        )
        .child(add_button(
            view,
            on_event(entity, AccountsPageEvent::New),
            cx,
        ))
}

/// `padding:8px 14px; background:#201e1d; color:#f3f2f2; font-weight:800`.
fn add_button(view: &AccountsPageViewModel, on_click: OnPlainClick, cx: &App) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .debug_selector(|| "settings-accounts-add".to_string())
        .id("settings-accounts-add")
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
        .child(view.add_button.clone())
}

const NAME_MIN_WIDTH: gpui::Pixels = px(140.0);
/// Includes [`INSTITUTION_GAP`], so the text keeps its 130px and the Unit column starts clear of it.
const INSTITUTION_WIDTH: gpui::Pixels = px(154.0);
const INSTITUTION_GAP: gpui::Pixels = px(24.0);
const UNIT_WIDTH: gpui::Pixels = px(90.0);
const ACTIONS_WIDTH: gpui::Pixels = px(150.0);

fn group_block(
    group: &AccountGroupView,
    view: &AccountsPageViewModel,
    focused: bool,
    entity: &Entity<AccountsPage>,
    cx: &App,
) -> impl IntoElement {
    let last = group.lines.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .mb(px(24.0))
        .child(
            div()
                .mb(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(14.0))
                .child(upper(&group.label)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(color::border(cx))
                .child(table_header(view, cx))
                .children(group.lines.iter().enumerate().map(|(position, line)| {
                    row(
                        line,
                        view,
                        position == last,
                        view.selected == Some(line.id),
                        focused,
                        entity,
                        cx,
                    )
                })),
        )
}

/// `padding:10px 16px; background:#eae9e9; font:800 10px; color:#605d5d`. The row's own 2px
/// selection bar sits inside its 16px padding, so the header reserves the same 2px.
fn table_header(view: &AccountsPageViewModel, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(10.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        .child(
            div()
                .flex_1()
                .min_w(NAME_MIN_WIDTH)
                .child(upper(&view.columns.name)),
        )
        .child(
            div()
                .w(INSTITUTION_WIDTH)
                .pr(INSTITUTION_GAP)
                .child(upper(&view.columns.institution)),
        )
        .child(div().w(UNIT_WIDTH).child(upper(&view.columns.unit)))
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child(upper(&view.columns.actions)),
        )
}

fn row(
    line: &AccountLine,
    view: &AccountsPageViewModel,
    last: bool,
    selected: bool,
    focused: bool,
    entity: &Entity<AccountsPage>,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    let id: AccountId = line.id;
    let on_row_click = on_event(entity, AccountsPageEvent::Open(id));

    div()
        .debug_selector(move || format!("settings-accounts-row-{id}"))
        .id(SharedString::from(format!("settings-accounts-row-{id}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .pr(px(16.0))
        .py(px(8.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected, |this| this.bg(color::chrome(cx)))
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| on_row_click(window, cx))
        // The selection bar is drawn only while the page has focus -- the tint alone still marks
        // the row when focus is on the index.
        .child(
            div()
                .w(px(2.0))
                .h(px(20.0))
                .flex_none()
                .when(selected && focused, |this| this.bg(color::foreground(cx))),
        )
        .child(
            div()
                .flex_1()
                .min_w(NAME_MIN_WIDTH)
                .pl(px(14.0))
                .truncate()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(line.name.clone()),
        )
        .child(
            div()
                .w(INSTITUTION_WIDTH)
                .pr(INSTITUTION_GAP)
                .truncate()
                .text_color(color::muted(cx))
                .child(line.institution.clone()),
        )
        .child(
            div()
                .w(UNIT_WIDTH)
                .text_color(color::muted(cx))
                .child(line.unit.clone()),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex()
                .justify_end()
                .gap(px(10.0))
                .child(row_action_button(
                    SharedString::from(format!("settings-accounts-edit-{id}")),
                    view.row_edit.clone(),
                    on_event(entity, AccountsPageEvent::EditRow(id)),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("settings-accounts-delete-{id}")),
                    view.row_delete.clone(),
                    on_event(entity, AccountsPageEvent::DeleteRow(id)),
                    cx,
                )),
        )
}

/// `padding:4px 10px; font-size:11px; border:1px solid rgba(32,30,29,.30); background:transparent`.
/// Stops the click reaching the row's own handler, which would otherwise also open the ledger.
fn row_action_button(
    id: SharedString,
    label: String,
    on_click: OnPlainClick,
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
        .py(px(4.0))
        .px(px(10.0))
        .border_1()
        .border_color(color::border(cx))
        .text_size(px(11.0))
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
}
