//! The Settings `View`, hosted by `Shell` (ADR-0013): the paged design of
//! `docs/ux/tui-mockups/05-settings/README.md`, per Window Size (ADR-0050). Settings has two
//! places, the index ([`index`]) and one page. At Medium they show one at a time: the full-page
//! index (TUI-047), or the page under its breadcrumb heading, where `p` opens the page picker
//! (TUI-048, [`picker`]) over it. At Large and X-Large they show side by side, the index docked
//! at 31 columns (TUI-050, TUI-051), and which one shows becomes which one has focus; the page's
//! heading is its name alone, and `p` does nothing, since every page is in view.
//!
//! The Window Size is read from the width on every draw ([`View::set_window_size`]), so a resize
//! across a Breakpoint keeps the page, its selection and the focus. A page's content stops at
//! 112 columns, the Desktop's 840px, at every size.
//!
//! The index selection and the open page are one value, [`SettingsView::selected`], shared with
//! `Shell` so it is remembered for the session: `g s` and `:settings` land on the index with the
//! page last left selected. Opening a page, `[` / `]` and the picker swap what Settings shows in
//! place; none of them push onto the Shell's view stack, so the back key on a page always returns
//! to the index ([`View::back`]).
//!
//! General and Display are form pages: rows of label, value and dim note, where `enter` opens a
//! field in place (TUI-020, `popup::settings::edit`) and the status line shows `INSERT` while it
//! is open. Opening and saving are one [`Action::SetSetting`] ([`field`]): the page returns it
//! without a value to open the field, the open field returns it with one to save, and `Shell`
//! routes it, keeping what it owns (Toasts, the Colour Theme and Appearance, the base unit guard)
//! and handing the rest back to [`View::update`]. What holds for the session only lives in
//! [`field::SettingsValues`], shared with `Shell` as the selection is.
//!
//! Units, Accounts and the six ledger-data pages after them are list pages, drawn by the one
//! list-page pattern ([`list`]). Units hosts the Units View itself ([`units`]), so its keys and
//! Dialogs are `lib-units`'; Accounts reads a fixture until the TUI moves onto `lib-accounts`
//! ([`accounts`]), and Institutions, Categories, Tags, Payees, Documents and Inventory do the same
//! on [`list::Page`]. `o` opens the full View where there is one, which `Shell` pushes Settings
//! under, so the back key there returns to the same page and row. Every other page draws its
//! heading and scope note over an empty body until the stage that builds it.

mod accounts;
mod categories;
mod display;
mod documents;
pub mod field;
mod general;
mod index;
mod institutions;
mod inventory;
mod list;
mod payees;
mod picker;
mod tags;
mod units;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent};
use lib_list_cursor::Step;
use lib_shell::WindowSize;
use lib_toast::ToastKind;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::colours::Colours;
use crate::msg;
use crate::popup::settings::edit::{FieldEditor, Outcome};
use crate::units::{TuiPorts, UnitsSetup};
use crate::view::{Action, View, ViewId};

use field::{SettingField, SettingsValues};
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

/// A page's content width at most, the Desktop's 840px at 7.5px a column.
const CONTENT_WIDTH: u16 = 112;

/// The Settings View: the index page or one page, the picker, and each built page's own state.
pub struct SettingsView {
    /// At Medium the place showing; at Large and X-Large, where both show, the one with focus.
    showing: Showing,
    /// The Window Size `Shell` last drew at, which picks the layout the keys and hints act on.
    size: WindowSize,
    /// The index selection and the open page, one value shared with `Shell` for the session.
    selected: Rc<Cell<SettingsPage>>,
    /// `Some` while the page picker is open.
    picker: Option<Picker>,
    /// The values that hold for the session, shared with `Shell` as `selected` is.
    values: Rc<RefCell<SettingsValues>>,
    /// The Open Ledger's Units ports, for the base unit; `None` for a View built without one.
    units: Option<TuiPorts>,
    /// `Some` while a field is open in place.
    editor: Option<FieldEditor>,
    general: general::GeneralPage,
    display: display::DisplayPage,
    /// Settings › Units, on the Open Ledger's Units; `None` for a View built without one.
    units_page: Option<units::UnitsPage>,
    accounts: accounts::AccountsPage,
    institutions: institutions::InstitutionsPage,
    categories: categories::CategoriesPage,
    tags: tags::TagsPage,
    payees: payees::PayeesPage,
    documents: documents::DocumentsPage,
    inventory: inventory::InventoryPage,
}

impl SettingsView {
    /// A View on its own selection and values, starting at General, with no Ledger's Units.
    /// `Shell` uses [`Self::remembering`].
    #[cfg(test)]
    pub fn new() -> Self {
        Self::remembering(Rc::default(), Rc::default(), None)
    }

    /// [`Self::new`] on `units`.
    #[cfg(test)]
    pub fn with_units(units: &UnitsSetup) -> Self {
        Self::remembering(Rc::default(), Rc::default(), Some(units))
    }

    /// A View on the session's selection and values, landing on the index with the selection
    /// selected; `units` are the Open Ledger's.
    pub fn remembering(
        selected: Rc<Cell<SettingsPage>>,
        values: Rc<RefCell<SettingsValues>>,
        units: Option<&UnitsSetup>,
    ) -> Self {
        Self {
            showing: Showing::Index,
            size: WindowSize::Medium,
            selected,
            picker: None,
            values,
            units: units.map(|setup| setup.ports.clone()),
            editor: None,
            general: general::GeneralPage::default(),
            display: display::DisplayPage::new(),
            units_page: units.map(units::UnitsPage::new),
            accounts: accounts::AccountsPage::new(),
            institutions: institutions::InstitutionsPage::new(),
            categories: categories::CategoriesPage::new(),
            tags: tags::TagsPage::new(),
            payees: payees::PayeesPage::new(),
            documents: documents::DocumentsPage::new(),
            inventory: inventory::InventoryPage::new(),
        }
    }

    /// The ledger-data list page for `page`, among those on the shared [`list::Page`] pattern.
    fn ledger_page(&self, page: SettingsPage) -> Option<&dyn list::Page> {
        Some(match page {
            SettingsPage::Institutions => &self.institutions,
            SettingsPage::Categories => &self.categories,
            SettingsPage::Tags => &self.tags,
            SettingsPage::Payees => &self.payees,
            SettingsPage::Documents => &self.documents,
            SettingsPage::Inventory => &self.inventory,
            _ => return None,
        })
    }

    fn ledger_page_mut(&mut self, page: SettingsPage) -> Option<&mut dyn list::Page> {
        Some(match page {
            SettingsPage::Institutions => &mut self.institutions,
            SettingsPage::Categories => &mut self.categories,
            SettingsPage::Tags => &mut self.tags,
            SettingsPage::Payees => &mut self.payees,
            SettingsPage::Documents => &mut self.documents,
            SettingsPage::Inventory => &mut self.inventory,
            _ => return None,
        })
    }

    fn selected(&self) -> SettingsPage {
        self.selected.get()
    }

    /// Whether the index docks beside the page: at Large and X-Large.
    fn docked(&self) -> bool {
        self.size != WindowSize::Medium
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
            // Docked, every page is in view, so there is no picker; `p` is still taken, so it
            // does nothing rather than reach the Shell.
            KeyCode::Char('p') => {
                if !self.docked() {
                    self.picker = Some(Picker::new(self.selected()));
                }
                Some(Action::NoOp)
            }
            _ => match self.selected() {
                SettingsPage::General => self.general.handle_key(key),
                SettingsPage::Display => self.display.handle_key(key),
                SettingsPage::Units => self.units_page.as_mut()?.handle_key(key),
                SettingsPage::Accounts => self.accounts.handle_key(key),
                page => self.ledger_page_mut(page)?.handle_key(key),
            },
        }
    }

    /// The page's own hints, before the paging hints every page ends with.
    fn page_hints(&self) -> Vec<(String, String)> {
        let mut hints = match self.selected() {
            SettingsPage::General | SettingsPage::Display => vec![
                ("j/k".into(), msg::tui_settings_hint_field()),
                ("enter".into(), msg::tui_settings_hint_edit()),
            ],
            SettingsPage::Units => self
                .units_page
                .as_ref()
                .map(units::UnitsPage::hints)
                .unwrap_or_default(),
            SettingsPage::Accounts => accounts::AccountsPage::hints(),
            page => self
                .ledger_page(page)
                .map(list::Page::hints)
                .unwrap_or_default(),
        };
        if self.selected() == SettingsPage::Display {
            hints.push(("space".into(), msg::tui_settings_hint_toggle()));
        }
        hints
    }

    /// Hands `key` to the open field: `enter` and `tab` save through [`Action::SetSetting`], which
    /// comes back to [`View::update`] to close it, and `esc` closes it at once.
    fn handle_editor_key(&mut self, key: KeyEvent) -> Option<Action> {
        let editor = self.editor.as_mut()?;
        let field = editor.field();
        Some(match editor.handle_key(key) {
            Outcome::Save(value) => Action::SetSetting(field, Some(value)),
            Outcome::SaveAndNext(value) => {
                match field.page() {
                    SettingsPage::General => self.general.step(Step::Next),
                    _ => self.display.step(Step::Next),
                }
                Action::SetSetting(field, Some(value))
            }
            Outcome::Keep => {
                self.editor = None;
                Action::NoOp
            }
            Outcome::Stay => Action::NoOp,
        })
    }

    /// Opens `field` in place on its page, selecting its row; a field that doesn't open in place
    /// (a popup's, or a read-only one) is left to `Shell`.
    fn open_field(&mut self, field: SettingField) {
        let opened = {
            let values = self.values.borrow();
            let context = self.context(&values);
            match field.page() {
                SettingsPage::General => general::draft(field, &context),
                _ => self.display.draft(field, &context),
            }
        };
        let Some((draft, was)) = opened else {
            return;
        };
        self.open_page(field.page());
        match field.page() {
            SettingsPage::General => self.general.select(field),
            _ => self.display.select(field),
        }
        self.editor = Some(FieldEditor::new(field, draft, was));
    }

    /// What a form page reads on this draw.
    fn context<'a>(&'a self, values: &'a SettingsValues) -> Context<'a> {
        Context {
            values,
            units: self.units.as_ref(),
            editor: self.editor.as_ref(),
        }
    }
}

/// What General and Display read on each draw besides their own selection.
struct Context<'a> {
    values: &'a SettingsValues,
    /// The Open Ledger's Units ports; `None` for a View built without them.
    units: Option<&'a TuiPorts>,
    /// The field open in place, if any.
    editor: Option<&'a FieldEditor>,
}

impl View for SettingsView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.editor.is_some() {
            return self.handle_editor_key(key);
        }
        // A Units Dialog takes every key, `[`, `]` and `p` included.
        if let Some(page) = self.units_page.as_mut().filter(|page| page.dialog_open()) {
            return page.handle_key(key);
        }
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
            Action::SetSetting(field, None) => self.open_field(*field),
            Action::SetSetting(field, Some(value)) => {
                if self
                    .editor
                    .as_ref()
                    .is_some_and(|editor| editor.field() == *field)
                {
                    self.editor = None;
                }
                // Only the session's own fields land here; `Shell` keeps the rest.
                self.values.borrow_mut().set(*field, value);
            }
            _ => {}
        }
    }

    /// An open field or the picker closes first, then a page steps back to the index with itself
    /// selected; on the index the Shell pops its view stack.
    fn back(&mut self) -> bool {
        if self.editor.take().is_some() || self.picker.take().is_some() {
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

    /// The picker, or a field open in place, which takes every key as a popup does.
    fn popup_open(&self) -> bool {
        self.picker.is_some() || self.editor.is_some()
    }

    fn editing(&self) -> bool {
        self.editor.is_some()
    }

    /// A Units Dialog opened from Settings › Units, which `Shell` draws over the frame.
    fn dialog_open(&self) -> bool {
        self.units_page
            .as_ref()
            .is_some_and(units::UnitsPage::dialog_open)
    }

    fn render_dialog(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        if let Some(page) = &self.units_page {
            page.render_dialog(frame, area, c);
        }
    }

    fn take_toasts(&mut self) -> Vec<(ToastKind, String)> {
        self.units_page
            .as_mut()
            .map(units::UnitsPage::take_toasts)
            .unwrap_or_default()
    }

    fn hints(&self, back: &str) -> Option<Vec<(String, String)>> {
        let hints = if let Some(editor) = &self.editor {
            editor.hints(back)
        } else if self.picker.is_some() {
            picker::hints(back)
        } else {
            match self.showing {
                Showing::Index => index::hints(back, self.docked()),
                Showing::Page => {
                    let mut hints = self.page_hints();
                    hints.push(("[/]".into(), msg::tui_settings_hint_pages()));
                    if self.docked() {
                        hints.push((back.to_string(), msg::tui_settings_hint_index()));
                    } else {
                        hints.extend([
                            ("p".into(), msg::tui_settings_hint_picker()),
                            (back.to_string(), msg::tui_settings_hint_settings()),
                        ]);
                    }
                    hints
                }
            }
        };
        Some(hints)
    }

    fn set_toasts_on(&mut self, on: bool) {
        self.display.set_toasts_on(on);
    }

    /// Docking closes the picker, which only Medium has; the page and the focus stay.
    fn set_window_size(&mut self, size: WindowSize) {
        self.size = size;
        if self.docked() {
            self.picker = None;
        }
        if let Some(page) = &mut self.units_page {
            page.set_window_size(size);
        }
    }

    fn view(&self, frame: &mut Frame<'_>, area: Rect, size: WindowSize, c: &Colours) {
        let inner = area.inner(Margin::new(1, 0));
        if size == WindowSize::Medium {
            match self.showing {
                Showing::Index => index::render(frame, inner, self.selected(), c),
                Showing::Page => self.render_page(frame, inner, false, true, c),
            }
        } else {
            let index_focused = self.showing == Showing::Index;
            let [dock, _, page] = Layout::horizontal([
                Constraint::Length(index::DOCKED_WIDTH),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .areas(inner);
            index::render_docked(frame, dock, self.selected(), index_focused, c);
            let page = match &self.units_page {
                // X-Large docks the Units detail pane at the View's right edge (TUI-055).
                Some(units) if self.selected() == SettingsPage::Units && units.has_pane() => {
                    let [page, _, pane] = Layout::horizontal([
                        Constraint::Min(0),
                        Constraint::Length(1),
                        Constraint::Length(crate::units::view::pane::WIDTH),
                    ])
                    .areas(page);
                    units.render_pane(frame, pane, c);
                    page
                }
                _ => page,
            };
            self.render_page(frame, page, true, !index_focused, c);
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
    /// A page, at most [`CONTENT_WIDTH`] wide from the left of `area`: its heading over a strong
    /// rule, then the rest below it for the page's body. The heading is the breadcrumb, or,
    /// `docked` beside the index, the page's name alone. Unless `focused`, the page draws no
    /// selection. A page not built yet draws its heading over an empty body.
    fn render_page(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        docked: bool,
        focused: bool,
        c: &Colours,
    ) {
        let page = self.selected();
        let area = Rect {
            width: area.width.min(CONTENT_WIDTH),
            ..area
        };
        let [heading, body] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
        let name = Span::styled(page.label(), Style::default().add_modifier(Modifier::BOLD));
        let title = if docked {
            Line::from(name)
        } else {
            Line::from(vec![
                Span::styled(msg::tui_settings_breadcrumb_parent(), c.muted()),
                Span::raw(" "),
                name,
            ])
        };
        let note = match (page, &self.units_page) {
            (SettingsPage::Units, Some(units)) => units.scope_note(),
            (SettingsPage::Accounts, _) => self.accounts.scope_note(),
            (page, _) if self.ledger_page(page).is_some() => self
                .ledger_page(page)
                .map(list::Page::scope_note)
                .unwrap_or_default(),
            _ => page.scope_note(),
        };
        render_heading(frame, heading, title, &note, c);
        let body = body.inner(Margin::new(0, 1));
        let values = self.values.borrow();
        let context = self.context(&values);
        match page {
            SettingsPage::General => self.general.render(frame, body, focused, &context, c),
            SettingsPage::Display => self.display.render(frame, body, focused, &context, c),
            SettingsPage::Units => {
                if let Some(units) = &self.units_page {
                    units.render(frame, body, focused, c);
                }
            }
            SettingsPage::Accounts => self.accounts.render(frame, body, focused, c),
            page => {
                if let Some(page) = self.ledger_page(page) {
                    page.render(frame, body, focused, c);
                }
            }
        }
    }
}

/// A heading row, `title` on the left and `note` dim on the right, over a strong rule: the index
/// page's, the picker's and every page's.
fn render_heading(frame: &mut Frame<'_>, area: Rect, title: Line<'_>, note: &str, c: &Colours) {
    render_ruled_heading(frame, area, title, note, BorderType::Thick, c);
}

/// [`render_heading`] over a `rule` of the given weight: the docked index's is light while the
/// page beside it has focus.
fn render_ruled_heading(
    frame: &mut Frame<'_>,
    area: Rect,
    title: Line<'_>,
    note: &str,
    rule: BorderType,
    c: &Colours,
) {
    let block = Block::new().borders(Borders::BOTTOM).border_type(rule);
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
/// the field, whose label it shows, its value and a dim note.
struct FieldRow {
    glyph: Option<Scope>,
    field: SettingField,
    value: String,
    note: String,
}

/// The field open in place, with the sub-rows its page gives it.
struct Open<'a> {
    editor: &'a FieldEditor,
    sub_rows: Vec<(String, String)>,
}

/// A form page's field rows from the top of `area`, the `selected` row reversed; `None` while the
/// page doesn't have focus. An `open` field draws its editor in its row's place, and every other
/// row dims. Returns the area below the rows, for what the page draws under them.
fn render_form(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[FieldRow],
    selected: Option<usize>,
    open: Option<Open<'_>>,
    c: &Colours,
) -> Rect {
    let is_open = |row: &FieldRow| {
        open.as_ref()
            .is_some_and(|open| open.editor.field() == row.field)
    };
    let heights: Vec<Constraint> = rows
        .iter()
        .map(|row| match &open {
            Some(open) if is_open(row) => {
                Constraint::Length(FieldEditor::height(open.sub_rows.len()))
            }
            _ => Constraint::Length(1),
        })
        .chain([Constraint::Min(0)])
        .collect();
    let areas = Layout::vertical(heights).split(area);
    for (index, (row, row_area)) in rows.iter().zip(areas.iter()).enumerate() {
        if let Some(open) = open.as_ref().filter(|_| is_open(row)) {
            open.editor.render(frame, *row_area, &open.sub_rows, c);
            continue;
        }
        let selected = open.is_none() && Some(index) == selected;
        if selected {
            frame.render_widget(Block::new().style(c.selection()), *row_area);
        }
        let dim = if selected {
            Style::default()
        } else {
            c.muted()
        };
        let value_style = if open.is_some() {
            c.muted()
        } else {
            Style::default()
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
            frame.render_widget(
                Paragraph::new(Span::styled(scope.glyph(), value_style)),
                glyph,
            );
        }
        frame.render_widget(Paragraph::new(Span::styled(row.field.label(), dim)), label);
        frame.render_widget(
            Paragraph::new(Span::styled(
                clip(&row.value, usize::from(value.width).saturating_sub(1)),
                value_style,
            )),
            value,
        );
        frame.render_widget(
            Paragraph::new(Span::styled(clip(&row.note, usize::from(note.width)), dim)),
            note,
        );
    }
    areas[rows.len()]
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

    /// The stub seed's Units on the stub Ledger, as an Open Ledger holds them.
    pub(super) fn stub_units() -> UnitsSetup {
        UnitsSetup::stub(&lib_config::KeyBindingConfig::default())
    }

    pub(super) fn render_at(view: &SettingsView, width: u16, height: u16) -> String {
        let c = &Colours::default();
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("test backend should initialise");
        terminal
            .draw(|frame| {
                let size = WindowSize::from_columns(frame.area().width);
                view.view(frame, frame.area(), size, c);
            })
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

    /// A view on `page` with the Shell's draw at `size` told it, as `Shell::draw` does.
    fn at(size: WindowSize, page: Option<SettingsPage>) -> SettingsView {
        let mut view = SettingsView::new();
        if let Some(page) = page {
            view.update(&Action::OpenSettingsPage(page));
        }
        view.set_window_size(size);
        view
    }

    /// The symbol at column `x` of row `y` in rendered `text`.
    fn cell(text: &str, x: usize, y: usize) -> String {
        text.lines()
            .nth(y)
            .and_then(|row| row.chars().nth(x))
            .map(String::from)
            .unwrap_or_default()
    }

    #[test]
    fn the_index_docks_only_at_large_and_x_large_and_content_stops_at_112_columns() {
        crate::locale::init_for_tests();
        // TUI-049: Medium at its reference frame, the drill-down. The page starts at column 1,
        // inside the View's margin, so its heading rule ends on column 112.
        let text = render_at(
            &at(WindowSize::Medium, Some(SettingsPage::General)),
            128,
            69,
        );
        assert!(
            text.lines()
                .next()
                .unwrap_or_default()
                .contains("Settings › General"),
            "{text}"
        );
        assert!(!text.contains("○ this device only"), "{text}");
        assert_eq!(cell(&text, 112, 1), "━", "{text}");
        assert_eq!(cell(&text, 113, 1), " ", "{text}");

        // TUI-050 and TUI-052: the index docks at 31 columns, then a gap, then the page from
        // column 33, its heading the name alone, stopping 112 columns on.
        for (width, height) in [(171, 53), (256, 72)] {
            let size = WindowSize::from_columns(width);
            let text = render_at(&at(size, Some(SettingsPage::General)), width, height);
            let heading = text.lines().next().unwrap_or_default();
            assert!(
                heading.contains("Settings") && heading.contains("16 pages"),
                "{text}"
            );
            assert!(!text.contains("Settings ›"), "{text}");
            assert_eq!(
                cell(&text, 31, 0),
                "│",
                "the dock's rule at {width}:\n{text}"
            );
            assert_eq!(
                cell(&text, 33, 0),
                "G",
                "the page's heading at {width}:\n{text}"
            );
            assert_eq!(cell(&text, 144, 1), "━", "{width}:\n{text}");
            assert_eq!(cell(&text, 145, 1), " ", "{width}:\n{text}");
            for page in SettingsPage::ALL {
                assert!(text.contains(&page.label()), "{page:?} at {width}:\n{text}");
            }
            assert!(text.contains("● syncs with the ledger"), "{text}");
            assert!(text.contains("○ this device only"), "{text}");
            assert!(text.contains("ledger name"), "{text}");
        }
    }

    #[test]
    fn docked_the_focused_side_draws_the_selection_and_the_index_marks_the_open_page() {
        crate::locale::init_for_tests();
        // TUI-050: the page focused, its field reversed and the index's open page barred.
        let view = at(WindowSize::Large, Some(SettingsPage::General));
        let text = render_at(&view, 171, 53);
        assert_eq!(cell(&text, 1, 2), "▌", "{text}");
        let c = Colours::default();
        let mut terminal =
            Terminal::new(TestBackend::new(171, 53)).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area(), WindowSize::Large, &c))
            .expect("drawing the settings view should not error");
        let buffer = terminal.backend().buffer();
        // The index row for General is not reversed; the page's first field, three rows below
        // its heading, is.
        assert_ne!(buffer[(5, 2)].style().bg, c.selection().bg);
        assert_eq!(buffer[(40, 3)].style().bg, c.selection().bg);

        // TUI-051: the index focused, its selection reversed, no bar, and no field reversed.
        let mut view = at(WindowSize::Large, None);
        view.handle_key(key(KeyCode::Char('j')));
        terminal
            .draw(|frame| view.view(frame, frame.area(), WindowSize::Large, &c))
            .expect("drawing the settings view should not error");
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(5, 3)].style().bg, c.selection().bg);
        assert_ne!(buffer[(40, 3)].style().bg, c.selection().bg);
        let text = render_at(&view, 171, 53);
        assert!(!text.contains('▌'), "{text}");
        assert!(text.contains("appearance"), "the page swaps live:\n{text}");
    }

    #[test]
    fn docked_focus_round_trips_between_the_index_and_the_page() {
        let mut view = at(WindowSize::Large, None);
        press(&mut view, &[KeyCode::Char('j')]);
        assert_eq!(
            view.showing,
            Showing::Index,
            "j moves the index, not the focus"
        );
        assert_eq!(view.selected(), SettingsPage::Display);

        press(&mut view, &[KeyCode::Enter]);
        assert_eq!(view.showing, Showing::Page);
        assert_eq!(view.selected(), SettingsPage::Display);

        assert!(view.back(), "esc on the page focuses the index");
        assert_eq!(view.showing, Showing::Index);
        assert_eq!(view.selected(), SettingsPage::Display);
        assert!(!view.back(), "esc on the index leaves Settings");
    }

    #[test]
    fn docked_brackets_stop_at_both_ends_and_p_is_inert() {
        let mut view = at(WindowSize::Large, Some(SettingsPage::General));
        press(&mut view, &[KeyCode::Char('[')]);
        assert_eq!(view.selected(), SettingsPage::General);
        view.update(&Action::OpenSettingsPage(SettingsPage::About));
        press(&mut view, &[KeyCode::Char(']')]);
        assert_eq!(view.selected(), SettingsPage::About);
        press(&mut view, &[KeyCode::Char('[')]);
        assert_eq!(view.selected(), SettingsPage::Logs);
        assert_eq!(view.showing, Showing::Page, "focus stays on the page");

        assert_eq!(
            view.handle_key(key(KeyCode::Char('p'))),
            Some(Action::NoOp),
            "p is taken, so it never reaches the Shell"
        );
        assert!(!view.popup_open());
    }

    #[test]
    fn a_resize_across_160_columns_keeps_the_page_its_selection_and_the_focus() {
        // The index page at Medium is the focused docked index at Large, and back.
        let mut view = at(WindowSize::Medium, None);
        press(&mut view, &[KeyCode::Char('j'), KeyCode::Char('j')]);
        view.set_window_size(WindowSize::Large);
        assert_eq!(
            (view.showing, view.selected()),
            (Showing::Index, SettingsPage::Units)
        );
        view.set_window_size(WindowSize::Medium);
        assert_eq!(
            (view.showing, view.selected()),
            (Showing::Index, SettingsPage::Units)
        );

        // A page keeps its own field selection; docking closes the Medium-only picker.
        let mut view = at(WindowSize::Medium, Some(SettingsPage::General));
        press(&mut view, &[KeyCode::Char('j'), KeyCode::Char('p')]);
        assert!(view.popup_open());
        view.set_window_size(WindowSize::XLarge);
        assert!(!view.popup_open());
        assert_eq!(
            (view.showing, view.selected()),
            (Showing::Page, SettingsPage::General)
        );
        view.set_window_size(WindowSize::Medium);
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::SetSetting(SettingField::Owner, None)),
            "owner, the field left selected, is still selected"
        );
    }

    #[test]
    fn docked_hints_end_pages_then_esc_index() {
        crate::locale::init_for_tests();
        let view = at(WindowSize::Large, None);
        let index = view.hints("esc").unwrap_or_default();
        assert!(
            index.contains(&("enter".into(), "focus page".into())),
            "{index:?}"
        );

        let view = at(WindowSize::XLarge, Some(SettingsPage::General));
        let hints = view.hints("esc").unwrap_or_default();
        let tail: Vec<_> = hints.iter().rev().take(2).rev().cloned().collect();
        assert_eq!(
            tail,
            vec![
                ("[/]".to_string(), "pages".to_string()),
                ("esc".to_string(), "index".to_string()),
            ]
        );
        assert!(!hints.iter().any(|(key, _)| key == "p"), "{hints:?}");
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
        let mut view = SettingsView::remembering(Rc::clone(&remembered), Rc::default(), None);
        view.update(&Action::OpenSettingsPage(SettingsPage::History));
        press(&mut view, &[KeyCode::Char('p')]);
        view.update(&Action::OpenSettings);
        assert_eq!(view.showing, Showing::Index);
        assert!(!view.popup_open());
        assert_eq!(remembered.get(), SettingsPage::History);

        let again = SettingsView::remembering(remembered, Rc::default(), None);
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
