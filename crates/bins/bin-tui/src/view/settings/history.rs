//! Settings › History (TUI-037): the Desktop's undo stack (16u), one linear list, newest first. A
//! line above it names the next undo and holds the area filter, a segmented row `f` cycles. The
//! rows are TIME · AREA · CHANGE · DEVICE. Undone changes sit above a reversed `▸ CURRENT STATE`
//! band, dim and struck through, until a new edit discards them; the applied rows sit below it.
//! The selected row names its action in the DEVICE column: `enter restore to here`, or `enter redo
//! to here` on an undone one. Drawn by the list-page pattern ([`super::list`]), so it docks the
//! detail pane at X-Large when #884 gives it one.
//!
//! A wireframe: there is no undo stack yet, so the rows are [`CHANGES`], and `enter`, `u` and
//! `ctrl r` flash the Shell's "not yet built". `u` and `ctrl r` are not bound app-wide: undo needs
//! its own design first, so they do something here only to say so.

use chrono::NaiveTime;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_locale::format::format_time_minutes;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::list::{self, Cell, Column, Row, Table, Tone};
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;

const TIME_WIDTH: u16 = 9;
const AREA_WIDTH: u16 = 10;
const DEVICE_WIDTH: u16 = 21;

/// wireframe: the whole stack's size, which the page's scope note counts; the rows below are the
/// newest of it.
const TOTAL_CHANGES: i64 = 214;

/// The area filter's segments, in the order `f` cycles them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Area {
    All,
    Transactions,
    Budgets,
    Settings,
}

impl Area {
    const ALL: [Self; 4] = [Self::All, Self::Transactions, Self::Budgets, Self::Settings];

    fn label(self) -> String {
        match self {
            Self::All => msg::tui_settings_history_area_all(),
            Self::Transactions => msg::tui_settings_history_area_transactions(),
            Self::Budgets => msg::tui_settings_history_area_budgets(),
            Self::Settings => msg::tui_settings_history_area_settings(),
        }
    }

    fn next(self) -> Self {
        let index = Self::ALL.iter().position(|a| *a == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

/// What a change touched; the AREA column's word, and which filter segment holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Transactions,
    Budgets,
    Tags,
    Bills,
    Inventory,
    Accounts,
}

impl Kind {
    fn label(self) -> String {
        match self {
            Self::Transactions => msg::tui_settings_history_kind_transactions(),
            Self::Budgets => msg::tui_settings_history_kind_budgets(),
            Self::Tags => msg::tui_settings_history_kind_tags(),
            Self::Bills => msg::tui_settings_history_kind_bills(),
            Self::Inventory => msg::tui_settings_history_kind_inventory(),
            Self::Accounts => msg::tui_settings_history_kind_accounts(),
        }
    }

    /// Whether the `area` filter shows a change of this kind: every other kind is Settings.
    fn in_area(self, area: Area) -> bool {
        match area {
            Area::All => true,
            Area::Transactions => self == Self::Transactions,
            Area::Budgets => self == Self::Budgets,
            Area::Settings => !matches!(self, Self::Transactions | Self::Budgets),
        }
    }
}

// wireframe: one change of the stack.
struct Change {
    time: Option<NaiveTime>,
    kind: Kind,
    what: &'static str,
    /// The device it was made on; `None` is this one.
    device: Option<&'static str>,
}

const fn change(
    time: Option<NaiveTime>,
    kind: Kind,
    what: &'static str,
    device: Option<&'static str>,
) -> Change {
    Change {
        time,
        kind,
        what,
        device,
    }
}

/// The changes undone and still redoable, newest first, above the marker.
const UNDONE: [Change; 2] = [
    change(
        NaiveTime::from_hms_opt(14, 21, 0),
        Kind::Transactions,
        "Recategorised Woolworths −86.40 → Groceries",
        None,
    ),
    change(
        NaiveTime::from_hms_opt(14, 19, 0),
        Kind::Budgets,
        "Dining limit 400.00 → 350.00",
        None,
    ),
];

/// The changes in force, newest first, below the marker.
const APPLIED: [Change; 9] = [
    change(
        NaiveTime::from_hms_opt(14, 12, 0),
        Kind::Transactions,
        "Added Camera House −4,380.00 · Everyday",
        None,
    ),
    change(
        NaiveTime::from_hms_opt(14, 5, 0),
        Kind::Tags,
        "Renamed tag holiday → travel",
        None,
    ),
    change(
        NaiveTime::from_hms_opt(13, 58, 0),
        Kind::Bills,
        "Marked Rates Q1 paid · 612.00",
        Some("laptop"),
    ),
    change(
        NaiveTime::from_hms_opt(13, 51, 0),
        Kind::Transactions,
        "Split Bunnings −142.75 three ways",
        None,
    ),
    change(
        NaiveTime::from_hms_opt(13, 44, 0),
        Kind::Inventory,
        "Added room Garage · 12 Elm St",
        None,
    ),
    change(
        NaiveTime::from_hms_opt(13, 30, 0),
        Kind::Transactions,
        "Imported 6 rows · anz-sep.qif",
        None,
    ),
    change(
        NaiveTime::from_hms_opt(12, 2, 0),
        Kind::Accounts,
        "Created ANZ Offset · aud",
        Some("laptop"),
    ),
    change(
        NaiveTime::from_hms_opt(11, 47, 0),
        Kind::Transactions,
        "Deleted duplicate Netflix −22.99",
        None,
    ),
    change(
        NaiveTime::from_hms_opt(11, 40, 0),
        Kind::Budgets,
        "Rollover applied · September",
        None,
    ),
];

pub(super) struct HistoryPage {
    area: Area,
    /// The selected row among those the filter shows: the undone ones, then the applied.
    selected: usize,
}

impl HistoryPage {
    pub(super) fn new() -> Self {
        Self {
            area: Area::All,
            selected: 0,
        }
    }

    /// The undone changes the filter shows, then the applied ones.
    fn visible(&self) -> (Vec<&'static Change>, Vec<&'static Change>) {
        let shown = |changes: &'static [Change]| {
            changes
                .iter()
                .filter(|change| change.kind.in_area(self.area))
                .collect::<Vec<_>>()
        };
        (shown(&UNDONE), shown(&APPLIED))
    }
}

impl list::Page for HistoryPage {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        let (undone, applied) = self.visible();
        if let Some(step) = list::motion(key) {
            self.selected =
                lib_list_cursor::step(self.selected, undone.len() + applied.len(), step);
            return Some(Action::NoOp);
        }
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('r') {
            return Some(Action::FlashNotYetBuilt("history redo"));
        }
        if key.modifiers != KeyModifiers::NONE {
            return None;
        }
        match key.code {
            KeyCode::Char('f') => {
                self.area = self.area.next();
                self.selected = 0;
                Some(Action::NoOp)
            }
            KeyCode::Char('u') => Some(Action::FlashNotYetBuilt("history undo")),
            KeyCode::Enter if self.selected < undone.len() => {
                Some(Action::FlashNotYetBuilt("history redo to here"))
            }
            KeyCode::Enter if self.selected < undone.len() + applied.len() => {
                Some(Action::FlashNotYetBuilt("history restore to here"))
            }
            _ => None,
        }
    }

    fn scope_note(&self) -> String {
        msg::tui_settings_scope_history_changes(TOTAL_CHANGES)
    }

    fn hints(&self) -> Vec<(String, String)> {
        vec![
            ("u".into(), msg::tui_settings_hint_undo()),
            ("ctrl r".into(), msg::tui_settings_hint_redo()),
            ("j/k".into(), msg::tui_settings_hint_row()),
            ("enter".into(), msg::tui_settings_hint_restore()),
            ("f".into(), msg::tui_settings_hint_area_filter()),
        ]
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let [toolbar, _, body] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(area);
        render_toolbar(frame, toolbar, self.area, c);

        let (undone, applied) = self.visible();
        let row = |index: usize, change: &Change, undone: bool| {
            let time = change.time.map(format_time_minutes).unwrap_or_default();
            let device = change
                .device
                .map_or_else(msg::tui_settings_history_this_device, str::to_string);
            let selected = index == self.selected;
            if undone {
                let device = if selected {
                    Cell::plain(msg::tui_settings_history_redo_to_here())
                } else {
                    Cell::muted(device)
                };
                Row::Item {
                    cells: vec![
                        Cell::muted(time),
                        Cell::muted(change.kind.label()),
                        Cell::tone(change.what, Tone::Struck),
                        device,
                    ],
                    selected,
                    dim: true,
                }
            } else {
                let device = if selected {
                    Cell::plain(msg::tui_settings_history_restore_to_here())
                } else {
                    Cell::muted(device)
                };
                Row::Item {
                    cells: vec![
                        Cell::plain(time),
                        Cell::muted(change.kind.label()),
                        Cell::plain(change.what),
                        device,
                    ],
                    selected,
                    dim: false,
                }
            }
        };
        let mut rows = Vec::new();
        for (index, change) in undone.iter().enumerate() {
            rows.push(row(index, change, true));
        }
        rows.push(Row::Band {
            label: msg::tui_settings_history_current_state(),
            note: msg::tui_settings_history_redoable(
                i64::try_from(UNDONE.len()).unwrap_or(i64::MAX),
            ),
        });
        for (index, change) in applied.iter().enumerate() {
            rows.push(row(undone.len() + index, change, false));
        }
        let table = Table {
            heading: msg::tui_settings_history_newest_first(),
            add: Vec::new(),
            columns: vec![
                Column::left(
                    msg::tui_settings_history_time(),
                    Constraint::Length(TIME_WIDTH),
                ),
                Column::left(
                    msg::tui_settings_history_area(),
                    Constraint::Length(AREA_WIDTH),
                ),
                Column::left(msg::tui_settings_history_change(), Constraint::Min(0)),
                Column::left(
                    msg::tui_settings_history_device(),
                    Constraint::Length(DEVICE_WIDTH),
                ),
            ],
            rows,
        };
        let notes = [msg::tui_settings_history_synced_note()];
        list::render(frame, body, &[table], None, &notes, focused, c);
    }
}

/// `next undo <change>` at the left, the area filter's segments at the right with the chosen one
/// reversed.
fn render_toolbar(frame: &mut Frame<'_>, area: Rect, filter: Area, c: &Colours) {
    let next = APPLIED.first().map_or("", |change| change.what);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(msg::tui_settings_history_next_undo(), c.muted()),
            Span::raw("  "),
            Span::styled(next, Style::default().add_modifier(Modifier::BOLD)),
        ])),
        area,
    );
    let mut spans = vec![
        Span::styled(msg::tui_settings_history_area(), c.muted()),
        Span::raw(" "),
    ];
    for segment in Area::ALL {
        let label = format!(" {} ", segment.label());
        spans.push(if segment == filter {
            Span::styled(label, c.selection())
        } else {
            Span::raw(label)
        });
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Right),
        area,
    );
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
        view.update(&Action::OpenSettingsPage(SettingsPage::History));
        view
    }

    #[test]
    fn at_96_by_30_the_page_draws_tui_037_with_the_marker_between_undone_and_applied() {
        crate::locale::init_for_tests();
        let text = render_at(&page(), 96, 30);
        for needle in [
            "Settings › History",
            "all devices · last 30 days · 214 changes",
            "next undo",
            "Added Camera House −4,380.00 · Everyday",
            "all  txn  budget  settings",
            "TIME",
            "DEVICE",
            "Recategorised Woolworths −86.40 → Groceries",
            "enter redo to here",
            "▸ CURRENT STATE",
            "2 changes can be redone · a new edit clears them",
            "Marked Rates Q1 paid · 612.00",
            "laptop",
            "Synced changes undo on every device.",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
        let undone = text.find("Recategorised").expect("undone row");
        let marker = text.find("CURRENT STATE").expect("marker");
        let applied = text.find("Added room Garage").expect("applied row");
        assert!(undone < marker && marker < applied, "{text}");
    }

    #[test]
    fn the_selected_applied_row_reads_enter_restore_to_here_and_the_undone_rows_are_struck() {
        crate::locale::init_for_tests();
        let mut view = page();
        view.handle_key(key(KeyCode::Char('j')));
        view.handle_key(key(KeyCode::Char('j')));
        let text = render_at(&view, 96, 30);
        assert!(text.contains("enter restore to here"), "{text}");
        assert!(!text.contains("enter redo to here"), "{text}");

        let backend = ratatui::backend::TestBackend::new(96, 30);
        let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
        let colours = Colours::default();
        terminal
            .draw(|frame| view.view(frame, frame.area(), lib_shell::WindowSize::Medium, &colours))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let struck = buffer
            .content()
            .iter()
            .any(|cell| cell.modifier.contains(Modifier::CROSSED_OUT) && cell.symbol() == "W");
        assert!(struck, "the undone change should be struck through");
    }

    #[test]
    fn f_cycles_the_area_filter_and_the_marker_stays() {
        crate::locale::init_for_tests();
        let mut view = page();
        view.handle_key(key(KeyCode::Char('f')));
        let text = render_at(&view, 96, 30);
        assert!(text.contains("Recategorised") && text.contains("Added Camera House"));
        assert!(!text.contains("Renamed tag"), "{text}");

        view.handle_key(key(KeyCode::Char('f')));
        let text = render_at(&view, 96, 30);
        assert!(
            text.contains("Dining limit") && !text.contains("Recategorised"),
            "{text}"
        );
        assert!(text.contains("CURRENT STATE"));

        view.handle_key(key(KeyCode::Char('f')));
        let text = render_at(&view, 96, 30);
        assert!(
            text.contains("Renamed tag") && !text.contains("Dining limit"),
            "{text}"
        );
        assert!(
            text.contains("CURRENT STATE"),
            "the marker draws with nothing undone"
        );

        view.handle_key(key(KeyCode::Char('f')));
        assert!(render_at(&view, 96, 30).contains("Dining limit"));
    }

    #[test]
    fn enter_u_and_ctrl_r_flash_not_yet_built_on_this_page_only() {
        let mut view = page();
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::FlashNotYetBuilt("history redo to here"))
        );
        view.handle_key(key(KeyCode::Char('G')));
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::FlashNotYetBuilt("history restore to here"))
        );
        assert_eq!(
            view.handle_key(key(KeyCode::Char('u'))),
            Some(Action::FlashNotYetBuilt("history undo"))
        );
        assert_eq!(
            view.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            Some(Action::FlashNotYetBuilt("history redo"))
        );

        for other in [
            SettingsPage::General,
            SettingsPage::Payees,
            SettingsPage::AiConnections,
        ] {
            let mut view = SettingsView::new();
            view.update(&Action::OpenSettingsPage(other));
            assert_eq!(view.handle_key(key(KeyCode::Char('u'))), None, "{other:?}");
            assert_eq!(
                view.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
                None,
                "{other:?}"
            );
        }
    }
}
