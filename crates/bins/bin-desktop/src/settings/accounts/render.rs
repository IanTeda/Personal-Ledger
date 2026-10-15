//! The **Accounts** page (`docs/ux/desktop-mockups/16-settings/README.md`'s 2o): a kicker (BY TYPE) with
//! **+ Add account** over one bordered table per account type in `accounts::GROUP_ORDER`
//! (NAME / INSTITUTION / UNIT / ACTIONS). It reuses the Accounts model and dialogs;
//! only the table is reduced to the handoff's columns and `padding:8px 16px` rows.
//!
//! [`AccountsPage`] draws it, and every click is bound to the same handler its key reaches
//! (`n`/`e`/`d`/`enter`).

use std::rc::Rc;

use gpui::{App, Context, Entity, SharedString, Window, div, prelude::*, px};

use lib_accounts::{self as accounts, Account};
use lib_locale::{Label, format::upper};

use super::{AccountsPage, count_text, held_separately_text, institution_label};
use crate::theme::color;

type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// The section heading's meta: `7 accounts · net worth 83,995.87 aud · vas, btc held
/// separately`. One net figure in the base Unit only, then every other Unit held named rather
/// than summed or dropped.
pub fn scope_note(accounts: &[Account], units: &[lib_units::UnitRow]) -> String {
    if accounts.is_empty() {
        return crate::msg::desktop_accounts_summary_empty();
    }
    let base_unit = units.iter().find(|unit| unit.is_base).map(|unit| unit.id);
    let net = accounts::net_worth(accounts, base_unit);

    let mut parts = vec![count_text(&net)];
    if let Some(base) = net
        .base_unit
        .and_then(|id| units.iter().find(|unit| unit.id == id))
    {
        let base = &base.code;
        let (negative, figure) = crate::view::format::amount(&net.base_net);
        let sign = if negative { "-" } else { "" };
        parts.push(format!(
            "{} {sign}{figure} {base}",
            crate::msg::desktop_accounts_net_worth()
        ));
    }
    parts.extend(held_separately_text(&net, units));
    parts.join(" \u{b7} ")
}

impl Render for AccountsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let accounts = self.accounts(cx);
        let units = self.ledger.read(cx).ledger().units();
        let selected = self.selected_index(cx);
        let focused = self.page_focused;
        div()
            .flex()
            .flex_col()
            .child(kicker_row(&entity, cx))
            .children(
                accounts::group_accounts(accounts).iter().map(|group| {
                    group_block(group, accounts, units, selected, focused, &entity, cx)
                }),
            )
            .when(accounts.is_empty(), |this| {
                this.child(
                    div()
                        .text_color(color::muted(cx))
                        .child(crate::msg::desktop_accounts_empty("n")),
                )
            })
    }
}

impl AccountsPage {
    /// The section heading's meta, over the page's live rows and Settings' Units.
    pub(crate) fn scope_note(&self, cx: &App) -> String {
        scope_note(self.accounts(cx), self.ledger.read(cx).ledger().units())
    }
}

/// A click handler that runs `act` on the page with the clicked account's id.
fn on_account(
    entity: &Entity<AccountsPage>,
    id: u32,
    act: fn(&mut AccountsPage, u32, &mut Context<'_, AccountsPage>),
) -> OnPlainClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| entity.update(cx, |view, cx| act(view, id, cx)))
}

fn kicker_row(entity: &Entity<AccountsPage>, cx: &App) -> impl IntoElement {
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
                .child(upper(&crate::msg::desktop_settings_accounts_kicker())),
        )
        .child(add_button(entity.clone(), cx))
}

/// `padding:8px 14px; background:#201e1d; color:#f3f2f2; font-weight:800`.
fn add_button(entity: Entity<AccountsPage>, cx: &App) -> impl IntoElement {
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
        .on_click(move |_event, _window, cx| entity.update(cx, AccountsPage::click_add))
        .child(crate::msg::desktop_accounts_add_button("+"))
}

/// The NAME column's floor: without one, the fixed columns beside it can squeeze it to nothing
/// in a narrow window, and the other cells shrink (and truncate) instead.
const NAME_MIN_WIDTH: gpui::Pixels = px(140.0);
/// Includes [`INSTITUTION_GAP`], so the text keeps its 130px and the Unit column starts clear of it.
const INSTITUTION_WIDTH: gpui::Pixels = px(154.0);
const INSTITUTION_GAP: gpui::Pixels = px(24.0);
const UNIT_WIDTH: gpui::Pixels = px(90.0);
const ACTIONS_WIDTH: gpui::Pixels = px(150.0);

fn group_block(
    group: &accounts::AccountGroup,
    accounts: &[Account],
    units: &[lib_units::UnitRow],
    selected: Option<usize>,
    focused: bool,
    entity: &Entity<AccountsPage>,
    cx: &App,
) -> impl IntoElement {
    let last = group.indices.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .mb(px(24.0))
        .child(
            div()
                .mb(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(14.0))
                .child(upper(&group.account_type.label())),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(color::border(cx))
                .child(table_header(cx))
                .children(
                    group
                        .indices
                        .iter()
                        .enumerate()
                        .filter_map(|(position, &index)| {
                            let account = accounts.get(index)?;
                            Some(row(
                                account,
                                units,
                                position == last,
                                selected == Some(index),
                                focused,
                                entity,
                                cx,
                            ))
                        }),
                ),
        )
}

/// `padding:10px 16px; background:#eae9e9; font:800 10px; color:#605d5d`. The row's own 2px
/// selection bar sits inside its 16px padding, so the header reserves the same 2px.
fn table_header(cx: &App) -> impl IntoElement {
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
                .child(upper(&lib_locale::msg::column_name())),
        )
        .child(
            div()
                .w(INSTITUTION_WIDTH)
                .pr(INSTITUTION_GAP)
                .child(upper(&lib_locale::msg::column_institution())),
        )
        .child(
            div()
                .w(UNIT_WIDTH)
                .child(upper(&lib_locale::msg::column_unit())),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child(upper(&lib_locale::msg::column_actions())),
        )
}

fn row(
    account: &Account,
    units: &[lib_units::UnitRow],
    last: bool,
    selected: bool,
    focused: bool,
    entity: &Entity<AccountsPage>,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    let id = account.id;
    let on_row_click = on_account(entity, id, AccountsPage::click_row);

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
                .child(account.name.clone()),
        )
        .child(
            div()
                .w(INSTITUTION_WIDTH)
                .pr(INSTITUTION_GAP)
                .truncate()
                .text_color(color::muted(cx))
                .child(institution_label(&account.institution)),
        )
        .child(
            div().w(UNIT_WIDTH).text_color(color::muted(cx)).child(
                units
                    .iter()
                    .find(|unit| unit.id == account.unit)
                    .map_or_else(String::new, |unit| unit.code.clone()),
            ),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex()
                .justify_end()
                .gap(px(10.0))
                .child(row_action_button(
                    SharedString::from(format!("settings-accounts-edit-{id}")),
                    crate::msg::desktop_accounts_row_edit(),
                    on_account(entity, id, AccountsPage::click_edit),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("settings-accounts-delete-{id}")),
                    crate::msg::desktop_accounts_row_delete(),
                    on_account(entity, id, AccountsPage::click_delete),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_note_reads_count_net_worth_and_held_units() {
        crate::locale::init_for_tests();
        let note = scope_note(&accounts::default_accounts(), &lib_units::default_units());
        assert_eq!(
            note,
            "7 accounts \u{b7} net worth 83,995.87 aud \u{b7} vas, btc held separately"
        );
    }

    #[test]
    fn scope_note_with_no_accounts_says_so() {
        crate::locale::init_for_tests();
        assert_eq!(
            scope_note(&[], &lib_units::default_units()),
            "no accounts yet"
        );
    }
}
