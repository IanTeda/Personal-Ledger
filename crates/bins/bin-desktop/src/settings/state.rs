//! The Settings destination's Entity (ADR-0032, ADR-0037). [`SettingsView`] holds `lib-settings`'
//! frame Presenter, which owns whether the index (Settings' resting view) or a page shows, the
//! index's cursor, the Medium drill-down and the page picker, and builds its pages ([`SettingsPages`]). It renders the Presenter's view
//! model (`impl Render` in `super`), sends the frame its keys and clicks, and reaches `Root` only
//! through [`ViewEvent`], which it re-emits from its pages, and [`PageChanged`], which the frame's
//! Router raises.
//!
//! Every page is a [`Page`](super::pages::Page) behind one `dyn` handle, so while a page has
//! focus this View forwards its keys, movement, `Esc`, Search and status to it without knowing
//! which page it is. Halfway state (#1036): every page, domain or config, still renders through
//! that trait.

use super::router::{FrameRouter, PageChanged};
use crate::{
    announcer::Announcer,
    navigation::{chord::chord, movement::Movement},
    settings::pages::{LedgerPages, Page, PageFrame, PageHandle, SettingsPages, SettingsSetup},
    settings::{SettingsFocus, SettingsPage},
    shell::view::{page_clicks::PresenterPage, statusline::PageStatus, window_size::WindowSize},
    view::event::{Reveal, ViewEvent},
    view::help::HelpGroup,
};
use gpui::{App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, Subscription};
use lib_list_cursor::Step;
use lib_settings::{
    ColourChoice, DeviceConfiguration, DisplayOutput, DisplaySettings, GeneralSettings, ScopeFacts,
    SettingsFrameEvent, SettingsFramePresenter, SettingsFrameSetup, SettingsFrameViewModel,
    ThisDevice, TracingLevel, TracingLog, keymap::settings_frame_keymap,
};
use lib_viper::keymap::KeyBindingConfig;
use lib_viper::presenter::Presenter;
use std::path::PathBuf;
use std::rc::Rc;

impl EventEmitter<ViewEvent> for SettingsView {}

impl EventEmitter<PageChanged> for SettingsView {}

/// The Settings View's own state. The index's cursor is the last page visited, so it survives
/// leaving and re-entering Settings, which lands on the index.
pub struct SettingsView {
    pages: SettingsPages,
    frame: SettingsFramePresenter,
    /// The page body's scroll, which also gives the Colour Theme grid its width.
    pub(super) scroll_handle: ScrollHandle,
    /// The Medium index page's scroll, its own so moving the selection doesn't reset it.
    pub(super) index_scroll: ScrollHandle,
    /// Whether the View zone has focus, told by `Root` as it draws the View.
    pub(super) zone_focused: bool,
    /// The Window Size the View was last drawn at, told by `Root` as it draws the View, so the
    /// pages act on the layout on screen. At Medium, Settings is a drill-down: no index is docked
    /// and a page's heading carries the page picker (DESKTOP-119, 120), which the frame is told as
    /// the Window Size changes.
    window_size: WindowSize,
    /// Keeps each page's events reaching this View: its `ViewEvent`s re-emitted for `Root`, its
    /// changes repainting this View and its row clicks handing it focus.
    _page_subscriptions: Vec<Subscription>,
}

/// The chord of the Shell's `back` under `config`, which the frame's legend shows.
pub(super) fn back_chord(config: &KeyBindingConfig) -> String {
    lib_shell::global_keymap(config)
        .chord(lib_viper::keymap::BACK)
        .unwrap_or_default()
        .to_string()
}

impl SettingsView {
    /// Builds the This device pages, which stand with no Ledger open; [`Self::open_ledger`] adds
    /// the This Ledger pages. The Tracing capture arrives later, through `Root::set_log_capture`,
    /// and the frame's `[keybindings]` through [`Self::set_keymaps`].
    pub fn new(cx: &mut Context<'_, Self>) -> Self {
        let pages = SettingsPages::new(cx);
        let _page_subscriptions = pages.all().flat_map(|page| page.subscribe(cx)).collect();
        let config = KeyBindingConfig::default();
        let frame = SettingsFramePresenter::new(SettingsFrameSetup {
            keymap: settings_frame_keymap(&config),
            back_chord: back_chord(&config),
            facts: ScopeFacts {
                log_capacity: lib_tracing::LOG_CAPACITY,
            },
            ledger_open: false,
        });
        Self {
            pages,
            frame,
            scroll_handle: ScrollHandle::new(),
            index_scroll: ScrollHandle::new(),
            zone_focused: false,
            window_size: WindowSize::Large,
            _page_subscriptions,
        }
    }

    /// The frame's keys under `config`'s overrides.
    pub fn set_keymaps(&mut self, config: &KeyBindingConfig, cx: &mut Context<'_, Self>) {
        self.frame.set_keymap(settings_frame_keymap(config));
        self.frame.set_back_chord(back_chord(config));
        self.pages
            .general
            .update(cx, |page, _| page.set_keymaps(config, back_chord(config)));
        self.pages
            .display
            .update(cx, |page, _| page.set_keymaps(config, back_chord(config)));
        self.pages
            .tracing
            .update(cx, |page, _| page.set_keymaps(config, back_chord(config)));
        self.pages.about.update(cx, |page, _| {
            page.set_keymap(settings_frame_keymap(config));
        });
        if let Some(pages) = &self.pages.ledger {
            pages
                .ledger
                .update(cx, |page, _| page.set_keymaps(config, back_chord(config)));
            pages.sync_server.update(cx, |page, _| {
                page.set_keymap(settings_frame_keymap(config));
            });
            pages.data_backup.update(cx, |page, _| {
                page.set_keymap(settings_frame_keymap(config));
            });
        }
    }

    // -----------------------------------------------------------------------------------------
    // Tracing
    // -----------------------------------------------------------------------------------------

    /// Hands the Tracing page the app's live log capture, opening on `level`.
    pub fn set_log_capture(
        &mut self,
        log: Rc<dyn TracingLog>,
        level: TracingLevel,
        cx: &mut Context<'_, Self>,
    ) {
        self.pages
            .tracing
            .update(cx, |page, cx| page.configure(log, level, cx));
    }

    /// The capture gained entries, so the Tracing page pulls them in.
    pub fn log_appended(&mut self, cx: &mut Context<'_, Self>) {
        self.pages.tracing.update(cx, |page, cx| page.appended(cx));
    }

    /// Builds the This Ledger pages on the Open Ledger's stores in `setup`, announcing through
    /// `announcer`, so they can be shown, in place of any built on a Ledger opened before.
    pub fn open_ledger(
        &mut self,
        announcer: &Entity<Announcer>,
        clock: &lib_core::Clock,
        setup: SettingsSetup,
        cx: &mut Context<'_, Self>,
    ) {
        let pages = LedgerPages::new(announcer, clock, setup, self.frame.keymap(), cx);
        self.pages.ledger = Some(pages);
        // Subscribed afresh, so a swapped-out Ledger's pages leave no subscription behind.
        self._page_subscriptions = self
            .pages
            .all()
            .flat_map(|page| page.subscribe(cx))
            .collect();
        self.handle(SettingsFrameEvent::LedgerOpened, cx);
        self.sync_frame(cx);
    }

    /// Drops the This Ledger pages with the Ledger they were built on. A This Ledger page on show
    /// gives way to the index, its cursor on General.
    pub fn close_ledger(&mut self, cx: &mut Context<'_, Self>) {
        self.pages.ledger = None;
        self._page_subscriptions = self
            .pages
            .all()
            .flat_map(|page| page.subscribe(cx))
            .collect();
        self.handle(SettingsFrameEvent::LedgerClosed, cx);
        self.sync_frame(cx);
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

    /// The open page's handle. The frame never shows a page it can't (a This Ledger page with no
    /// Ledger open), and the pages are built before the frame hears a Ledger opened, so the
    /// selected page is always built.
    #[expect(
        clippy::expect_used,
        reason = "the frame shows a This Ledger page only once told a Ledger opened, after its pages are built, and falls back to General as one closes"
    )]
    pub(super) fn selected_page(&self) -> &dyn PageHandle {
        self.pages
            .get(self.frame.selected())
            .expect("the selected Settings page is one that is built")
    }

    pub(super) fn pages(&self) -> &SettingsPages {
        &self.pages
    }

    /// The frame as drawn.
    pub(super) fn view_model(&self) -> SettingsFrameViewModel {
        self.frame.view_model()
    }

    /// The `T` page's Entity, for what `Root` drives on a page: a module Router's entry point and
    /// the Settings snapshots. Found through `T::PAGE`, so a new page needs
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

    /// The open page, while it has focus: the page, rather than the frame, then takes its keys,
    /// movement and status. `None` on the index.
    fn focused_page(&self) -> Option<&dyn PageHandle> {
        (self.frame.focus() == SettingsFocus::Page).then(|| self.selected_page())
    }

    /// The space the pages are drawn in, as the pages are told it.
    fn frame_space(&self) -> PageFrame {
        PageFrame {
            window_size: self.window_size,
            body_width: f32::from(self.scroll_handle.bounds().size.width),
        }
    }

    /// Tells every page the frame, so a page acts on the layout on screen, not the one it last saw.
    pub(super) fn sync_frame(&self, cx: &mut App) {
        let frame = self.frame_space();
        for page in self.pages.all() {
            page.set_frame(frame, cx);
        }
    }

    pub fn set_zone_focused(&mut self, focused: bool) {
        self.zone_focused = focused;
    }

    /// Tells the View the Window Size it is drawn at, and the frame whether that makes Settings a
    /// drill-down. Leaving Medium closes the picker, which only Medium draws.
    pub fn set_window_size(&mut self, window_size: WindowSize, cx: &mut Context<'_, Self>) {
        let changed = window_size != self.window_size;
        self.window_size = window_size;
        self.sync_frame(cx);
        if changed {
            self.handle(
                SettingsFrameEvent::DrillDown(window_size == WindowSize::Medium),
                cx,
            );
        }
    }

    // -----------------------------------------------------------------------------------------
    // The frame's Presenter
    // -----------------------------------------------------------------------------------------

    /// A key, click or command for the frame, as one input, committed once it is handled. One sent
    /// while another module's input is being handled (a hand-off from the palette's no Ledger open
    /// content) runs inside it.
    fn send_frame(&mut self, event: SettingsFrameEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the frame, then brings the pages and scrolls into line with where it left
    /// the selection and focus: a newly shown page starts at its top, a page losing the keyboard is
    /// told so, and one (re)opened is told it has it, so `:settings display` on Display puts the
    /// keyboard back on its first control. A restored page keeps its scroll. The index scrolls to
    /// its cursor's row.
    fn handle(&mut self, event: SettingsFrameEvent, cx: &mut Context<'_, Self>) {
        let page_before = self.frame.selected();
        let focus_before = self.frame.focus();
        let opens = matches!(
            event,
            SettingsFrameEvent::OpenPage(_)
                | SettingsFrameEvent::ClickIndexPage(_)
                | SettingsFrameEvent::ClickPicker(_)
                | SettingsFrameEvent::StepPage(_)
        ) || (self.frame.picker_open() && event == SettingsFrameEvent::Select);
        let mut router = FrameRouter { cx, refused: false };
        self.frame.handle(event, &mut router);
        let reopened = opens && !router.refused;
        let page = self.frame.selected();
        let focus = self.frame.focus();
        let moved = page != page_before;
        if moved {
            if !matches!(event, SettingsFrameEvent::Restore(_)) {
                self.scroll_handle.set_offset(gpui::Point::default());
            }
            if let Some(before) = self.pages.get(page_before) {
                before.set_page_focused(false, cx);
            }
        }
        if moved || reopened || focus != focus_before {
            self.selected_page().set_page_focused(false, cx);
            if focus == SettingsFocus::Page {
                self.selected_page().set_page_focused(true, cx);
            }
        }
        let entered = event == SettingsFrameEvent::Entered;
        if focus == SettingsFocus::Index && (moved || entered || focus != focus_before) {
            self.reveal_index_row();
        }
        cx.notify();
    }

    /// Scrolls the index page just enough to show the cursor's row, as the cursor moves or the
    /// index comes back on show.
    fn reveal_index_row(&self) {
        self.index_scroll
            .scroll_to_item(super::index_page_child(self.frame.selected()));
    }

    /// The frame's key by `chord`, when the frame takes it as things stand. The page has had the
    /// key first, so the Colour Theme grid has kept its own `h` or been left by it.
    fn frame_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.frame.keymap().event(chord) else {
            return false;
        };
        if !self.frame.takes(&event) {
            return false;
        }
        self.send_frame(event, cx);
        true
    }

    // -----------------------------------------------------------------------------------------
    // The interface `Root` forwards to (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// Settings' keys at the key route's module-keymap step, by `chord` (`keystroke` its last key),
    /// in order: the focused page's module keymap, then the page's own keys (Display's controls and
    /// Colour Theme grid, Tracing's, General's), then the frame's keymap. The page goes first, so a
    /// page's `h`/`l` win over the frame's `h`. Not while the page picker is open, whose keys
    /// [`Self::key_ahead`] reads in its mode.
    pub fn module_key(
        &mut self,
        chord: &str,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if self.frame.picker_open() {
            return false;
        }
        self.sync_frame(cx);
        if let Some(page) = self.focused_page() {
            // A page's own keys read one key at a time, with no `Ctrl`, `Alt` or `Cmd` (ADR-0037).
            let modifiers = &keystroke.modifiers;
            let plain =
                !chord.contains(' ') && !(modifiers.control || modifiers.alt || modifiers.platform);
            if page.module_key(chord, cx) || (plain && page.key_ahead(keystroke, cx)) {
                cx.notify();
                return true;
            }
        }
        self.frame_key(chord, cx)
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

    /// Every key but `Esc` while the page picker is open, which reads them in its own mode: its
    /// moves step the highlight and `select` opens the chosen page. Every other key is swallowed,
    /// as before the frame was a module, so the picker's query is not typed into yet. `Esc` comes
    /// back as [`Self::escape`]. Every other key is the module step's ([`Self::module_key`]).
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if keystroke.key == "escape" || !self.frame.picker_open() {
            return false;
        }
        let event = chord(keystroke).and_then(|chord| self.frame.keymap().event(&chord));
        if let Some(
            event
            @ (SettingsFrameEvent::Move(Step::Next | Step::Prev) | SettingsFrameEvent::Select),
        ) = event
        {
            self.send_frame(event, cx);
        }
        cx.notify();
        true
    }

    /// A movement the Shell fell back on: a page with its own selection moves it, the index steps
    /// through the pages as the frame's own keys do, and anything else scrolls.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        if let Some(page) = self.focused_page()
            && page.apply_movement(movement, cx)
        {
            return;
        }
        let step = match movement {
            Movement::Next => Some(Step::Next),
            Movement::Prev => Some(Step::Prev),
            Movement::First => Some(Step::First),
            Movement::Last => Some(Step::Last),
            _ => None,
        };
        match step {
            Some(step) if self.frame.focus() == SettingsFocus::Index => {
                self.send_frame(SettingsFrameEvent::Move(step), cx);
            }
            _ => crate::view::scroll_by_movement(&self.scroll_handle, movement),
        }
        cx.notify();
    }

    /// `Esc` closes the page picker, then what the open page has open (the Colour Theme grid among
    /// it), then leaves the page for the index.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if !self.frame.picker_open()
            && let Some(page) = self.focused_page()
            && page.escape(cx)
        {
            return true;
        }
        if !self.frame.takes_back() {
            return false;
        }
        self.send_frame(SettingsFrameEvent::Back, cx);
        true
    }

    /// Settings has no Search of its own: `/` goes to the open page's Entity, and is inert
    /// where it has none rather than opening a mode with no box.
    pub fn start_search(&mut self, cx: &mut Context<'_, Self>) -> bool {
        self.focused_page()
            .is_some_and(|page| page.start_search(cx))
    }

    /// Settings has just become the destination, by `g s`, `:settings` or the rail item (even with
    /// a page showing), or at a start on Settings: it lands on the index, its resting view, with
    /// the cursor on the last-visited page, which starts at its top when opened.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.scroll_handle.set_offset(gpui::Point::default());
        self.handle(SettingsFrameEvent::Entered, cx);
    }

    /// Opens the page that owns the record, with focus in it, and has the page show it.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        let Some(page) = super::for_reveal(&reveal) else {
            return;
        };
        if self.open_page(page, cx) {
            self.selected_page().reveal(reveal, cx);
        }
    }

    /// The status line's legend for the index, the open page or the picker, and the page's scope
    /// note. `None` while the View zone doesn't have focus. A page's own legend gains the frame's
    /// Medium keys.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        if !self.zone_focused {
            return None;
        }
        let right = super::scope_note(self.frame.selected());
        if self.frame.picker_open() {
            return Some(PageStatus {
                hints: self.frame.hints(),
                right,
            });
        }
        match self.focused_page() {
            Some(page) => {
                let mut status = page.page_status(cx)?;
                status.hints.extend(self.frame.hints());
                Some(status)
            }
            None => Some(PageStatus {
                hints: self.frame.hints(),
                right,
            }),
        }
    }

    /// The `?` cheat-sheet's Settings group: the index keys, then the open page's.
    pub fn help_group(&self, cx: &App) -> Option<HelpGroup> {
        let keys = self
            .frame
            .help()
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
        self.frame.selected()
    }

    pub fn focus(&self) -> SettingsFocus {
        self.frame.focus()
    }

    /// Back or Forward onto `page`, which shows the page itself with the keyboard in it, or the
    /// launch restore, which [`Self::on_enter`] then returns to the index. A This Ledger page with
    /// no Ledger open restores General instead.
    pub fn restore(&mut self, page: SettingsPage, cx: &mut Context<'_, Self>) {
        self.handle(SettingsFrameEvent::Restore(page), cx);
    }

    /// Opens a page with focus in it -- what `:settings <page>`, the old `:accounts` /
    /// `:categories` / `:payees` / `:tags` aliases and every hand-off to a moved noun land on (the
    /// keyboard model's "commands and hand-offs land in the page"). Whether the page opened: a This
    /// Ledger page with no Ledger open is refused. Not an input of its own: it is part of the
    /// command or hand-off that asked, so a palette Command run on the page commits once, and the
    /// Shell shows a refusal's Notice itself.
    pub fn open_page(&mut self, section: SettingsPage, cx: &mut Context<'_, Self>) -> bool {
        self.handle(SettingsFrameEvent::OpenPage(section), cx);
        self.frame.selected() == section
    }

    /// Moves focus into the open page, as a click on one of its rows does.
    pub fn focus_page_from_click(&mut self, cx: &mut Context<'_, Self>) {
        self.send_frame(SettingsFrameEvent::PageClicked, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Display
    // -----------------------------------------------------------------------------------------

    /// The Display output the Views draw with, which the Display page's Presenter holds.
    pub fn display_output(&self, cx: &App) -> DisplayOutput {
        self.pages.display.read(cx).output()
    }

    /// Start Sidebar minimised as the state file remembered it, at launch.
    pub fn set_start_sidebar_minimised(&mut self, minimised: bool, cx: &mut Context<'_, Self>) {
        self.pages.display.update(cx, |page, cx| {
            page.set_start_sidebar_minimised(minimised, cx)
        });
    }

    /// `theme` or `appearance` from the palette, saved and applied as Display's own choice is.
    pub fn choose_colour(&mut self, colour: ColourChoice, cx: &mut Context<'_, Self>) {
        self.pages
            .display
            .update(cx, |page, cx| page.choose_colour(colour, cx));
    }

    // -----------------------------------------------------------------------------------------
    // General
    // -----------------------------------------------------------------------------------------

    /// Writes General's and Display's settings to `device` from now on, showing them as start-up
    /// read them.
    pub fn configure_device(
        &mut self,
        device: Rc<dyn DeviceConfiguration>,
        general: GeneralSettings,
        display: DisplaySettings,
        cx: &mut Context<'_, Self>,
    ) {
        let for_general = device.clone();
        self.pages
            .general
            .update(cx, |page, cx| page.configure(for_general, general, cx));
        self.pages
            .display
            .update(cx, |page, cx| page.configure(device, display, cx));
    }

    pub fn set_this_device(&mut self, device: ThisDevice, cx: &mut Context<'_, Self>) {
        self.pages
            .general
            .update(cx, |page, cx| page.set_this_device(device, cx));
    }

    /// The Ledger file the state file will remember at quit, once `:open` has opened one.
    pub fn set_last_ledger(&mut self, last_ledger: Option<PathBuf>, cx: &mut Context<'_, Self>) {
        self.pages
            .general
            .update(cx, |page, cx| page.set_last_ledger(last_ledger, cx));
    }

    /// `:toasts on|off`, saved as General's own choice is.
    pub fn set_toasts(&mut self, on: bool, cx: &mut Context<'_, Self>) {
        self.pages
            .general
            .update(cx, |page, cx| page.set_toasts(on, cx));
    }
}

/// The frame's clicks: the docked index rail's rows, the Medium index page's rows, the
/// breadcrumb and the picker, each as one input.
impl PresenterPage<SettingsFrameEvent> for SettingsView {
    fn send(&mut self, event: SettingsFrameEvent, cx: &mut Context<'_, Self>) {
        self.send_frame(event, cx);
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
        let storage = crate::shell::router::LedgerStores::sample(today);
        let setup = SettingsSetup {
            units: crate::units::UnitsSetup::sample(&storage),
            institutions: crate::institutions::InstitutionsSetup::sample(&storage),
            payees: crate::payees::PayeesSetup::sample(&storage),
            document_types: crate::document_types::DocumentTypesSetup::sample(&storage),
            accounts: crate::accounts::AccountsSetup::sample(&storage),
            categories: crate::categories::CategoriesSetup::sample(&storage),
            tags: crate::tags::TagsSetup::sample(&storage),
            inventory: crate::inventory::InventorySetup::sample(&storage),
            ledger: crate::settings::ledger::LedgerKeys::new(
                &KeyBindingConfig::default(),
                &storage.ports,
            ),
        };
        let announcer = cx.new(|_| Announcer::default());
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
            view.open_ledger(&announcer, &clock, setup, cx);
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
