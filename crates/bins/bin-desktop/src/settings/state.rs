//! The Settings destination's Entity (ADR-0032, ADR-0037). [`SettingsView`] owns the View state
//! that belongs to Settings: the page on show, where keyboard focus sits and the Medium page
//! picker. It builds its pages ([`SettingsPages`]), renders itself (`impl Render` in `super`),
//! handles the index and its focus keys, and reaches `Root` only through [`ViewEvent`], which it
//! re-emits from its pages.
//!
//! Every page is a [`Page`](super::pages::Page) behind one `dyn` handle, so while a page has
//! focus this View forwards its keys, movement, `Esc`, Search and status to it without knowing
//! which page it is.

use super::hints::{settings_index_hints, settings_medium_page_hints, settings_picker_hints};
use crate::{
    ledger_store::LedgerStore,
    navigation::movement::Movement,
    settings::display::DisplayPreferences,
    settings::general::GeneralPage,
    settings::pages::{LedgerPages, Page, PageFrame, PageHandle, SettingsPages, SettingsSetup},
    settings::picker::PagePicker,
    settings::{SettingsFocus, SettingsPage},
    shell::view::{
        statusline::{PageStatus, owned_hints},
        window_size::WindowSize,
    },
    view::event::{Reveal, ViewEvent, ViewMode},
    view::help::HelpGroup,
};
use gpui::{App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, Subscription};
use lib_list_cursor::Step;

impl EventEmitter<ViewEvent> for SettingsView {}

/// The Settings View's own state. The page is the last one visited, so it survives leaving and
/// re-entering Settings, as `g s` relies on.
pub struct SettingsView {
    pages: SettingsPages,
    selected_section: SettingsPage,
    focus: SettingsFocus,
    /// The page body's scroll, which also gives the Colour Theme grid its width.
    pub(super) scroll_handle: ScrollHandle,
    /// The Medium index page's scroll, its own so moving the selection doesn't reset it.
    pub(super) index_scroll: ScrollHandle,
    /// Whether the View zone has focus, told by `Root` as it draws the View.
    pub(super) zone_focused: bool,
    /// The Window Size the View was last drawn at, told by `Root` as it draws the View, so its
    /// keys and status line act on the layout on screen. At Medium, Settings is a drill-down: the
    /// index is a page of its own and a page's heading carries the page picker (DESKTOP-119).
    window_size: WindowSize,
    /// The Medium page picker, while it is open.
    picker: Option<PagePicker>,
    /// Keeps each page's events reaching this View: its `ViewEvent`s re-emitted for `Root`, its
    /// changes repainting this View and its row clicks handing it focus.
    _page_subscriptions: Vec<Subscription>,
}

impl SettingsView {
    /// Builds the This device pages, which stand with no Ledger open; [`Self::open_ledger`] adds
    /// the This Ledger pages. The Tracing capture arrives later, through `Root::set_log_capture`.
    pub fn new(cx: &mut Context<'_, Self>) -> Self {
        let pages = SettingsPages::new(cx);
        let _page_subscriptions = pages.all().flat_map(|page| page.subscribe(cx)).collect();
        Self {
            pages,
            selected_section: SettingsPage::default(),
            focus: SettingsFocus::default(),
            scroll_handle: ScrollHandle::new(),
            index_scroll: ScrollHandle::new(),
            zone_focused: false,
            window_size: WindowSize::Large,
            picker: None,
            _page_subscriptions,
        }
    }

    /// Builds the This Ledger pages on the Open Ledger's `ledger`, so they can be shown, in place
    /// of any built on a Ledger opened before.
    pub fn open_ledger(
        &mut self,
        ledger: &Entity<LedgerStore>,
        clock: &lib_core::Clock,
        setup: SettingsSetup,
        cx: &mut Context<'_, Self>,
    ) {
        let pages = LedgerPages::new(ledger, clock, setup, cx);
        self.pages.ledger = Some(pages);
        // Subscribed afresh, so a swapped-out Ledger's pages leave no subscription behind.
        self._page_subscriptions = self
            .pages
            .all()
            .flat_map(|page| page.subscribe(cx))
            .collect();
        self.sync_frame(cx);
        cx.notify();
    }

    /// Drops the This Ledger pages with the Ledger they were built on. A This Ledger page on show
    /// gives way to General, as at a start with no Ledger open.
    pub fn close_ledger(&mut self, cx: &mut Context<'_, Self>) {
        self.pages.ledger = None;
        self._page_subscriptions = self
            .pages
            .all()
            .flat_map(|page| page.subscribe(cx))
            .collect();
        if !self.can_show(self.selected_section) {
            self.selected_section = SettingsPage::General;
        }
        self.sync_frame(cx);
        cx.notify();
    }

    /// Whether a Ledger is open, so the This Ledger pages are built.
    pub fn ledger_open(&self) -> bool {
        self.pages.ledger.is_some()
    }

    /// Whether `page` can be shown: every This device page, and a This Ledger page only with a
    /// Ledger open.
    pub fn can_show(&self, page: SettingsPage) -> bool {
        self.pages.get(page).is_some()
    }

    /// The open page's handle. Every way onto a page checks [`Self::can_show`] first, so the
    /// selected page is always built.
    #[expect(
        clippy::expect_used,
        reason = "select_page and set_selected_section only ever select a page can_show allows"
    )]
    pub(super) fn selected_page(&self) -> &dyn PageHandle {
        self.pages
            .get(self.selected_section)
            .expect("the selected Settings page is one that is built")
    }

    pub(super) fn pages(&self) -> &SettingsPages {
        &self.pages
    }

    /// The `T` page's Entity, for what `Root` drives on a page: a module Router's entry point, the
    /// Tracing capture feed and the Settings snapshots. Found through `T::PAGE`, so a new page needs
    /// no accessor of its own (ADR-0037's 2026-10-30 amendment).
    /// A This Ledger page is built only with a Ledger open, so only a caller on the Open Ledger asks
    /// for one.
    #[expect(
        clippy::expect_used,
        reason = "SettingsPages::get and T::PAGE can't disagree, page_returns_the_page_for_every_settings_page checks each page, and only a caller on the Open Ledger asks for a This Ledger page"
    )]
    pub fn page<T: Page>(&self) -> Entity<T> {
        self.pages
            .get(T::PAGE)
            .expect("a This Ledger page is asked for only with a Ledger open")
            .view()
            .downcast::<T>()
            // The `AnyView` handed back isn't `Debug`, so `expect` needs it dropped.
            .map_err(drop)
            .expect("`SettingsPages::get` hands back `T::PAGE`'s page, which is a `T`")
    }

    /// The open page, while it has focus: the page, rather than this View, then takes its keys,
    /// movement and status. `None` on the index.
    fn focused_page(&self) -> Option<&dyn PageHandle> {
        (self.focus == SettingsFocus::Page).then(|| self.selected_page())
    }

    /// The space the pages are drawn in, as the pages are told it.
    fn frame(&self) -> PageFrame {
        PageFrame {
            window_size: self.window_size,
            body_width: f32::from(self.scroll_handle.bounds().size.width),
        }
    }

    /// Tells every page the frame, so a page acts on the layout on screen, not the one it last saw.
    pub(super) fn sync_frame(&self, cx: &mut App) {
        let frame = self.frame();
        for page in self.pages.all() {
            page.set_frame(frame, cx);
        }
    }

    pub fn set_zone_focused(&mut self, focused: bool) {
        self.zone_focused = focused;
    }

    /// Tells the View the Window Size it is drawn at. Leaving Medium closes the picker, which only
    /// Medium draws.
    pub fn set_window_size(&mut self, window_size: WindowSize, cx: &mut Context<'_, Self>) {
        self.window_size = window_size;
        self.sync_frame(cx);
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
    // The interface `Root` forwards to (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// The focused page's module keymap, by `chord`, ahead of every key of Settings' own. Not while
    /// the page picker is open, which takes every key.
    pub fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        if self.picker.is_some() {
            return false;
        }
        self.sync_frame(cx);
        let taken = self
            .focused_page()
            .is_some_and(|page| page.module_key(chord, cx));
        if taken {
            cx.notify();
        }
        taken
    }

    /// A palette Command on `section`'s page, by its keymap `command` name. Whether the page has it.
    pub fn module_command(
        &mut self,
        section: SettingsPage,
        command: &str,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        self.sync_frame(cx);
        let taken = self
            .pages
            .get(section)
            .is_some_and(|page| page.module_command(command, cx));
        if taken {
            cx.notify();
        }
        taken
    }

    /// Settings' own keys ahead of the Shell's fallback, in order: the focused page's keys (the Display page's
    /// controls and Colour Theme grid and Tracing's among them), then the index-and-page focus
    /// keys. The page goes first, since a page's `h`/`l` must win over the focus keys' `h`. `false`
    /// for any key none of them takes. `Esc` is left to [`Self::escape`].
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
        // on `left` before the page could fold its own tree, or take the Display page's `h`.
        self.sync_frame(cx);
        let taken = self
            .focused_page()
            .is_some_and(|page| page.key_ahead(keystroke, cx))
            || self.focus_key(keystroke, cx);
        if taken {
            cx.notify();
        }
        taken
    }

    /// A key no other step took goes to the open page's own Entity; the static pages claim their
    /// keys ahead of the Shell's fallback.
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

    /// `Esc` closes what the open page has open (the Colour Theme grid among it), then leaves the
    /// page for the index.
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

    /// Opens the page that owns the record, with focus in it, and has the page show it.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        let Some(page) = SettingsPage::for_reveal(&reveal) else {
            return;
        };
        if !self.open_page(page, cx) {
            return;
        }
        self.selected_page().reveal(reveal, cx);
    }

    /// The status line's legend for the index or the open page, and the page's scope note. `None` while the View zone doesn't have focus.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        if !self.zone_focused {
            return None;
        }
        if self.picker.is_some() {
            return Some(PageStatus {
                hints: owned_hints(settings_picker_hints()),
                right: self.selected_section.scope_note(),
            });
        }
        let mut status = match self.focused_page() {
            Some(page) => page.page_status(cx)?,
            None => PageStatus {
                hints: owned_hints(settings_index_hints()),
                right: self.selected_section.scope_note(),
            },
        };
        if self.drill_down() && self.focus == SettingsFocus::Page {
            status
                .hints
                .extend(owned_hints(settings_medium_page_hints()));
        }
        Some(status)
    }

    /// The `?` cheat-sheet's Settings group: the index keys, then the open page's.
    pub fn help_group(&self, cx: &App) -> Option<HelpGroup> {
        let keys = owned_hints(settings_index_hints())
            .into_iter()
            .chain(self.selected_page().hints(cx))
            .map(|hint| (hint.action, hint.chord))
            .collect();
        Some(HelpGroup {
            heading: crate::msg::desktop_help_settings_label(),
            keys,
        })
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

    /// Restores the last-visited page at launch, without moving focus. A This Ledger page with no
    /// Ledger open restores General instead.
    pub fn set_selected_section(&mut self, cx: &mut Context<'_, Self>, section: SettingsPage) {
        self.selected_section = if self.can_show(section) {
            section
        } else {
            SettingsPage::General
        };
        cx.notify();
    }

    /// Opens a page with focus in it -- what `:settings <page>`, the old `:accounts` /
    /// `:categories` / `:payees` / `:tags` aliases and every hand-off to a moved noun land on (the
    /// keyboard model's "commands and hand-offs land in the page"). Whether the page opened: a This
    /// Ledger page with no Ledger open is refused.
    pub fn open_page(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) -> bool {
        if !self.select_page(section, cx) {
            return false;
        }
        self.focus_page(cx);
        cx.notify();
        true
    }

    /// Moves focus into the open page, as a click on one of its rows does.
    pub fn focus_page_from_click(&mut self, cx: &mut Context<'_, Self>) {
        self.focus_page(cx);
        cx.notify();
    }

    /// Swaps the page on show. A new page starts at its top, with no control focused. The page
    /// showing loses focus even when it is the one selected again, so `:settings display` on
    /// Display puts the keyboard back on its first control. A This Ledger page with no Ledger open
    /// is refused on the status line, and the page on show stays; whether the page was selected.
    fn select_page(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) -> bool {
        if !self.can_show(section) {
            cx.emit(ViewEvent::SetStatus(Some(
                crate::msg::desktop_status_refused_no_ledger(":open", ":new"),
            )));
            return false;
        }
        if self.selected_section != section {
            self.scroll_handle.set_offset(gpui::Point::default());
        }
        let changed = self.selected_section != section;
        self.selected_page().set_page_focused(false, cx);
        self.selected_section = section;
        if changed {
            cx.emit(ViewEvent::SettingsPageChanged(section));
        }
        true
    }

    /// Moves focus from the index rail into the open page (`l`/`enter`). A page with nothing to
    /// focus keeps focus on the index, as a quiet no-op.
    fn focus_page(&mut self, cx: &mut Context<'_, Self>) {
        // At Medium the page is what's on screen, so even a page with nothing to focus opens.
        if !self.selected_section.has_controls() && !self.drill_down() {
            return;
        }
        self.focus = SettingsFocus::Page;
        self.selected_page().set_page_focused(true, cx);
    }

    fn focus_index(&mut self, cx: &mut Context<'_, Self>) {
        self.focus = SettingsFocus::Index;
        self.selected_page().set_page_focused(false, cx);
        self.reveal_index_row();
    }

    /// Scrolls the Medium index page just enough to show the selected row, as the selection moves
    /// or the index comes back on show.
    fn reveal_index_row(&self) {
        if !self.drill_down() {
            return;
        }
        self.index_scroll
            .scroll_to_item(super::index_page_child(self.selected_section));
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
        if self.select_page(pages[next], cx) {
            self.reveal_index_row();
        }
    }

    /// An index entry's click: swaps the body to that page and leaves focus on the index.
    pub(super) fn click_index(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) {
        if self.select_page(section, cx) {
            self.focus = SettingsFocus::Index;
        }
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

    /// A picker row's click: straight to that page. A refused page leaves the picker open.
    pub(super) fn click_picker_page(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) {
        if !self.can_show(section) {
            self.select_page(section, cx);
            return;
        }
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
        if self.select_page(pages[next], cx) {
            self.focus_page(cx);
        }
    }

    fn open_picker(&mut self, cx: &mut Context<'_, Self>) {
        self.picker = Some(PagePicker::new(self.selected_section));
        cx.emit(ViewEvent::EnterMode(ViewMode::Filter));
    }

    fn close_picker(&mut self, cx: &mut Context<'_, Self>) {
        self.picker = None;
        cx.emit(ViewEvent::ExitMode);
    }

    /// Every key while the picker is open: `j`/`k` or `↓`/`↑` move, `enter` opens the chosen page.
    /// Anything else is swallowed. `Esc` comes back as [`Self::escape`].
    fn picker_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        match keystroke.key.as_str() {
            "k" | "up" => picker.step(Step::Prev),
            "j" | "down" => picker.step(Step::Next),
            "enter" => {
                let section = picker.chosen();
                self.click_picker_page(section, cx);
            }
            _ => {}
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
                if self.pages.display.read(cx).colour_theme_focus().is_none() =>
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
        self.pages.display.read(cx).preferences()
    }

    /// Sets the Display Preferences from outside the page (launch), without telling `Root`,
    /// which made the change itself.
    pub fn set_display(&mut self, display: DisplayPreferences, cx: &mut Context<'_, Self>) {
        self.pages
            .display
            .update(cx, |page, cx| page.set_preferences(display, cx));
    }

    // -----------------------------------------------------------------------------------------
    // General
    // -----------------------------------------------------------------------------------------

    /// The General page, which owns On start and Toasts and shows this device (ADR-0037).
    pub fn general(&self) -> &Entity<GeneralPage> {
        &self.pages.general
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, EntityId, TestAppContext};

    use super::*;
    use crate::{
        accounts::AccountsPage,
        categories::CategoriesPage,
        document_types::DocumentTypesPage,
        institutions::InstitutionsPage,
        inventory::InventoryPage,
        payees::PayeesPage,
        settings::{
            about::AboutPage, data_backup::DataBackupPage, display::DisplayPage,
            general::GeneralPage, ledger::LedgerPage, sync_server::SyncServerPage,
            tracing::TracingPage,
        },
        tags::TagsPage,
        units::UnitsPage,
    };

    /// `page::<T>()`'s Entity, and `T::PAGE`, for the page type that draws `page`. The match is
    /// exhaustive, so a new page fails to compile here until it is covered.
    fn typed(view: &SettingsView, page: SettingsPage) -> (EntityId, SettingsPage) {
        fn found<T: Page>(view: &SettingsView) -> (EntityId, SettingsPage) {
            (view.page::<T>().entity_id(), T::PAGE)
        }
        match page {
            SettingsPage::General => found::<GeneralPage>(view),
            SettingsPage::Ledger => found::<LedgerPage>(view),
            SettingsPage::Display => found::<DisplayPage>(view),
            SettingsPage::Units => found::<UnitsPage>(view),
            SettingsPage::Institutions => found::<InstitutionsPage>(view),
            SettingsPage::Accounts => found::<AccountsPage>(view),
            SettingsPage::Categories => found::<CategoriesPage>(view),
            SettingsPage::Tags => found::<TagsPage>(view),
            SettingsPage::Payees => found::<PayeesPage>(view),
            SettingsPage::Documents => found::<DocumentTypesPage>(view),
            SettingsPage::Inventory => found::<InventoryPage>(view),
            SettingsPage::SyncServer => found::<SyncServerPage>(view),
            SettingsPage::DataBackup => found::<DataBackupPage>(view),
            SettingsPage::Tracing => found::<TracingPage>(view),
            SettingsPage::About => found::<AboutPage>(view),
        }
    }

    #[gpui::test]
    fn page_returns_the_page_for_every_settings_page(cx: &mut TestAppContext) {
        crate::locale::init_for_tests();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 30).expect("a valid date");
        let storage = crate::ledger_store::LedgerSetup::stub(today);
        let setup = SettingsSetup {
            units: crate::units::UnitsSetup::stub(&storage),
            institutions: crate::institutions::InstitutionsSetup::stub(&storage),
            payees: crate::payees::PayeesSetup::stub(&storage),
            document_types: crate::document_types::DocumentTypesSetup::stub(&storage),
            accounts: crate::accounts::AccountsSetup::stub(&storage),
            categories: crate::categories::CategoriesSetup::stub(&storage),
            tags: crate::tags::TagsSetup::stub(&storage),
            inventory: crate::inventory::InventorySetup::stub(&storage),
        };
        let ledger =
            cx.new(|_| LedgerStore::with_storage(lib_ledger::Ledger::stub_on(today), storage));
        let clock = lib_core::Clock::fixed(today);
        let view = cx.new(SettingsView::new);
        view.update(cx, |view, cx| {
            for page in SettingsPage::ALL {
                assert_eq!(
                    view.can_show(page),
                    !page.needs_ledger(),
                    "with no Ledger open only the This device pages are built"
                );
            }
            view.open_ledger(&ledger, &clock, setup, cx);
        });

        view.read_with(cx, |view, _| {
            for page in SettingsPage::ALL {
                let (entity, named) = typed(view, page);
                assert_eq!(
                    named, page,
                    "the type drawing {page:?} names it as its page"
                );
                assert_eq!(
                    entity,
                    view.pages
                        .get(page)
                        .expect("every page is built with a Ledger open")
                        .view()
                        .entity_id(),
                    "page::<T>() for {page:?} is the page Settings shows"
                );
            }
        });
    }
}
