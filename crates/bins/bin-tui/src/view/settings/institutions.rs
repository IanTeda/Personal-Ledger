//! Settings › Institutions (TUI-040): INSTITUTION · ACCOUNT TYPES · UNIT · ACCOUNTS, A–Z, then a
//! `selected` box naming the Institution's Accounts and the full Account type set, the chosen types
//! bright and the rest dim. Drawn by the list-page pattern ([`super::list`]).
//!
//! A wireframe: the rows are the fixture's until #805's Institutions chain moves the TUI onto
//! `lib-institutions`, so `n`, `e` and `d` show the Shell's "not yet built" flash. The types are
//! `lib_core::AccountType`, the one enum the Accounts page groups by, so the two pages cannot
//! disagree. The counts are derived from the rows on each draw.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_core::AccountType;
use lib_locale::Label as _;
use ratatui::{Frame, layout::Constraint, layout::Rect};

use super::accounts::GROUPS;
use super::list::{self, Cell, Column, Detail, Page, Row, Table, Tone};
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

pub(super) struct InstitutionsPage {
    selected: usize,
}

impl InstitutionsPage {
    pub(super) fn new() -> Self {
        Self { selected: 0 }
    }
}

/// An Account type's name as the page spells it, lower case.
fn type_name(kind: &AccountType) -> String {
    kind.label().to_lowercase()
}

fn joined(names: impl IntoIterator<Item = String>) -> String {
    names.into_iter().collect::<Vec<_>>().join(" · ")
}

impl Page for InstitutionsPage {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(step) = list::motion(key) {
            self.selected = lib_list_cursor::step(self.selected, FIXTURE.len(), step);
            return Some(Action::NoOp);
        }
        if key.modifiers != KeyModifiers::NONE {
            return None;
        }
        match key.code {
            KeyCode::Char('n') => Some(Action::FlashNotYetBuilt("institution new")),
            KeyCode::Char('e') => Some(Action::FlashNotYetBuilt("institution edit")),
            KeyCode::Char('d') => Some(Action::FlashNotYetBuilt("institution delete")),
            _ => None,
        }
    }

    fn scope_note(&self) -> String {
        msg::tui_settings_scope_institutions(i64::try_from(FIXTURE.len()).unwrap_or(i64::MAX))
    }

    fn hints(&self) -> Vec<(String, String)> {
        vec![
            ("j/k".into(), msg::tui_settings_hint_row()),
            ("n".into(), msg::tui_settings_hint_add()),
            ("e".into(), msg::tui_settings_hint_edit()),
            ("d".into(), msg::tui_settings_hint_delete()),
        ]
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
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
        list::render(frame, area, &[table], Some(&detail), &notes, focused, c);
    }
}

#[cfg(test)]
mod tests {
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
}
