//! The **3a** Accounts page (`docs/ux/desktop/Accounts/README.md`): a full-width management page,
//! not a scoped ledger -- a header row (title, count and net worth line, **+ Add account**), a 2px
//! rule, then one bordered table per account type in the fixed `accounts::GROUP_ORDER`
//! (NAME / INSTITUTION / UNIT / BALANCE / ACTIONS).
//!
//! Like `view::settings`, the page owns its scroll container directly: `Shell` scrolls a group
//! into view with `ScrollHandle::scroll_to_item`, which addresses the container's *direct*
//! children, so those children are laid out in the fixed order below (see
//! [`GROUP_CHILD_OFFSET`]). It draws no context rail beside it -- the mockup shows none.
//!
//! Every action here (row click, edit, delete, add) is a callback into `Shell`, so the keyboard
//! (`n`/`e`/`d`/`enter`) and the mouse reach the same handlers.

pub mod add_dialog;
pub mod edit_dialog;
mod select_field;

use std::rc::Rc;

use gpui::{AnyElement, App, ScrollHandle, SharedString, Window, div, prelude::*, px};

use crate::{
    accounts::{self, Account},
    settings::UnitRow,
    theme::color,
};

/// Called with an account's [`Account::id`].
pub type OnAccountClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnAddClick = Rc<dyn Fn(&mut Window, &mut App)>;
type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// The scroll container's direct children before the first group block: the header row, then the
/// 2px rule. `Shell` adds a group's position to this to scroll that block into view.
pub const GROUP_CHILD_OFFSET: usize = 2;

pub struct AccountsPageProps<'a> {
    pub accounts: &'a [Account],
    /// Settings' live units: the base Unit for the net worth line, and which units are
    /// currencies (every other kind reads as a quantity, `1,240 u`).
    pub units: &'a [UnitRow],
    /// Index into `accounts` of the selected row.
    pub selected: Option<usize>,
    pub on_add_click: OnAddClick,
    pub on_row_click: OnAccountClick,
    pub on_edit_click: OnAccountClick,
    pub on_delete_click: OnAccountClick,
}

pub fn render(
    focused: bool,
    scroll_handle: &ScrollHandle,
    props: AccountsPageProps<'_>,
) -> AnyElement {
    let groups = accounts::group_accounts(props.accounts);

    div()
        .id("accounts")
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
        .child(page_header(&props))
        .child(
            div()
                .h(px(2.0))
                .flex_none()
                .bg(color::STRUCTURAL_RULE)
                .mt(px(14.0))
                .mb(px(24.0)),
        )
        .children(
            groups
                .iter()
                .map(|group| group_block(group, focused, &props)),
        )
        .when(props.accounts.is_empty(), |this| {
            this.child(
                div()
                    .text_color(color::INK_SECONDARY)
                    .child("No accounts yet. Press n to add one."),
            )
        })
        .into_any_element()
}

fn page_header(props: &AccountsPageProps<'_>) -> impl IntoElement {
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
                        .child("Accounts"),
                )
                .child(summary_line(props)),
        )
        .child(add_button(props.on_add_click.clone()))
}

/// `7 accounts · net worth 83,995.87 aud · vas, btc held separately`: one net figure in the base
/// Unit only, then every other Unit held named rather than summed or dropped (the Desktop
/// Accounts map's net worth decision).
fn summary_line(props: &AccountsPageProps<'_>) -> impl IntoElement {
    let line = div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap(px(4.0))
        .text_size(px(11.5))
        .text_color(color::INK_TERTIARY);

    if props.accounts.is_empty() {
        return line.child("no accounts yet");
    }

    let base_unit = props
        .units
        .iter()
        .find(|unit| unit.is_base)
        .map(|unit| unit.code.as_str());
    let net = accounts::net_worth(props.accounts, base_unit);

    let mut line = line.child(net.count_text());
    if let Some(base) = &net.base_unit {
        let (negative, figure) = accounts::format_amount(&net.base_net);
        line = line.child("\u{b7} net worth").child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(if negative {
                    color::ACCENT_TEXT
                } else {
                    color::INK
                })
                .child(format!("{figure} {base}")),
        );
    }
    if let Some(held) = net.held_separately_text() {
        line = line.child(format!("\u{b7} {held}"));
    }
    line
}

/// `padding:10px 16px; background:#201e1d; color:#f3f2f2; font-weight:800`.
fn add_button(on_click: OnAddClick) -> impl IntoElement {
    div()
        .id("accounts-add")
        .cursor_pointer()
        .flex_none()
        .py(px(10.0))
        .px(px(16.0))
        .bg(color::INK)
        .text_color(color::INK_ON_DARK)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child("+ Add account")
}

/// The NAME column's floor: without one, the fixed columns beside it can squeeze it to nothing in
/// a narrow window, and the other cells shrink (and truncate) instead.
const NAME_MIN_WIDTH: gpui::Pixels = px(140.0);
const INSTITUTION_WIDTH: gpui::Pixels = px(170.0);
const UNIT_WIDTH: gpui::Pixels = px(70.0);
const BALANCE_WIDTH: gpui::Pixels = px(150.0);
const ACTIONS_WIDTH: gpui::Pixels = px(150.0);

fn group_block(
    group: &accounts::AccountGroup,
    focused: bool,
    props: &AccountsPageProps<'_>,
) -> impl IntoElement {
    let last = group.indices.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .mb(px(28.0))
        .child(
            div()
                .mb(px(14.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(16.0))
                .child(accounts::type_label(&group.account_type)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(color::BORDER)
                .child(table_header())
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
                            ))
                        }),
                ),
        )
}

/// `padding:10px 16px; background:#eae9e9; font:800 10px; color:#605d5d`. The row's own 2px
/// selection bar sits inside its 16px padding, so the header reserves the same 2px.
fn table_header() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(10.0))
        .bg(color::CHROME)
        .border_b(px(1.0))
        .border_color(color::BORDER)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_SECONDARY)
        .child(div().flex_1().min_w(NAME_MIN_WIDTH).child("NAME"))
        .child(div().w(INSTITUTION_WIDTH).child("INSTITUTION"))
        .child(div().w(UNIT_WIDTH).child("UNIT"))
        .child(
            div()
                .w(BALANCE_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child("BALANCE"),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child("ACTIONS"),
        )
}

fn row(
    account: &Account,
    last: bool,
    selected: bool,
    focused: bool,
    props: &AccountsPageProps<'_>,
) -> impl IntoElement {
    let id = account.id;
    let (negative, amount) = accounts::format_amount(&account.balance);
    let is_currency = props
        .units
        .iter()
        .find(|unit| unit.code == account.unit)
        .is_none_or(|unit| unit.kind == "currency");
    let balance = if is_currency {
        amount
    } else {
        format!("{amount} u")
    };

    let on_row_click = props.on_row_click.clone();
    let on_edit_click = props.on_edit_click.clone();
    let on_delete_click = props.on_delete_click.clone();

    div()
        .id(SharedString::from(format!("accounts-row-{id}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .pr(px(16.0))
        .py(px(10.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::HAIRLINE)
        })
        .when(selected, |this| this.bg(color::CHROME))
        .hover(|style| style.bg(color::HOVER_TINT))
        .on_click(move |_event, window, cx| on_row_click(id, window, cx))
        // The selection bar is drawn only while the page has focus (the same rule the view's own
        // left border follows) -- the tint alone still marks the row when focus is elsewhere.
        .child(
            div()
                .w(px(2.0))
                .h(px(20.0))
                .flex_none()
                .when(selected && focused, |this| this.bg(color::INK)),
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
                .truncate()
                .text_color(color::INK_SECONDARY)
                .child(account.institution.clone()),
        )
        .child(
            div()
                .w(UNIT_WIDTH)
                .text_color(color::INK_SECONDARY)
                .child(account.unit.clone()),
        )
        .child(
            div()
                .w(BALANCE_WIDTH)
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .text_color(if negative {
                    color::ACCENT_TEXT
                } else {
                    color::INK
                })
                .child(balance),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex()
                .justify_end()
                .gap(px(10.0))
                .child(row_action_button(
                    SharedString::from(format!("accounts-edit-{id}")),
                    "edit",
                    Rc::new(move |window: &mut Window, cx: &mut App| on_edit_click(id, window, cx)),
                ))
                .child(row_action_button(
                    SharedString::from(format!("accounts-delete-{id}")),
                    "delete",
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_delete_click(id, window, cx)
                    }),
                )),
        )
}

/// `padding:4px 10px; font-size:11px; border:1px solid rgba(32,30,29,.30); background:transparent`.
/// Stops the click reaching the row's own handler, which would otherwise also flash the ledger
/// stub.
fn row_action_button(
    id: SharedString,
    label: &'static str,
    on_click: OnPlainClick,
) -> impl IntoElement {
    div()
        .id(id)
        .cursor_pointer()
        .py(px(4.0))
        .px(px(10.0))
        .border_1()
        .border_color(color::BORDER)
        .text_size(px(11.0))
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
}
