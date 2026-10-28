//! The Units `View`, hosted by `Shell` (ADR-0013): the `lib-units` page Presenter drawn in the
//! §4a layout of `docs/ux/tui-mockups/07-units/README.md`. The left column holds the Units and
//! Price Sources tables above the selected row's summary; the right holds the weekly close and
//! weekly prices regions above the keys box. Every row, label and hint comes from the page view
//! model; price history has no Presenter yet, so its two regions say so (#746's findings log).
//!
//! Keys become chord text and are looked up in the shared Units keymap, so `[keybindings]`
//! overrides reach them as on the Desktop. The arrows walk the tables and `Enter` edits, as the
//! Desktop's router movements and double-click do. While a Dialog is open it takes every key
//! ([`View::dialog_open`]) and draws over the frame ([`View::render_dialog`]).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_list_cursor::Step;
use lib_locale::format::upper;
use lib_shell::WindowSize;
use lib_toast::ToastKind;
use lib_units::SharedPorts;
use lib_units::presenter::{
    SourceLine, UnitLine, UnitsPageEvent, UnitsPagePresenter, UnitsPageViewModel,
};
use lib_viper::keymap::Keymap;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crate::colours::Colours;
use crate::msg;
use crate::units::router::Router;
use crate::units::view::dialog::Dialog;
use crate::units::{TuiPorts, UnitsSetup};
use crate::view::{Action, View, ViewId};

/// Width of the left column: the two tables and the summary. Widened past §4a's own "~36 cols"
/// for less cramped columns and the summary box's horizontal padding.
const LEFT_COLUMN_WIDTH: u16 = 46;

/// Width of the Units table's CODE column.
const UNIT_CODE_WIDTH: u16 = 6;

/// Width of the Units table's right-aligned PRICE SOURCE column, the longest built-in's name.
const UNIT_SOURCE_WIDTH: u16 = 14;

/// Width of the Price Sources table's right-aligned LAST UPDATED column.
const SOURCE_UPDATED_WIDTH: u16 = 18;

/// The ✗ before LAST UPDATED for a source that hasn't fetched yet, as the Desktop draws it. A
/// glyph, not a Message.
const NOT_FETCHED_GLYPH: &str = "\u{2717} ";

/// Rows inside the summary box's border: the title row, a second line, a rule, then two facts.
const SUMMARY_BOX_CONTENT_HEIGHT: u16 = 5;

/// The summary section: its heading, rule and the bordered box. Fixed, so extra terminal height
/// goes to the tables above it.
const SUMMARY_SECTION_HEIGHT: u16 = 2 + SUMMARY_BOX_CONTENT_HEIGHT + 2;

/// Width of a summary fact's label column.
const SUMMARY_LABEL_WIDTH: usize = 16;

/// Height of the weekly close region: heading, rule and the room its chart will take.
const WEEKLY_CLOSE_HEIGHT: u16 = 12;

/// Height of the keys box: heading, rule, the hints row, then one row per command hint.
const KEYBIND_HINTS_HEIGHT: u16 = 3 + 3;

/// What separates one key hint from the next.
const HINT_SEPARATOR: &str = "  \u{b7}  ";

/// The Units page: its Presenter on the shared ports, and the Dialog open over it.
pub struct UnitsView {
    presenter: UnitsPagePresenter,
    ports: TuiPorts,
    keymap: Keymap<UnitsPageEvent>,
    dialog: Option<Dialog>,
    /// Raised by the Presenters since `Shell` last took them.
    toasts: Vec<(ToastKind, String)>,
}

impl UnitsView {
    pub fn new(setup: &UnitsSetup) -> Self {
        let shared: SharedPorts = setup.ports.clone();
        Self {
            presenter: UnitsPagePresenter::new(
                shared,
                std::rc::Rc::new(chrono::Utc::now),
                setup.keymap.clone(),
            ),
            ports: setup.ports.clone(),
            keymap: setup.keymap.clone(),
            dialog: None,
            toasts: Vec::new(),
        }
    }

    /// The page as drawn, for the tests.
    pub fn view_model(&self) -> UnitsPageViewModel {
        self.presenter.view_model()
    }

    /// Sends `event` to the page Presenter, opens any Dialog it asked for, and commits whatever it
    /// wrote as one transaction.
    fn send(&mut self, event: UnitsPageEvent) {
        let mut router = Router::new(self.ports.clone(), &mut self.toasts);
        self.presenter.handle(event, &mut router);
        let opened = router.opened.take();
        self.ports.borrow_mut().commit();
        if opened.is_some() {
            self.dialog = opened;
        }
    }

    /// Hands `key` to the open Dialog, commits what its confirm wrote, and once it closes tells
    /// the page what it did.
    fn send_to_dialog(&mut self, key: KeyEvent) {
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        let mut router = Router::new(self.ports.clone(), &mut self.toasts);
        dialog.handle_key(key, &mut router);
        let closed = router.closed.take();
        self.ports.borrow_mut().commit();
        if let Some(outcome) = closed {
            self.dialog = None;
            self.send(UnitsPageEvent::DialogClosed(outcome));
        }
    }

    /// The page event `key` means: its chord in the Units keymap, or an arrow or `Enter`.
    fn page_event(&self, key: KeyEvent) -> Option<UnitsPageEvent> {
        if let Some(event) = chord(key).and_then(|chord| self.keymap.event(&chord)) {
            return Some(event);
        }
        Some(match key.code {
            KeyCode::Down => UnitsPageEvent::Move(Step::Next),
            KeyCode::Up => UnitsPageEvent::Move(Step::Prev),
            KeyCode::Home => UnitsPageEvent::Move(Step::First),
            KeyCode::End => UnitsPageEvent::Move(Step::Last),
            KeyCode::PageDown => UnitsPageEvent::Move(Step::HalfPageDown),
            KeyCode::PageUp => UnitsPageEvent::Move(Step::HalfPageUp),
            KeyCode::Enter => UnitsPageEvent::Edit,
            _ => return None,
        })
    }
}

/// `key` as `[keybindings]` writes a chord (`ctrl+d`, `G`, `tab`), or `None` for one the keymap
/// can't name. crossterm already reports a shifted letter as its capital.
fn chord(key: KeyEvent) -> Option<String> {
    let base = match key.code {
        KeyCode::Char(ch) => ch.to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::BackTab => "shift+tab".to_string(),
        KeyCode::Enter => "enter".to_string(),
        _ => return None,
    };
    let mut chord = String::new();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        chord.push_str("ctrl+");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        chord.push_str("alt+");
    }
    chord.push_str(&base);
    Some(chord)
}

impl View for UnitsView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.dialog.is_some() {
            self.send_to_dialog(key);
            return Some(Action::NoOp);
        }
        let event = self.page_event(key)?;
        self.send(event);
        Some(Action::NoOp)
    }

    /// The command window's `unit new`, `unit edit` and `unit delete` act on the page as its
    /// `n`, `e` and `d` do.
    fn update(&mut self, action: &Action) {
        let event = match action {
            Action::OpenNewUnitPopup => UnitsPageEvent::NewUnit,
            Action::OpenEditUnitPopup => UnitsPageEvent::Edit,
            Action::OpenDeleteUnitPopup => UnitsPageEvent::Delete,
            _ => return,
        };
        self.send(event);
    }

    fn view(&self, frame: &mut Frame<'_>, area: Rect, _size: WindowSize, c: &Colours) {
        let view = self.presenter.view_model();
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        // rows[0] is left blank — breathing space between the shell's title bar and the tables,
        // matching the dashboard's own spacer rows.
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(LEFT_COLUMN_WIDTH), Constraint::Min(0)])
            .spacing(2)
            .split(rows[1]);

        render_left_column(frame, columns[0], &view, c);
        render_price_history(frame, columns[1], &view, c);
    }

    fn id(&self) -> ViewId {
        ViewId::Units
    }

    fn title(&self) -> String {
        msg::tui_view_units_title()
    }

    fn take_toasts(&mut self) -> Vec<(ToastKind, String)> {
        std::mem::take(&mut self.toasts)
    }

    fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }

    fn render_dialog(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        if let Some(dialog) = &self.dialog {
            dialog.render(frame, area, c);
        }
    }
}

/// The left column: the two tables above the summary, which is sized to its content so the
/// tables take whatever height is left.
fn render_left_column(frame: &mut Frame<'_>, area: Rect, view: &UnitsPageViewModel, c: &Colours) {
    let sources_height = u16::try_from(view.price_sources.len())
        .unwrap_or(u16::MAX)
        .saturating_add(4);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(sources_height),
            Constraint::Length(SUMMARY_SECTION_HEIGHT),
        ])
        .split(area);

    render_unit_table(frame, rows[0], view, c);
    render_source_table(frame, rows[1], view, c);
    render_summary(frame, rows[2], view, c);
}

/// A table section's heading row: a dim label left and a dim tag right.
fn render_heading(frame: &mut Frame<'_>, area: Rect, label: &str, tag: &str, c: &Colours) {
    let [left, right] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(u16::try_from(tag.chars().count()).unwrap_or(0)),
    ])
    .areas(area);
    let dim = c.muted();
    frame.render_widget(Paragraph::new(Span::styled(label.to_string(), dim)), left);
    frame.render_widget(
        Paragraph::new(Span::styled(tag.to_string(), dim)).alignment(Alignment::Right),
        right,
    );
}

/// The first row `visible` rows can start at and still show row `selected`.
fn scroll_offset(selected: Option<usize>, visible: usize) -> usize {
    match selected {
        Some(index) if visible > 0 && index >= visible => index + 1 - visible,
        _ => 0,
    }
}

/// The Units table (top left): a "UNITS · N OF total" heading over CODE / TYPE / PRICE SOURCE,
/// scrolled to keep the selection in view, with a scrollbar on its right edge.
fn render_unit_table(frame: &mut Frame<'_>, area: Rect, view: &UnitsPageViewModel, c: &Colours) {
    let [content, scrollbar_column] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(1)])
            .spacing(1)
            .areas(area);
    let [heading, rule, header, body] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(content);

    let visible = view.units.len().min(usize::from(body.height));
    let tag = msg::tui_units_list_heading_tag(&visible.to_string(), &view.units.len().to_string());
    render_heading(frame, heading, &msg::tui_units_list_heading(), &tag, c);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rule);

    let columns = unit_row_columns(header);
    let dim = c.muted();
    let labels = &view.unit_columns;
    frame.render_widget(
        Paragraph::new(Span::styled(upper(&labels.code), dim)),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(upper(&labels.kind), dim)),
        columns[1],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(upper(&labels.price_source), dim)).alignment(Alignment::Right),
        columns[2],
    );

    let selected = view.units.iter().position(|unit| unit.selected);
    let offset = scroll_offset(selected, usize::from(body.height));
    for (index, unit) in view.units.iter().skip(offset).take(visible).enumerate() {
        let row = Rect {
            y: body.y + u16::try_from(index).unwrap_or(0),
            height: 1,
            ..body
        };
        render_unit_row(frame, row, unit, c);
    }

    let mut scrollbar_state = ScrollbarState::new(view.units.len())
        .viewport_content_length(visible)
        .position(offset);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        Rect {
            y: body.y,
            height: body.height,
            ..scrollbar_column
        },
        &mut scrollbar_state,
    );
}

/// One Unit row. Its type carries the base and inactive flags (`Currency · base`); an inactive
/// Unit draws dim throughout, and the selected row reverses.
fn render_unit_row(frame: &mut Frame<'_>, area: Rect, unit: &UnitLine, c: &Colours) {
    if unit.selected {
        frame.render_widget(Block::new().style(c.selection()), area);
    }
    let dim = c.muted();
    let inactive = unit.inactive_tag.is_some();
    let code_style = if inactive { dim } else { Style::default() };
    let rest = if unit.selected && !inactive {
        Style::default()
    } else {
        dim
    };
    let kind = [
        Some(&unit.kind),
        unit.base_flag.as_ref(),
        unit.inactive_tag.as_ref(),
    ]
    .into_iter()
    .flatten()
    .cloned()
    .collect::<Vec<_>>()
    .join(" \u{b7} ");

    let columns = unit_row_columns(area);
    frame.render_widget(
        Paragraph::new(Span::styled(unit.code.as_str(), code_style)),
        columns[0],
    );
    frame.render_widget(Paragraph::new(Span::styled(kind, rest)), columns[1]);
    frame.render_widget(
        Paragraph::new(Span::styled(unit.price_source.as_str(), rest)).alignment(Alignment::Right),
        columns[2],
    );
}

fn unit_row_columns(area: Rect) -> [Rect; 3] {
    Layout::horizontal([
        Constraint::Length(UNIT_CODE_WIDTH),
        Constraint::Min(0),
        Constraint::Length(UNIT_SOURCE_WIDTH),
    ])
    .spacing(1)
    .areas(area)
}

/// The Price Sources table, under the Units: NAME and LAST UPDATED. `Tab` moves the selection
/// between the two.
fn render_source_table(frame: &mut Frame<'_>, area: Rect, view: &UnitsPageViewModel, c: &Colours) {
    // A blank row parts it from the Units table; the scrollbar column stays clear, as above.
    let [_, content] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let [content, _] = Layout::horizontal([Constraint::Min(0), Constraint::Length(1)])
        .spacing(1)
        .areas(content);
    let [heading, rule, header, body] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(content);

    render_heading(frame, heading, &upper(&view.price_sources_heading), "", c);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rule);
    let dim = c.muted();
    let [name, updated] = source_row_columns(header);
    frame.render_widget(
        Paragraph::new(Span::styled(upper(&view.source_columns.name), dim)),
        name,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(upper(&view.source_columns.last_updated), dim))
            .alignment(Alignment::Right),
        updated,
    );

    for (index, source) in view
        .price_sources
        .iter()
        .take(usize::from(body.height))
        .enumerate()
    {
        let row = Rect {
            y: body.y + u16::try_from(index).unwrap_or(0),
            height: 1,
            ..body
        };
        render_source_row(frame, row, source, c);
    }
}

fn render_source_row(frame: &mut Frame<'_>, area: Rect, source: &SourceLine, c: &Colours) {
    if source.selected {
        frame.render_widget(Block::new().style(c.selection()), area);
    }
    let rest = if source.selected {
        Style::default()
    } else {
        c.muted()
    };
    let [name, updated] = source_row_columns(area);
    frame.render_widget(Paragraph::new(source.name.as_str()), name);
    let mut spans = Vec::new();
    if source.not_fetched {
        spans.push(Span::styled(NOT_FETCHED_GLYPH, c.muted()));
    }
    spans.push(Span::styled(source.last_updated.as_str(), rest));
    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Right),
        updated,
    );
}

fn source_row_columns(area: Rect) -> [Rect; 2] {
    Layout::horizontal([Constraint::Min(0), Constraint::Length(SOURCE_UPDATED_WIDTH)])
        .spacing(1)
        .areas(area)
}

/// The summary (bottom left): the selected row's title over its facts, in a bordered box under a
/// "SUMMARY" heading.
fn render_summary(frame: &mut Frame<'_>, area: Rect, view: &UnitsPageViewModel, c: &Colours) {
    let [heading, rule, boxed] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(area);
    render_heading(frame, heading, &msg::tui_units_summary_heading(), "", c);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rule);

    let block = Block::bordered().padding(Padding::horizontal(1));
    let inner = block.inner(boxed);
    frame.render_widget(block, boxed);
    let [title, second, divider, first_fact, second_fact] =
        Layout::vertical([Constraint::Length(1); 5]).areas(inner);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = c.muted();
    let fact = |label: &str, value: &str| {
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{label:<SUMMARY_LABEL_WIDTH$}"), dim),
            Span::raw(value.to_string()),
        ]))
    };
    if let Some(unit) = view.units.iter().find(|unit| unit.selected) {
        let [code, kind] = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(u16::try_from(unit.kind.chars().count()).unwrap_or(0)),
        ])
        .areas(title);
        frame.render_widget(Paragraph::new(Span::styled(unit.code.as_str(), bold)), code);
        frame.render_widget(
            Paragraph::new(Span::styled(unit.kind.as_str(), dim)).alignment(Alignment::Right),
            kind,
        );
        frame.render_widget(Paragraph::new(unit.name.as_str()), second);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), divider);
        let flags = [unit.base_flag.as_ref(), unit.inactive_tag.as_ref()]
            .into_iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>()
            .join(" \u{b7} ");
        let columns = &view.unit_columns;
        frame.render_widget(fact(&columns.price_source, &unit.price_source), first_fact);
        frame.render_widget(fact(&columns.flags, &flags), second_fact);
    } else if let Some(source) = view.price_sources.iter().find(|source| source.selected) {
        frame.render_widget(
            Paragraph::new(Span::styled(source.name.as_str(), bold)),
            title,
        );
        frame.render_widget(Block::new().borders(Borders::BOTTOM), divider);
        let columns = &view.source_columns;
        frame.render_widget(fact(&columns.source, &source.address), first_fact);
        frame.render_widget(
            fact(&columns.last_updated, &source.last_updated),
            second_fact,
        );
    }
}

/// The right column: the weekly close and weekly prices regions above the keys box, per §4a
/// ("the rest is price history").
fn render_price_history(frame: &mut Frame<'_>, area: Rect, view: &UnitsPageViewModel, c: &Colours) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(WEEKLY_CLOSE_HEIGHT),
            Constraint::Length(1), // blank spacer
            Constraint::Min(0),
            Constraint::Length(1), // blank spacer
            Constraint::Length(KEYBIND_HINTS_HEIGHT),
            Constraint::Length(1), // blank spacer, above the shell's footer
        ])
        .split(area);

    render_empty_region(frame, rows[0], &msg::tui_units_weekly_close_heading(), c);
    render_empty_region(frame, rows[2], &msg::tui_units_weekly_prices_heading(), c);
    render_keybind_hints(frame, rows[4], view, c);
}

/// A price history region with nothing to draw yet: its heading, its rule and a dim note.
fn render_empty_region(frame: &mut Frame<'_>, area: Rect, heading: &str, c: &Colours) {
    let [heading_row, rule, note] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(area);
    render_heading(frame, heading_row, heading, "", c);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rule);
    frame.render_widget(
        Paragraph::new(Span::styled(
            msg::tui_units_price_history_empty(),
            c.muted(),
        )),
        note,
    );
}

/// The keys box (bottom right): the page's hints for the selected row's table, each chord bold
/// and its action dim, then the `:` commands that act on the page from the command line.
fn render_keybind_hints(frame: &mut Frame<'_>, area: Rect, view: &UnitsPageViewModel, c: &Colours) {
    let command_hints = [
        msg::tui_units_command_new(),
        msg::tui_units_command_edit(),
        msg::tui_units_command_price(),
    ];
    let rows = Layout::vertical([Constraint::Length(1); KEYBIND_HINTS_HEIGHT as usize]).split(area);

    let dim = c.muted();
    render_heading(frame, rows[0], &msg::tui_units_hints_heading(), "", c);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let mut spans = Vec::new();
    for (index, hint) in view.hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(HINT_SEPARATOR, dim));
        }
        spans.push(Span::styled(hint.chord.as_str(), bold));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(hint.action.as_str(), dim));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), rows[2]);

    for (line, row) in command_hints.iter().zip(rows.iter().skip(3)) {
        frame.render_widget(Paragraph::new(Span::styled(line.as_str(), dim)), *row);
    }
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn units_view() -> UnitsView {
        crate::locale::init_for_tests();
        UnitsView::new(&UnitsSetup::stub(&KeyBindingConfig::default()))
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(view: &UnitsView, width: u16, height: u16) -> String {
        let c = Colours::default();
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area(), WindowSize::Medium, &c))
            .expect("drawing the units view should not error");
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn a_key_becomes_the_chord_keybindings_writes() {
        assert_eq!(chord(key(KeyCode::Char('d'))).as_deref(), Some("d"));
        let shifted = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
        assert_eq!(chord(shifted).as_deref(), Some("G"));
        let ctrl = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(chord(ctrl).as_deref(), Some("ctrl+d"));
        assert_eq!(chord(key(KeyCode::Tab)).as_deref(), Some("tab"));
        assert_eq!(chord(key(KeyCode::F(1))), None);
    }

    #[test]
    fn a_bound_key_reaches_the_presenter() {
        let mut view = units_view();
        let first = view.view_model().units[0].id;

        assert_eq!(view.handle_key(key(KeyCode::Char('j'))), Some(Action::NoOp));

        let selected = view
            .view_model()
            .units
            .iter()
            .find(|unit| unit.selected)
            .map(|unit| unit.id);
        assert_ne!(
            selected,
            Some(first),
            "j moves the selection off the first Unit"
        );
    }

    #[test]
    fn an_unbound_key_falls_through_to_the_shell() {
        let mut view = units_view();
        assert_eq!(view.handle_key(key(KeyCode::Char('z'))), None);
    }

    #[test]
    fn n_opens_a_dialog_and_esc_closes_it() {
        let mut view = units_view();
        view.handle_key(key(KeyCode::Char('n')));
        assert!(view.dialog_open());
        assert!(view.view_model().open_dialog.is_some());

        view.handle_key(key(KeyCode::Esc));
        assert!(!view.dialog_open());
        assert!(
            view.view_model().open_dialog.is_none(),
            "the page is told it closed"
        );
    }

    #[test]
    fn draws_both_tables_and_the_selected_units_summary() {
        let view = units_view();
        let text = render(&view, 96, 30);
        let model = view.view_model();

        for unit in &model.units {
            assert!(text.contains(&unit.code), "{} missing:\n{text}", unit.code);
        }
        assert!(text.contains("PRICE SOURCES"), "{text}");
        assert!(text.contains("SUMMARY"), "{text}");
        let selected = model
            .units
            .iter()
            .find(|unit| unit.selected)
            .expect("one is selected");
        assert!(
            text.contains(&selected.name),
            "the summary names it:\n{text}"
        );
    }

    #[test]
    fn the_keys_box_draws_the_page_hints() {
        let view = units_view();
        let text = render(&view, 160, 30);
        for hint in view.view_model().hints {
            assert!(
                text.contains(&hint.action),
                "{} missing:\n{text}",
                hint.action
            );
        }
    }

    #[test]
    fn the_price_history_regions_say_nothing_is_recorded() {
        let text = render(&units_view(), 96, 30);
        assert!(text.contains("WEEKLY CLOSE"), "{text}");
        assert!(text.contains("WEEKLY PRICES"), "{text}");
        assert!(
            text.contains(&msg::tui_units_price_history_empty()),
            "{text}"
        );
    }

    #[test]
    fn the_selection_scrolls_into_view_in_a_short_frame() {
        assert_eq!(scroll_offset(Some(2), 5), 0);
        assert_eq!(scroll_offset(Some(5), 5), 1);
        assert_eq!(scroll_offset(None, 5), 0);
    }

    #[test]
    fn title_is_units_and_prices() {
        let view = units_view();
        assert_eq!(view.title(), "Units & prices");
        assert_eq!(view.id(), ViewId::Units);
    }
}
