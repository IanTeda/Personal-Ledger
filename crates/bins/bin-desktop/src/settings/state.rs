//! The Settings destination's Entity (ADR-0032, ADR-0037). [`SettingsView`] owns the View state
//! that belongs to Settings: the page on show, where keyboard focus sits, the Display page's
//! focused control and Preferences, the Tracing page's log, and the Documents and Inventory
//! pages' selections. It renders itself (`impl Render` in `super`), handles the index, its focus
//! keys and the static pages' keys, movement and `Esc`, and reaches `Shell` only through
//! [`ViewEvent`]. It builds the Units, Institutions and Clear logs dialogs, which `Shell`'s Dialog
//! host draws and confirms.
//!
//! A page with Ledger data that has moved into its own Entity (Accounts, [`AccountsView`];
//! Payees, [`PayeesView`]) is drawn by that Entity, and this View forwards the page's keys,
//! movement and status to it while the page has focus. The rest (Categories, Tags, Documents,
//! Inventory) are still
//! `Shell`'s: it hands their drawn page in as a [`DataPage`] each frame, and handles their keys
//! before forwarding to this Entity, until each moves in its own ticket.

use std::collections::HashSet;

use gpui::{
    App, Context, Entity, EventEmitter, Keystroke, ListOffset, ListState, ScrollHandle,
    Subscription, px,
};
use lib_colour_theme::ColourTheme;

use super::{
    DataPage, colour_theme,
    hints::{
        settings_colour_grid_hints, settings_display_hints, settings_index_hints,
        settings_plain_page_hints, settings_tracing_hints,
    },
    tracing::LOG_LINE_STEP,
};
use crate::{
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    institutions::{InstitutionsStore, form::AddInstitutionForm},
    navigation::key_router::Movement,
    settings::{
        DISPLAY_FIELD_COUNT, DISPLAY_FIELD_SIDEBAR, SettingsDialog, SettingsFocus, SettingsPage,
        display::{DATE_STYLE_CHOICES, DisplayPreferences, RowDensity, StatusGlyphs},
        inventory::InventoryRow,
        step_choice,
        tracing_log::{LogChange, LogView, TracingLevel},
    },
    theme::colours::ColourChange,
    units::{
        UnitsStore,
        form::{DeleteUnitForm, UnitForm},
    },
    view::{
        accounts::state::AccountsView,
        event::{Reveal, ViewEvent},
        payees::state::{PayeesView, RowClicked},
    },
};

impl EventEmitter<ViewEvent> for SettingsView {}

/// What the Settings pages read: the Units, Price Sources and Institutions tables, and the Entity
/// of each page with Ledger data that draws itself.
pub struct SettingsSources {
    pub units: Entity<UnitsStore>,
    pub institutions: Entity<InstitutionsStore>,
    pub accounts: Entity<AccountsView>,
    pub payees: Entity<PayeesView>,
}

/// The open page's own Entity, while it has focus.
#[derive(Clone)]
enum FocusedPage {
    Accounts(Entity<AccountsView>),
    Payees(Entity<PayeesView>),
}

impl FocusedPage {
    fn key_ahead(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.key_ahead(keystroke, cx)),
        }
    }

    fn handle_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.handle_key(keystroke, cx)),
        }
    }

    fn apply_movement(&self, movement: Movement, cx: &mut App) {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.apply_movement(movement, cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.apply_movement(movement, cx)),
        }
    }

    fn escape(&self, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.escape(cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.escape(cx)),
        }
    }

    fn start_search(&self, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.start_search(cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.start_search(cx)),
        }
    }

    fn search_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        match self {
            Self::Accounts(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
            Self::Payees(page) => page.update(cx, |page, cx| page.search_key(keystroke, cx)),
        }
    }

    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        match self {
            Self::Accounts(page) => page.read(cx).page_status(cx),
            Self::Payees(page) => page.read(cx).page_status(cx),
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
    display_field: Option<usize>,
    display: DisplayPreferences,
    documents_selected: Option<u32>,
    inventory_selected: Option<InventoryRow>,
    inventory_expanded: HashSet<u32>,
    /// The Colour Theme card the keyboard is on, while the Display page's grid has focus.
    colour_theme_focus: Option<usize>,
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
        Self {
            sources,
            selected_section: SettingsPage::default(),
            focus: SettingsFocus::default(),
            display_field: None,
            display: DisplayPreferences::default(),
            documents_selected: None,
            inventory_selected: None,
            inventory_expanded: HashSet::new(),
            colour_theme_focus: None,
            log,
            log_list,
            scroll_handle: ScrollHandle::new(),
            zone_focused: false,
            data_page: None,
            _payees_row_clicked,
        }
    }

    pub(super) fn sources(&self) -> &SettingsSources {
        &self.sources
    }

    /// The Accounts page's Entity, for `Shell`'s Dialog host and the `accounts` command.
    pub fn accounts(&self) -> &Entity<AccountsView> {
        &self.sources.accounts
    }

    /// The Payees page's Entity, for `Shell`'s Dialog host.
    pub fn payees(&self) -> &Entity<PayeesView> {
        &self.sources.payees
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

    /// Settings' keys ahead of the router, in order: the Display page's controls, the Tracing
    /// page's keys, the index-and-page focus keys, then the Colour Theme grid. `false` for any
    /// key none of them takes. `Esc` is left to [`Self::escape`].
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.key == "escape" {
            return false;
        }
        let taken = self.display_form_key(keystroke, cx)
            || self.tracing_key(keystroke, cx)
            || self.focus_key(keystroke, cx)
            || self.colour_theme_grid_key(keystroke, cx)
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
    /// click. A page with its own Entity moves its own selection; any other page scrolls.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        if let Some(page) = self.focused_page() {
            page.apply_movement(movement, cx);
            return;
        }
        // A movement the grid did not take (`Ctrl-d`/`Ctrl-u`) leaves it, as any other key does.
        self.colour_theme_focus = None;
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
        if self.colour_theme_focus.is_some() {
            cx.emit(ViewEvent::SetStatus(None));
            self.leave_colour_theme_grid();
        } else if self.focus == SettingsFocus::Page {
            self.focus_index();
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
        self.focus_index();
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
            hints: self.hints(),
            right: self.selected_section.scope_note(),
        })
    }

    /// The legend for Settings' current focus: the index rail's keys, or the open page's. The
    /// pages `Shell` still draws have their own.
    fn hints(&self) -> Vec<(&'static str, String)> {
        if self.focus == SettingsFocus::Index {
            return settings_index_hints();
        }
        match self.selected_section {
            SettingsPage::Display if self.colour_theme_focus.is_some() => {
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

    pub fn display_field(&self) -> Option<usize> {
        self.display_field
    }

    pub fn colour_theme_focus(&self) -> Option<usize> {
        self.colour_theme_focus
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
        self.select_page(section);
        self.focus_page();
        cx.notify();
    }

    /// Moves focus into the open page, as a click on one of its rows does.
    pub fn focus_page_from_click(&mut self, cx: &mut Context<'_, Self>) {
        self.focus_page();
        cx.notify();
    }

    /// Swaps the page on show. A new page starts at its top, with no control focused.
    fn select_page(&mut self, section: SettingsPage) {
        if self.selected_section != section {
            self.scroll_handle.set_offset(gpui::Point::default());
        }
        self.selected_section = section;
        self.colour_theme_focus = None;
        self.display_field = None;
    }

    /// Moves focus from the index rail into the open page (`l`/`enter`). A page with nothing to
    /// focus keeps focus on the index, as a quiet no-op.
    fn focus_page(&mut self) {
        if !self.selected_section.has_controls() {
            return;
        }
        self.focus = SettingsFocus::Page;
        self.display_field = (self.selected_section == SettingsPage::Display).then_some(0);
    }

    fn focus_index(&mut self) {
        self.focus = SettingsFocus::Index;
        self.colour_theme_focus = None;
        self.display_field = None;
    }

    fn apply_section_movement(&mut self, movement: Movement, _cx: &mut Context<'_, Self>) {
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
        self.select_page(pages[next]);
    }

    /// An index entry's click: swaps the body to that page and leaves focus on the index.
    pub(super) fn click_index(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) {
        self.select_page(section);
        self.focus = SettingsFocus::Index;
        cx.notify();
    }

    /// Settings' own focus keys: `l`/`right`/`enter` on the index step into the page, `h`/`left`
    /// on the page step back out (not from the Colour Theme grid, which has its own `h`).
    fn focus_key(&mut self, keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        if keystroke.modifiers.shift {
            return false;
        }
        match (self.focus, keystroke.key.as_str()) {
            (SettingsFocus::Index, "l" | "right" | "enter") => {
                self.focus_page();
                true
            }
            (SettingsFocus::Page, "h" | "left") if self.colour_theme_focus.is_none() => {
                self.focus_index();
                true
            }
            _ => false,
        }
    }

    // -----------------------------------------------------------------------------------------
    // Display
    // -----------------------------------------------------------------------------------------

    pub fn display(&self) -> DisplayPreferences {
        self.display
    }

    /// Sets the Display Preferences from outside the page (launch, `:toasts`), without telling
    /// `Shell`, which made the change itself.
    pub fn set_display(&mut self, display: DisplayPreferences, cx: &mut Context<'_, Self>) {
        self.display = display;
        cx.notify();
    }

    /// Applies a change made on the page and hands the result to `Shell`.
    fn change_display(
        &mut self,
        change: impl FnOnce(&mut DisplayPreferences),
        cx: &mut Context<'_, Self>,
    ) {
        change(&mut self.display);
        cx.emit(ViewEvent::DisplayChanged(self.display));
        cx.notify();
    }

    pub(super) fn click_date_style(
        &mut self,
        style: Option<lib_core::DateStyle>,
        cx: &mut Context<'_, Self>,
    ) {
        self.change_display(|display| display.date_style = style, cx);
    }

    pub(super) fn click_row_density(&mut self, density: RowDensity, cx: &mut Context<'_, Self>) {
        self.change_display(|display| display.row_density = density, cx);
    }

    pub(super) fn click_status_glyphs(&mut self, glyphs: StatusGlyphs, cx: &mut Context<'_, Self>) {
        self.change_display(|display| display.status_glyphs = glyphs, cx);
    }

    /// "Start Sidebar minimised" takes effect at the next launch, so the current rail is left
    /// alone.
    pub(super) fn click_start_sidebar_minimised(&mut self, cx: &mut Context<'_, Self>) {
        self.change_display(
            |display| display.start_sidebar_minimised = !display.start_sidebar_minimised,
            cx,
        );
    }

    pub(super) fn click_toasts(&mut self, on: bool, cx: &mut Context<'_, Self>) {
        self.change_display(|display| display.toasts_on = on, cx);
    }

    /// A card click selects its Colour Theme and clears any keyboard focus left on the grid.
    pub(super) fn click_colour_theme(&mut self, id: &'static str, cx: &mut Context<'_, Self>) {
        self.colour_theme_focus = None;
        apply_colour(ColourChange::Theme(id), cx);
        cx.notify();
    }

    /// The Display page's form keys: `j`/`k` walk the controls and on past the last into the
    /// Colour Theme grid, `h`/`l` change a segmented or radio control (or clear/tick the
    /// checkbox) in place, `enter`/`space` toggles the checkbox.
    fn display_form_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let Some(field) = self.display_field else {
            return false;
        };
        if self.focus != SettingsFocus::Page
            || self.selected_section != SettingsPage::Display
            || keystroke.modifiers.shift
        {
            return false;
        }
        let delta = match keystroke.key.as_str() {
            "h" | "left" => -1,
            "l" | "right" => 1,
            _ => 0,
        };
        match keystroke.key.as_str() {
            "j" | "down" => {
                if field + 1 >= DISPLAY_FIELD_COUNT {
                    self.display_field = None;
                    self.colour_theme_focus = Some(colour_theme::chosen_index(cx));
                } else {
                    self.display_field = Some(field + 1);
                }
                true
            }
            "k" | "up" => {
                self.display_field = Some(field.saturating_sub(1));
                true
            }
            "h" | "left" | "l" | "right" => {
                self.change_display(
                    |display| match field {
                        0 => {
                            display.date_style =
                                step_choice(&DATE_STYLE_CHOICES, display.date_style, delta);
                        }
                        1 => {
                            display.row_density =
                                step_choice(&RowDensity::ALL, display.row_density, delta);
                        }
                        2 => {
                            display.status_glyphs =
                                step_choice(&StatusGlyphs::ALL, display.status_glyphs, delta);
                        }
                        DISPLAY_FIELD_SIDEBAR => display.start_sidebar_minimised = delta > 0,
                        _ => display.toasts_on = delta < 0,
                    },
                    cx,
                );
                true
            }
            "enter" | "space" => {
                if field == DISPLAY_FIELD_SIDEBAR {
                    self.click_start_sidebar_minimised(cx);
                }
                true
            }
            _ => false,
        }
    }

    /// Settings' Colour Theme grid (`docs/colour-themes-design.md` "Settings"): focus arrives
    /// from the Display page's last control at the chosen card; arrows or `h`/`j`/`k`/`l` move
    /// focus, `Enter` selects, and `Tab` leaves it, going on to the next zone. Moving focus never
    /// previews. Any key the grid does not take leaves it.
    fn colour_theme_grid_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let Some(index) = self.colour_theme_focus else {
            return false;
        };
        if self.focus != SettingsFocus::Page || self.selected_section != SettingsPage::Display {
            self.colour_theme_focus = None;
            return false;
        }
        let key = keystroke.key.as_str();
        match key {
            "tab" => {
                self.colour_theme_focus = None;
                false
            }
            "enter" => {
                if let Some(theme) = ColourTheme::built_in().get(index) {
                    apply_colour(ColourChange::Theme(theme.id), cx);
                }
                true
            }
            _ => {
                let columns =
                    colour_theme::grid_columns(f32::from(self.scroll_handle.bounds().size.width));
                let len = ColourTheme::built_in().len();
                // `k` on the top row climbs back to the last control above the grid.
                if matches!(key, "k" | "up") && index < columns {
                    self.leave_colour_theme_grid();
                    return true;
                }
                // `h` at the first column steps back out to the index.
                if matches!(key, "h" | "left") && index % columns.max(1) == 0 {
                    self.focus_index();
                    return true;
                }
                self.colour_theme_focus = colour_theme::grid_move(index, len, columns, key);
                self.colour_theme_focus.is_some()
            }
        }
    }

    /// Steps from the Colour Theme grid back up to the Display page's last control.
    fn leave_colour_theme_grid(&mut self) {
        self.colour_theme_focus = None;
        self.display_field = Some(DISPLAY_FIELD_COUNT - 1);
    }

    // -----------------------------------------------------------------------------------------
    // Units, Price Sources and Institutions
    // -----------------------------------------------------------------------------------------

    /// **+ Add unit** opens the Add unit dialog.
    pub(super) fn click_add_unit(&mut self, cx: &mut Context<'_, Self>) {
        open_dialog(SettingsDialog::AddUnit(UnitForm::default()), cx);
    }

    /// A Units row's **edit** opens the Edit unit dialog filled from the row.
    pub(super) fn click_edit_unit(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.sources.units.read(cx).unit(cx, index) else {
            return;
        };
        let form = UnitForm::from_row(row);
        open_dialog(SettingsDialog::EditUnit(index, form), cx);
    }

    /// A Units row's **delete** opens the destructive Delete unit confirm dialog.
    pub(super) fn click_delete_unit(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.sources.units.read(cx).unit(cx, index) else {
            return;
        };
        let form = DeleteUnitForm::new(row.code.as_str());
        open_dialog(SettingsDialog::DeleteUnit(index, form), cx);
    }

    /// **+ Add institution** opens the Add institution dialog, its Default unit seeded from the
    /// Units table's first row.
    pub(super) fn click_add_institution(&mut self, cx: &mut Context<'_, Self>) {
        let form = AddInstitutionForm::new(self.sources.units.read(cx).units(cx));
        open_dialog(SettingsDialog::AddInstitution(form), cx);
    }

    /// The buttons with no dialog or behaviour yet (Price Sources' test, edit, delete and add,
    /// Institutions' edit and delete, Sync now, Backup now, Export ledger) say so on the status
    /// line.
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

/// Applies a Colour Theme once the current update is done: the change refreshes every window,
/// which must not happen while `Shell` is mid-update.
fn apply_colour(change: ColourChange, cx: &mut Context<'_, SettingsView>) {
    cx.defer(move |cx| change.apply(cx));
}
