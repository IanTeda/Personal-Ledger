//! Settings › Units (TUI-033, TUI-053, TUI-054): the UNITS and PRICE SOURCES tables, `tab`
//! moving between them, drawn by the list-page pattern ([`super::list`]), with notes on the
//! selected row under them.
//!
//! The page hosts one [`UnitsView`] on the Open Ledger's shared ports: its `lib-units` Presenter,
//! its Dialogs and the Toasts they raise. Every key but `o` goes to it, so `n`, `e`, `d`, `t` and
//! `tab` act here exactly as in the Units View, `[keybindings]` overrides included, and a Dialog
//! opened here draws over Settings. Only the drawing is Settings': the tables in TUI-033's
//! columns, read from the same page view model. At X-Large the page docks the Units View's
//! detail pane (TUI-055) at the View's right edge, and outcomes land in its ACTIVITY.
//!
//! `o` opens the full Units View, which `Shell` pushes Settings under, so `esc` there comes back
//! to this page and row.
//!
//! Real throughout: the rows, the flags and the usage note's counts are the Ledger's. `lib-units`
//! has no default Unit, so the card's `default` chip has nothing to show.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_shell::WindowSize;
use lib_toast::ToastKind;
use lib_units::UnitsPorts as _;
use lib_units::presenter::UnitsPageViewModel;
use ratatui::{Frame, layout::Constraint, layout::Rect};

use super::list::{self, Cell, Column, Row, Table, Tone};
use crate::colours::Colours;
use crate::msg;
use crate::units::view::units::UnitsView;
use crate::units::{TuiPorts, UnitsSetup};
use crate::view::{Action, View};

/// Widths of the Units table's CODE, NAME, PRICE SOURCE and TYPE columns; FLAGS takes the rest.
const CODE_WIDTH: u16 = 6;
const NAME_WIDTH: u16 = 22;
const SOURCE_WIDTH: u16 = 14;
const TYPE_WIDTH: u16 = 10;

/// Width of the Price Sources table's LAST UPDATED column; SOURCE takes the rest.
const UPDATED_WIDTH: u16 = 16;

/// What `[keybindings]` can't rebind on this page: `o`, which opens the full Units View.
const OPEN_VIEW: char = 'o';

pub(super) struct UnitsPage {
    view: UnitsView,
    ports: TuiPorts,
}

impl UnitsPage {
    pub(super) fn new(setup: &UnitsSetup) -> Self {
        Self {
            view: UnitsView::new(setup),
            ports: setup.ports.clone(),
        }
    }

    /// `o` opens the Units View; every other key is the Units View's, a Dialog's included.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.view.dialog_open()
            && key.code == KeyCode::Char(OPEN_VIEW)
            && key.modifiers == KeyModifiers::NONE
        {
            return Some(Action::OpenUnits);
        }
        self.view.handle_key(key)
    }

    pub(super) fn dialog_open(&self) -> bool {
        self.view.dialog_open()
    }

    pub(super) fn render_dialog(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        self.view.render_dialog(frame, area, c);
    }

    pub(super) fn take_toasts(&mut self) -> Vec<(ToastKind, String)> {
        self.view.take_toasts()
    }

    /// Docks the detail pane at X-Large, so outcomes land in its ACTIVITY.
    pub(super) fn set_window_size(&mut self, size: WindowSize) {
        self.view.set_window_size(size);
    }

    /// Whether the detail pane is docked, which takes the View's right edge.
    pub(super) fn has_pane(&self) -> bool {
        self.view.view_model().pane.is_some()
    }

    /// The heading's note, counting the Units.
    pub(super) fn scope_note(&self) -> String {
        let count = self.view.view_model().units.len();
        msg::tui_settings_scope_units(i64::try_from(count).unwrap_or(i64::MAX))
    }

    /// The page's keys: the Units View's for the selected row's table, then `o`.
    pub(super) fn hints(&self) -> Vec<(String, String)> {
        self.view
            .view_model()
            .hints
            .into_iter()
            .map(|hint| (hint.chord, hint.action))
            .chain([(
                OPEN_VIEW.to_string(),
                msg::tui_settings_hint_open_units_view(),
            )])
            .collect()
    }

    pub(super) fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let view = self.view.view_model();
        list::render(
            frame,
            area,
            &[unit_table(&view), source_table(&view)],
            None,
            &self.notes(&view),
            focused,
            c,
        );
    }

    pub(super) fn render_pane(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        self.view
            .render_pane(frame, area, &self.view.view_model(), c);
    }

    /// What the selected row is used by. A Unit's accounts and transactions are its holdings';
    /// a Price Source's Units are the rows that name it.
    fn notes(&self, view: &UnitsPageViewModel) -> Vec<String> {
        if let Some(unit) = view.units.iter().find(|unit| unit.selected) {
            let holdings = self.ports.borrow().holdings().holdings(unit.id);
            let transactions: u32 = holdings.iter().map(|holding| holding.transactions).sum();
            let usage = if holdings.is_empty() {
                msg::tui_settings_units_note_unused(&unit.code)
            } else {
                msg::tui_settings_units_note_usage(
                    &unit.code,
                    i64::try_from(holdings.len()).unwrap_or(i64::MAX),
                    i64::from(transactions),
                )
            };
            return vec![usage, msg::tui_settings_units_note_code()];
        }
        if let Some(source) = view.price_sources.iter().find(|source| source.selected) {
            let codes: Vec<&str> = view
                .units
                .iter()
                .filter(|unit| unit.price_source == source.name)
                .map(|unit| unit.code.as_str())
                .collect();
            return vec![lib_units::msg::units_price_source_usage(
                i64::try_from(codes.len()).unwrap_or(i64::MAX),
                &codes.join(", "),
            )];
        }
        Vec::new()
    }
}

/// UNITS: CODE · NAME · FLAGS · PRICE SOURCE · TYPE. The base Unit's flag is an accent chip; an
/// inactive Unit's row is dim, its flag a plain chip.
fn unit_table(view: &UnitsPageViewModel) -> Table {
    let labels = &view.unit_columns;
    Table {
        heading: msg::tui_settings_units_heading(),
        add: vec![("n".into(), msg::tui_settings_units_add_unit())],
        columns: vec![
            Column::left(labels.code.clone(), Constraint::Length(CODE_WIDTH)),
            Column::left(labels.name.clone(), Constraint::Length(NAME_WIDTH)),
            Column::left(labels.flags.clone(), Constraint::Min(0)),
            Column::left(
                labels.price_source.clone(),
                Constraint::Length(SOURCE_WIDTH),
            ),
            Column::left(labels.kind.clone(), Constraint::Length(TYPE_WIDTH)),
        ],
        rows: view
            .units
            .iter()
            .map(|unit| {
                let mut flags = Vec::new();
                if let Some(base) = &unit.base_flag {
                    flags.push((base.clone(), Tone::Chip(true)));
                }
                if let Some(inactive) = &unit.inactive_tag {
                    if !flags.is_empty() {
                        flags.push((" ".into(), Tone::Plain));
                    }
                    flags.push((inactive.clone(), Tone::Chip(false)));
                }
                let source = if unit.price_source.is_empty() {
                    "\u{2014}".to_string()
                } else {
                    unit.price_source.clone()
                };
                Row::Item {
                    cells: vec![
                        Cell::plain(&unit.code),
                        Cell::plain(&unit.name),
                        Cell(flags),
                        Cell::muted(source),
                        Cell::plain(&unit.kind),
                    ],
                    selected: unit.selected,
                    dim: unit.inactive_tag.is_some(),
                }
            })
            .collect(),
    }
}

/// PRICE SOURCES: NAME · SOURCE · LAST UPDATED.
fn source_table(view: &UnitsPageViewModel) -> Table {
    let labels = &view.source_columns;
    Table {
        heading: view.price_sources_heading.clone(),
        add: vec![("n".into(), msg::tui_settings_units_add_source())],
        columns: vec![
            Column::left(labels.name.clone(), Constraint::Length(NAME_WIDTH)),
            Column::left(labels.source.clone(), Constraint::Min(0)),
            Column::left(
                labels.last_updated.clone(),
                Constraint::Length(UPDATED_WIDTH),
            ),
        ],
        rows: view
            .price_sources
            .iter()
            .map(|source| Row::Item {
                cells: vec![
                    Cell::plain(&source.name),
                    Cell::plain(&source.address),
                    Cell::muted(&source.last_updated),
                ],
                selected: source.selected,
                dim: false,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;
    use crate::view::settings::tests::{render_at, stub_units};
    use crate::view::settings::{SettingsPage, SettingsView};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// Settings › Units on the stub Ledger, with the Shell's draw at `width` told it.
    fn units_page(width: u16) -> SettingsView {
        let mut view = SettingsView::with_units(&stub_units());
        view.update(&Action::OpenSettingsPage(SettingsPage::Units));
        view.set_window_size(WindowSize::from_columns(width));
        view
    }

    #[test]
    fn the_page_lists_both_tables_with_their_add_keys_and_the_selected_unit_s_usage() {
        crate::locale::init_for_tests();
        // TUI-033 at the floor, and TUI-053 at Medium's reference frame.
        for (width, height) in [(96, 30), (128, 69)] {
            let text = render_at(&units_page(width), width, height);
            for needle in [
                "Settings › Units",
                "ledger data · synced · 3 units",
                "UNITS",
                "n add unit",
                "CODE",
                "FLAGS",
                "PRICE SOURCES",
                "n add source",
                "LAST UPDATED",
                "Australian Dollar",
                "[base]",
                "CoinGecko",
                "aud is used by 5 accounts and 772 transactions. Renaming it is safe.",
                "Deleting asks you to type the code.",
            ] {
                assert!(
                    text.contains(needle),
                    "{needle} missing at {width}:\n{text}"
                );
            }
        }
    }

    #[test]
    fn at_large_the_index_docks_and_the_page_heading_is_its_name_alone() {
        crate::locale::init_for_tests();
        // TUI-054: nothing on the page changes.
        let text = render_at(&units_page(171), 171, 53);
        assert!(text.contains("17 pages"), "{text}");
        assert!(!text.contains("Settings › Units"), "{text}");
        assert!(
            text.contains("n add unit") && text.contains("[base]"),
            "{text}"
        );
        assert!(!text.contains("ACTIVITY"), "no pane below X-Large:\n{text}");
    }

    #[test]
    fn at_x_large_the_page_docks_the_units_detail_pane() {
        crate::locale::init_for_tests();
        let text = render_at(&units_page(256), 256, 72);
        assert!(
            text.contains("ACTIVITY") && text.contains("n add unit"),
            "{text}"
        );
    }

    #[test]
    fn the_units_keys_reach_lib_units_and_o_opens_the_units_view() {
        crate::locale::init_for_tests();
        let mut view = units_page(96);
        assert_eq!(
            view.handle_key(key(KeyCode::Char('o'))),
            Some(Action::OpenUnits)
        );

        // `tab` moves to the Price Sources, whose first row's note names the Units it values.
        view.handle_key(key(KeyCode::Tab));
        let text = render_at(&view, 96, 30);
        assert!(text.contains("Values 1 unit: btc."), "{text}");
        // Testing isn't built in `lib-units` yet, which it says through the page's Toasts.
        view.handle_key(key(KeyCode::Char('t')));
        assert_eq!(
            view.take_toasts(),
            vec![(
                ToastKind::Info,
                "test price source — not yet built".to_string()
            )]
        );

        for code in [KeyCode::Char('n'), KeyCode::Char('e'), KeyCode::Char('d')] {
            let mut view = units_page(96);
            view.handle_key(key(code));
            assert!(view.dialog_open(), "{code:?} opens a Dialog");
            // The Dialog takes every key: `]` doesn't page away under it.
            view.handle_key(key(KeyCode::Char(']')));
            assert!(view.dialog_open());
            view.handle_key(key(KeyCode::Esc));
            assert!(!view.dialog_open(), "esc cancels the Dialog");
            let text = render_at(&view, 96, 30);
            assert!(text.contains("Settings › Units"), "{text}");
        }
    }

    /// Units keeps its Presenter's hints rather than a key table (#898): they are built from the
    /// `lib-viper` Keymap the Presenter dispatches on. This holds the Settings side to it: every
    /// key the hint bar names, on either table, is one the page takes.
    #[test]
    fn every_key_in_the_hint_bar_is_handled() {
        crate::locale::init_for_tests();
        for on_sources in [false, true] {
            let fresh = || {
                let mut view = units_page(96);
                if on_sources {
                    view.handle_key(key(KeyCode::Tab));
                }
                view
            };
            let hints = fresh().hints("esc").unwrap_or_default();
            let page_hints = hints.iter().take_while(|(chord, _)| chord != "[/]");
            assert!(page_hints.clone().count() >= 5, "{hints:?}");
            for (chord, hint) in page_hints {
                for part in chord.split('/') {
                    let code = match part {
                        "tab" => KeyCode::Tab,
                        "enter" => KeyCode::Enter,
                        single if single.chars().count() == 1 => {
                            KeyCode::Char(single.chars().next().unwrap_or_default())
                        }
                        other => panic!("`{other}` ({hint}) isn't a key this test reads"),
                    };
                    assert!(
                        fresh().handle_key(key(code)).is_some(),
                        "`{part}` ({hint}) is in the hint bar but isn't handled"
                    );
                }
            }
        }
    }
}
