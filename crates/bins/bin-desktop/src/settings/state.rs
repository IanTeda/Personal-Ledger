//! The Settings destination's Entity (ADR-0032, ADR-0037). [`SettingsView`] owns the View state
//! that belongs to Settings: the page on show, where keyboard focus sits, the Display page's
//! focused control and Preferences, the Tracing page's log, and the Documents and Inventory
//! pages' selections. It renders itself (`impl Render` in `super`), handles the index, its focus
//! keys and the static pages' keys, movement and `Esc`, and reaches `Shell` only through
//! [`ViewEvent`]. It builds the Clear logs dialog, which `Shell`'s Dialog host draws and confirms.
//!
//! A page with Ledger data that has moved into its own Entity (Accounts, [`AccountsPage`];
//! Payees, [`PayeesPage`]; Tags, [`TagsPage`]; Units, [`UnitsPage`]; Institutions, [`InstitutionsPage`]) is drawn by
//! that Entity, and this View forwards the page's keys, movement and status to it while the page
//! has focus. The rest (Categories, Documents, Inventory) are still `Shell`'s: it hands their
//! drawn page in as a [`DataPage`] each frame, and handles their keys before forwarding to this
//! Entity, until each moves in its own ticket.

use std::collections::HashSet;

use super::{
    DataPage,
    hints::{
        settings_colour_grid_hints, settings_display_hints, settings_index_hints,
        settings_plain_page_hints, settings_tracing_hints,
    },
    tracing::LOG_LINE_STEP,
};
use crate::settings::payees::{PayeesPage, RowClicked};
use crate::settings::tags::{RowClicked as TagsRowClicked, TagsPage};
use crate::{
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    navigation::key_router::Movement,
    settings::accounts::AccountsPage,
    settings::institutions::InstitutionsPage,
    settings::units::UnitsPage,
    settings::{
        SettingsDialog, SettingsFocus, SettingsPage,
        display::{DisplayKey, DisplayPage, DisplayPreferences},
        inventory::InventoryRow,
        step_choice,
        tracing_log::{LogChange, LogView, TracingLevel},
    },
    view::event::{Reveal, ViewEvent},
};
use gpui::{
    App, Context, Entity, EventEmitter, Keystroke, ListOffset, ListState, ScrollHandle,
    Subscription, px,
};

impl EventEmitter<ViewEvent> for SettingsView {}

/// What the Settings pages read: the Entity of each page with Ledger data that draws itself.
pub struct SettingsSources {
    pub display: Entity<DisplayPage>,
    pub institutions: Entity<InstitutionsPage>,
    pub units: Entity<UnitsPage>,
    pub accounts: Entity<AccountsPage>,
    pub payees: Entity<PayeesPage>,
    pub tags: Entity<TagsPage>,
}

/// The open page's own Entity, while it has focus.
#[derive(Clone)]
enum FocusedPage {
    Accounts(Entity<AccountsPage>),
    Payees(Entity<PayeesPage>),
    Tags(Entity<TagsPage>),
    Units(Entity<UnitsPage>),
    Institutions(Entity<InstitutionsPage>),
}

impl FocusedPage {
    fn key_ahead(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Tags(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Units(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Institutions(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
        }
    }

    fn handle_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Tags(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Units(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Institutions(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
        }
    }

    /// Whether the page took the movement; one without a selection leaves Settings to scroll.
    fn apply_movement(&self, movement: Movement, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => {
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
            Self::Units(page) => page.update(cx, |page, cx| page.apply_movement(movement, cx)),
            Self::Institutions(page) => {
                page.update(cx, |page, cx| page.apply_movement(movement, cx))
            }
        }
    }

    fn escape(&self, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Tags(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Units(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Institutions(page) => page.update(cx, |page, cx| page.escape(cx)),
        }
    }

    fn start_search(&self, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Tags(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Units(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Institutions(page) => page.update(cx, |page, cx| page.start_search(cx)),
        }
    }

    fn search_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Tags(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Units(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Institutions(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
        }
    }

    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        match self {
            Self::Accounts(page) => page.read(cx).page_status(cx),
            Self::Payees(page) => page.read(cx).page_status(cx),
            Self::Tags(page) => page.read(cx).page_status(cx),
            Self::Units(page) => page.read(cx).page_status(cx),
            Self::Institutions(page) => page.read(cx).page_status(cx),
        }
    }
}

/// The Settings View's own state. The page is the last one visited, so it survives leaving and
/// re-entering Settings, as `g s` relies on. The Tracing page's log mirror and its list live here
/// too: the capture feed in `Shell` pushes into them, and only this page reads them.
pub struct SettingsView {
    sources: SettingsSources,
    selected_section: SettingsPage,
    focus: SettingsFocus,
    documents_selected: Option<u32>,
    inventory_selected: Option<InventoryRow>,
    inventory_expanded: HashSet<u32>,
    log: LogView,
    log_list: ListState,
    /// The page body's scroll, which also gives the Colour Theme grid its width.
    pub(super) scroll_handle: ScrollHandle,
    /// Whether the View zone has focus, told by `Shell` as it draws the View.
    pub(super) zone_focused: bool,
    /// The page `Shell` still draws, handed in for the frame it is drawn in.
    pub(super) data_page: Option<DataPage>,
    /// Keeps the Payees page's row clicks reaching this View, which owns the page's focus.
    _payees_row_clicked: Subscription,
    _tags_row_clicked: Subscription,
}

impl SettingsView {
    /// `log` mirrors the live capture; `log_list` is the virtualised list that shows it, and must
    /// be kept in step with `log` by [`Self::pull_log`] and [`Self::replace_log`].
    pub fn new(
        sources: SettingsSources,
        log: LogView,
        log_list: ListState,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let _payees_row_clicked = cx
            .subscribe(&sources.payees, |view, _page, _event: &RowClicked, cx| {
                view.focus_page_from_click(cx)
            });
        let _tags_row_clicked = cx
            .subscribe(&sources.tags, |view, _page, _event: &TagsRowClicked, cx| {
                view.focus_page_from_click(cx)
            });
        Self {
            sources,
            selected_section: SettingsPage::default(),
            focus: SettingsFocus::default(),
            documents_selected: None,
            inventory_selected: None,
            inventory_expanded: HashSet::new(),
            log,
            log_list,
            scroll_handle: ScrollHandle::new(),
            zone_focused: false,
            data_page: None,
            _payees_row_clicked,
            _tags_row_clicked,
        }
    }

    pub(super) fn sources(&self) -> &SettingsSources {
        &self.sources
    }

    /// The Accounts page's Entity, for `Shell`'s Dialog host and the `accounts` command.
    pub fn accounts(&self) -> &Entity<AccountsPage> {
        &self.sources.accounts
    }

    /// The Payees page's Entity, for `Shell`'s Dialog host.
    pub fn payees(&self) -> &Entity<PayeesPage> {
        &self.sources.payees
    }

    /// The Tags page's Entity, for `Shell`'s Dialog host and the `tags merge` command.
    pub fn tags(&self) -> &Entity<TagsPage> {
        &self.sources.tags
    }

    /// The Display page's Entity, for the Settings snapshots.
    pub fn display_page(&self) -> &Entity<DisplayPage> {
        &self.sources.display
    }

    /// The Units page's Entity, for the Settings snapshots.
    pub fn units(&self) -> &Entity<UnitsPage> {
        &self.sources.units
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
            SettingsPage::Payees => Some(FocusedPage::Payees(self.sources.payees.clone())),
            SettingsPage::Tags => Some(FocusedPage::Tags(self.sources.tags.clone())),
            SettingsPage::Units => Some(FocusedPage::Units(self.sources.units.clone())),
            SettingsPage::Institutions => {
                Some(FocusedPage::Institutions(self.sources.institutions.clone()))
            }
            _ => None,
        }
    }

    pub fn set_zone_focused(&mut self, focused: bool) {
        self.zone_focused = focused;
    }

    /// Hands in the page `Shell` still draws, for the frame about to be drawn. `None` on a page
    /// this View draws itself.
    pub fn set_data_page(&mut self, page: Option<DataPage>) {
        self.data_page = page;
    }

    // -----------------------------------------------------------------------------------------
    // The interface `Shell` forwards to (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// Settings' keys ahead of the router, in order: the Display page's controls and Colour Theme
    /// grid, the Tracing page's keys, then the index-and-page focus keys. The Display page goes
    /// first, since its `h`/`l` must win over the focus keys' `h`. `false` for any key none of them
    /// takes. `Esc` is left to [`Self::escape`].
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.key == "escape" {
            return false;
        }
        let taken = self.display_key(keystroke, cx)
            || self.tracing_key(keystroke, cx)
            || self.focus_key(keystroke, cx)
            || self
                .focused_page()
                .is_some_and(|page| page.key_ahead(keystroke, cx));
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
        if let Some(page) = self.focused_page() {
            return page.page_status(cx);
        }
        Some(PageStatus {
            hints: self.hints(cx),
            right: self.selected_section.scope_note(),
        })
    }

    /// The legend for Settings' current focus: the index rail's keys, or the open page's. The
    /// pages `Shell` still draws have their own.
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
            SettingsPage::Tracing => settings_tracing_hints(),
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
        if !self.selected_section.has_controls() {
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

    /// The buttons with no dialog or behaviour yet (Sync now, Backup now, Export
    /// ledger) say so on the status line.
    pub(super) fn not_built(&mut self, message: String, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(message)));
    }

    // -----------------------------------------------------------------------------------------
    // Tracing
    // -----------------------------------------------------------------------------------------

    pub fn log(&self) -> &LogView {
        &self.log
    }

    pub fn log_list(&self) -> &ListState {
        &self.log_list
    }

    /// Hands the page a new capture, opening on `log`'s level, and restarts the list at its top.
    pub fn replace_log(&mut self, cx: &mut Context<'_, Self>, log: LogView) {
        self.log = log;
        self.log_list.reset(self.log.visible().len());
        cx.notify();
    }

    /// Pulls new entries from the capture and mirrors them onto the list. Notifies only when
    /// something changed, so an idle capture doesn't redraw the page.
    pub fn pull_log(&mut self, cx: &mut Context<'_, Self>) -> LogChange {
        let change = self.log.pull();
        if change != LogChange::default() {
            self.mirror_log_change(change);
            cx.notify();
        }
        change
    }

    /// Mirrors a pull onto the list: evicted rows leave the bottom, new ones arrive at the top.
    /// A reader at the very top keeps seeing the newest; one scrolled down stays where they are.
    fn mirror_log_change(&mut self, change: LogChange) {
        let list = &self.log_list;
        let top = list.logical_scroll_top();
        let at_top = top.item_ix == 0 && top.offset_in_item <= px(0.0);
        let count = list.item_count();
        let evicted = change.evicted.min(count);
        list.splice(count - evicted..count, 0);
        list.splice(0..0, change.added);
        if at_top {
            list.scroll_to(ListOffset::default());
        }
    }

    /// Edits the log mirror in place (its level and clear), then redraws. The list is reset to
    /// the mirror's visible rows, as a level or clear changes which rows show.
    fn update_log(&mut self, cx: &mut Context<'_, Self>, edit: impl FnOnce(&mut LogView)) {
        edit(&mut self.log);
        self.log_list.reset(self.log.visible().len());
        cx.notify();
    }

    /// Empties the capture, once Clear logs is confirmed.
    pub fn clear_log(&mut self, cx: &mut Context<'_, Self>) {
        self.update_log(cx, LogView::clear);
    }

    /// The level radios filter the log box at once, including what is already there, and start
    /// it back at the newest entry.
    pub(super) fn set_tracing_level(&mut self, level: TracingLevel, cx: &mut Context<'_, Self>) {
        self.update_log(cx, |log| log.set_level(level));
    }

    /// **Clear logs** (and `c`) asks first, since the capture is emptied for good (#502).
    pub(super) fn click_clear_logs(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(None));
        open_dialog(SettingsDialog::ClearLogs, cx);
    }

    /// The Tracing page's keys while it has focus, ahead of the focus keys so `h` steps the level
    /// rather than leaving (Display's radio grammar; `esc` leaves): `h`/`l` the level, `j`/`k` a
    /// line, `J`/`K` a page, `G` the oldest entry, `c` Clear logs. Focus never enters the box
    /// itself.
    fn tracing_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if self.focus != SettingsFocus::Page || self.selected_section != SettingsPage::Tracing {
            return false;
        }
        let list = self.log_list.clone();
        let page = list.viewport_bounds().size.height.max(LOG_LINE_STEP);
        match (keystroke.modifiers.shift, keystroke.key.as_str()) {
            (false, "h" | "left") => self.step_tracing_level(-1, cx),
            (false, "l" | "right") => self.step_tracing_level(1, cx),
            (false, "j" | "down") => list.scroll_by(LOG_LINE_STEP),
            (false, "k" | "up") => list.scroll_by(-LOG_LINE_STEP),
            (true, "j") => list.scroll_by(page),
            (true, "k") => list.scroll_by(-page),
            (true, "g") => {
                if let Some(last) = list.item_count().checked_sub(1) {
                    list.scroll_to_reveal_item(last);
                }
            }
            (false, "c") => self.click_clear_logs(cx),
            _ => return false,
        }
        true
    }

    fn step_tracing_level(&mut self, delta: isize, cx: &mut Context<'_, Self>) {
        let level = step_choice(&TracingLevel::ALL, self.log.level(), delta);
        self.set_tracing_level(level, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Documents and Inventory selections, which `Shell` still drives
    // -----------------------------------------------------------------------------------------

    pub fn documents_selected(&self) -> Option<u32> {
        self.documents_selected
    }

    pub fn inventory_selected(&self) -> Option<InventoryRow> {
        self.inventory_selected
    }

    /// The Properties shown open on the Inventory page.
    pub fn inventory_expanded(&self) -> &HashSet<u32> {
        &self.inventory_expanded
    }

    pub fn set_documents_selected(&mut self, cx: &mut Context<'_, Self>, id: Option<u32>) {
        self.documents_selected = id;
        cx.notify();
    }

    pub fn set_inventory_selected(
        &mut self,
        cx: &mut Context<'_, Self>,
        row: Option<InventoryRow>,
    ) {
        self.inventory_selected = row;
        cx.notify();
    }

    /// Shows Property `id` open. Returns whether it was closed before, so the caller can tell a
    /// real change from a no-op.
    pub fn expand(&mut self, cx: &mut Context<'_, Self>, id: u32) -> bool {
        let inserted = self.inventory_expanded.insert(id);
        cx.notify();
        inserted
    }

    /// Shows Property `id` closed. Returns whether it was open before.
    pub fn collapse(&mut self, cx: &mut Context<'_, Self>, id: u32) -> bool {
        let removed = self.inventory_expanded.remove(&id);
        cx.notify();
        removed
    }

    /// Edits the expanded Properties in place, for the fold and disclosure rules to run on.
    pub fn update_inventory_expanded(
        &mut self,
        cx: &mut Context<'_, Self>,
        edit: impl FnOnce(&mut HashSet<u32>),
    ) {
        edit(&mut self.inventory_expanded);
        cx.notify();
    }
}

/// Asks `Shell`'s Dialog host to open a Settings dialog.
fn open_dialog(dialog: SettingsDialog, cx: &mut Context<'_, SettingsView>) {
    cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Settings(
        dialog,
    ))));
}
