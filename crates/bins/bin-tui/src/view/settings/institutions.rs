//! Settings › Institutions (TUI-040): INSTITUTION · ACCOUNT TYPES · UNIT · ACCOUNTS, A–Z, then a
//! `selected` box naming the Institution's Accounts and the full Account type set, the chosen types
//! bright and the rest dim. Drawn by the list-page pattern ([`super::list`]).
//!
//! A wireframe: the rows are the fixture's until #805's Institutions chain moves the TUI onto
//! `lib-institutions`, so `n`, `e` and `d` show the Shell's "not yet built" flash. The types are
//! `lib_core::AccountType`, the one enum the Accounts page groups by, so the two pages cannot
//! disagree. The counts are derived from the rows on each draw.
//!
//! At X-Large the page docks its detail pane (DESKTOP-141, [`super::pane`]) in place of the
//! `selected` box: INSTITUTION, its ACCOUNTS with their balances read from the Accounts page's
//! fixture, and ACTIVITY.

use crossterm::event::KeyEvent;
use lib_core::AccountType;
use lib_locale::Label as _;
use ratatui::{Frame, layout::Constraint, layout::Rect};

use super::accounts::GROUPS;
use super::accounts::fixture::{self as accounts, BASE_UNIT};
use super::keys::{self, Binding, Press};
use super::list::{self, Cell, Column, Detail, Page, Row, Table, Tone};
use super::pane::{Group, Line, Pane};
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;

const TYPES_WIDTH: u16 = 26;
const UNIT_WIDTH: u16 = 6;
const ACCOUNTS_WIDTH: u16 = 10;

// wireframe: TUI-040's six Institutions.
struct FixtureInstitution {
    name: &'static str,
    types: &'static [AccountType],
    unit: &'static str,
    accounts: &'static [&'static str],
}

const FIXTURE: [FixtureInstitution; 6] = [
    FixtureInstitution {
        name: "American Express",
        types: &[AccountType::CreditCard],
        unit: "aud",
        accounts: &["Amex Platinum"],
    },
    FixtureInstitution {
        name: "ANZ",
        types: &[AccountType::Bank, AccountType::Loan],
        unit: "aud",
        accounts: &["ANZ Everyday", "ANZ Offset", "Home Loan"],
    },
    FixtureInstitution {
        name: "AustralianSuper",
        types: &[AccountType::Investment],
        unit: "aud",
        accounts: &[],
    },
    FixtureInstitution {
        name: "Crypto.com",
        types: &[AccountType::Investment],
        unit: "btc",
        accounts: &["Bitcoin"],
    },
    FixtureInstitution {
        name: "Vanguard",
        types: &[AccountType::Investment],
        unit: "vas",
        accounts: &["Vanguard VAS"],
    },
    FixtureInstitution {
        name: "Westpac",
        types: &[AccountType::Bank],
        unit: "aud",
        accounts: &["Westpac Savings"],
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Row,
    New,
    Edit,
    Delete,
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
];

pub(super) struct InstitutionsPage {
    selected: usize,
}

impl InstitutionsPage {
    pub(super) fn new() -> Self {
        Self { selected: 0 }
    }
}

/// An Account type's name as the page spells it, lower case.
pub(super) fn type_name(kind: &AccountType) -> String {
    kind.label().to_lowercase()
}

fn joined(names: impl IntoIterator<Item = String>) -> String {
    names.into_iter().collect::<Vec<_>>().join(" · ")
}

impl Page for InstitutionsPage {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        Some(match keys::lookup(KEYS, key)? {
            Command::Row => {
                let step = list::motion(key)?;
                self.selected = lib_list_cursor::step(self.selected, FIXTURE.len(), step);
                Action::NoOp
            }
            Command::New => Action::FlashNotYetBuilt("institution new"),
            Command::Edit => Action::FlashNotYetBuilt("institution edit"),
            Command::Delete => Action::FlashNotYetBuilt("institution delete"),
        })
    }

    fn scope_note(&self) -> String {
        msg::tui_settings_scope_institutions(i64::try_from(FIXTURE.len()).unwrap_or(i64::MAX))
    }

    fn hints(&self) -> Vec<(String, String)> {
        keys::hints(KEYS)
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, pane: bool, c: &Colours) {
        let rows = FIXTURE
            .iter()
            .enumerate()
            .map(|(index, institution)| Row::Item {
                cells: vec![
                    Cell::plain(institution.name),
                    Cell::plain(joined(institution.types.iter().map(type_name))),
                    Cell::plain(institution.unit),
                    Cell::plain(institution.accounts.len().to_string()),
                ],
                selected: index == self.selected,
                dim: false,
            })
            .collect();
        let table = Table {
            heading: msg::tui_settings_sorted_a_z(),
            add: vec![("n".into(), msg::tui_settings_institutions_add())],
            columns: vec![
                Column::left(msg::tui_settings_institutions_name(), Constraint::Min(0)),
                Column::left(
                    msg::tui_settings_institutions_types(),
                    Constraint::Length(TYPES_WIDTH),
                ),
                Column::left(
                    msg::tui_settings_institutions_unit(),
                    Constraint::Length(UNIT_WIDTH),
                ),
                Column::right(
                    msg::tui_settings_institutions_accounts(),
                    Constraint::Length(ACCOUNTS_WIDTH),
                ),
            ],
            rows,
        };
        let institution = &FIXTURE[self.selected];
        let accounts = if institution.accounts.is_empty() {
            Cell::muted(msg::tui_settings_institutions_no_accounts())
        } else {
            Cell::plain(institution.accounts.join(" · "))
        };
        let mut types = Vec::new();
        for kind in GROUPS {
            if !types.is_empty() {
                types.push((" · ".to_string(), Tone::Muted));
            }
            let tone = if institution.types.contains(&kind) {
                Tone::Plain
            } else {
                Tone::Muted
            };
            types.push((type_name(&kind), tone));
        }
        let detail = Detail {
            title: msg::tui_settings_institutions_selected(institution.name),
            lines: vec![
                (msg::tui_settings_institutions_accounts_label(), accounts),
                (msg::tui_settings_institutions_types_label(), Cell(types)),
                (
                    msg::tui_settings_institutions_unit_label(),
                    Cell(vec![
                        (institution.unit.to_string(), Tone::Plain),
                        (
                            msg::tui_settings_institutions_unit_effect(institution.unit),
                            Tone::Muted,
                        ),
                    ]),
                ),
            ],
        };
        let notes = [msg::tui_settings_institutions_note_delete()];
        let detail = (!pane).then_some(&detail);
        list::render(frame, area, &[table], detail, &notes, focused, c);
    }

    fn pane(&self) -> Pane {
        let institution = &FIXTURE[self.selected];
        let count = i64::try_from(institution.accounts.len()).unwrap_or(i64::MAX);
        let types = joined(institution.types.iter().map(type_name));
        let about = Group::new(msg::tui_settings_pane_institution())
            .title(institution.name)
            .meta(joined([
                types.clone(),
                msg::tui_settings_pane_default_unit(institution.unit),
            ]))
            .fact(msg::tui_settings_institutions_types_label(), types)
            .fact(
                msg::tui_settings_institutions_unit_label(),
                institution.unit,
            )
            .fact(
                msg::tui_settings_pane_used_by(),
                msg::tui_settings_pane_account_count(count),
            )
            .keys(keys::named(KEYS, &[Command::Edit, Command::Delete]));

        let held: Vec<_> = accounts::accounts()
            .into_iter()
            .filter(|account| institution.accounts.contains(&account.name))
            .collect();
        let mut group = Group::new(msg::tui_settings_pane_accounts());
        for account in &held {
            group = group
                .line(Line::Split(
                    Cell::plain(account.name),
                    Cell::plain(msg::tui_settings_pane_amount(
                        &crate::format::money(&account.balance, account.places),
                        account.unit,
                    )),
                ))
                .line(Line::Text(Cell::muted(type_name(&account.kind))));
        }
        let group = if held.is_empty() {
            group.note(msg::tui_settings_institutions_no_accounts())
        } else {
            let total = held.iter().fold(
                lib_core::Money(bigdecimal::BigDecimal::default()),
                |total, account| {
                    lib_core::Money(total.0 + &account.balance.0 * &account.base_price.0)
                },
            );
            group.line(Line::Split(
                Cell::tone(msg::tui_settings_pane_total(), Tone::Bold),
                Cell::tone(
                    msg::tui_settings_pane_amount(&crate::format::money(&total, 2), BASE_UNIT),
                    Tone::Bold,
                ),
            ))
        };
        Pane::with_activity(vec![about, group])
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;
    use crate::view::View;
    use crate::view::settings::tests::render_at;
    use crate::view::settings::{SettingsPage, SettingsView};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn page() -> SettingsView {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Institutions));
        view
    }

    #[test]
    fn the_page_renders_tui_040_at_the_floor() {
        crate::locale::init_for_tests();
        let text = render_at(&page(), 96, 30);
        for needle in [
            "Settings › Institutions",
            "6 institutions",
            "n add institution",
            "ACCOUNT TYPES",
            "American Express",
            "AustralianSuper",
            "bank · loan",
            "selected · American Express",
            "Amex Platinum",
            "credit card",
            "Deleting an institution that still holds accounts",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
    }

    #[test]
    fn j_selects_the_next_institution_and_its_accounts_follow() {
        crate::locale::init_for_tests();
        let mut view = page();
        view.handle_key(key(KeyCode::Char('j')));
        let text = render_at(&view, 96, 30);
        assert!(text.contains("selected · ANZ"), "{text}");
        assert!(
            text.contains("ANZ Everyday · ANZ Offset · Home Loan"),
            "{text}"
        );
    }

    #[test]
    fn n_e_and_d_flash_not_yet_built() {
        let mut page = InstitutionsPage::new();
        for (code, command) in [
            (KeyCode::Char('n'), "institution new"),
            (KeyCode::Char('e'), "institution edit"),
            (KeyCode::Char('d'), "institution delete"),
        ] {
            assert_eq!(
                page.handle_key(key(code)),
                Some(Action::FlashNotYetBuilt(command))
            );
        }
    }

    #[test]
    fn the_hint_bar_is_the_keys_the_page_takes() {
        keys::tests::assert_the_hints_are_the_keys(KEYS, InstitutionsPage::new().hints(), |key| {
            InstitutionsPage::new().handle_key(key)
        });
    }
}
