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
    accounts::AccountsPage,
    categories::CategoriesPage,
    institutions::InstitutionsPage,
    inventory::InventoryPage,
    ledger_store::LedgerStore,
    navigation::movement::Movement,
    payees::PayeesPage,
    settings::display::{DisplayPage, DisplayPreferences},
    settings::documents::DocumentsPage,
    settings::general::GeneralPage,
    settings::pages::{PageFrame, PageHandle, SettingsPages, SettingsSetup},
    settings::picker::PagePicker,
    settings::tracing::TracingPage,
    settings::{SettingsFocus, SettingsPage},
    shell::view::{
        statusline::{PageStatus, owned_hints},
        window_size::WindowSize,
    },
    tags::TagsPage,
    units::UnitsPage,
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
    /// Builds the pages from the Ledger. The Tracing capture arrives later, through
    /// `Root::set_log_capture`.
    pub fn new(
        ledger: &Entity<LedgerStore>,
        clock: &lib_core::Clock,
        setup: SettingsSetup,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let pages = SettingsPages::new(ledger, clock, setup, cx);
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

    pub(super) fn pages(&self) -> &SettingsPages {
        &self.pages
    }

    /// The Accounts page's Entity, for the `accounts` command.
    pub fn accounts(&self) -> &Entity<AccountsPage> {
        &self.pages.accounts
    }

    /// The Categories page's Entity, for the Settings snapshots.
    pub fn categories(&self) -> &Entity<CategoriesPage> {
        &self.pages.categories
    }

    /// The Payees page's Entity, for the Settings snapshots.
    pub fn payees(&self) -> &Entity<PayeesPage> {
        &self.pages.payees
    }

    /// The Tags page's Entity, for the `tags merge` command and the Settings snapshots.
    pub fn tags(&self) -> &Entity<TagsPage> {
        &self.pages.tags
    }

    /// The Document Types page's Entity, for the Settings snapshots.
    pub fn documents(&self) -> &Entity<DocumentsPage> {
        &self.pages.documents
    }

    /// The Inventory page's Entity, for the Settings snapshots.
    pub fn inventory(&self) -> &Entity<InventoryPage> {
        &self.pages.inventory
    }

    /// The General page's Entity, for the Settings snapshots.
    pub fn general(&self) -> &Entity<GeneralPage> {
        &self.pages.general
    }

    /// The Display page's Entity, for the Settings snapshots.
    pub fn display_page(&self) -> &Entity<DisplayPage> {
        &self.pages.display
    }

    /// The Units page's Entity, for the Settings snapshots.
    pub fn units(&self) -> &Entity<UnitsPage> {
        &self.pages.units
    }

    /// The Tracing page's Entity, for `Root`'s capture feed and the Settings snapshots.
    pub fn tracing(&self) -> &Entity<TracingPage> {
        &self.pages.tracing
    }

    /// The Institutions page's Entity, for the Settings snapshots.
    pub fn institutions(&self) -> &Entity<InstitutionsPage> {
        &self.pages.institutions
    }

    /// The open page, while it has focus: the page, rather than this View, then takes its keys,
    /// movement and status. `None` on the index.
    fn focused_page(&self) -> Option<&dyn PageHandle> {
        (self.focus == SettingsFocus::Page).then(|| self.pages.get(self.selected_section))
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
        let page = match reveal {
            Reveal::Account(_) => SettingsPage::Accounts,
            Reveal::Payee(_) => SettingsPage::Payees,
            // The rest belong to other Views.
            Reveal::AccountTransactions(_)
            | Reveal::Transaction(_)
            | Reveal::BillEntry(_)
            | Reveal::BillPlan(_)
            | Reveal::BillsPeriod(_) => return,
        };
        self.open_page(page, cx);
        self.pages.get(page).reveal(reveal, cx);
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
            .chain(self.pages.get(self.selected_section).hints(cx))
            .map(|(keys, action)| (action, keys))
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
        let changed = self.selected_section != section;
        self.selected_section = section;
        if changed {
            cx.emit(ViewEvent::SettingsPageChanged(section));
        }
        self.pages
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
            self.pages
                .display
                .update(cx, |page, cx| page.focus_first(cx));
        }
    }

    fn focus_index(&mut self, cx: &mut Context<'_, Self>) {
        self.focus = SettingsFocus::Index;
        self.pages
            .display
            .update(cx, |page, cx| page.clear_focus(cx));
        self.reveal_index_row();
    }

    /// Scrolls the Medium index page just enough to show the selected row, as the selection moves
    /// or the index comes back on show.
    fn reveal_index_row(&self) {
        if !self.drill_down() {
            return;
        }
        let row = SettingsPage::ALL
            .iter()
            .position(|page| *page == self.selected_section)
            .unwrap_or(0);
        self.index_scroll
            .scroll_to_item(super::INDEX_PAGE_FIRST_ROW + row);
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
        self.reveal_index_row();
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

    /// Sets the Display Preferences from outside the page (launch, `:toasts`), without telling
    /// `Root`, which made the change itself.
    pub fn set_display(&mut self, display: DisplayPreferences, cx: &mut Context<'_, Self>) {
        self.pages
            .display
            .update(cx, |page, cx| page.set_preferences(display, cx));
    }
}
