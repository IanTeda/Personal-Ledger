//! The Settings destination's Entity (ADR-0032, ADR-0037). [`SettingsView`] owns the View state
//! that belongs to Settings: the page on show, where keyboard focus sits, the Display page's
//! focused control and Preferences. It renders itself (`impl Render` in `super`), handles the index, its focus
//! keys and the static pages' keys, movement and `Esc`, and reaches `Shell` only through
//! [`ViewEvent`].
//!
//! A page with Ledger data that has moved into its own Entity (Accounts, [`AccountsPage`];
//! Categories, [`CategoriesPage`]; Payees, [`PayeesPage`]; Tags, [`TagsPage`]; Units, [`UnitsPage`];
//! Institutions, [`InstitutionsPage`]; Document Types, [`DocumentsPage`]; Inventory,
//! [`InventoryPage`]; Tracing, [`TracingPage`]) is drawn by that Entity, and this View forwards the page's keys, movement
//! and status to it while the page has focus.

use super::hints::{
    settings_categories_hints, settings_colour_grid_hints, settings_display_hints,
    settings_documents_hints, settings_index_hints, settings_medium_page_hints,
    settings_payees_hints, settings_picker_hints, settings_plain_page_hints, settings_tags_hints,
    settings_tracing_hints,
};
use crate::settings::categories::{CategoriesPage, CategoryRowClicked};
use crate::settings::documents::{DocumentsPage, RowClicked as DocumentsRowClicked};
use crate::settings::inventory::{InventoryPage, RowClicked as InventoryRowClicked};
use crate::settings::payees::{PayeesPage, RowClicked};
use crate::settings::tags::{RowClicked as TagsRowClicked, TagsPage};
use crate::{
    chrome::{statusline::PageStatus, window_size::WindowSize},
    navigation::key_router::Movement,
    settings::accounts::{AccountsPage, hints::accounts_hints},
    settings::institutions::InstitutionsPage,
    settings::picker::PagePicker,
    settings::tracing::TracingPage,
    settings::units::UnitsPage,
    settings::{
        SettingsFocus, SettingsPage,
        display::{DisplayKey, DisplayPage, DisplayPreferences},
    },
    settings::{about::AboutPage, data_backup::DataBackupPage, sync_server::SyncServerPage},
    view::event::{Reveal, ViewEvent, ViewMode},
};
use gpui::{App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, Subscription};
use lib_list_cursor::Step;

impl EventEmitter<ViewEvent> for SettingsView {}

/// What the Settings pages read: the Entity of each page with Ledger data that draws itself.
pub struct SettingsSources {
    pub display: Entity<DisplayPage>,
    pub institutions: Entity<InstitutionsPage>,
    pub sync_server: Entity<SyncServerPage>,
    pub data_backup: Entity<DataBackupPage>,
    pub about: Entity<AboutPage>,
    pub units: Entity<UnitsPage>,
    pub accounts: Entity<AccountsPage>,
    pub categories: Entity<CategoriesPage>,
    pub payees: Entity<PayeesPage>,
    pub tags: Entity<TagsPage>,
    pub documents: Entity<DocumentsPage>,
    pub inventory: Entity<InventoryPage>,
    pub tracing: Entity<TracingPage>,
}

/// The open page's own Entity, while it has focus.
#[derive(Clone)]
enum FocusedPage {
    Accounts(Entity<AccountsPage>),
    Categories(Entity<CategoriesPage>),
    Payees(Entity<PayeesPage>),
    Tags(Entity<TagsPage>),
    Documents(Entity<DocumentsPage>),
    Inventory(Entity<InventoryPage>),
    Units(Entity<UnitsPage>),
    Institutions(Entity<InstitutionsPage>),
    Tracing(Entity<TracingPage>),
}

impl FocusedPage {
    fn key_ahead(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Categories(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Tags(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Documents(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Inventory(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Units(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Institutions(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Tracing(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
        }
    }

    fn handle_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Categories(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Tags(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Documents(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Inventory(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Units(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Institutions(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Tracing(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
        }
    }

    /// Whether the page took the movement; one without a selection leaves Settings to scroll.
    fn apply_movement(&self, movement: Movement, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => {
                page.update(cx, |page, cx| page.apply_movement(movement, cx));
                true
            }
            Self::Categories(page) => {
                page.update(cx, |page, cx| page.apply_movement(movement, cx));
                true
            }
            Self::Payees(page) => {
                page.update(cx, |page, cx| page.apply_movement(movement, cx));
                true
            }
            Self::Tags(page) => {
                page.update(cx, |page, cx| page.apply_movement(movement, cx));
                true
            }
            Self::Documents(page) => {
                page.update(cx, |page, cx| page.apply_movement(movement, cx));
                true
            }
            Self::Inventory(page) => {
                page.update(cx, |page, cx| page.apply_movement(movement, cx));
                true
            }
            Self::Units(page) => page.update(cx, |page, cx| page.apply_movement(movement, cx)),
            Self::Institutions(page) => {
                page.update(cx, |page, cx| page.apply_movement(movement, cx))
            }
            Self::Tracing(page) => page.update(cx, |page, cx| page.apply_movement(movement, cx)),
        }
    }

    fn escape(&self, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Categories(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Tags(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Documents(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Inventory(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Units(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Institutions(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Tracing(page) => page.update(cx, |page, cx| page.escape(cx)),
        }
    }

    fn start_search(&self, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Categories(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Tags(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Documents(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Inventory(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Units(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Institutions(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Tracing(page) => page.update(cx, |page, cx| page.start_search(cx)),
        }
    }

    fn search_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Categories(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Tags(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Documents(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Inventory(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Units(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Institutions(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Tracing(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
        }
    }

    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        match self {
            Self::Accounts(page) => page.read(cx).page_status(cx),
            Self::Categories(page) => page.read(cx).page_status(cx),
            Self::Payees(page) => page.read(cx).page_status(cx),
            Self::Tags(page) => page.read(cx).page_status(cx),
            Self::Documents(page) => page.read(cx).page_status(cx),
            Self::Inventory(page) => page.read(cx).page_status(cx),
            Self::Units(page) => page.read(cx).page_status(cx),
            Self::Institutions(page) => page.read(cx).page_status(cx),
            Self::Tracing(page) => page.read(cx).page_status(cx),
        }
    }
}

/// The Settings View's own state. The page is the last one visited, so it survives leaving and
/// re-entering Settings, as `g s` relies on.
pub struct SettingsView {
    sources: SettingsSources,
    selected_section: SettingsPage,
    focus: SettingsFocus,
    /// The page body's scroll, which also gives the Colour Theme grid its width.
    pub(super) scroll_handle: ScrollHandle,
    /// Whether the View zone has focus, told by `Shell` as it draws the View.
    pub(super) zone_focused: bool,
    /// The Window Size the View was last drawn at, told by `Shell` as it draws the View, so its
    /// keys and status line act on the layout on screen. At Medium, Settings is a drill-down: the
    /// index is a page of its own and a page's heading carries the page picker (DESKTOP-119).
    window_size: WindowSize,
    /// The Medium page picker, while it is open.
    picker: Option<PagePicker>,
    /// Keeps the Categories and Payees pages' row clicks reaching this View, which owns the page's
    /// focus.
    _categories_row_clicked: Subscription,
    _payees_row_clicked: Subscription,
    _tags_row_clicked: Subscription,
    _documents_row_clicked: Subscription,
    _inventory_row_clicked: Subscription,
}

impl SettingsView {
    pub fn new(sources: SettingsSources, cx: &mut Context<'_, Self>) -> Self {
        let _categories_row_clicked = cx.subscribe(
            &sources.categories,
            |view, _page, _event: &CategoryRowClicked, cx| view.focus_page_from_click(cx),
        );
        let _payees_row_clicked = cx
            .subscribe(&sources.payees, |view, _page, _event: &RowClicked, cx| {
                view.focus_page_from_click(cx)
            });
        let _tags_row_clicked = cx
            .subscribe(&sources.tags, |view, _page, _event: &TagsRowClicked, cx| {
                view.focus_page_from_click(cx)
            });
        let _documents_row_clicked = cx.subscribe(
            &sources.documents,
            |view, _page, _event: &DocumentsRowClicked, cx| view.focus_page_from_click(cx),
        );
        let _inventory_row_clicked = cx.subscribe(
            &sources.inventory,
            |view, _page, _event: &InventoryRowClicked, cx| view.focus_page_from_click(cx),
        );
        Self {
            sources,
            selected_section: SettingsPage::default(),
            focus: SettingsFocus::default(),
            scroll_handle: ScrollHandle::new(),
            zone_focused: false,
            window_size: WindowSize::Large,
            picker: None,
            _categories_row_clicked,
            _payees_row_clicked,
            _tags_row_clicked,
            _documents_row_clicked,
            _inventory_row_clicked,
        }
    }

    pub(super) fn sources(&self) -> &SettingsSources {
        &self.sources
    }

    /// The Accounts page's Entity, for `Shell`'s Dialog host and the `accounts` command.
    pub fn accounts(&self) -> &Entity<AccountsPage> {
        &self.sources.accounts
    }

    /// The Categories page's Entity, for `Shell`'s Dialog host and the Settings snapshots.
    pub fn categories(&self) -> &Entity<CategoriesPage> {
        &self.sources.categories
    }

    /// The Payees page's Entity, for `Shell`'s Dialog host.
    pub fn payees(&self) -> &Entity<PayeesPage> {
        &self.sources.payees
    }

    /// The Tags page's Entity, for `Shell`'s Dialog host and the `tags merge` command.
    pub fn tags(&self) -> &Entity<TagsPage> {
        &self.sources.tags
    }

    /// The Document Types page's Entity, for `Shell`'s Dialog host and the Settings snapshots.
    pub fn documents(&self) -> &Entity<DocumentsPage> {
        &self.sources.documents
    }

    /// The Inventory page's Entity, for `Shell`'s Dialog host and the Settings snapshots.
    pub fn inventory(&self) -> &Entity<InventoryPage> {
        &self.sources.inventory
    }

    /// The Display page's Entity, for the Settings snapshots.
    pub fn display_page(&self) -> &Entity<DisplayPage> {
        &self.sources.display
    }

    /// The Units page's Entity, for the Settings snapshots.
    pub fn units(&self) -> &Entity<UnitsPage> {
        &self.sources.units
    }

    /// The Tracing page's Entity, for `Shell`'s capture feed and Dialog host and the Settings
    /// snapshots.
    pub fn tracing(&self) -> &Entity<TracingPage> {
        &self.sources.tracing
    }

    /// The Institutions page's Entity, for the Settings snapshots.
    pub fn institutions(&self) -> &Entity<InstitutionsPage> {
        &self.sources.institutions
    }

    /// The open page's own Entity while it has focus: the page, rather than this View, then takes
    /// its keys, movement and status. `None` on the index and on a page this View drives.
    fn focused_page(&self) -> Option<FocusedPage> {
        if self.focus != SettingsFocus::Page {
            return None;
        }
        match self.selected_section {
            SettingsPage::Accounts => Some(FocusedPage::Accounts(self.sources.accounts.clone())),
            SettingsPage::Categories => {
                Some(FocusedPage::Categories(self.sources.categories.clone()))
            }
            SettingsPage::Payees => Some(FocusedPage::Payees(self.sources.payees.clone())),
            SettingsPage::Tags => Some(FocusedPage::Tags(self.sources.tags.clone())),
            SettingsPage::Documents => Some(FocusedPage::Documents(self.sources.documents.clone())),
            SettingsPage::Inventory => Some(FocusedPage::Inventory(self.sources.inventory.clone())),
            SettingsPage::Units => Some(FocusedPage::Units(self.sources.units.clone())),
            SettingsPage::Institutions => {
                Some(FocusedPage::Institutions(self.sources.institutions.clone()))
            }
            SettingsPage::Tracing => Some(FocusedPage::Tracing(self.sources.tracing.clone())),
            _ => None,
        }
    }

    pub fn set_zone_focused(&mut self, focused: bool) {
        self.zone_focused = focused;
    }

    /// Tells the View the Window Size it is drawn at. Leaving Medium closes the picker, which only
    /// Medium draws.
    pub fn set_window_size(&mut self, window_size: WindowSize, cx: &mut Context<'_, Self>) {
        self.window_size = window_size;
        if window_size != WindowSize::Medium && self.picker.is_some() {
            self.close_picker(cx);
        }
    }

    /// Whether Settings is drawn as a drill-down, the index and each page taking the View in turn.
    pub(super) fn drill_down(&self) -> bool {
        self.window_size == WindowSize::Medium
    }

    pub(super) fn picker(&self) -> Option<&PagePicker> {
        self.picker.as_ref()
    }

    // -----------------------------------------------------------------------------------------
    // The interface `Shell` forwards to (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// Settings' keys ahead of the router, in order: the Display page's controls and Colour Theme
    /// grid, the focused page's keys (Tracing's among them), then the index-and-page focus keys. The Display page goes
    /// first, since its `h`/`l` must win over the focus keys' `h`. `false` for any key none of them
    /// takes. `Esc` is left to [`Self::escape`].
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.key == "escape" {
            return false;
        }
        if self.picker.is_some() {
            self.picker_key(keystroke, cx);
            cx.notify();
            return true;
        }
        if self.medium_page_key(keystroke, cx) {
            cx.notify();
            return true;
        }
        // The focused page goes before `focus_key`, which would otherwise move focus to the index
        // on `left` before the page could fold its own tree.
        let taken = self.display_key(keystroke, cx)
            || self
                .focused_page()
                .is_some_and(|page| page.key_ahead(keystroke, cx))
            || self.focus_key(keystroke, cx);
        if taken {
            cx.notify();
        }
        taken
    }

    /// A router leftover goes to the open page's own Entity; the static pages claim their keys
    /// ahead of the router.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        self.focused_page()
            .is_some_and(|page| page.handle_key(keystroke, cx))
    }

    /// `j`/`k`/`g g`/`G` on the index step through the pages and swap the page live, like an index
    /// click. A page with its own selection moves it; any other page scrolls.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        if let Some(page) = self.focused_page()
            && page.apply_movement(movement, cx)
        {
            return;
        }
        // A movement the grid did not take (`Ctrl-d`/`Ctrl-u`) leaves it, as any other key does.
        self.sources
            .display
            .update(cx, |page, cx| page.leave_grid(cx));
        if self.focus == SettingsFocus::Index
            && matches!(
                movement,
                Movement::Next | Movement::Prev | Movement::First | Movement::Last
            )
        {
            self.apply_section_movement(movement, cx);
        } else {
            crate::view::scroll_by_movement(&self.scroll_handle, movement);
        }
        cx.notify();
    }

    /// `Esc` closes what the open page has open, then leaves the Colour Theme grid for the
    /// control above it, or the page for the index.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.picker.is_some() {
            self.close_picker(cx);
            cx.notify();
            return true;
        }
        if let Some(page) = self.focused_page()
            && page.escape(cx)
        {
            return true;
        }
        if self.selected_section == SettingsPage::Display
            && self.sources.display.update(cx, |page, cx| page.escape(cx))
        {
            cx.notify();
            return true;
        }
        if self.focus == SettingsFocus::Page {
            self.focus_index(cx);
        } else {
            return false;
        }
        cx.notify();
        true
    }

    /// Settings has no Search of its own: `/` goes to the open page's Entity, and is inert
    /// where it has none rather than opening a mode with no box.
    pub fn start_search(&mut self, cx: &mut Context<'_, Self>) -> bool {
        self.focused_page()
            .is_some_and(|page| page.start_search(cx))
    }

    pub fn search_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        self.focused_page()
            .is_some_and(|page| page.search_key(keystroke, cx))
    }

    /// Settings has just become the destination: focus goes back to the index (`g s` "lands on
    /// the index"), and the page on show, the last-visited one, starts at its top.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.scroll_handle.set_offset(gpui::Point::default());
        self.focus_index(cx);
        cx.notify();
    }

    /// Opens the page that owns the record, with focus in it, and has its Entity show it.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        match reveal {
            Reveal::Account(_) => {
                self.open_page(SettingsPage::Accounts, cx);
                self.sources
                    .accounts
                    .update(cx, |page, cx| page.reveal(reveal, cx));
            }
            Reveal::Payee(_) => {
                self.open_page(SettingsPage::Payees, cx);
                self.sources
                    .payees
                    .update(cx, |page, cx| page.reveal(reveal, cx));
            }
            // The rest belong to other Views.
            Reveal::AccountTransactions(_)
            | Reveal::Transaction(_)
            | Reveal::BillEntry(_)
            | Reveal::BillPlan(_)
            | Reveal::BillsPeriod(_) => {}
        }
    }

    /// The status line's legend for the index or the open page, and the page's scope note, or
    /// the open page's own Entity's status. `None` while the View zone doesn't have focus.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        if !self.zone_focused {
            return None;
        }
        if self.picker.is_some() {
            return Some(PageStatus {
                hints: settings_picker_hints(),
                right: self.selected_section.scope_note(),
            });
        }
        let mut status = match self.focused_page() {
            Some(page) => page.page_status(cx)?,
            None => PageStatus {
                hints: self.hints(cx),
                right: self.selected_section.scope_note(),
            },
        };
        if self.drill_down() && self.focus == SettingsFocus::Page {
            status.hints.extend(settings_medium_page_hints());
        }
        Some(status)
    }

    /// The `?` cheat-sheet's Settings group as `(action, keys)`: the index keys, then the open
    /// page's.
    pub fn cheat_sheet(&self, cx: &App) -> Vec<(String, &'static str)> {
        let page = match self.selected_section {
            SettingsPage::Accounts => accounts_hints(),
            SettingsPage::Categories => settings_categories_hints(),
            SettingsPage::Tags => settings_tags_hints(),
            SettingsPage::Documents => settings_documents_hints(),
            SettingsPage::Payees => settings_payees_hints(),
            SettingsPage::Inventory => self.sources.inventory.read(cx).hints(cx),
            SettingsPage::Display => {
                let mut keys = settings_display_hints();
                keys.extend(settings_colour_grid_hints().into_iter().take(2));
                keys
            }
            SettingsPage::Tracing => settings_tracing_hints(),
            _ => settings_plain_page_hints(),
        };
        settings_index_hints()
            .into_iter()
            .chain(page)
            .map(|(keys, action)| (action, keys))
            .collect()
    }

    /// The legend for Settings' current focus: the index rail's keys, or the open page's.
    fn hints(&self, cx: &App) -> Vec<(&'static str, String)> {
        if self.focus == SettingsFocus::Index {
            return settings_index_hints();
        }
        match self.selected_section {
            SettingsPage::Display
                if self.sources.display.read(cx).colour_theme_focus().is_some() =>
            {
                settings_colour_grid_hints()
            }
            SettingsPage::Display => settings_display_hints(),
            _ => settings_plain_page_hints(),
        }
    }

    // -----------------------------------------------------------------------------------------
    // Pages and focus
    // -----------------------------------------------------------------------------------------

    pub fn selected_section(&self) -> SettingsPage {
        self.selected_section
    }

    pub fn focus(&self) -> SettingsFocus {
        self.focus
    }

    /// Restores the last-visited page at launch, without moving focus.
    pub fn set_selected_section(&mut self, cx: &mut Context<'_, Self>, section: SettingsPage) {
        self.selected_section = section;
        cx.notify();
    }

    /// Opens a page with focus in it -- what `:settings <page>`, the old `:accounts` /
    /// `:categories` / `:payees` / `:tags` aliases and every hand-off to a moved noun land on (the
    /// keyboard model's "commands and hand-offs land in the page").
    pub fn open_page(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) {
        self.select_page(section, cx);
        self.focus_page(cx);
        cx.notify();
    }

    /// Moves focus into the open page, as a click on one of its rows does.
    pub fn focus_page_from_click(&mut self, cx: &mut Context<'_, Self>) {
        self.focus_page(cx);
        cx.notify();
    }

    /// Swaps the page on show. A new page starts at its top, with no control focused.
    fn select_page(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) {
        if self.selected_section != section {
            self.scroll_handle.set_offset(gpui::Point::default());
        }
        self.selected_section = section;
        self.sources
            .display
            .update(cx, |page, cx| page.clear_focus(cx));
    }

    /// Moves focus from the index rail into the open page (`l`/`enter`). A page with nothing to
    /// focus keeps focus on the index, as a quiet no-op.
    fn focus_page(&mut self, cx: &mut Context<'_, Self>) {
        // At Medium the page is what's on screen, so even a page with nothing to focus opens.
        if !self.selected_section.has_controls() && !self.drill_down() {
            return;
        }
        self.focus = SettingsFocus::Page;
        if self.selected_section == SettingsPage::Display {
            self.sources
                .display
                .update(cx, |page, cx| page.focus_first(cx));
        }
    }

    fn focus_index(&mut self, cx: &mut Context<'_, Self>) {
        self.focus = SettingsFocus::Index;
        self.sources
            .display
            .update(cx, |page, cx| page.clear_focus(cx));
    }

    fn apply_section_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let pages = SettingsPage::ALL;
        let last = pages.len() - 1;
        let current = pages
            .iter()
            .position(|section| *section == self.selected_section)
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => (current + 1).min(last),
            Movement::Prev => current.saturating_sub(1),
            Movement::First => 0,
            _ => last,
        };
        self.select_page(pages[next], cx);
    }

    /// An index entry's click: swaps the body to that page and leaves focus on the index.
    pub(super) fn click_index(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) {
        self.select_page(section, cx);
        self.focus = SettingsFocus::Index;
        cx.notify();
    }

    // -----------------------------------------------------------------------------------------
    // Medium: the drill-down and its page picker (DESKTOP-119 to DESKTOP-121)
    // -----------------------------------------------------------------------------------------

    /// An index page row's click at Medium: opens that page in the index's place.
    pub(super) fn click_index_page(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) {
        self.open_page(section, cx);
    }

    /// The heading's **Settings**: back to the index, with the page still selected.
    pub(super) fn click_index_crumb(&mut self, cx: &mut Context<'_, Self>) {
        if self.picker.is_some() {
            self.close_picker(cx);
        }
        self.focus_index(cx);
        cx.notify();
    }

    /// The heading's caret: opens the picker, or closes it if it is open.
    pub(super) fn click_picker_caret(&mut self, cx: &mut Context<'_, Self>) {
        if self.picker.is_some() {
            self.close_picker(cx);
        } else {
            self.open_picker(cx);
        }
        cx.notify();
    }

    /// A picker row's click: straight to that page.
    pub(super) fn click_picker_page(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) {
        self.close_picker(cx);
        self.open_page(section, cx);
    }

    /// A page's own keys at Medium: `[`/`]` step to the previous and next page in index order, and
    /// `p` opens the picker. `Esc` back to the index is [`Self::escape`]'s.
    fn medium_page_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if !self.drill_down()
            || self.focus != SettingsFocus::Page
            || modifiers.control
            || modifiers.alt
            || modifiers.platform
        {
            return false;
        }
        match keystroke.key.as_str() {
            "[" => self.step_page(Step::Prev, cx),
            "]" => self.step_page(Step::Next, cx),
            "p" if !modifiers.shift => self.open_picker(cx),
            _ => return false,
        }
        true
    }

    fn step_page(&mut self, step: Step, cx: &mut Context<'_, Self>) {
        let pages = SettingsPage::ALL;
        let next = lib_list_cursor::step(self.selected_section.index(), pages.len(), step);
        self.select_page(pages[next], cx);
        self.focus_page(cx);
    }

    fn open_picker(&mut self, cx: &mut Context<'_, Self>) {
        self.picker = Some(PagePicker::new(self.selected_section));
        cx.emit(ViewEvent::EnterMode(ViewMode::Filter));
    }

    fn close_picker(&mut self, cx: &mut Context<'_, Self>) {
        self.picker = None;
        cx.emit(ViewEvent::ExitMode);
    }

    /// Every key while the picker is open: `↑`/`↓` move, `enter` opens the chosen page, `backspace`
    /// and typing edit the filter. `Esc` comes back as [`Self::escape`].
    fn picker_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        match keystroke.key.as_str() {
            "up" => picker.step(Step::Prev),
            "down" => picker.step(Step::Next),
            "backspace" => picker.backspace(),
            "enter" => {
                if let Some(section) = picker.chosen() {
                    self.click_picker_page(section, cx);
                }
            }
            _ => {
                if let Some(ch) = crate::app::typed_char(keystroke) {
                    picker.push(ch);
                }
            }
        }
    }

    /// Settings' own focus keys: `l`/`right`/`enter` on the index step into the page, `h`/`left`
    /// on the page step back out (not from the Colour Theme grid, which has its own `h`).
    fn focus_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.modifiers.shift {
            return false;
        }
        match (self.focus, keystroke.key.as_str()) {
            (SettingsFocus::Index, "l" | "right" | "enter") => {
                self.focus_page(cx);
                true
            }
            (SettingsFocus::Page, "h" | "left")
                if self.sources.display.read(cx).colour_theme_focus().is_none() =>
            {
                self.focus_index(cx);
                true
            }
            _ => false,
        }
    }

    // -----------------------------------------------------------------------------------------
    // Display
    // -----------------------------------------------------------------------------------------

    /// The Display Preferences, which the Display page owns (ADR-0037).
    pub fn display(&self, cx: &App) -> DisplayPreferences {
        self.sources.display.read(cx).preferences()
    }

    /// Sets the Display Preferences from outside the page (launch, `:toasts`), without telling
    /// `Shell`, which made the change itself.
    pub fn set_display(&mut self, display: DisplayPreferences, cx: &mut Context<'_, Self>) {
        self.sources
            .display
            .update(cx, |page, cx| page.set_preferences(display, cx));
    }

    /// The Display page's keys while it has focus. Its `h` at the grid's first column leaves the
    /// page for the index, the one focus change the page cannot make itself.
    fn display_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if self.focus != SettingsFocus::Page || self.selected_section != SettingsPage::Display {
            return false;
        }
        let body_width = f32::from(self.scroll_handle.bounds().size.width);
        match self
            .sources
            .display
            .update(cx, |page, cx| page.key(keystroke, body_width, cx))
        {
            DisplayKey::Unhandled => false,
            DisplayKey::Handled => true,
            DisplayKey::FocusIndex => {
                self.focus_index(cx);
                true
            }
        }
    }
}
