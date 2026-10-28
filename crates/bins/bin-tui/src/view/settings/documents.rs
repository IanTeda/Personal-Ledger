//! Settings › Documents (TUI-045): the Document types in the Documents type filter's order
//! (ADR-0031), TYPE · TRACKS · REMIND · TAX YEAR · FILES, with four dim notes under the table.
//! `J`/`K` move the selected type. Drawn by the list-page pattern ([`super::list`]).
//!
//! A wireframe: the types are the fixture's until #805's Documents chain moves the TUI onto
//! `lib-documents`, so `n`, `e` and `x` show the Shell's "not yet built" flash; reordering is real
//! but lasts only while the page does. The counts are derived from the rows on each draw.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, layout::Constraint, layout::Rect};

use super::list::{self, Cell, Column, Page, Row, Table};
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;

const TRACKS_WIDTH: u16 = 8;
const REMIND_WIDTH: u16 = 18;
const TAX_YEAR_WIDTH: u16 = 8;
const FILES_WIDTH: u16 = 6;

/// The date a Document type reads off its files.
#[derive(Debug, Clone, Copy)]
enum Tracks {
    Renews,
    Ends,
    Expires,
}

/// How far ahead of that date the Event is raised.
#[derive(Debug, Clone, Copy)]
enum Lead {
    Days(i64),
    Months(i64),
}

// wireframe: TUI-045's seven Document types and the files without one.
#[derive(Debug, Clone)]
struct FixtureType {
    name: &'static str,
    tracks: Option<(Tracks, Lead)>,
    tax_year: bool,
    files: i64,
}

const UNTYPED: i64 = 20;

fn fixture() -> Vec<FixtureType> {
    let plain = |name, tax_year, files| FixtureType {
        name,
        tracks: None,
        tax_year,
        files,
    };
    let tracked = |name, tracks, lead, files| FixtureType {
        name,
        tracks: Some((tracks, lead)),
        tax_year: false,
        files,
    };
    vec![
        plain("Receipts", true, 186),
        plain("Statements", true, 94),
        plain("Tax", true, 31),
        tracked("Insurance", Tracks::Renews, Lead::Days(30), 18),
        tracked("Warranties & manuals", Tracks::Ends, Lead::Days(30), 42),
        tracked("Contracts", Tracks::Ends, Lead::Days(60), 9),
        tracked("Identity", Tracks::Expires, Lead::Months(6), 6),
    ]
}

pub(super) struct DocumentsPage {
    types: Vec<FixtureType>,
    selected: usize,
}

impl DocumentsPage {
    pub(super) fn new() -> Self {
        Self {
            types: fixture(),
            selected: 0,
        }
    }

    /// Swaps the selected type with its neighbour and keeps the selection on it.
    fn shift(&mut self, up: bool) {
        let other = if up {
            self.selected.checked_sub(1)
        } else {
            Some(self.selected + 1).filter(|next| *next < self.types.len())
        };
        if let Some(other) = other {
            self.types.swap(self.selected, other);
            self.selected = other;
        }
    }

    fn typed_files(&self) -> i64 {
        self.types.iter().map(|kind| kind.files).sum()
    }
}

fn dash() -> Cell {
    Cell::muted(msg::tui_settings_documents_none())
}

impl Page for DocumentsPage {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(step) = list::motion(key) {
            self.selected = lib_list_cursor::step(self.selected, self.types.len(), step);
            return Some(Action::NoOp);
        }
        if key.modifiers != KeyModifiers::NONE && key.modifiers != KeyModifiers::SHIFT {
            return None;
        }
        match key.code {
            KeyCode::Char('J') => {
                self.shift(false);
                Some(Action::NoOp)
            }
            KeyCode::Char('K') => {
                self.shift(true);
                Some(Action::NoOp)
            }
            KeyCode::Char('n') => Some(Action::FlashNotYetBuilt("document type new")),
            KeyCode::Char('e') => Some(Action::FlashNotYetBuilt("document type edit")),
            KeyCode::Char('x') => Some(Action::FlashNotYetBuilt("document type remove")),
            _ => None,
        }
    }

    fn scope_note(&self) -> String {
        msg::tui_settings_scope_documents(
            i64::try_from(self.types.len()).unwrap_or(i64::MAX),
            &crate::format::count(self.typed_files()),
            &crate::format::count(self.typed_files() + UNTYPED),
        )
    }

    fn hints(&self) -> Vec<(String, String)> {
        vec![
            ("j/k".into(), msg::tui_settings_hint_row()),
            ("J/K".into(), msg::tui_settings_hint_move()),
            ("n".into(), msg::tui_settings_hint_add()),
            ("e".into(), msg::tui_settings_hint_edit()),
            ("x".into(), msg::tui_settings_hint_remove()),
        ]
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let rows = self
            .types
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                let (tracks, remind) = match kind.tracks {
                    Some((tracks, lead)) => (
                        Cell::plain(match tracks {
                            Tracks::Renews => msg::tui_settings_documents_renews(),
                            Tracks::Ends => msg::tui_settings_documents_ends(),
                            Tracks::Expires => msg::tui_settings_documents_expires(),
                        }),
                        Cell::plain(match lead {
                            Lead::Days(n) => msg::tui_settings_documents_lead_days(n),
                            Lead::Months(n) => msg::tui_settings_documents_lead_months(n),
                        }),
                    ),
                    None => (dash(), dash()),
                };
                Row::Item {
                    cells: vec![
                        Cell::plain(kind.name),
                        tracks,
                        remind,
                        Cell::plain(if kind.tax_year {
                            msg::tui_settings_documents_yes()
                        } else {
                            msg::tui_settings_documents_no()
                        }),
                        Cell::plain(crate::format::count(kind.files)),
                    ],
                    selected: index == self.selected,
                    dim: false,
                }
            })
            .collect();
        let table = Table {
            heading: msg::tui_settings_documents_order(),
            add: vec![("n".into(), msg::tui_settings_documents_add())],
            columns: vec![
                Column::left(msg::tui_settings_documents_type(), Constraint::Min(0)),
                Column::left(
                    msg::tui_settings_documents_tracks(),
                    Constraint::Length(TRACKS_WIDTH),
                ),
                Column::left(
                    msg::tui_settings_documents_remind(),
                    Constraint::Length(REMIND_WIDTH),
                ),
                Column::left(
                    msg::tui_settings_documents_tax_year(),
                    Constraint::Length(TAX_YEAR_WIDTH),
                ),
                Column::right(
                    msg::tui_settings_documents_files(),
                    Constraint::Length(FILES_WIDTH),
                ),
            ],
            rows,
        };
        let notes = [
            msg::tui_settings_documents_note_tracks(),
            msg::tui_settings_documents_note_remind(),
            msg::tui_settings_documents_note_tax_year(),
            msg::tui_settings_documents_note_remove(&crate::format::count(UNTYPED)),
        ];
        list::render(frame, area, &[table], None, &notes, focused, c);
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
        view.update(&Action::OpenSettingsPage(SettingsPage::Documents));
        view
    }

    #[test]
    fn the_page_renders_tui_045_with_counts_derived_from_its_rows() {
        crate::locale::init_for_tests();
        let text = render_at(&page(), 96, 30);
        for needle in [
            "Settings › Documents",
            "7 types · 386 of 406 files typed",
            "IN TYPE-FILTER ORDER",
            "TAX YEAR",
            "Warranties & manuals",
            "30 days before",
            "6 months before",
            "expires",
            "Tracks: the date a document of this type carries",
            "20 untyped stay in the Inbox.",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
    }

    #[test]
    fn shift_j_and_k_move_the_selected_type_and_the_selection_follows() {
        let mut page = DocumentsPage::new();
        page.handle_key(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::SHIFT));
        assert_eq!(page.selected, 1);
        assert_eq!(page.types[0].name, "Statements");
        assert_eq!(page.types[1].name, "Receipts");
        page.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT));
        assert_eq!(page.types[0].name, "Receipts");
        // At the top, K has nowhere to go.
        page.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT));
        assert_eq!((page.selected, page.types[0].name), (0, "Receipts"));
    }

    #[test]
    fn n_e_and_x_flash_not_yet_built() {
        let mut page = DocumentsPage::new();
        for (code, command) in [
            (KeyCode::Char('n'), "document type new"),
            (KeyCode::Char('e'), "document type edit"),
            (KeyCode::Char('x'), "document type remove"),
        ] {
            assert_eq!(
                page.handle_key(key(code)),
                Some(Action::FlashNotYetBuilt(command))
            );
        }
    }
}
