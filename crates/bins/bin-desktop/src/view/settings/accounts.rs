//! The **Accounts** page (`docs/ux/desktop/16-settings/README.md`'s 2o): a kicker (BY TYPE) with
//! **+ Add account** over one bordered table per account type in `accounts::GROUP_ORDER`
//! (NAME / INSTITUTION / UNIT / ACTIONS). It reuses the Accounts model and dialogs;
//! only the table is reduced to the handoff's columns and `padding:8px 16px` rows.
//!
//! Every action (row click, edit, delete, add) is a callback into `Shell`, so the keyboard
//! (`n`/`e`/`d`/`enter`) and the mouse reach the same handlers.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use lib_locale::{Label, format::upper};

use crate::{
    accounts::{self, Account},
    settings::UnitRow,
    theme::color,
};

/// Called with an account's [`Account::id`].
pub type OnAccountClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnAddClick = Rc<dyn Fn(&mut Window, &mut App)>;
type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

pub struct AccountsPageProps<'a> {
    pub accounts: &'a [Account],
    /// Settings' live units: which are currencies (every other kind reads as a quantity,
    /// `1,240 u`), and the base Unit the section meta's net worth is summed in.
    pub units: &'a [UnitRow],
    /// Index into `accounts` of the selected row.
    pub selected: Option<usize>,
    pub on_add_click: OnAddClick,
    pub on_row_click: OnAccountClick,
    pub on_edit_click: OnAccountClick,
    pub on_delete_click: OnAccountClick,
}

/// The section heading's meta: `7 accounts · net worth 83,995.87 aud · vas, btc held
/// separately`. One net figure in the base Unit only, then every other Unit held named rather
/// than summed or dropped.
pub fn scope_note(accounts: &[Account], units: &[UnitRow]) -> String {
    if accounts.is_empty() {
        return crate::msg::desktop_accounts_summary_empty();
    }
    let base_unit = units
        .iter()
        .find(|unit| unit.is_base)
        .map(|unit| unit.code.as_str());
    let net = accounts::net_worth(accounts, base_unit);

    let mut parts = vec![net.count_text()];
    if let Some(base) = &net.base_unit {
        let (negative, figure) = crate::view::format::amount(&net.base_net);
        let sign = if negative { "-" } else { "" };
        parts.push(format!(
            "{} {sign}{figure} {base}",
            crate::msg::desktop_accounts_net_worth()
        ));
    }
    parts.extend(net.held_separately_text());
    parts.join(" \u{b7} ")
}

pub fn render(props: &AccountsPageProps<'_>, focused: bool, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(kicker_row(props.on_add_click.clone(), cx))
        .children(
            accounts::group_accounts(props.accounts)
                .iter()
                .map(|group| group_block(group, focused, props, cx)),
        )
        .when(props.accounts.is_empty(), |this| {
            this.child(
                div()
                    .text_color(color::muted(cx))
                    .child(crate::msg::desktop_accounts_empty("n")),
            )
        })
        .into_any_element()
}

fn kicker_row(on_add_click: OnAddClick, cx: &App) -> impl IntoElement {
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
        .child(add_button(on_add_click, cx))
}

/// `padding:8px 14px; background:#201e1d; color:#f3f2f2; font-weight:800`.
fn add_button(on_click: OnAddClick, cx: &App) -> impl IntoElement {
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
    focused: bool,
    props: &AccountsPageProps<'_>,
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
                            let account = props.accounts.get(index)?;
                            Some(row(
                                account,
                                position == last,
                                props.selected == Some(index),
                                focused,
                                props,
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
    last: bool,
    selected: bool,
    focused: bool,
    props: &AccountsPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    let id = account.id;
    let on_row_click = props.on_row_click.clone();
    let on_edit_click = props.on_edit_click.clone();
    let on_delete_click = props.on_delete_click.clone();

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
        .on_click(move |_event, window, cx| on_row_click(id, window, cx))
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
                .child(accounts::institution_label(&account.institution)),
        )
        .child(
            div()
                .w(UNIT_WIDTH)
                .text_color(color::muted(cx))
                .child(account.unit.clone()),
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
                    Rc::new(move |window: &mut Window, cx: &mut App| on_edit_click(id, window, cx)),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("settings-accounts-delete-{id}")),
                    crate::msg::desktop_accounts_row_delete(),
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_delete_click(id, window, cx)
                    }),
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
        let note = scope_note(
            &accounts::default_accounts(),
            &crate::settings::default_units(),
        );
        assert_eq!(
            note,
            "7 accounts \u{b7} net worth 83,995.87 aud \u{b7} vas, btc held separately"
        );
    }

    #[test]
    fn scope_note_with_no_accounts_says_so() {
        crate::locale::init_for_tests();
        assert_eq!(
            scope_note(&[], &crate::settings::default_units()),
            "no accounts yet"
        );
    }
}
