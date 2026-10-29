//! Settings › Accounts (TUI-041): the Accounts grouped by type, each group under a dim
//! `type · count` label, NAME · INSTITUTION · UNIT · BALANCE, negatives in the accent, and net
//! worth in the heading's note. Drawn by the list-page pattern ([`super::list`]).
//!
//! A wireframe: the rows are [`fixture`]'s until #805's Accounts chain moves the TUI onto
//! `lib-accounts`, so `n`, `e`, `d` and `enter` show the Shell's "not yet built" flash. `o` opens
//! the Accounts View, which `Shell` pushes Settings under, so `esc` there comes back to this
//! page and row. The counts and net worth are derived from the rows on each draw.
//!
//! At X-Large the page docks its detail pane (TUI-056, DESKTOP-142, [`super::pane`]): ACCOUNT,
//! RECENT TRANSACTIONS and ACTIVITY for the selected Account, drawn from the same fixture.

pub(super) mod fixture;

use crossterm::event::{KeyCode, KeyEvent};
use lib_core::{AccountType, Money};
use lib_locale::Label as _;
use ratatui::{Frame, layout::Constraint, layout::Rect};

use super::institutions::type_name;
use super::keys::{self, Binding, Press};
use super::list::{self, Cell, Column, Row, Table, Tone};
use super::pane::{Group, Line, Pane};
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;
use fixture::{BASE_UNIT, FixtureAccount};

/// The groups, in the page's order. A type no Account has draws no group.
pub(super) const GROUPS: [AccountType; 5] = [
    AccountType::Bank,
    AccountType::Cash,
    AccountType::CreditCard,
    AccountType::Loan,
    AccountType::Investment,
];

/// Widths of the INSTITUTION, UNIT and BALANCE columns; NAME takes the rest.
const INSTITUTION_WIDTH: u16 = 18;
const UNIT_WIDTH: u16 = 6;
const BALANCE_WIDTH: u16 = 15;

/// How many transactions the pane's RECENT TRANSACTIONS lists, as DESKTOP-142 does.
const RECENT: usize = 4;

/// Width of a recent transaction's day column in the pane.
const DAY_WIDTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Row,
    Ledger,
    New,
    Edit,
    Delete,
    OpenView,
}

/// The page's keys and its hint bar, one table (#898).
const KEYS: &[Binding<Command>] = &[
    Binding {
        keys: keys::ROW_KEYS,
        label: "j/k",
        hint: msg::tui_settings_hint_row,
        command: Command::Row,
    },
    Binding {
        keys: &[Press::key(KeyCode::Enter)],
        label: "enter",
        hint: msg::tui_settings_hint_open_ledger,
        command: Command::Ledger,
    },
    Binding {
        keys: &[Press::char('n')],
        label: "n",
        hint: msg::tui_settings_hint_add,
        command: Command::New,
    },
    Binding {
        keys: &[Press::char('e')],
        label: "e",
        hint: msg::tui_settings_hint_edit,
        command: Command::Edit,
    },
    Binding {
        keys: &[Press::char('d')],
        label: "d",
        hint: msg::tui_settings_hint_delete,
        command: Command::Delete,
    },
    Binding {
        keys: &[Press::char('o')],
        label: "o",
        hint: msg::tui_settings_hint_open_accounts_view,
        command: Command::OpenView,
    },
];

pub(super) struct AccountsPage {
    /// The Accounts in the page's order: by group, then the Ledger's.
    accounts: Vec<FixtureAccount>,
    selected: usize,
}

impl AccountsPage {
    pub(super) fn new() -> Self {
        let mut accounts = fixture::accounts();
        accounts.sort_by_key(|account| GROUPS.iter().position(|kind| *kind == account.kind));
        Self {
            accounts,
            selected: 0,
        }
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        Some(match keys::lookup(KEYS, key)? {
            Command::Row => {
                let step = list::motion(key)?;
                self.selected = lib_list_cursor::step(self.selected, self.accounts.len(), step);
                Action::NoOp
            }
            Command::Ledger => Action::FlashNotYetBuilt("account ledger"),
            Command::New => Action::FlashNotYetBuilt("account new"),
            Command::Edit => Action::FlashNotYetBuilt("account edit"),
            Command::Delete => Action::FlashNotYetBuilt("account delete"),
            Command::OpenView => Action::OpenAccounts,
        })
    }

    /// The heading's note: the count and net worth, every balance in the base Unit.
    pub(super) fn scope_note(&self) -> String {
        let net_worth = self.accounts.iter().fold(
            Money(bigdecimal::BigDecimal::default()),
            |total, account| Money(total.0 + &account.balance.0 * &account.base_price.0),
        );
        msg::tui_settings_scope_accounts(
            i64::try_from(self.accounts.len()).unwrap_or(i64::MAX),
            &crate::format::money(&net_worth, 2),
            BASE_UNIT,
        )
    }

    pub(super) fn hints() -> Vec<(String, String)> {
        keys::hints(KEYS)
    }

    pub(super) fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let notes = [
            msg::tui_settings_accounts_note_form(),
            msg::tui_settings_accounts_note_net_worth(BASE_UNIT),
        ];
        list::render(frame, area, &[self.table()], None, &notes, focused, c);
    }

    /// The detail pane for the selected Account: what it is and its keys, then its latest
    /// transactions, then ACTIVITY.
    pub(super) fn pane(&self) -> Pane {
        let Some(account) = self.accounts.get(self.selected) else {
            return Pane::with_activity(Vec::new());
        };
        let mut about = Group::new(msg::tui_settings_pane_account())
            .title(account.name)
            .meta(
                [
                    type_name(&account.kind),
                    account.institution.to_string(),
                    account.unit.to_string(),
                ]
                .join(" · "),
            )
            .fact(
                msg::tui_settings_pane_balance(),
                Cell::tone(
                    msg::tui_settings_pane_amount(
                        &crate::format::money(&account.balance, account.places),
                        account.unit,
                    ),
                    tone(&account.balance),
                ),
            );
        if let Some(offsets) = account.offsets {
            about = about.fact(msg::tui_settings_pane_offsets(), offsets);
        }
        about = about.fact(
            msg::tui_settings_pane_transactions(),
            crate::format::count(account.transactions),
        );
        if let Some(last) = account.recent.first().and_then(|r| day(r.on)) {
            about = about.fact(
                msg::tui_settings_pane_last_transaction(),
                crate::format::date(last),
            );
        }
        let about = about.keys(keys::named(
            KEYS,
            &[Command::Ledger, Command::Edit, Command::Delete],
        ));

        let mut recent = Group::new(msg::tui_settings_pane_recent_transactions());
        for transaction in account.recent.iter().take(RECENT) {
            let on = day(transaction.on)
                .map(crate::format::day_month)
                .unwrap_or_default();
            let amount = Money(bigdecimal::BigDecimal::new(
                transaction.minor.into(),
                account.places,
            ));
            let mut text = crate::format::money(&amount, account.places);
            if transaction.minor > 0 {
                text.insert(0, '+');
            }
            recent = recent.line(Line::Split(
                Cell(vec![
                    (format!("{on:<DAY_WIDTH$}"), Tone::Muted),
                    (transaction.what.to_string(), Tone::Plain),
                ]),
                Cell::tone(text, tone(&amount)),
            ));
        }
        let recent = if account.recent.is_empty() {
            recent.note(msg::tui_settings_pane_no_transactions())
        } else {
            let key = keys::named(KEYS, &[Command::Ledger])
                .into_iter()
                .next()
                .map(|(key, _)| key)
                .unwrap_or_default();
            recent.note(msg::tui_settings_pane_recent_note(&key))
        };
        Pane::with_activity(vec![about, recent])
    }

    fn table(&self) -> Table {
        let mut rows = Vec::new();
        for kind in GROUPS {
            let count = self.accounts.iter().filter(|a| a.kind == kind).count();
            if count == 0 {
                continue;
            }
            rows.push(Row::Group(msg::tui_settings_list_group(
                &kind.label(),
                &crate::format::count(i64::try_from(count).unwrap_or(i64::MAX)),
            )));
            for (index, account) in self.accounts.iter().enumerate() {
                if account.kind == kind {
                    rows.push(Row::Item {
                        cells: cells(account),
                        selected: index == self.selected,
                        dim: false,
                    });
                }
            }
        }
        Table {
            heading: msg::tui_settings_accounts_heading(),
            add: vec![("n".into(), msg::tui_settings_accounts_add())],
            columns: vec![
                Column::left(msg::tui_settings_accounts_name(), Constraint::Min(0)),
                Column::left(
                    msg::tui_settings_accounts_institution(),
                    Constraint::Length(INSTITUTION_WIDTH),
                ),
                Column::left(
                    msg::tui_settings_accounts_unit(),
                    Constraint::Length(UNIT_WIDTH),
                ),
                Column::right(
                    msg::tui_settings_accounts_balance(),
                    Constraint::Length(BALANCE_WIDTH),
                ),
            ],
            rows,
        }
    }
}

/// A fixture day, `None` if it isn't one.
fn day((year, month, day): (i32, u32, u32)) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::from_ymd_opt(year, month, day)
}

/// Negative amounts in the accent, as the table draws them.
fn tone(amount: &Money) -> Tone {
    if amount.0 < bigdecimal::BigDecimal::default() {
        Tone::Negative
    } else {
        Tone::Plain
    }
}

/// An Account's cells. A base Unit balance shows as money, any other as a quantity of its Unit.
fn cells(account: &FixtureAccount) -> Vec<Cell> {
    let amount = crate::format::money(&account.balance, account.places);
    let balance = if account.unit == BASE_UNIT {
        amount
    } else {
        msg::tui_settings_accounts_quantity(&amount)
    };
    let negative = account.balance.0 < bigdecimal::BigDecimal::default();
    vec![
        Cell::plain(account.name),
        Cell::plain(account.institution),
        Cell::plain(account.unit),
        Cell::tone(
            balance,
            if negative {
                Tone::Negative
            } else {
                Tone::Plain
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;
    use crate::view::View;
    use crate::view::settings::tests::render_at;
    use crate::view::settings::{SettingsPage, SettingsView};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn accounts_page() -> SettingsView {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Accounts));
        view
    }

    #[test]
    fn the_page_groups_accounts_by_type_with_net_worth_in_the_heading() {
        crate::locale::init_for_tests();
        // TUI-041 at the floor and at Medium's reference frame.
        for (width, height) in [(96, 30), (128, 69)] {
            let text = render_at(&accounts_page(), width, height);
            for needle in [
                "Settings › Accounts",
                "7 accounts · net worth \u{2212}113,943.13 aud",
                "BY TYPE",
                "n add account",
                "INSTITUTION",
                "BANK · 3",
                "CREDIT CARD · 1",
                "LOAN · 1",
                "INVESTMENT · 2",
                "ANZ Everyday",
                "American Express",
                "\u{2212}381,311.34",
                "1,240 u",
                "0.4120 u",
                "Unit balances convert to aud",
            ] {
                assert!(
                    text.contains(needle),
                    "{needle} missing at {width}:\n{text}"
                );
            }
        }
    }

    #[test]
    fn its_keys_flash_not_yet_built_and_o_opens_the_accounts_view() {
        let mut view = accounts_page();
        for (code, command) in [
            (KeyCode::Char('n'), "account new"),
            (KeyCode::Char('e'), "account edit"),
            (KeyCode::Char('d'), "account delete"),
            (KeyCode::Enter, "account ledger"),
        ] {
            assert_eq!(
                view.handle_key(key(code)),
                Some(Action::FlashNotYetBuilt(command))
            );
        }
        assert_eq!(
            view.handle_key(key(KeyCode::Char('o'))),
            Some(Action::OpenAccounts)
        );
    }

    #[test]
    fn j_and_k_walk_the_accounts_across_their_groups() {
        let mut page = AccountsPage::new();
        page.handle_key(key(KeyCode::Char('j')));
        page.handle_key(key(KeyCode::Char('j')));
        page.handle_key(key(KeyCode::Char('j')));
        assert_eq!(page.accounts[page.selected].name, "Amex Platinum");
        page.handle_key(key(KeyCode::Char('G')));
        assert_eq!(page.accounts[page.selected].name, "Bitcoin");
        page.handle_key(key(KeyCode::Char('k')));
        assert_eq!(page.accounts[page.selected].name, "Vanguard VAS");
    }

    #[test]
    fn the_hint_bar_is_the_keys_the_page_takes() {
        keys::tests::assert_the_hints_are_the_keys(KEYS, AccountsPage::hints(), |key| {
            AccountsPage::new().handle_key(key)
        });
    }
}
