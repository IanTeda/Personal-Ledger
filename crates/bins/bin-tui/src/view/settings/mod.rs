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
//! The index groups its pages by scope ([`Group`], ADR-0051): this device (General, Display, AI
//! Connections, Logs, About), then this ledger (Ledger … History). With no Ledger open
//! ([`SettingsView::without_ledger`], TUI-059) the this ledger rows are drawn dim, and opening one
//! by `enter`, `[` / `]` or the picker answers [`Action::NoLedgerOpen`] and stays put; a remembered
//! this ledger page gives way to General.
//!
//! General, Ledger and Display are form pages: rows of label, value and dim note, where `enter` opens a
//! field in place (TUI-020, `popup::settings::edit`) and the status line shows `INSERT` while it
//! is open. Opening and saving are one [`Action::SetSetting`] ([`field`]): the page returns it
//! without a value to open the field, the open field returns it with one to save, and `Shell`
//! routes it, keeping what it owns (Toasts, the Colour Theme and Appearance, the base unit guard)
//! and handing the rest back to [`View::update`]. The values the form pages show live in
//! [`field::SettingsValues`], shared with `Shell` as the selection is.
//!
//! Units, Accounts and the six ledger-data pages after them are list pages, drawn by the one
//! list-page pattern ([`list`]). Units hosts the Units View itself ([`units`]), so its keys and
//! Dialogs are `lib-units`'; Accounts reads a fixture until the TUI moves onto `lib-accounts`
//! ([`accounts`]), and Institutions, Categories, Tags, Payees, Documents and Inventory do the same
//! on [`list::Page`]. `o` opens the full View where there is one, which `Shell` pushes Settings
//! under, so the back key there returns to the same page and row. At X-Large every list page
//! docks a detail pane for its selected row at the View's right edge (#884): Units' from its
//! Presenter, the rest from their fixtures ([`pane`]).
//!
//! Logs and About are this-device pages with something real behind them: Logs reads the
//! `LogBuffer` `main` hands `Shell`, which passes it to each Settings View it builds ([`logs`]),
//! and About's links reach the browser and clipboard through `Shell` ([`about`]). Sync Server and
//! Backup are wireframes with nothing behind them yet ([`sync_server`], [`backup`]); each opens an
//! Edit overlay on `e` (`popup::settings::overlay`), which the View draws over the dimmed page
//! like the picker and which, like a field open in place, takes every key and shows `INSERT`.
//! AI Connections and History are wireframes too, on [`list::Page`] ([`ai_connections`],
//! [`history`]): AI Connections opens the same overlay frame for a model provider's `n` and `e`,
//! and History's undo keys only flash "not yet built", as `u` and `ctrl r` bind nowhere else.
//!
//! Every page but Units reads its keys from one table ([`keys`]), which both its `handle_key`
//! and its hint bar are built from, so the two can't drift; Units keeps its Presenter's hints.
//! `:sync now|edit` and `:backup now|export|git` run a page's key by name ([`PageCommand`]), on
//! the page opened first, through the same `run` the key takes.

mod about;
mod accounts;
mod ai_connections;
mod backup;
mod categories;
mod display;
mod documents;
pub mod field;
mod general;
mod history;
mod index;
mod institutions;
mod inventory;
mod keys;
mod ledger;
mod list;
mod logs;
mod pane;
mod payees;
mod picker;
mod sync_server;
mod tags;
mod units;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent};
use lib_list_cursor::Step;
use lib_shell::WindowSize;
use lib_toast::ToastKind;
use lib_tracing::LogBuffer;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::colours::Colours;
use crate::msg;
use crate::popup::settings::edit::{FieldEditor, Outcome};
use crate::units::{TuiPorts, UnitsSetup};
use crate::view::{Action, View, ViewId};

use field::{SettingField, SettingsValues};
use picker::Picker;

pub use backup::BackupCommand;
pub use sync_server::SyncCommand;

/// The seventeen Settings pages. [`SettingsPage::ALL`] is the one page order the index, the
/// picker and `[` / `]` all read, so the three can't drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsPage {
    #[default]
    General,
    Display,
    AiConnections,
    Logs,
    About,
    Ledger,
    Units,
    Institutions,
    Accounts,
    Categories,
    Tags,
    Payees,
    Documents,
    Inventory,
    SyncServer,
    Backup,
    History,
}

/// Which of the index's two groups a page sits under (ADR-0051, `GLOSSARY.md` **Preference**).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// What belongs to the device: Client-scoped Preferences and Configuration. These pages never
    /// sync and work with no Ledger open.
    ThisDevice,
    /// What the Open Ledger stores, or acts on its file. Refused with no Ledger open.
    ThisLedger,
}

impl Group {
    /// The group's heading on the index, the docked index and the picker.
    pub fn heading(self) -> String {
        match self {
            Self::ThisDevice => msg::tui_settings_group_this_device(),
            Self::ThisLedger => msg::tui_settings_group_this_ledger(),
        }
    }
}

impl SettingsPage {
    /// Every page, in the index's order: this device's, then this ledger's, as the Desktop's.
    pub const ALL: [SettingsPage; 17] = [
        Self::General,
        Self::Display,
        Self::AiConnections,
        Self::Logs,
        Self::About,
        Self::Ledger,
        Self::Units,
        Self::Institutions,
        Self::Accounts,
        Self::Categories,
        Self::Tags,
        Self::Payees,
        Self::Documents,
        Self::Inventory,
        Self::SyncServer,
        Self::Backup,
        Self::History,
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
            Self::Ledger => msg::tui_settings_page_ledger(),
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
            Self::Ledger => msg::tui_settings_description_ledger(),
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

    /// The index group the page sits under.
    pub fn group(self) -> Group {
        match self {
            Self::General | Self::Display | Self::AiConnections | Self::Logs | Self::About => {
                Group::ThisDevice
            }
            _ => Group::ThisLedger,
        }
    }

    /// Whether the page needs an Open Ledger: every this ledger page does.
    pub fn needs_ledger(self) -> bool {
        self.group() == Group::ThisLedger
    }

    /// The page heading's right-aligned scope note. Counts join it as each page's stage derives
    /// them from its rows; none is stored here.
    pub fn scope_note(self) -> String {
        match self {
            Self::General => msg::tui_settings_scope_general(),
            Self::Ledger => msg::tui_settings_scope_ledger(),
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
            Self::Backup => msg::tui_settings_scope_backup(backup::SIZE),
            Self::History => msg::tui_settings_scope_history(),
            Self::Logs => msg::tui_settings_scope_logs(&crate::format::count(
                i64::try_from(lib_tracing::LOG_CAPACITY).unwrap_or(i64::MAX),
            )),
            Self::About => msg::tui_settings_scope_about(),
        }
    }

    /// The name `:settings <page>` takes: command syntax the user types, so stable English
    /// rather than a Message, as command names are.
    pub fn command_name(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Ledger => "ledger",
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

/// A page's key run by name (#898): `:sync now|edit` and `:backup now|export|git`. Each reaches
/// the same `run` as the page's key, on the page opened first, as `:settings <page>` opens it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageCommand {
    Sync(SyncCommand),
    Backup(BackupCommand),
}

impl PageCommand {
    /// The page the command's key is on.
    pub fn page(self) -> SettingsPage {
        match self {
            Self::Sync(_) => SettingsPage::SyncServer,
            Self::Backup(_) => SettingsPage::Backup,
        }
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
    /// Whether a Ledger is open: without one the this ledger pages are refused.
    ledger_open: bool,
    general: general::GeneralPage,
    ledger: ledger::LedgerPage,
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
    ai_connections: ai_connections::AiConnectionsPage,
    history: history::HistoryPage,
    sync_server: sync_server::SyncServerPage,
    backup: backup::BackupPage,
    logs: logs::LogsPage,
    about: about::AboutPage,
}

impl SettingsView {
    /// A View on its own selection and values, starting at General, with no Ledger's Units.
    /// `Shell` uses [`Self::remembering`].
    #[cfg(test)]
    pub fn new() -> Self {
        Self::with_logs(LogBuffer::new(lib_tracing::LOG_CAPACITY))
    }

    /// [`Self::new`] on `units`.
    #[cfg(test)]
    pub fn with_units(units: &UnitsSetup) -> Self {
        Self::remembering(
            Rc::default(),
            Rc::default(),
            Some(units),
            LogBuffer::new(lib_tracing::LOG_CAPACITY),
        )
    }

    /// [`Self::new`] showing `logs` on Settings › Logs.
    #[cfg(test)]
    pub fn with_logs(logs: LogBuffer) -> Self {
        Self::remembering(Rc::default(), Rc::default(), None, logs)
    }

    /// A View on the session's selection and values, landing on the index with the selection
    /// selected; `units` are the Open Ledger's, and `logs` the events `main` captures.
    pub fn remembering(
        selected: Rc<Cell<SettingsPage>>,
        values: Rc<RefCell<SettingsValues>>,
        units: Option<&UnitsSetup>,
        logs: LogBuffer,
    ) -> Self {
        Self {
            showing: Showing::Index,
            size: WindowSize::Medium,
            selected,
            picker: None,
            values,
            units: units.map(|setup| setup.ports.clone()),
            editor: None,
            ledger_open: true,
            general: general::GeneralPage::new(),
            ledger: ledger::LedgerPage::default(),
            display: display::DisplayPage::new(),
            units_page: units.map(units::UnitsPage::new),
            accounts: accounts::AccountsPage::new(),
            institutions: institutions::InstitutionsPage::new(),
            categories: categories::CategoriesPage::new(),
            tags: tags::TagsPage::new(),
            payees: payees::PayeesPage::new(),
            documents: documents::DocumentsPage::new(),
            inventory: inventory::InventoryPage::new(),
            ai_connections: ai_connections::AiConnectionsPage::new(),
            history: history::HistoryPage::new(),
            sync_server: sync_server::SyncServerPage::default(),
            backup: backup::BackupPage::default(),
            logs: logs::LogsPage::new(logs),
            about: about::AboutPage::default(),
        }
    }

    /// With no Ledger open: the this ledger pages are refused, and a remembered one gives way to
    /// General (TUI-059).
    pub fn without_ledger(mut self) -> Self {
        self.ledger_open = false;
        self.leave_this_ledger();
        self
    }

    /// Selects General in place of a this ledger page, with no Ledger open.
    fn leave_this_ledger(&mut self) {
        if self.refused(self.selected()) {
            self.selected.set(SettingsPage::General);
        }
    }

    /// Whether `page` can't open: it needs a Ledger and none is open.
    fn refused(&self, page: SettingsPage) -> bool {
        !self.ledger_open && page.needs_ledger()
    }

    /// Shows `page` as [`Self::open_page`] does, or, when it is refused, answers
    /// [`Action::NoLedgerOpen`] and leaves everything as it was.
    fn go_to(&mut self, page: SettingsPage) -> Action {
        if self.refused(page) {
            return Action::NoLedgerOpen;
        }
        self.open_page(page);
        Action::NoOp
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
            SettingsPage::AiConnections => &self.ai_connections,
            SettingsPage::History => &self.history,
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
            SettingsPage::AiConnections => &mut self.ai_connections,
            SettingsPage::History => &mut self.history,
            _ => return None,
        })
    }

    fn selected(&self) -> SettingsPage {
        self.selected.get()
    }

    /// The detail pane `page` docks at X-Large when it draws one from its fixture: Accounts and
    /// the pages on [`list::Page`]. Units' comes from its Presenter, and a form page docks none.
    fn fixture_pane(&self, page: SettingsPage) -> Option<pane::Pane> {
        match page {
            SettingsPage::Accounts => Some(self.accounts.pane()),
            page => self.ledger_page(page).map(list::Page::pane),
        }
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
            KeyCode::Enter | KeyCode::Char('l') => return Some(self.go_to(self.selected())),
            _ => return None,
        };
        self.selected.set(self.selected().step(step));
        Some(Action::NoOp)
    }

    fn handle_page_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('[') => Some(self.go_to(self.selected().step(Step::Prev))),
            KeyCode::Char(']') => Some(self.go_to(self.selected().step(Step::Next))),
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
                SettingsPage::Ledger => self.ledger.handle_key(key),
                SettingsPage::Display => {
                    let hint_bar = self.values.borrow().configuration.hint_bar;
                    self.display.handle_key(key, hint_bar)
                }
                SettingsPage::Units => self.units_page.as_mut()?.handle_key(key),
                SettingsPage::Accounts => self.accounts.handle_key(key),
                SettingsPage::SyncServer => self.sync_server.handle_key(key),
                SettingsPage::Backup => self.backup.handle_key(key),
                SettingsPage::Logs => self.logs.handle_key(key),
                SettingsPage::About => self.about.handle_key(key),
                page => self.ledger_page_mut(page)?.handle_key(key),
            },
        }
    }

    /// The page's own hints, before the paging hints every page ends with.
    fn page_hints(&self) -> Vec<(String, String)> {
        match self.selected() {
            SettingsPage::General => general::GeneralPage::hints(),
            SettingsPage::Ledger => ledger::LedgerPage::hints(),
            SettingsPage::Display => display::DisplayPage::hints(),
            SettingsPage::Units => self
                .units_page
                .as_ref()
                .map(units::UnitsPage::hints)
                .unwrap_or_default(),
            SettingsPage::Accounts => accounts::AccountsPage::hints(),
            SettingsPage::SyncServer => sync_server::SyncServerPage::hints(),
            SettingsPage::Backup => backup::BackupPage::hints(),
            SettingsPage::Logs => logs::LogsPage::hints(),
            SettingsPage::About => about::AboutPage::hints(),
            page => self
                .ledger_page(page)
                .map(list::Page::hints)
                .unwrap_or_default(),
        }
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
                    SettingsPage::Ledger => self.ledger.step(Step::Next),
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
                SettingsPage::General => self.general.draft(field, &context),
                SettingsPage::Ledger => ledger::draft(field, &context),
                _ => self.display.draft(field, &context),
            }
        };
        let Some((draft, was)) = opened else {
            return;
        };
        if self.refused(field.page()) {
            return;
        }
        self.open_page(field.page());
        match field.page() {
            SettingsPage::General => self.general.select(field),
            SettingsPage::Ledger => self.ledger.select(field),
            _ => self.display.select(field),
        }
        self.editor = Some(FieldEditor::new(field, draft, was));
    }

    /// Closes whichever of a field open in place, the Unit prices view, the picker, the Clear logs
    /// confirm or an Edit overlay is open (each takes every key, so at most one is); `false` when
    /// none was.
    fn back_to_page(&mut self) -> bool {
        self.editor.take().is_some()
            || self
                .units_page
                .as_mut()
                .is_some_and(units::UnitsPage::close_prices)
            || self.picker.take().is_some()
            || self.logs.close_confirm()
            || self.sync_server.close_overlay()
            || self.backup.close_overlay()
            || self.ai_connections.close_overlay()
    }

    /// Whether a page's Edit overlay is open: Sync Server's, Backup's or AI Connections'.
    fn overlay_open(&self) -> bool {
        self.sync_server.overlay_open()
            || self.backup.overlay_open()
            || self.ai_connections.overlay_open()
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

/// What General, Ledger and Display read on each draw besides their own selection.
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
        // A Units Dialog and the Unit prices view take every key, `[`, `]` and `p` included.
        if let Some(page) = self
            .units_page
            .as_mut()
            .filter(|page| page.dialog_open() || page.prices_open())
        {
            return page.handle_key(key);
        }
        // The Clear logs confirm and an Edit overlay take every key, `[`, `]` and `p` included.
        if self.logs.confirm_open() {
            return self.logs.handle_key(key);
        }
        if self.sync_server.overlay_open() {
            return self.sync_server.handle_key(key);
        }
        if self.backup.overlay_open() {
            return self.backup.handle_key(key);
        }
        if self.ai_connections.overlay_open() {
            return list::Page::handle_key(&mut self.ai_connections, key);
        }
        if let Some(picker) = &mut self.picker {
            return match picker.handle_key(key) {
                picker::Outcome::Open(page) => Some(self.go_to(page)),
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
                self.leave_this_ledger();
            }
            Action::OpenSettingsPage(page) => {
                self.go_to(*page);
            }
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

    /// An open field, the picker, the Clear logs confirm or an Edit overlay closes first, then a
    /// page steps back to the index with itself selected; on the index the Shell pops its view
    /// stack.
    fn back(&mut self) -> bool {
        if self.back_to_page() {
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

    /// The picker, the Clear logs confirm, an Edit overlay, or a field open in place, which takes
    /// every key as a popup does.
    fn popup_open(&self) -> bool {
        self.picker.is_some()
            || self.editor.is_some()
            || self.logs.confirm_open()
            || self.overlay_open()
    }

    /// A field open in place, or an Edit overlay, whose fields take typing.
    fn editing(&self) -> bool {
        self.editor.is_some() || self.overlay_open()
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

    /// Opens the command's page, closing whatever was open over it, then runs its key.
    fn run_page_command(&mut self, command: PageCommand) -> Option<Action> {
        self.back_to_page();
        if self.refused(command.page()) {
            return Some(Action::NoLedgerOpen);
        }
        self.open_page(command.page());
        Some(match command {
            PageCommand::Sync(command) => self.sync_server.run(command),
            PageCommand::Backup(command) => self.backup.run(command),
        })
    }

    fn take_toasts(&mut self) -> Vec<(ToastKind, String)> {
        let mut toasts = self
            .units_page
            .as_mut()
            .map(units::UnitsPage::take_toasts)
            .unwrap_or_default();
        toasts.extend(self.logs.take_toasts());
        toasts
    }

    fn hints(&self, back: &str) -> Option<Vec<(String, String)>> {
        let prices = self
            .units_page
            .as_ref()
            .and_then(|page| page.prices_hints(back));
        let hints = if let Some(editor) = &self.editor {
            editor.hints(back)
        } else if let Some(prices) = prices {
            prices
        } else if self.picker.is_some() {
            picker::hints(back)
        } else if self.overlay_open() {
            // The overlay draws its own keys; the bar names only the way out.
            vec![(back.to_string(), msg::tui_settings_hint_close_dialog())]
        } else {
            match self.showing {
                Showing::Index => index::hints(back, self.docked(), self.ledger_open),
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
        self.general.set_toasts_on(on);
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
                Showing::Index => index::render(frame, inner, self.selected(), self.ledger_open, c),
                Showing::Page => self.render_page(frame, inner, false, true, false, c),
            }
        } else {
            let index_focused = self.showing == Showing::Index;
            let [dock, _, page] = Layout::horizontal([
                Constraint::Length(index::DOCKED_WIDTH),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .areas(inner);
            index::render_docked(
                frame,
                dock,
                self.selected(),
                index_focused,
                self.ledger_open,
                c,
            );
            // X-Large docks a list page's detail pane at the View's right edge (TUI-055–057).
            let split = |page: Rect| {
                let [page, _, pane] = Layout::horizontal([
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(crate::view::detail_pane::WIDTH),
                ])
                .areas(page);
                (page, pane)
            };
            let selected = self.selected();
            let fixture_pane = (size == WindowSize::XLarge)
                .then(|| self.fixture_pane(selected))
                .flatten();
            let docks_pane = fixture_pane.is_some();
            let page = match (&self.units_page, fixture_pane) {
                (Some(units), _) if selected == SettingsPage::Units && units.has_pane() => {
                    let (page, pane) = split(page);
                    units.render_pane(frame, pane, c);
                    page
                }
                (_, Some(fixture)) => {
                    let (page, pane) = split(page);
                    pane::render(frame, pane, &fixture, &selected.label(), c);
                    page
                }
                _ => page,
            };
            self.render_page(frame, page, true, !index_focused, docks_pane, c);
        }
        if let Some(picker) = &self.picker {
            frame.render_widget(crate::popup::Dim, area);
            picker.render(frame, area, self.selected(), self.ledger_open, c);
        } else if self.logs.confirm_open() {
            frame.render_widget(crate::popup::Dim, area);
            self.logs.render_confirm(frame, area, c);
        } else if self.overlay_open() {
            frame.render_widget(crate::popup::Dim, area);
            self.sync_server.render_overlay(frame, area, c);
            self.backup.render_overlay(frame, area, c);
            self.ai_connections.render_overlay(frame, area, c);
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
        pane: bool,
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
        // Docked, a this ledger page is in view beside the index with no Ledger to show.
        if self.refused(page) {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    msg::tui_status_no_ledger_open(
                        crate::shell::OPEN_COMMAND,
                        crate::shell::NEW_COMMAND,
                    ),
                    c.muted(),
                )),
                body,
            );
            return;
        }
        let values = self.values.borrow();
        let context = self.context(&values);
        match page {
            SettingsPage::General => self.general.render(frame, body, focused, &context, c),
            SettingsPage::Ledger => self.ledger.render(frame, body, focused, &context, c),
            SettingsPage::Display => self.display.render(frame, body, focused, &context, c),
            SettingsPage::Units => {
                if let Some(units) = &self.units_page {
                    units.render(frame, body, focused, c);
                }
            }
            SettingsPage::Accounts => self.accounts.render(frame, body, focused, c),
            SettingsPage::SyncServer => self.sync_server.render(frame, body, c),
            SettingsPage::Backup => self.backup.render(frame, body, c),
            SettingsPage::Logs => self.logs.render(frame, body, focused, c),
            SettingsPage::About => self.about.render(frame, body, focused, c),
            page => {
                if let Some(page) = self.ledger_page(page) {
                    page.render(frame, body, focused, pane, c);
                }
            }
        }
        if let Some(editor) = &self.editor {
            let echo = Rect {
                y: body.bottom().saturating_sub(1),
                height: body.height.min(1),
                ..body
            };
            frame.render_widget(Clear, echo);
            frame.render_widget(Paragraph::new(set_echo(editor, c)), echo);
        }
    }
}

/// The open field as the `:set` that makes the same edit (TUI-020), which is how the typed form
/// is learnt: `:set fy-start january▌ — the same edit as a command`.
fn set_echo(editor: &FieldEditor, c: &Colours) -> Line<'static> {
    Line::from(vec![
        Span::raw(format!(
            ":set {} {}",
            editor.field().command_name(),
            editor.value()
        )),
        Span::styled("▌", c.accent()),
        Span::styled(format!("  {}", msg::tui_settings_set_echo()), c.muted()),
    ])
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

/// One row of a form page: the field, whose label it shows, its value and a dim note.
struct FieldRow {
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
        let [label, value, note] = Layout::horizontal([
            Constraint::Length(FIELD_LABEL_WIDTH),
            Constraint::Length(FIELD_VALUE_WIDTH),
            Constraint::Min(0),
        ])
        .areas(*row_area);
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

/// A read-only row of a wireframe page (Sync Server, Backup): the label dim, then its value.
fn render_fact(frame: &mut Frame<'_>, area: Rect, label: &str, value: Line<'_>, c: &Colours) {
    let [label_area, value_area] =
        Layout::horizontal([Constraint::Length(FIELD_LABEL_WIDTH), Constraint::Min(0)]).areas(area);
    frame.render_widget(Paragraph::new(Span::styled(label, c.muted())), label_area);
    frame.render_widget(Paragraph::new(value), value_area);
}

/// A row of a page's own keys under its facts, as the hint bar draws them: the key bold, its
/// label after it.
fn render_keys(frame: &mut Frame<'_>, area: Rect, keys: &[(&str, String)]) {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let spans: Vec<Span<'_>> = keys
        .iter()
        .flat_map(|(key, label)| [Span::styled(*key, bold), Span::raw(format!(" {label}   "))])
        .collect();
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// A wireframe moment, `date` at `hour:minute`, as the pages show one: `14 sept 2026 · 14:22`.
fn moment(date: Option<chrono::NaiveDate>, hour: u32, minute: u32) -> String {
    let (Some(date), Some(time)) = (date, chrono::NaiveTime::from_hms_opt(hour, minute, 0)) else {
        return String::new();
    };
    format!(
        "{} · {}",
        crate::format::date(date),
        lib_locale::format::format_time_minutes(time)
    )
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
        assert!(!text.contains("this device \u{2500}"), "{text}");
        assert_eq!(cell(&text, 112, 1), "━", "{text}");
        assert_eq!(cell(&text, 113, 1), " ", "{text}");

        // TUI-050 and TUI-052: the index docks at 31 columns, then a gap, then the page from
        // column 33, its heading the name alone, stopping 112 columns on.
        for (width, height) in [(171, 53), (256, 72)] {
            let size = WindowSize::from_columns(width);
            let text = render_at(&at(size, Some(SettingsPage::General)), width, height);
            let heading = text.lines().next().unwrap_or_default();
            assert!(
                heading.contains("Settings") && heading.contains("17 pages"),
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
            assert!(text.contains("this device \u{2500}"), "{text}");
            assert!(text.contains("this ledger \u{2500}"), "{text}");
            assert!(text.contains("on start"), "{text}");
        }
    }

    #[test]
    fn docked_the_focused_side_draws_the_selection_and_the_index_marks_the_open_page() {
        crate::locale::init_for_tests();
        // TUI-050: the page focused, its field reversed and the index's open page barred.
        let view = at(WindowSize::Large, Some(SettingsPage::General));
        let text = render_at(&view, 171, 53);
        // Below the heading, its rule and the this device heading.
        assert_eq!(cell(&text, 1, 3), "▌", "{text}");
        let c = Colours::default();
        let mut terminal =
            Terminal::new(TestBackend::new(171, 53)).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area(), WindowSize::Large, &c))
            .expect("drawing the settings view should not error");
        let buffer = terminal.backend().buffer();
        // The index row for General is not reversed; the page's first field, three rows below
        // its heading, is.
        assert_ne!(buffer[(5, 3)].style().bg, c.selection().bg);
        assert_eq!(buffer[(40, 3)].style().bg, c.selection().bg);

        // TUI-051: the index focused, its selection reversed, no bar, and no field reversed.
        let mut view = at(WindowSize::Large, None);
        view.handle_key(key(KeyCode::Char('j')));
        terminal
            .draw(|frame| view.view(frame, frame.area(), WindowSize::Large, &c))
            .expect("drawing the settings view should not error");
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(5, 4)].style().bg, c.selection().bg);
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
        view.update(&Action::OpenSettingsPage(SettingsPage::History));
        press(&mut view, &[KeyCode::Char(']')]);
        assert_eq!(view.selected(), SettingsPage::History);
        press(&mut view, &[KeyCode::Char('[')]);
        assert_eq!(view.selected(), SettingsPage::Backup);
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
            (Showing::Index, SettingsPage::AiConnections)
        );
        view.set_window_size(WindowSize::Medium);
        assert_eq!(
            (view.showing, view.selected()),
            (Showing::Index, SettingsPage::AiConnections)
        );

        // A page keeps its own field selection; docking closes the Medium-only picker.
        let mut view = at(WindowSize::Medium, Some(SettingsPage::Ledger));
        press(&mut view, &[KeyCode::Char('j'), KeyCode::Char('p')]);
        assert!(view.popup_open());
        view.set_window_size(WindowSize::XLarge);
        assert!(!view.popup_open());
        assert_eq!(
            (view.showing, view.selected()),
            (Showing::Page, SettingsPage::Ledger)
        );
        view.set_window_size(WindowSize::Medium);
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::SetSetting(SettingField::Owner, None)),
            "owner, the field left selected, is still selected"
        );
    }

    /// The detail pane's columns of `text`, rendered at 256 columns: the 53 at the right edge.
    fn pane_of(text: &str) -> String {
        text.lines()
            .map(|line| line.chars().skip(256 - 53).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn accounts_docks_tui_056s_pane_at_x_large() {
        crate::locale::init_for_tests();
        let mut view = at(WindowSize::XLarge, Some(SettingsPage::Accounts));
        press(&mut view, &[KeyCode::Char('j')]);
        let pane = pane_of(&render_at(&view, 256, 72));
        for needle in [
            "ACCOUNT",
            "ANZ Offset",
            "bank · ANZ · aud",
            "61,204.10 aud",
            "offsets           Home Loan",
            "212",
            "enter open ledger  ·  e edit  ·  d delete",
            "RECENT TRANSACTIONS",
            "Interest",
            "+84.12",
            "\u{2212}3,412.00",
            "Open ledger (enter) shows them all.",
            "ACTIVITY",
            "This session's Accounts outcomes",
        ] {
            assert!(pane.contains(needle), "{needle} missing:\n{pane}");
        }
    }

    #[test]
    fn history_docks_tui_057s_pane_with_no_activity() {
        crate::locale::init_for_tests();
        let mut view = at(WindowSize::XLarge, Some(SettingsPage::History));
        // Past the two undone changes and two applied ones to Marked Rates Q1 paid.
        press(&mut view, &[KeyCode::Char('j'); 4]);
        let pane = pane_of(&render_at(&view, 256, 72));
        for needle in [
            "CHANGE",
            "Marked Rates Q1 paid",
            "bills · laptop · today",
            "synced",
            "enter restore to here",
            "BEFORE → AFTER",
            "due → paid",
            "RESTORING HERE UNDOES",
            "Added Camera House",
            "Renamed tag holiday → travel",
        ] {
            assert!(pane.contains(needle), "{needle} missing:\n{pane}");
        }
        assert!(!pane.contains("ACTIVITY"), "{pane}");

        // An undone change offers redo, and lists what redoing reapplies.
        press(&mut view, &[KeyCode::Home]);
        let pane = pane_of(&render_at(&view, 256, 72));
        assert!(pane.contains("enter redo to here"), "{pane}");
        assert!(pane.contains("REDOING TO HERE REAPPLIES"), "{pane}");
    }

    #[test]
    fn every_list_page_docks_a_pane_at_x_large_and_a_form_page_none() {
        crate::locale::init_for_tests();
        let list_pages = [
            SettingsPage::Institutions,
            SettingsPage::Accounts,
            SettingsPage::Categories,
            SettingsPage::Tags,
            SettingsPage::Payees,
            SettingsPage::Documents,
            SettingsPage::Inventory,
            SettingsPage::AiConnections,
        ];
        for page in list_pages {
            let pane = pane_of(&render_at(&at(WindowSize::XLarge, Some(page)), 256, 72));
            assert!(pane.contains("ACTIVITY"), "{page:?} docks no pane:\n{pane}");
            let large = render_at(&at(WindowSize::Large, Some(page)), 213, 53);
            assert!(!large.contains("ACTIVITY"), "{page:?} docks at Large");
        }
        let general = render_at(
            &at(WindowSize::XLarge, Some(SettingsPage::General)),
            256,
            72,
        );
        assert!(!general.contains("ACTIVITY"));
    }

    #[test]
    fn the_pane_takes_the_place_of_the_selected_box() {
        crate::locale::init_for_tests();
        let page = Some(SettingsPage::Institutions);
        let large = render_at(&at(WindowSize::Large, page), 213, 53);
        assert!(large.contains("selected · American Express"), "{large}");
        let x_large = render_at(&at(WindowSize::XLarge, page), 256, 72);
        assert!(
            !x_large.contains("selected · American Express"),
            "{x_large}"
        );
        let pane = pane_of(&x_large);
        for needle in ["INSTITUTION", "American Express", "Amex Platinum", "Total"] {
            assert!(pane.contains(needle), "{needle} missing:\n{pane}");
        }
    }

    #[test]
    fn a_pane_names_only_the_keys_that_act_on_its_row() {
        crate::locale::init_for_tests();
        // Housing has a subcategory, so `s` adds one; Rent, a leaf, takes none here.
        let mut view = at(WindowSize::XLarge, Some(SettingsPage::Categories));
        let pane = pane_of(&render_at(&view, 256, 72));
        assert!(pane.contains("s add sub"), "{pane}");
        assert!(pane.contains("SUBCATEGORIES"), "{pane}");
        press(&mut view, &[KeyCode::Char('j')]);
        let pane = pane_of(&render_at(&view, 256, 72));
        assert!(pane.contains("under Housing"), "{pane}");
        assert!(!pane.contains("s add sub"), "{pane}");

        // Only a likely duplicate merges.
        let mut view = at(WindowSize::XLarge, Some(SettingsPage::Tags));
        assert!(!pane_of(&render_at(&view, 256, 72)).contains("m merge"));
        press(&mut view, &[KeyCode::Char('j'); 4]);
        let pane = pane_of(&render_at(&view, 256, 72));
        assert!(pane.contains("m merge"), "{pane}");
        assert!(pane.contains("LIKELY DUPLICATE"), "{pane}");

        // `x` removes a provider and revokes an assistant.
        let mut view = at(WindowSize::XLarge, Some(SettingsPage::AiConnections));
        assert!(pane_of(&render_at(&view, 256, 72)).contains("x remove"));
        press(&mut view, &[KeyCode::Tab]);
        let pane = pane_of(&render_at(&view, 256, 72));
        assert!(pane.contains("x revoke"), "{pane}");
        assert!(pane.contains("ASSISTANT"), "{pane}");
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
        assert_eq!(view.selected(), SettingsPage::AiConnections);
        press(&mut view, &[KeyCode::Char('G')]);
        assert_eq!(view.selected(), SettingsPage::History);
        press(&mut view, &[KeyCode::Char('j')]);
        assert_eq!(view.selected(), SettingsPage::History);
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
            assert_eq!(view.selected(), SettingsPage::AiConnections);

            assert!(view.back(), "back on a page stays in Settings");
            assert_eq!(view.showing, Showing::Index);
            assert_eq!(view.selected(), SettingsPage::AiConnections);
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

        // This device runs into this ledger.
        view.update(&Action::OpenSettingsPage(SettingsPage::About));
        press(&mut view, &[KeyCode::Char(']')]);
        assert_eq!(view.selected(), SettingsPage::Ledger);

        view.update(&Action::OpenSettingsPage(SettingsPage::History));
        press(&mut view, &[KeyCode::Char(']')]);
        assert_eq!(view.selected(), SettingsPage::History);
        press(&mut view, &[KeyCode::Char('[')]);
        assert_eq!(view.selected(), SettingsPage::Backup);
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
            Some(SettingsPage::History)
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
        let mut view = SettingsView::remembering(
            Rc::clone(&remembered),
            Rc::default(),
            None,
            LogBuffer::new(4),
        );
        view.update(&Action::OpenSettingsPage(SettingsPage::History));
        press(&mut view, &[KeyCode::Char('p')]);
        view.update(&Action::OpenSettings);
        assert_eq!(view.showing, Showing::Index);
        assert!(!view.popup_open());
        assert_eq!(remembered.get(), SettingsPage::History);

        let again = SettingsView::remembering(remembered, Rc::default(), None, LogBuffer::new(4));
        assert_eq!(again.selected(), SettingsPage::History);
        assert_eq!(again.showing, Showing::Index);
    }

    #[test]
    fn the_index_lists_every_page_with_its_description() {
        crate::locale::init_for_tests();
        let text = render(&SettingsView::new());
        assert!(text.contains("Settings"), "{text}");
        assert!(text.contains("17 pages"), "{text}");
        for page in SettingsPage::ALL {
            let row = text
                .lines()
                .find(|line| {
                    line.contains(&page.description().chars().take(20).collect::<String>())
                })
                .unwrap_or_default();
            assert!(row.contains(&page.label()), "{page:?} missing:\n{text}");
        }
        assert!(!text.contains('\u{25cf}'), "a scope glyph:\n{text}");
    }

    #[test]
    fn the_index_groups_its_pages_under_this_device_then_this_ledger() {
        crate::locale::init_for_tests();
        let text = render(&SettingsView::new());
        let row = |needle: &str| text.lines().position(|line| line.contains(needle));
        let rows = [
            row("this device \u{2500}"),
            row("on start, toasts"),
            row("version, licences"),
            row("this ledger \u{2500}"),
            row("ledger name, owner"),
            row("every change"),
        ];
        assert!(rows.iter().all(Option::is_some), "{rows:?}\n{text}");
        assert!(rows.is_sorted(), "{rows:?}\n{text}");
    }

    #[test]
    fn with_no_ledger_open_the_index_counts_and_dims_the_this_ledger_pages() {
        crate::locale::init_for_tests();
        let c = Colours::default();
        let view = SettingsView::new().without_ledger();
        let mut terminal =
            Terminal::new(TestBackend::new(96, 28)).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area(), WindowSize::Medium, &c))
            .expect("drawing the settings view should not error");
        let buffer = terminal.backend().buffer();
        let muted = c.muted().fg;
        // The colour of `label`'s first cell, on the row that names it at the name column.
        let name_colour = |label: &str| {
            (0..buffer.area.height).find_map(|y| {
                let row: String = (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect();
                row.starts_with(&format!(" {label} "))
                    .then(|| buffer[(1, y)].fg)
            })
        };
        assert_eq!(name_colour("Units"), muted);
        assert_eq!(name_colour("History"), muted);
        assert_ne!(name_colour("Display"), muted);
        let heading: String = (0..buffer.area.width)
            .map(|x| buffer[(x, 0)].symbol())
            .collect();
        assert!(
            heading.contains("17 pages \u{b7} 5 without a Ledger"),
            "{heading}"
        );
    }

    #[test]
    fn with_no_ledger_open_a_this_ledger_page_is_refused_and_nothing_moves() {
        let mut view = SettingsView::new().without_ledger();
        press(&mut view, &[KeyCode::Char('G')]);
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::NoLedgerOpen)
        );
        assert_eq!(view.showing, Showing::Index);

        view.update(&Action::OpenSettingsPage(SettingsPage::About));
        assert_eq!(
            view.handle_key(key(KeyCode::Char(']'))),
            Some(Action::NoLedgerOpen)
        );
        assert_eq!(view.selected(), SettingsPage::About);

        press(&mut view, &[KeyCode::Char('p'), KeyCode::Char('j')]);
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::NoLedgerOpen)
        );
        assert!(view.popup_open(), "the picker stays open");
        assert_eq!(view.selected(), SettingsPage::About);

        view.update(&Action::OpenSettingsPage(SettingsPage::Units));
        assert_eq!(view.selected(), SettingsPage::About);
        assert_eq!(
            view.run_page_command(PageCommand::Sync(SyncCommand::Now)),
            Some(Action::NoLedgerOpen)
        );
    }

    #[test]
    fn with_no_ledger_open_a_remembered_this_ledger_page_gives_way_to_general() {
        let remembered = Rc::new(Cell::new(SettingsPage::Tags));
        let view = SettingsView::remembering(
            Rc::clone(&remembered),
            Rc::default(),
            None,
            LogBuffer::new(4),
        )
        .without_ledger();
        assert_eq!(view.selected(), SettingsPage::General);

        remembered.set(SettingsPage::Logs);
        let view = SettingsView::remembering(remembered, Rc::default(), None, LogBuffer::new(4))
            .without_ledger();
        assert_eq!(view.selected(), SettingsPage::Logs);
    }

    #[test]
    fn with_no_ledger_open_a_docked_this_ledger_page_says_so_and_won_t_take_focus() {
        crate::locale::init_for_tests();
        let mut view = SettingsView::new().without_ledger();
        view.set_window_size(WindowSize::Large);
        press(&mut view, &[KeyCode::Char('G')]);
        let text = render_at(&view, 171, 40);
        assert!(
            text.contains("No Ledger open: :open or :new first"),
            "{text}"
        );
        assert_eq!(
            view.handle_key(key(KeyCode::Enter)),
            Some(Action::NoLedgerOpen)
        );
        assert_eq!(view.showing, Showing::Index);
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
