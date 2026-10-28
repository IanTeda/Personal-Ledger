//! The Settings `View`, hosted by `Shell` (ADR-0013): the paged design of
//! `docs/ux/tui-mockups/05-settings/README.md`, at Medium (ADR-0050). Settings shows one of two
//! places in the View: the full-page index (TUI-047, [`index`]) or one page under its breadcrumb
//! heading, and on a page `p` opens the page picker (TUI-048, [`picker`]) over it.
//!
//! The index selection and the open page are one value, [`SettingsView::selected`], shared with
//! `Shell` so it is remembered for the session: `g s` and `:settings` land on the index with the
//! page last left selected. Opening a page, `[` / `]` and the picker swap what Settings shows in
//! place; none of them push onto the Shell's view stack, so the back key on a page always returns
//! to the index ([`View::back`]).
//!
//! What's real: [`display`]'s Toasts, Colour Theme, Appearance and Locale rows. [`general`] is a
//! wireframe form until Stage 2 of the README's work plan (`CLAUDE_CODE.md`). Every other page
//! draws its heading and scope note over an empty body until the stage that builds it.

mod display;
mod general;
mod index;
mod picker;

use std::cell::Cell;
use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent};
use lib_list_cursor::Step;
use lib_shell::WindowSize;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::colours::Colours;
use crate::msg;
use crate::view::{Action, View, ViewId};

use picker::Picker;

/// The sixteen Settings pages. [`SettingsPage::ALL`] is the one page order the index, the picker
/// and `[` / `]` all read, so the three can't drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsPage {
    #[default]
    General,
    Display,
    Units,
    Institutions,
    Accounts,
    Categories,
    Tags,
    Payees,
    Documents,
    Inventory,
    AiConnections,
    SyncServer,
    Backup,
    History,
    Logs,
    About,
}

/// Where a page's values live, drawn as the index's one-glyph scope column. Never colour alone:
/// the two glyphs are distinct shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Stored in the Ledger and synced as Change Sets (`●`).
    Synced,
    /// Configuration on this device, never synced (`○`).
    ThisDevice,
    /// Nothing stored (About).
    None,
}

impl Scope {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Synced => "●",
            Self::ThisDevice => "○",
            Self::None => " ",
        }
    }
}

impl SettingsPage {
    /// Every page, in the index's order: the Desktop's, with AI Connections and History in the
    /// places the TUI handoff gives them.
    pub const ALL: [SettingsPage; 16] = [
        Self::General,
        Self::Display,
        Self::Units,
        Self::Institutions,
        Self::Accounts,
        Self::Categories,
        Self::Tags,
        Self::Payees,
        Self::Documents,
        Self::Inventory,
        Self::AiConnections,
        Self::SyncServer,
        Self::Backup,
        Self::History,
        Self::Logs,
        Self::About,
    ];

    /// This page's position in [`Self::ALL`].
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|page| *page == self)
            .unwrap_or_default()
    }

    /// The page `step` moves to from this one, clamped at both ends.
    fn step(self, step: Step) -> Self {
        Self::ALL[lib_list_cursor::step(self.index(), Self::ALL.len(), step)]
    }

    /// The name the index, the picker and the breadcrumb show.
    pub fn label(self) -> String {
        match self {
            Self::General => msg::tui_settings_page_general(),
            Self::Display => msg::tui_settings_page_display(),
            Self::Units => msg::tui_settings_page_units(),
            Self::Institutions => msg::tui_settings_page_institutions(),
            Self::Accounts => msg::tui_settings_page_accounts(),
            Self::Categories => msg::tui_settings_page_categories(),
            Self::Tags => msg::tui_settings_page_tags(),
            Self::Payees => msg::tui_settings_page_payees(),
            Self::Documents => msg::tui_settings_page_documents(),
            Self::Inventory => msg::tui_settings_page_inventory(),
            Self::AiConnections => msg::tui_settings_page_ai_connections(),
            Self::SyncServer => msg::tui_settings_page_sync_server(),
            Self::Backup => msg::tui_settings_page_backup(),
            Self::History => msg::tui_settings_page_history(),
            Self::Logs => msg::tui_settings_page_logs(),
            Self::About => msg::tui_settings_page_about(),
        }
    }

    /// The one-line description beside the name on the index page.
    pub fn description(self) -> String {
        match self {
            Self::General => msg::tui_settings_description_general(),
            Self::Display => msg::tui_settings_description_display(),
            Self::Units => msg::tui_settings_description_units(),
            Self::Institutions => msg::tui_settings_description_institutions(),
            Self::Accounts => msg::tui_settings_description_accounts(),
            Self::Categories => msg::tui_settings_description_categories(),
            Self::Tags => msg::tui_settings_description_tags(),
            Self::Payees => msg::tui_settings_description_payees(),
            Self::Documents => msg::tui_settings_description_documents(),
            Self::Inventory => msg::tui_settings_description_inventory(),
            Self::AiConnections => msg::tui_settings_description_ai_connections(),
            Self::SyncServer => msg::tui_settings_description_sync_server(),
            Self::Backup => msg::tui_settings_description_backup(),
            Self::History => msg::tui_settings_description_history(),
            Self::Logs => msg::tui_settings_description_logs(),
            Self::About => msg::tui_settings_description_about(),
        }
    }

    /// Where the page's values live, by the Preference / Configuration split (ADR-0014). Display
    /// mixes both, so it takes the scope of most of its rows and marks each row itself.
    pub fn scope(self) -> Scope {
        match self {
            Self::General
            | Self::Units
            | Self::Institutions
            | Self::Accounts
            | Self::Categories
            | Self::Tags
            | Self::Payees
            | Self::Documents
            | Self::Inventory
            | Self::History => Scope::Synced,
            Self::Display | Self::AiConnections | Self::SyncServer | Self::Backup | Self::Logs => {
                Scope::ThisDevice
            }
            Self::About => Scope::None,
        }
    }

    /// The page heading's right-aligned scope note. Counts join it as each page's stage derives
    /// them from its rows; none is stored here.
    pub fn scope_note(self) -> String {
        match self {
            Self::General => msg::tui_settings_scope_general(),
            Self::Display => msg::tui_settings_scope_display(),
            Self::Units
            | Self::Institutions
            | Self::Accounts
            | Self::Categories
            | Self::Tags
            | Self::Payees
            | Self::Documents
            | Self::Inventory => msg::tui_settings_scope_ledger_data(),
            Self::AiConnections => msg::tui_settings_scope_ai_connections(),
            Self::SyncServer => msg::tui_settings_scope_sync_server(),
            Self::Backup => msg::tui_settings_scope_backup(),
            Self::History => msg::tui_settings_scope_history(),
            Self::Logs => msg::tui_settings_scope_logs(),
            Self::About => msg::tui_settings_scope_about(),
        }
    }

    /// The name `:settings <page>` takes: command syntax the user types, so stable English
    /// rather than a Message, as command names are.
    pub fn command_name(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Display => "display",
            Self::Units => "units",
            Self::Institutions => "institutions",
            Self::Accounts => "accounts",
            Self::Categories => "categories",
            Self::Tags => "tags",
            Self::Payees => "payees",
            Self::Documents => "documents",
            Self::Inventory => "inventory",
            Self::AiConnections => "ai-connections",
            Self::SyncServer => "sync-server",
            Self::Backup => "backup",
            Self::History => "history",
            Self::Logs => "logs",
            Self::About => "about",
        }
    }

    /// The page `:settings <page>` names, ignoring case.
    pub fn from_command_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|page| page.command_name().eq_ignore_ascii_case(name))
    }
}

/// Which of Settings' two places shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Showing {
    #[default]
    Index,
    Page,
}

/// The Settings View: the index page or one page, the picker, and each built page's own state.
pub struct SettingsView {
    showing: Showing,
    /// The index selection and the open page, one value shared with `Shell` for the session.
    selected: Rc<Cell<SettingsPage>>,
    /// `Some` while the page picker is open.
    picker: Option<Picker>,
    general: general::GeneralPage,
    display: display::DisplayPage,
}

impl SettingsView {
    /// A View on its own selection, starting at General. `Shell` uses [`Self::remembering`].
    #[cfg(test)]
    pub fn new() -> Self {
        Self::remembering(Rc::default())
    }

    /// A View on the session's selection, landing on the index with it selected.
    pub fn remembering(selected: Rc<Cell<SettingsPage>>) -> Self {
        Self {
            showing: Showing::Index,
            selected,
            picker: None,
            general: general::GeneralPage::default(),
            display: display::DisplayPage::new(),
        }
    }

    fn selected(&self) -> SettingsPage {
        self.selected.get()
    }

    /// Shows `page` in place of whatever was showing, closing the picker.
    fn open_page(&mut self, page: SettingsPage) {
        self.selected.set(page);
        self.showing = Showing::Page;
        self.picker = None;
    }

    fn handle_index_key(&mut self, key: KeyEvent) -> Option<Action> {
        let step = match key.code {
            KeyCode::Char('j') | KeyCode::Down => Step::Next,
            KeyCode::Char('k') | KeyCode::Up => Step::Prev,
            // Bare `g` is the Shell's jump leader, so `Home` stands in for it, as on every list
            // View (`view::accounts`).
            KeyCode::Home => Step::First,
            KeyCode::Char('G') | KeyCode::End => Step::Last,
            KeyCode::Enter | KeyCode::Char('l') => {
                self.open_page(self.selected());
                return Some(Action::NoOp);
            }
            _ => return None,
        };
        self.selected.set(self.selected().step(step));
        Some(Action::NoOp)
    }

    fn handle_page_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('[') => {
                self.selected.set(self.selected().step(Step::Prev));
                Some(Action::NoOp)
            }
            KeyCode::Char(']') => {
                self.selected.set(self.selected().step(Step::Next));
                Some(Action::NoOp)
            }
            KeyCode::Char('p') => {
                self.picker = Some(Picker::new(self.selected()));
                Some(Action::NoOp)
            }
            _ => match self.selected() {
                SettingsPage::General => self.general.handle_key(key),
                SettingsPage::Display => self.display.handle_key(key),
                _ => None,
            },
        }
    }

    /// The page's own hints, before the paging hints every page ends with.
    fn page_hints(&self) -> Vec<(String, String)> {
        match self.selected() {
            SettingsPage::General | SettingsPage::Display => vec![
                ("j/k".into(), msg::tui_settings_hint_field()),
                ("enter".into(), msg::tui_settings_hint_edit()),
            ],
            _ => Vec::new(),
        }
    }
}

impl View for SettingsView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(picker) = &mut self.picker {
            return match picker.handle_key(key) {
                picker::Outcome::Open(page) => {
                    self.open_page(page);
                    Some(Action::NoOp)
                }
                picker::Outcome::Close => {
                    self.picker = None;
                    Some(Action::NoOp)
                }
                picker::Outcome::Moved => Some(Action::NoOp),
                picker::Outcome::Ignored => None,
            };
        }
        match self.showing {
            Showing::Index => self.handle_index_key(key),
            Showing::Page => self.handle_page_key(key),
        }
    }

    fn update(&mut self, action: &Action) {
        match action {
            // `g s` and `:settings` land on the index, even from a page of an open Settings.
            Action::OpenSettings => {
                self.showing = Showing::Index;
                self.picker = None;
            }
            Action::OpenSettingsPage(page) => self.open_page(*page),
            _ => {}
        }
    }

    /// The picker closes first, then a page steps back to the index with itself selected; on the
    /// index the Shell pops its view stack.
    fn back(&mut self) -> bool {
        if self.picker.take().is_some() {
            return true;
        }
        match self.showing {
            Showing::Page => {
                self.showing = Showing::Index;
                true
            }
            Showing::Index => false,
        }
    }

    fn popup_open(&self) -> bool {
        self.picker.is_some()
    }

    fn hints(&self, back: &str) -> Option<Vec<(String, String)>> {
        let hints = if self.picker.is_some() {
            picker::hints(back)
        } else {
            match self.showing {
                Showing::Index => index::hints(back),
                Showing::Page => {
                    let mut hints = self.page_hints();
                    hints.extend([
                        ("[/]".into(), msg::tui_settings_hint_pages()),
                        ("p".into(), msg::tui_settings_hint_picker()),
                        (back.to_string(), msg::tui_settings_hint_settings()),
                    ]);
                    hints
                }
            }
        };
        Some(hints)
    }

    fn set_toasts_on(&mut self, on: bool) {
        self.display.set_toasts_on(on);
    }

    fn view(&self, frame: &mut Frame<'_>, area: Rect, _size: WindowSize, c: &Colours) {
        let inner = area.inner(Margin::new(1, 0));
        match self.showing {
            Showing::Index => index::render(frame, inner, self.selected(), c),
            Showing::Page => self.render_page(frame, inner, c),
        }
        if let Some(picker) = &self.picker {
            frame.render_widget(crate::popup::Dim, area);
            picker.render(frame, area, self.selected(), c);
        }
    }

    fn id(&self) -> ViewId {
        ViewId::Settings
    }

    fn title(&self) -> String {
        lib_locale::msg::nav_settings()
    }
}

impl SettingsView {
    /// A page: the breadcrumb heading over a strong rule, then the whole View below it for the
    /// page's body. A page not built yet draws its heading over an empty body.
    fn render_page(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let page = self.selected();
        let [heading, body] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
        let title = Line::from(vec![
            Span::styled(msg::tui_settings_breadcrumb_parent(), c.muted()),
            Span::raw(" "),
            Span::styled(page.label(), Style::default().add_modifier(Modifier::BOLD)),
        ]);
        render_heading(frame, heading, title, &page.scope_note(), c);
        let body = body.inner(Margin::new(0, 1));
        match page {
            SettingsPage::General => self.general.render(frame, body, c),
            SettingsPage::Display => self.display.render(frame, body, c),
            _ => {}
        }
    }
}

/// A heading row, `title` on the left and `note` dim on the right, over a strong rule: the index
/// page's, the picker's and every page's.
fn render_heading(frame: &mut Frame<'_>, area: Rect, title: Line<'_>, note: &str, c: &Colours) {
    let block = Block::new()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Thick);
    let row = block.inner(area);
    frame.render_widget(block, area);
    let note_width = u16::try_from(note.chars().count()).unwrap_or(u16::MAX);
    let [left, right] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(note_width)]).areas(row);
    frame.render_widget(Paragraph::new(title), left);
    frame.render_widget(Paragraph::new(Span::styled(note, c.muted())), right);
}

/// The label column of a form page's field rows.
const FIELD_LABEL_WIDTH: u16 = 22;

/// The value column of a form page's field rows.
const FIELD_VALUE_WIDTH: u16 = 28;

/// One row of a form page: an optional scope glyph (Display marks each row, as it mixes scopes),
/// the label, the value and a dim note.
struct FieldRow {
    glyph: Option<Scope>,
    label: String,
    value: String,
    note: String,
}

/// A form page's field rows from the top of `area`, the `selected` row reversed.
fn render_fields(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[FieldRow],
    selected: usize,
    c: &Colours,
) {
    let areas = Layout::vertical(vec![Constraint::Length(1); rows.len()]).split(area);
    for (index, (row, row_area)) in rows.iter().zip(areas.iter()).enumerate() {
        let selected = index == selected;
        if selected {
            frame.render_widget(Block::new().style(c.selection()), *row_area);
        }
        let dim = if selected {
            Style::default()
        } else {
            c.muted()
        };
        let glyph_width = if row.glyph.is_some() { 2 } else { 0 };
        let [glyph, label, value, note] = Layout::horizontal([
            Constraint::Length(glyph_width),
            Constraint::Length(FIELD_LABEL_WIDTH),
            Constraint::Length(FIELD_VALUE_WIDTH),
            Constraint::Min(0),
        ])
        .areas(*row_area);
        if let Some(scope) = row.glyph {
            frame.render_widget(Paragraph::new(scope.glyph()), glyph);
        }
        frame.render_widget(Paragraph::new(Span::styled(row.label.as_str(), dim)), label);
        frame.render_widget(
            Paragraph::new(clip(&row.value, usize::from(value.width).saturating_sub(1))),
            value,
        );
        frame.render_widget(
            Paragraph::new(Span::styled(clip(&row.note, usize::from(note.width)), dim)),
            note,
        );
    }
}

/// `text` cut to `width` cells, ending in `…` when it doesn't fit: cells clip, they don't wrap.
fn clip(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut clipped: String = text.chars().take(width.saturating_sub(1)).collect();
    clipped.push('…');
    clipped
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn press(view: &mut SettingsView, codes: &[KeyCode]) {
        for code in codes {
            view.handle_key(key(*code));
        }
    }

    pub(super) fn render_at(view: &SettingsView, width: u16, height: u16) -> String {
        let c = &Colours::default();
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area(), WindowSize::Medium, c))
            .expect("drawing the settings view should not error");
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

    fn render(view: &SettingsView) -> String {
        render_at(view, 96, 28)
    }

    #[test]
    fn the_page_order_lists_each_page_once() {
        for (position, page) in SettingsPage::ALL.into_iter().enumerate() {
            assert_eq!(page.index(), position, "{page:?} is listed twice");
        }
    }

    #[test]
    fn every_page_has_a_distinct_command_name_that_round_trips() {
        let names: std::collections::HashSet<_> = SettingsPage::ALL
            .iter()
            .map(|page| page.command_name())
            .collect();
        assert_eq!(names.len(), SettingsPage::ALL.len());
        for page in SettingsPage::ALL {
            assert_eq!(
                SettingsPage::from_command_name(page.command_name()),
                Some(page)
            );
        }
        assert_eq!(
            SettingsPage::from_command_name("Sync-Server"),
            Some(SettingsPage::SyncServer)
        );
        assert_eq!(SettingsPage::from_command_name("nope"), None);
    }

    #[test]
    fn it_opens_on_the_index_at_general() {
        let view = SettingsView::new();
        assert_eq!(view.showing, Showing::Index);
        assert_eq!(view.selected(), SettingsPage::General);
        assert_eq!(view.id(), ViewId::Settings);
    }

    #[test]
    fn on_the_index_j_k_home_and_g_move_the_selection_and_clamp() {
        let mut view = SettingsView::new();
        press(&mut view, &[KeyCode::Char('k')]);
        assert_eq!(view.selected(), SettingsPage::General);
        press(&mut view, &[KeyCode::Char('j'), KeyCode::Down]);
        assert_eq!(view.selected(), SettingsPage::Units);
        press(&mut view, &[KeyCode::Char('G')]);
        assert_eq!(view.selected(), SettingsPage::About);
        press(&mut view, &[KeyCode::Char('j')]);
        assert_eq!(view.selected(), SettingsPage::About);
        press(&mut view, &[KeyCode::Home]);
        assert_eq!(view.selected(), SettingsPage::General);
        assert_eq!(view.showing, Showing::Index);
    }

    #[test]
    fn enter_or_l_opens_the_selected_page_and_back_returns_with_it_selected() {
        for open in [KeyCode::Enter, KeyCode::Char('l')] {
            let mut view = SettingsView::new();
            press(&mut view, &[KeyCode::Char('j'), KeyCode::Char('j'), open]);
            assert_eq!(view.showing, Showing::Page);
            assert_eq!(view.selected(), SettingsPage::Units);

            assert!(view.back(), "back on a page stays in Settings");
            assert_eq!(view.showing, Showing::Index);
            assert_eq!(view.selected(), SettingsPage::Units);
            assert!(!view.back(), "back on the index leaves Settings");
        }
    }

    #[test]
    fn brackets_step_pages_and_are_inert_at_both_ends() {
        let mut view = SettingsView::new();
        press(&mut view, &[KeyCode::Enter, KeyCode::Char('[')]);
        assert_eq!(view.selected(), SettingsPage::General);
        press(&mut view, &[KeyCode::Char(']')]);
        assert_eq!(view.selected(), SettingsPage::Display);
        press(&mut view, &[KeyCode::Char('[')]);
        assert_eq!(view.selected(), SettingsPage::General);

        view.update(&Action::OpenSettingsPage(SettingsPage::About));
        press(&mut view, &[KeyCode::Char(']')]);
        assert_eq!(view.selected(), SettingsPage::About);
        press(&mut view, &[KeyCode::Char('[')]);
        assert_eq!(view.selected(), SettingsPage::Logs);
        assert_eq!(view.showing, Showing::Page);
    }

    #[test]
    fn brackets_and_p_do_nothing_on_the_index() {
        let mut view = SettingsView::new();
        assert_eq!(view.handle_key(key(KeyCode::Char(']'))), None);
        assert_eq!(view.handle_key(key(KeyCode::Char('p'))), None);
        assert_eq!(view.selected(), SettingsPage::General);
        assert!(!view.popup_open());
    }

    #[test]
    fn the_picker_opens_on_the_current_page_moves_and_opens_another() {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Tags));
        press(&mut view, &[KeyCode::Char('p')]);
        assert!(view.popup_open());
        assert_eq!(
            view.picker.as_ref().map(Picker::selected),
            Some(SettingsPage::Tags)
        );

        press(
            &mut view,
            &[KeyCode::Char('j'), KeyCode::Down, KeyCode::Char('k')],
        );
        assert_eq!(
            view.picker.as_ref().map(Picker::selected),
            Some(SettingsPage::Payees)
        );
        press(&mut view, &[KeyCode::Char('G')]);
        assert_eq!(
            view.picker.as_ref().map(Picker::selected),
            Some(SettingsPage::About)
        );
        press(&mut view, &[KeyCode::Char('g')]);
        assert_eq!(
            view.picker.as_ref().map(Picker::selected),
            Some(SettingsPage::General)
        );
        press(
            &mut view,
            &[KeyCode::Up, KeyCode::Char('j'), KeyCode::Enter],
        );

        assert!(!view.popup_open());
        assert_eq!(view.showing, Showing::Page);
        assert_eq!(view.selected(), SettingsPage::Display);
    }

    #[test]
    fn esc_closes_the_picker_and_stays_on_the_page() {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Backup));
        press(
            &mut view,
            &[KeyCode::Char('p'), KeyCode::Char('j'), KeyCode::Esc],
        );
        assert!(!view.popup_open());
        assert_eq!(view.showing, Showing::Page);
        assert_eq!(view.selected(), SettingsPage::Backup);
    }

    #[test]
    fn back_closes_the_picker_before_it_leaves_the_page() {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Logs));
        press(&mut view, &[KeyCode::Char('p')]);
        assert!(view.back());
        assert!(!view.popup_open());
        assert_eq!(view.showing, Showing::Page);
        assert!(view.back());
        assert_eq!(view.showing, Showing::Index);
    }

    #[test]
    fn open_settings_returns_a_page_to_the_index_and_the_selection_is_shared() {
        let remembered: Rc<Cell<SettingsPage>> = Rc::default();
        let mut view = SettingsView::remembering(Rc::clone(&remembered));
        view.update(&Action::OpenSettingsPage(SettingsPage::History));
        press(&mut view, &[KeyCode::Char('p')]);
        view.update(&Action::OpenSettings);
        assert_eq!(view.showing, Showing::Index);
        assert!(!view.popup_open());
        assert_eq!(remembered.get(), SettingsPage::History);

        let again = SettingsView::remembering(remembered);
        assert_eq!(again.selected(), SettingsPage::History);
        assert_eq!(again.showing, Showing::Index);
    }

    #[test]
    fn the_index_lists_every_page_with_its_glyph_and_description() {
        crate::locale::init_for_tests();
        let text = render(&SettingsView::new());
        assert!(text.contains("Settings"), "{text}");
        assert!(text.contains("16 pages"), "{text}");
        for page in SettingsPage::ALL {
            let row = text
                .lines()
                .find(|line| line.contains(&page.label()))
                .unwrap_or_else(|| panic!("{page:?} missing:\n{text}"));
            assert!(
                row.contains(page.scope().glyph()),
                "{page:?}'s glyph:\n{row}"
            );
            let start: String = page.description().chars().take(20).collect();
            assert!(row.contains(&start), "{page:?}'s description:\n{row}");
        }
    }

    #[test]
    fn the_index_pins_its_legend_to_the_bottom_on_a_taller_terminal() {
        crate::locale::init_for_tests();
        for height in [28, 50] {
            let text = render_at(&SettingsView::new(), 96, height);
            let last = text.lines().last().unwrap_or_default();
            assert!(last.contains("syncs with the ledger"), "{height}:\n{text}");
            assert!(last.contains("this device only"), "{height}:\n{text}");
        }
    }

    #[test]
    fn the_index_reverses_the_selected_row_only() {
        crate::locale::init_for_tests();
        let c = Colours::default();
        let mut view = SettingsView::new();
        press(&mut view, &[KeyCode::Char('j')]);
        let mut terminal =
            Terminal::new(TestBackend::new(96, 28)).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area(), WindowSize::Medium, &c))
            .expect("drawing the settings view should not error");
        let buffer = terminal.backend().buffer();
        let selection = c.selection().bg.unwrap_or_default();
        let reversed = |label: &str| {
            (0..buffer.area.height)
                .find(|&y| {
                    let row: String = (0..buffer.area.width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect();
                    row.contains(label)
                })
                .is_some_and(|y| {
                    (0..buffer.area.width).all(|x| {
                        buffer[(x, y)].bg == selection || x == 0 || x == buffer.area.width - 1
                    })
                })
        };
        assert!(reversed(&SettingsPage::Display.label()));
        assert!(!reversed(&SettingsPage::General.label()));
    }

    #[test]
    fn a_page_draws_its_breadcrumb_and_scope_note() {
        crate::locale::init_for_tests();
        for page in SettingsPage::ALL {
            let mut view = SettingsView::new();
            view.update(&Action::OpenSettingsPage(page));
            let text = render(&view);
            let heading = text.lines().next().unwrap_or_default();
            assert!(
                heading.contains(&format!("Settings › {}", page.label())),
                "{heading}"
            );
            assert!(heading.contains(&page.scope_note()), "{heading}");
        }
    }

    #[test]
    fn the_picker_draws_every_page_and_marks_the_open_one() {
        crate::locale::init_for_tests();
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Display));
        press(&mut view, &[KeyCode::Char('p')]);
        let text = render(&view);
        assert!(text.contains("Go to Settings Page"), "{text}");
        for page in SettingsPage::ALL {
            assert!(text.contains(&page.label()), "{page:?} missing:\n{text}");
        }
        let marked = text
            .lines()
            .find(|line| line.contains('•'))
            .unwrap_or_default();
        assert!(marked.contains(&SettingsPage::Display.label()), "{text}");
    }

    #[test]
    fn hints_end_with_the_paging_keys_on_a_page() {
        crate::locale::init_for_tests();
        let mut view = SettingsView::new();
        let index = view.hints("esc").unwrap_or_default();
        assert!(
            index
                .iter()
                .any(|(key, label)| key == "enter" && label == "open")
        );

        view.update(&Action::OpenSettingsPage(SettingsPage::Tags));
        let hints = view.hints("esc").unwrap_or_default();
        let tail: Vec<_> = hints
            .iter()
            .rev()
            .take(3)
            .rev()
            .map(|(key, label)| format!("{key} {label}"))
            .collect();
        assert_eq!(tail, ["[/] pages", "p picker", "esc settings"]);
    }

    #[test]
    fn clip_ends_in_an_ellipsis_only_when_it_cuts() {
        assert_eq!(clip("units", 5), "units");
        assert_eq!(clip("units and prices", 6), "units…");
    }
}
