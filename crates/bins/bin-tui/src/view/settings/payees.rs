//! Settings › Payees (TUI-044): NAME · CATEGORY · MATCH RULES, A–Z, with no count or spend columns,
//! then a `selected` box with all the match rules, the rename aliases (ADR-0012) and the default
//! Category. Drawn by the list-page pattern ([`super::list`]).
//!
//! A wireframe: the rows are the fixture's until #805's Payees chain moves the TUI onto
//! `lib-payees`, so `n`, `e` and `d` show the Shell's "not yet built" flash. The count is derived
//! from the rows on each draw.

use chrono::NaiveDate;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Constraint, layout::Rect};

use super::keys::{self, Binding, Press};
use super::list::{self, Cell, Column, Detail, Page, Row, Table, Tone};
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;

const NAME_WIDTH: u16 = 20;
const CATEGORY_WIDTH: u16 = 14;
/// The rules the row shows before `+n`.
const SHOWN_RULES: usize = 2;

// wireframe: TUI-044's eight Payees.
struct FixturePayee {
    name: &'static str,
    category: &'static str,
    rules: &'static [&'static str],
    /// A previous name and the day the Payee was renamed.
    alias: Option<(&'static str, (i32, u32, u32))>,
}

const fn payee(
    name: &'static str,
    category: &'static str,
    rules: &'static [&'static str],
) -> FixturePayee {
    FixturePayee {
        name,
        category,
        rules,
        alias: None,
    }
}

const FIXTURE: [FixturePayee; 8] = [
    payee("Bunnings", "Household", &["BUNNINGS*"]),
    payee("Camera House", "Household", &["CAMERA HOUSE*"]),
    payee("Coles", "Groceries", &["COLES *", "COLES EXPRESS*"]),
    payee("Dept of Education", "Salary", &["DEPT EDUCATION PAY*"]),
    payee("Netflix", "Household", &["NETFLIX.COM"]),
    payee("Rates", "Housing", &[]),
    payee("Telstra", "Utilities", &["TELSTRA*"]),
    FixturePayee {
        name: "Woolworths Metro",
        category: "Groceries",
        rules: &["WOOLWORTHS*", "WW METRO*", "WOOLIES ONLINE"],
        alias: Some(("Woolworths", (2026, 9, 2))),
    },
];

#[derive(Debug, Clone, Copy)]
enum Command {
    Row,
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
        hint: msg::tui_settings_hint_open_payees_view,
        command: Command::OpenView,
    },
];

pub(super) struct PayeesPage {
    selected: usize,
}

impl PayeesPage {
    pub(super) fn new() -> Self {
        Self { selected: 0 }
    }
}

impl Page for PayeesPage {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        Some(match keys::lookup(KEYS, key)? {
            Command::Row => {
                let step = list::motion(key)?;
                self.selected = lib_list_cursor::step(self.selected, FIXTURE.len(), step);
                Action::NoOp
            }
            Command::New => Action::FlashNotYetBuilt("payee new"),
            Command::Edit => Action::FlashNotYetBuilt("payee edit"),
            Command::Delete => Action::FlashNotYetBuilt("payee delete"),
            Command::OpenView => Action::OpenPayees,
        })
    }

    fn scope_note(&self) -> String {
        msg::tui_settings_scope_payees(i64::try_from(FIXTURE.len()).unwrap_or(i64::MAX))
    }

    fn hints(&self) -> Vec<(String, String)> {
        keys::hints(KEYS)
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let rows = FIXTURE
            .iter()
            .enumerate()
            .map(|(index, payee)| {
                let rules = if payee.rules.is_empty() {
                    Cell::muted(msg::tui_settings_payees_no_rules())
                } else {
                    let mut spans = vec![(
                        payee.rules[..payee.rules.len().min(SHOWN_RULES)].join(" · "),
                        Tone::Plain,
                    )];
                    if payee.rules.len() > SHOWN_RULES {
                        spans.push((
                            format!(" +{}", payee.rules.len() - SHOWN_RULES),
                            Tone::Muted,
                        ));
                    }
                    Cell(spans)
                };
                Row::Item {
                    cells: vec![Cell::plain(payee.name), Cell::plain(payee.category), rules],
                    selected: index == self.selected,
                    dim: false,
                }
            })
            .collect();
        let table = Table {
            heading: msg::tui_settings_sorted_a_z(),
            add: vec![("n".into(), msg::tui_settings_payees_add())],
            columns: vec![
                Column::left(
                    msg::tui_settings_payees_name(),
                    Constraint::Length(NAME_WIDTH),
                ),
                Column::left(
                    msg::tui_settings_payees_category(),
                    Constraint::Length(CATEGORY_WIDTH),
                ),
                Column::left(msg::tui_settings_payees_rules(), Constraint::Min(0)),
            ],
            rows,
        };
        let payee = &FIXTURE[self.selected];
        let mut lines = vec![(
            msg::tui_settings_payees_rules_label(),
            if payee.rules.is_empty() {
                Cell::muted(msg::tui_settings_payees_no_rules())
            } else {
                Cell::plain(payee.rules.join(" · "))
            },
        )];
        if let Some((alias, (year, month, day))) = payee.alias {
            let renamed = NaiveDate::from_ymd_opt(year, month, day)
                .map(crate::format::date)
                .unwrap_or_default();
            lines.push((
                msg::tui_settings_payees_alias_label(),
                Cell(vec![
                    (alias.to_string(), Tone::Plain),
                    (msg::tui_settings_payees_renamed(&renamed), Tone::Muted),
                ]),
            ));
        }
        lines.push((
            msg::tui_settings_payees_default_label(),
            Cell(vec![
                (payee.category.to_string(), Tone::Plain),
                (msg::tui_settings_payees_default_effect(), Tone::Muted),
            ]),
        ));
        let detail = Detail {
            title: msg::tui_settings_selected(payee.name),
            lines,
        };
        let notes = [msg::tui_settings_payees_note()];
        list::render(frame, area, &[table], Some(&detail), &notes, focused, c);
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
        view.update(&Action::OpenSettingsPage(SettingsPage::Payees));
        view
    }

    #[test]
    fn the_page_renders_tui_044_with_no_count_or_spend_columns() {
        crate::locale::init_for_tests();
        let mut view = page();
        view.handle_key(key(KeyCode::Char('G')));
        let text = render_at(&view, 96, 30);
        for needle in [
            "Settings › Payees",
            "8 payees",
            "n add payee",
            "MATCH RULES",
            "COLES * · COLES EXPRESS*",
            "no rules",
            "WOOLWORTHS* · WW METRO* +1",
            "selected · Woolworths Metro",
            "WOOLWORTHS* · WW METRO* · WOOLIES ONLINE",
            "also known as",
            "renamed",
            "filled in on import, still editable",
            "Renaming keeps the old name as an alias",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
        assert!(!text.contains("SPEND") && !text.contains("COUNT"), "{text}");
    }

    #[test]
    fn keys_flash_not_yet_built_and_o_opens_the_payees_view() {
        let mut page = PayeesPage::new();
        for (code, command) in [
            (KeyCode::Char('n'), "payee new"),
            (KeyCode::Char('e'), "payee edit"),
            (KeyCode::Char('d'), "payee delete"),
        ] {
            assert_eq!(
                page.handle_key(key(code)),
                Some(Action::FlashNotYetBuilt(command))
            );
        }
        assert_eq!(
            page.handle_key(key(KeyCode::Char('o'))),
            Some(Action::OpenPayees)
        );
    }

    #[test]
    fn the_hint_bar_is_the_keys_the_page_takes() {
        keys::tests::assert_the_hints_are_the_keys(KEYS, PayeesPage::new().hints(), |key| {
            PayeesPage::new().handle_key(key)
        });
    }
}
