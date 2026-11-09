//! The Settings frame's Presenter: whether the Settings index or a page shows, the index's cursor,
//! the Medium drill-down and its page picker. The pages themselves are drawn by their own
//! Presenters; the frame only chooses among them.
//!
//! The index is Settings' resting view at every Window Size (#1102): entering Settings (`g s`,
//! `:settings`, the rail item, a start on Settings) shows it, with its cursor on the page last
//! left; `enter`, a click, a command, the picker, a Reveal or Back and Forward show a page, and
//! `esc`, `h` and `left` on a page return to the index. History records pages only, so moving the
//! cursor announces nothing; opening a page does.
//!
//! At Large and X-Large a click on the docked index while a page shows opens the clicked page and
//! keeps the keyboard on the docked index ([`SettingsFocus::Dock`], #1110): `j` and `k` then show
//! each page in turn, `enter`, `l` or `right` hand the page the keyboard, and `esc` returns to the
//! index. At Medium no index is docked, so there the page takes the keyboard.
//!
//! With no Ledger open the This Ledger pages stay on the index, drawn muted, and choosing one by
//! any route (a key, a click, the picker, a command) is refused on the status line and leaves the
//! frame as it was (`docs/settings.md`). A This Ledger page is never the cursor then: a restore
//! falls back to General, and a closing Ledger returns a This Ledger page to the index on General.

use lib_list_cursor::Step;
use lib_toast::Notice;
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP, SELECT};
use lib_viper::presenter::Presenter;
use lib_viper::view::{Hint, count};

use crate::keymap::{ENTER_PAGE, NEXT_PAGE, PICKER, PREVIOUS_PAGE};
use crate::msg;
use crate::page::{ScopeFacts, ScopeGroup, SettingsPage};
use crate::protocols::SettingsRouter;

/// What the frame is told: its keys (from [`crate::keymap`]), its clicks, and what the Shell
/// tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFrameEvent {
    /// On the index, step its cursor; in the picker, step its highlight.
    Move(Step),
    /// `enter`: the page under the index's cursor, or the picker's chosen page.
    Select,
    /// `l`: the page under the index's cursor.
    EnterPage,
    /// `h` or `left`: from the page back to the index.
    FocusIndex,
    /// At Medium, on a page: the page before or after in index order.
    StepPage(Step),
    /// At Medium, on a page: open the page picker.
    OpenPicker,
    /// The Shell's `back` (`esc`): close the picker, else leave the page for the index.
    Back,
    /// A click on a docked index row, at Large and X-Large: that page opens, with the keyboard in
    /// it from the index, or left on the docked index from a page.
    ClickIndex(SettingsPage),
    /// A click on a row of the index on show: that page opens in the index's place.
    ClickIndexPage(SettingsPage),
    /// A click on the Medium breadcrumb's **Settings**: back to the index.
    ClickCrumb,
    /// A click on the Medium breadcrumb's caret: open or close the picker.
    ClickCaret,
    /// A click on a picker row: straight to that page.
    ClickPicker(SettingsPage),
    /// A click on a row of the page on show, which hands the page the keyboard.
    PageClicked,
    /// A command or hand-off (`settings units`, a Reveal) opens a page with the keyboard in it.
    OpenPage(SettingsPage),
    /// Back or Forward onto `page`, which shows the page itself with the keyboard in it, without
    /// telling the Shell back. The launch restore sends it too, then [`Self::Entered`], so a start
    /// on Settings shows the index with its cursor on the restored page.
    Restore(SettingsPage),
    /// Settings has just become the destination (`g s`, `:settings`, the rail item, even with a
    /// page already showing), which lands on the index with its cursor where it was.
    Entered,
    /// A Ledger opened, so the This Ledger pages can show.
    LedgerOpened,
    /// The Ledger closed, so the This Ledger pages can't: one showing gives way to the index,
    /// with the cursor on General.
    LedgerClosed,
    /// Whether Settings is drawn as the Medium drill-down, the index and each page taking the View
    /// in turn. Leaving it closes the picker, which only the drill-down draws.
    DrillDown(bool),
}

impl SettingsFrameEvent {
    /// Whether a key sends the event, so the frame may decline it to the page or the Shell.
    fn is_key(self) -> bool {
        matches!(
            self,
            Self::Move(_)
                | Self::Select
                | Self::EnterPage
                | Self::FocusIndex
                | Self::StepPage(_)
                | Self::OpenPicker
                | Self::Back
        )
    }
}

/// What Settings shows, at every Window Size, and so where the keyboard sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsFocus {
    /// The Settings index shows, Settings' resting view, and no page is open. Its cursor is the
    /// frame's selected page.
    #[default]
    Index,
    /// The selected page shows, with the keyboard in it.
    Page,
    /// The selected page shows, with the keyboard on the docked index beside it, at Large and
    /// X-Large after a docked row is clicked while a page shows: its row is drawn selected, and
    /// `j` and `k` show each page in turn.
    Dock,
}

/// What the Client hands the frame when it builds it.
#[derive(Debug, Clone)]
pub struct SettingsFrameSetup {
    pub keymap: Keymap<SettingsFrameEvent>,
    /// The chord of the Shell's `back`, which the Shell keeps, for the legend.
    pub back_chord: String,
    pub facts: ScopeFacts,
    pub ledger_open: bool,
}

/// One index row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRow {
    pub page: SettingsPage,
    pub label: String,
    /// The line under the label on the index page.
    pub description: String,
    /// The page on show, drawn selected in the docked index, whether the keyboard is in the page or
    /// on the docked index. No row is while the index shows.
    pub selected: bool,
    /// The index's cursor while the index shows: highlighted on the index page, and tinted, not
    /// selected, in the docked index (DESKTOP-181).
    pub cursor: bool,
    /// A This Ledger page with no Ledger open: drawn dim, and refused when chosen.
    pub muted: bool,
}

/// One group of the index, with its rows in index order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexGroup {
    pub group: ScopeGroup,
    pub label: String,
    /// Its page count and whether it syncs, beside the heading on the Medium index page.
    pub scope_line: String,
    /// Where its settings live, under the heading on the Medium index page.
    pub description: String,
    pub rows: Vec<IndexRow>,
}

/// The Medium page heading: **Settings** back to the index, then the page's name, which carries
/// the picker's caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breadcrumb {
    pub settings: String,
    pub page: String,
}

/// One picker row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerRow {
    pub page: SettingsPage,
    pub label: String,
    /// The highlighted row, the one `enter` opens.
    pub chosen: bool,
    pub muted: bool,
}

/// One group of the picker: the pages matching the query, in index order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerGroup {
    pub group: ScopeGroup,
    pub label: String,
    pub rows: Vec<PickerRow>,
}

/// The open page picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerView {
    pub query: String,
    /// The groups with a page matching the query; empty when none matches.
    pub groups: Vec<PickerGroup>,
    pub chosen: Option<SettingsPage>,
}

/// The frame as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsFrameViewModel {
    /// The page on show, or the index's cursor while the index shows.
    pub selected: SettingsPage,
    pub focus: SettingsFocus,
    pub ledger_open: bool,
    pub drill_down: bool,
    /// The index shows, in the page's place, at every Window Size: Settings' resting view.
    pub index_page: bool,
    /// The heading beside the docked index: `Settings - Display`.
    pub heading: String,
    /// The Medium heading on a page, `None` elsewhere.
    pub breadcrumb: Option<Breadcrumb>,
    /// The Medium index page's page count.
    pub index_count: String,
    pub groups: Vec<IndexGroup>,
    /// The page on show's scope note, right of its heading and on the status line.
    pub scope_note: String,
    pub picker: Option<PickerView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Picker {
    query: String,
    /// The highlighted page; kept while it matches the query, else the first match is shown.
    chosen: SettingsPage,
}

impl Picker {
    fn matches(&self) -> Vec<SettingsPage> {
        let query = self.query.to_lowercase();
        SettingsPage::ALL
            .into_iter()
            .filter(|page| page.label().to_lowercase().contains(&query))
            .collect()
    }

    /// The highlighted page among the matches, `None` when nothing matches.
    fn chosen(&self) -> Option<SettingsPage> {
        lib_list_cursor::resolve(self.matches(), Some(self.chosen))
    }
}

/// The Settings frame's Presenter.
#[derive(Debug, Clone)]
pub struct SettingsFramePresenter {
    keymap: Keymap<SettingsFrameEvent>,
    back_chord: String,
    facts: ScopeFacts,
    selected: SettingsPage,
    focus: SettingsFocus,
    ledger_open: bool,
    drill_down: bool,
    picker: Option<Picker>,
}

impl SettingsFramePresenter {
    /// The frame on the index, its cursor on General, docked rather than drilled down.
    pub fn new(setup: SettingsFrameSetup) -> Self {
        Self {
            keymap: setup.keymap,
            back_chord: setup.back_chord,
            facts: setup.facts,
            selected: SettingsPage::default(),
            focus: SettingsFocus::default(),
            ledger_open: setup.ledger_open,
            drill_down: false,
            picker: None,
        }
    }

    pub fn keymap(&self) -> &Keymap<SettingsFrameEvent> {
        &self.keymap
    }

    /// Replaces the keymap, for a `[keybindings]` change.
    pub fn set_keymap(&mut self, keymap: Keymap<SettingsFrameEvent>) {
        self.keymap = keymap;
    }

    /// Replaces the chord of the Shell's `back` the legend shows, for a `[keybindings]` change.
    pub fn set_back_chord(&mut self, back_chord: String) {
        self.back_chord = back_chord;
    }

    /// The page on show, or the index's cursor while the index shows: the last-visited page the
    /// Client remembers.
    pub fn selected(&self) -> SettingsPage {
        self.selected
    }

    pub fn focus(&self) -> SettingsFocus {
        self.focus
    }

    /// Whether the picker is open, so the Client offers it every key.
    pub fn picker_open(&self) -> bool {
        self.picker.is_some()
    }

    /// Whether `page` can show: every This device page, and a This Ledger page only with a Ledger
    /// open.
    pub fn can_show(&self, page: SettingsPage) -> bool {
        self.ledger_open || !page.needs_ledger()
    }

    /// Whether `back` has something to back out of: the picker, or a page on show, the keyboard in
    /// it or on the docked index.
    pub fn takes_back(&self) -> bool {
        self.picker.is_some() || self.focus != SettingsFocus::Index
    }

    /// The frame's status-line hints as things stand: the picker's while it is open, the index's
    /// while it has the keyboard, and what a page adds at Medium. A docked page adds none, its own
    /// hints being the page's. Each chord is read from the keymap, so a rebind shows.
    pub fn hints(&self) -> Vec<Hint> {
        if self.picker.is_some() {
            return vec![
                Hint::new(self.pair(MOVE_DOWN, MOVE_UP), msg::settings_hint_page()),
                Hint::new(self.chord(SELECT), msg::settings_hint_open()),
                Hint::new(self.back_chord.clone(), msg::settings_hint_close()),
            ];
        }
        match self.focus {
            SettingsFocus::Index | SettingsFocus::Dock => self.index_hints(),
            SettingsFocus::Page if self.drill_down => vec![
                Hint::new(
                    self.pair(PREVIOUS_PAGE, NEXT_PAGE),
                    msg::settings_hint_pages(),
                ),
                Hint::new(self.chord(PICKER), msg::settings_hint_picker()),
                Hint::new(self.back_chord.clone(), msg::settings_hint_settings()),
            ],
            SettingsFocus::Page => Vec::new(),
        }
    }

    /// The frame's part of the `?` cheat sheet's Settings group: the index's keys, ahead of the
    /// page's own.
    pub fn help(&self) -> Vec<Hint> {
        self.index_hints()
    }
}

/// The frame is a composite host, not a page: it has no Dialog of its own to hear back from, and
/// the Client offers each key to the open page ahead of it, so it implements [`Presenter`] alone
/// (ADR-0056).
impl Presenter for SettingsFramePresenter {
    type Event = SettingsFrameEvent;
    type ViewModel = SettingsFrameViewModel;
    type Router<'r> = dyn SettingsRouter + 'r;

    fn handle(&mut self, event: SettingsFrameEvent, router: &mut Self::Router<'_>) {
        if !self.takes(&event) {
            return;
        }
        if self.picker.is_some() && event.is_key() {
            self.picker_key(event, router);
            return;
        }
        match event {
            SettingsFrameEvent::Move(step) => {
                let next =
                    lib_list_cursor::step(self.selected.index(), SettingsPage::ALL.len(), step);
                let page = SettingsPage::ALL[next];
                if self.focus == SettingsFocus::Dock {
                    self.show_docked(page, router);
                } else if self.allowed(page, router) {
                    self.selected = page;
                }
            }
            SettingsFrameEvent::Select | SettingsFrameEvent::EnterPage => {
                if self.focus == SettingsFocus::Dock {
                    self.focus = SettingsFocus::Page;
                } else {
                    self.open_page(self.selected, router);
                }
            }
            SettingsFrameEvent::PageClicked => self.focus = SettingsFocus::Page,
            SettingsFrameEvent::FocusIndex
            | SettingsFrameEvent::Entered
            | SettingsFrameEvent::Back => {
                self.focus = SettingsFocus::Index;
            }
            SettingsFrameEvent::StepPage(step) => {
                let next =
                    lib_list_cursor::step(self.selected.index(), SettingsPage::ALL.len(), step);
                self.open_page(SettingsPage::ALL[next], router);
            }
            SettingsFrameEvent::OpenPicker => self.open_picker(router),
            SettingsFrameEvent::ClickIndex(page) if self.focus != SettingsFocus::Index => {
                self.show_docked(page, router);
            }
            SettingsFrameEvent::ClickIndex(page)
            | SettingsFrameEvent::ClickIndexPage(page)
            | SettingsFrameEvent::OpenPage(page) => {
                self.open_page(page, router);
            }
            SettingsFrameEvent::ClickCrumb => {
                self.close_picker(router);
                self.focus = SettingsFocus::Index;
            }
            SettingsFrameEvent::ClickCaret => {
                if self.picker.is_some() {
                    self.close_picker(router);
                } else {
                    self.open_picker(router);
                }
            }
            SettingsFrameEvent::ClickPicker(page) => self.pick(page, router),
            SettingsFrameEvent::Restore(page) => {
                self.selected = if self.can_show(page) {
                    page
                } else {
                    SettingsPage::General
                };
                self.focus = SettingsFocus::Page;
            }
            SettingsFrameEvent::LedgerOpened => self.ledger_open = true,
            SettingsFrameEvent::LedgerClosed => {
                self.ledger_open = false;
                if !self.can_show(self.selected) {
                    self.selected = SettingsPage::General;
                    self.focus = SettingsFocus::Index;
                }
            }
            SettingsFrameEvent::DrillDown(drill_down) => {
                self.drill_down = drill_down;
                if !drill_down {
                    self.close_picker(router);
                }
                // Medium docks no index, so the page on show takes the keyboard.
                if drill_down && self.focus == SettingsFocus::Dock {
                    self.focus = SettingsFocus::Page;
                }
            }
        }
    }

    fn view_model(&self) -> SettingsFrameViewModel {
        let settings = lib_locale::msg::nav_settings();
        let page_label = self.selected.label();
        SettingsFrameViewModel {
            selected: self.selected,
            focus: self.focus,
            ledger_open: self.ledger_open,
            drill_down: self.drill_down,
            index_page: self.focus == SettingsFocus::Index,
            heading: msg::settings_page_heading(&settings, &page_label),
            breadcrumb: (self.drill_down && self.focus == SettingsFocus::Page).then_some(
                Breadcrumb {
                    settings,
                    page: page_label,
                },
            ),
            index_count: msg::settings_index_count(count(SettingsPage::ALL.len())),
            groups: ScopeGroup::ALL
                .into_iter()
                .map(|group| IndexGroup {
                    group,
                    label: group.label(),
                    scope_line: group.scope_line(),
                    description: group.description(),
                    rows: group
                        .pages()
                        .map(|page| IndexRow {
                            page,
                            label: page.label(),
                            description: page.description(),
                            selected: page == self.selected && self.focus != SettingsFocus::Index,
                            cursor: page == self.selected && self.focus == SettingsFocus::Index,
                            muted: !self.can_show(page),
                        })
                        .collect(),
                })
                .collect(),
            scope_note: self.selected.scope_note(self.facts),
            picker: self.picker.as_ref().map(|picker| self.picker_view(picker)),
        }
    }

    /// Whether the frame takes `event` as things stand, so a key it declines goes on to the page
    /// or the Shell (the page's keys go first, so its `h` wins over [`SettingsFrameEvent::FocusIndex`]).
    /// The open picker takes every key. Anything but a key is always taken.
    fn takes(&self, event: &SettingsFrameEvent) -> bool {
        if !event.is_key() || self.picker.is_some() {
            return true;
        }
        // The keyboard is on the index on show, or on the docked index beside a page.
        let on_index = self.focus != SettingsFocus::Page;
        let medium_page = self.drill_down && !on_index;
        match event {
            SettingsFrameEvent::Move(step) => {
                on_index && matches!(step, Step::Next | Step::Prev | Step::First | Step::Last)
            }
            SettingsFrameEvent::Select | SettingsFrameEvent::EnterPage => on_index,
            SettingsFrameEvent::FocusIndex => self.focus != SettingsFocus::Index,
            SettingsFrameEvent::StepPage(_) | SettingsFrameEvent::OpenPicker => medium_page,
            SettingsFrameEvent::Back => self.takes_back(),
            _ => false,
        }
    }
}

impl SettingsFramePresenter {
    /// Whether `page` may be chosen. With no Ledger open a This Ledger page is refused on the
    /// status line, and the frame stays as it was.
    fn allowed(&self, page: SettingsPage, router: &mut dyn SettingsRouter) -> bool {
        if self.can_show(page) {
            return true;
        }
        router.notify(Notice::Status(msg::settings_status_refused_no_ledger(
            ":open", ":new",
        )));
        false
    }

    /// Shows `page` with the keyboard in it, a page with nothing to focus included, and announces
    /// it for the Shell's history, which records pages only. The Shell ignores a page it already
    /// holds, so a page reopened from the index is announced again. Whether `page` now shows.
    fn open_page(&mut self, page: SettingsPage, router: &mut dyn SettingsRouter) -> bool {
        if !self.allowed(page, router) {
            return false;
        }
        self.selected = page;
        self.focus = SettingsFocus::Page;
        router.page_changed(page);
        true
    }

    /// Shows `page` with the keyboard left on the docked index, announcing it for the Shell's
    /// history as any page shown is. A refused page leaves the frame as it was.
    fn show_docked(&mut self, page: SettingsPage, router: &mut dyn SettingsRouter) {
        if !self.allowed(page, router) {
            return;
        }
        self.focus = SettingsFocus::Dock;
        if self.selected != page {
            self.selected = page;
            router.page_changed(page);
        }
    }

    fn open_picker(&mut self, router: &mut dyn SettingsRouter) {
        self.picker = Some(Picker {
            query: String::new(),
            chosen: self.selected,
        });
        router.picker_opened();
    }

    fn close_picker(&mut self, router: &mut dyn SettingsRouter) {
        if self.picker.take().is_some() {
            router.picker_closed();
        }
    }

    /// Straight to `page` from the picker. A refused page leaves the picker open.
    fn pick(&mut self, page: SettingsPage, router: &mut dyn SettingsRouter) {
        if !self.allowed(page, router) {
            return;
        }
        self.close_picker(router);
        self.open_page(page, router);
    }

    /// A key while the picker is open: the moves step its highlight through the matches, `enter`
    /// opens it, `back` closes it, and every other key is swallowed.
    fn picker_key(&mut self, event: SettingsFrameEvent, router: &mut dyn SettingsRouter) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        match event {
            SettingsFrameEvent::Move(step) => {
                let matches = picker.matches();
                if let Some(page) = lib_list_cursor::step_id(&matches, picker.chosen(), step) {
                    picker.chosen = page;
                }
            }
            SettingsFrameEvent::Select => {
                if let Some(page) = picker.chosen() {
                    self.pick(page, router);
                }
            }
            SettingsFrameEvent::Back => self.close_picker(router),
            _ => {}
        }
    }

    fn picker_view(&self, picker: &Picker) -> PickerView {
        let matches = picker.matches();
        let chosen = picker.chosen();
        PickerView {
            query: picker.query.clone(),
            groups: ScopeGroup::ALL
                .into_iter()
                .map(|group| PickerGroup {
                    group,
                    label: group.label(),
                    rows: group
                        .pages()
                        .filter(|page| matches.contains(page))
                        .map(|page| PickerRow {
                            page,
                            label: page.label(),
                            chosen: Some(page) == chosen,
                            muted: !self.can_show(page),
                        })
                        .collect(),
                })
                .filter(|group| !group.rows.is_empty())
                .collect(),
            chosen,
        }
    }

    fn index_hints(&self) -> Vec<Hint> {
        vec![
            Hint::new(self.pair(MOVE_DOWN, MOVE_UP), msg::settings_hint_page()),
            Hint::new(self.pair(ENTER_PAGE, SELECT), msg::settings_hint_open()),
        ]
    }

    fn chord(&self, command: &str) -> String {
        self.keymap.chord(command).unwrap_or_default().to_string()
    }

    /// Two commands' chords as one hint shows them, `j/k`.
    fn pair(&self, first: &str, second: &str) -> String {
        format!("{}/{}", self.chord(first), self.chord(second))
    }
}

#[cfg(test)]
mod tests {
    use lib_viper::keymap::KeyBindingConfig;

    use super::*;
    use crate::fake::{RecordingRouter, Routed};
    use crate::keymap::settings_frame_keymap;

    use SettingsFrameEvent as E;

    fn locale() {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
    }

    fn frame_on(config: &KeyBindingConfig, ledger_open: bool) -> SettingsFramePresenter {
        locale();
        SettingsFramePresenter::new(SettingsFrameSetup {
            keymap: settings_frame_keymap(config),
            back_chord: "esc".to_string(),
            facts: ScopeFacts { log_capacity: 500 },
            ledger_open,
        })
    }

    fn frame() -> SettingsFramePresenter {
        frame_on(&KeyBindingConfig::default(), true)
    }

    fn no_ledger() -> SettingsFramePresenter {
        frame_on(&KeyBindingConfig::default(), false)
    }

    /// Sends each event in turn, as a Client would, offering keys only when the frame takes them.
    fn send(
        frame: &mut SettingsFramePresenter,
        events: impl IntoIterator<Item = SettingsFrameEvent>,
    ) -> RecordingRouter {
        let mut router = RecordingRouter::default();
        for event in events {
            frame.handle(event, &mut router);
        }
        router
    }

    fn medium(frame: &mut SettingsFramePresenter) {
        send(frame, [SettingsFrameEvent::DrillDown(true)]);
    }

    /// The pages whose index rows are drawn selected, and the one under the cursor.
    fn rows(frame: &SettingsFramePresenter) -> (Vec<SettingsPage>, Vec<SettingsPage>) {
        let model = frame.view_model();
        let rows: Vec<_> = model.groups.iter().flat_map(|g| g.rows.clone()).collect();
        (
            rows.iter()
                .filter(|row| row.selected)
                .map(|row| row.page)
                .collect(),
            rows.iter()
                .filter(|row| row.cursor)
                .map(|row| row.page)
                .collect(),
        )
    }

    #[test]
    fn opens_on_the_index_with_its_cursor_on_general() {
        let frame = frame();
        let model = frame.view_model();

        assert_eq!(model.selected, SettingsPage::General);
        assert_eq!(model.focus, SettingsFocus::Index);
        assert_eq!(model.heading, "Settings - General");
        assert_eq!(model.scope_note, "this device · never synced");
        assert!(
            model.index_page,
            "the index is the resting view, docked or not"
        );
        assert_eq!(model.picker, None);
    }

    #[test]
    fn moving_on_the_index_moves_its_cursor_and_announces_nothing() {
        let mut frame = frame();

        let router = send(&mut frame, [E::Move(Step::Next), E::Move(Step::Next)]);

        assert_eq!(frame.selected(), SettingsPage::Tracing);
        assert!(
            router.pages_changed().is_empty(),
            "history records pages, not the cursor"
        );
        assert_eq!(
            frame.focus(),
            SettingsFocus::Index,
            "the index keeps the keyboard"
        );
        assert!(frame.view_model().index_page);
        assert_eq!(
            rows(&frame),
            (vec![], vec![SettingsPage::Tracing]),
            "no row is selected while the index shows; the cursor's is tinted"
        );
    }

    #[test]
    fn opening_the_page_under_the_cursor_selects_its_row_and_announces_it() {
        let mut frame = frame();

        let router = send(&mut frame, [E::Move(Step::Next), E::Select]);

        assert_eq!(frame.focus(), SettingsFocus::Page);
        assert!(!frame.view_model().index_page);
        assert_eq!(router.pages_changed(), [SettingsPage::Display]);
        assert_eq!(rows(&frame), (vec![SettingsPage::Display], vec![]));

        let router = send(&mut frame, [E::Back, E::EnterPage]);
        assert_eq!(
            router.pages_changed(),
            [SettingsPage::Display],
            "a reopened page is announced again; the Shell ignores a page it holds"
        );
    }

    #[test]
    fn first_and_last_reach_either_end_and_movement_stops_there() {
        let mut frame = frame();

        let router = send(&mut frame, [E::Move(Step::Prev)]);
        assert_eq!(frame.selected(), SettingsPage::General);
        assert!(
            router.pages_changed().is_empty(),
            "no change, no announcement"
        );

        send(&mut frame, [E::Move(Step::Last), E::Move(Step::Next)]);
        assert_eq!(frame.selected(), SettingsPage::DataBackup);

        send(&mut frame, [E::Move(Step::First)]);
        assert_eq!(frame.selected(), SettingsPage::General);
    }

    #[test]
    fn a_click_on_either_index_opens_the_page() {
        for click in [E::ClickIndex, E::ClickIndexPage] {
            let mut frame = frame();

            let router = send(&mut frame, [click(SettingsPage::Payees)]);

            assert_eq!(frame.selected(), SettingsPage::Payees);
            assert_eq!(frame.focus(), SettingsFocus::Page);
            assert!(!frame.view_model().index_page);
            assert_eq!(router.pages_changed(), [SettingsPage::Payees]);
        }
    }

    /// The frame with Tags open, then Payees clicked in the docked index.
    fn docked_on_payees() -> SettingsFramePresenter {
        let mut frame = frame();
        send(
            &mut frame,
            [
                E::OpenPage(SettingsPage::Tags),
                E::ClickIndex(SettingsPage::Payees),
            ],
        );
        frame
    }

    #[test]
    fn a_docked_click_while_a_page_shows_keeps_the_keyboard_on_the_docked_index() {
        let mut frame = frame();
        send(&mut frame, [E::OpenPage(SettingsPage::Tags)]);

        let router = send(&mut frame, [E::ClickIndex(SettingsPage::Payees)]);

        assert_eq!(frame.selected(), SettingsPage::Payees);
        assert_eq!(frame.focus(), SettingsFocus::Dock);
        assert!(!frame.view_model().index_page, "the page shows");
        assert_eq!(
            rows(&frame),
            (vec![SettingsPage::Payees], vec![]),
            "the docked row is drawn selected, not tinted"
        );
        assert_eq!(router.pages_changed(), [SettingsPage::Payees]);
        assert_eq!(
            frame.hints(),
            frame.index_hints(),
            "the docked index's keys, not the page's"
        );
    }

    #[test]
    fn on_the_docked_index_j_and_k_show_each_page_in_turn() {
        let mut frame = docked_on_payees();

        let router = send(&mut frame, [E::Move(Step::Next), E::Move(Step::Next)]);

        assert_eq!(frame.selected(), SettingsPage::Inventory);
        assert_eq!(frame.focus(), SettingsFocus::Dock);
        assert!(!frame.view_model().index_page);
        assert_eq!(rows(&frame), (vec![SettingsPage::Inventory], vec![]));
        assert_eq!(
            router.pages_changed(),
            [SettingsPage::Documents, SettingsPage::Inventory],
            "each page shown is a history entry"
        );

        let router = send(&mut frame, [E::Move(Step::Prev)]);
        assert_eq!(frame.selected(), SettingsPage::Documents);
        assert_eq!(router.pages_changed(), [SettingsPage::Documents]);
    }

    #[test]
    fn on_the_docked_index_enter_or_l_moves_the_keyboard_into_the_page() {
        for into in [E::Select, E::EnterPage, E::PageClicked] {
            let mut frame = docked_on_payees();

            let router = send(&mut frame, [into]);

            assert_eq!(frame.focus(), SettingsFocus::Page, "{into:?}");
            assert_eq!(frame.selected(), SettingsPage::Payees);
            assert!(router.routed.is_empty(), "the page already shows");
            assert!(
                !frame.takes(&E::Move(Step::Next)),
                "{into:?}: the page has its keys"
            );
        }
    }

    #[test]
    fn on_the_docked_index_esc_h_and_left_return_to_the_index() {
        for leave in [E::Back, E::FocusIndex] {
            let mut frame = docked_on_payees();
            assert!(frame.takes_back());
            assert!(frame.takes(&leave));

            let router = send(&mut frame, [leave]);

            assert_eq!(frame.focus(), SettingsFocus::Index, "{leave:?}");
            assert!(frame.view_model().index_page);
            assert_eq!(
                rows(&frame),
                (vec![], vec![SettingsPage::Payees]),
                "the cursor stays"
            );
            assert!(router.routed.is_empty());
        }
    }

    #[test]
    fn the_docked_index_takes_the_index_s_keys_and_not_medium_s() {
        let frame = docked_on_payees();

        for key in [
            E::Move(Step::Next),
            E::Move(Step::Last),
            E::Select,
            E::EnterPage,
            E::FocusIndex,
            E::Back,
        ] {
            assert!(frame.takes(&key), "{key:?}");
        }
        for key in [
            E::Move(Step::HalfPageDown),
            E::StepPage(Step::Next),
            E::OpenPicker,
        ] {
            assert!(!frame.takes(&key), "{key:?}");
        }
    }

    #[test]
    fn a_docked_click_on_the_page_on_show_hands_the_keyboard_to_the_docked_index() {
        let mut frame = frame();
        send(&mut frame, [E::OpenPage(SettingsPage::Tags)]);

        let router = send(&mut frame, [E::ClickIndex(SettingsPage::Tags)]);

        assert_eq!(frame.focus(), SettingsFocus::Dock);
        assert!(router.routed.is_empty(), "the page already shows");
    }

    #[test]
    fn with_no_ledger_a_docked_step_onto_a_this_ledger_page_is_refused_and_stays() {
        let mut frame = no_ledger();
        send(
            &mut frame,
            [
                E::OpenPage(SettingsPage::Tracing),
                E::ClickIndex(SettingsPage::About),
            ],
        );
        assert_eq!(frame.focus(), SettingsFocus::Dock);

        let router = send(
            &mut frame,
            [E::Move(Step::Next), E::ClickIndex(SettingsPage::Units)],
        );

        assert_eq!(frame.selected(), SettingsPage::About);
        assert_eq!(frame.focus(), SettingsFocus::Dock);
        assert!(router.pages_changed().is_empty());
        assert_eq!(router.statuses().len(), 2);
    }

    #[test]
    fn a_closing_ledger_returns_a_docked_this_ledger_page_to_the_index_on_general() {
        let mut frame = docked_on_payees();

        send(&mut frame, [E::LedgerClosed]);

        assert_eq!(frame.selected(), SettingsPage::General);
        assert_eq!(frame.focus(), SettingsFocus::Index);
    }

    #[test]
    fn resizing_to_medium_hands_a_docked_page_the_keyboard() {
        let mut frame = docked_on_payees();

        medium(&mut frame);

        assert_eq!(
            frame.focus(),
            SettingsFocus::Page,
            "Medium docks no index, so the page keeps showing with the keyboard"
        );
        assert_eq!(frame.selected(), SettingsPage::Payees);
        assert!(frame.view_model().breadcrumb.is_some());
    }

    #[test]
    fn with_no_ledger_the_this_ledger_pages_are_muted_and_never_show() {
        let mut frame = no_ledger();
        let model = frame.view_model();
        for row in model.groups.iter().flat_map(|group| group.rows.iter()) {
            assert_eq!(row.muted, row.page.needs_ledger(), "{:?}", row.page);
        }

        let router = send(
            &mut frame,
            [
                E::Move(Step::Last),
                E::ClickIndex(SettingsPage::Units),
                E::OpenPage(SettingsPage::Payees),
                E::Restore(SettingsPage::Ledger),
            ],
        );

        assert_eq!(frame.selected(), SettingsPage::General);
        assert!(router.pages_changed().is_empty());
        assert_eq!(
            router.statuses(),
            vec!["No Ledger open: :open or :new first".to_string(); 3],
            "each way onto a This Ledger page is refused; a restore falls back quietly"
        );
    }

    #[test]
    fn stepping_from_the_last_this_device_page_with_no_ledger_is_refused_and_stays() {
        let mut frame = no_ledger();
        send(&mut frame, [E::Move(Step::Next); 3]);
        assert_eq!(frame.selected(), SettingsPage::About);

        let router = send(&mut frame, [E::Move(Step::Next)]);

        assert_eq!(frame.selected(), SettingsPage::About);
        assert_eq!(frame.focus(), SettingsFocus::Index);
        assert_eq!(router.statuses().len(), 1);
    }

    #[test]
    fn a_ledger_opening_and_closing_shows_and_hides_the_this_ledger_pages() {
        let mut frame = no_ledger();

        send(
            &mut frame,
            [E::LedgerOpened, E::ClickIndex(SettingsPage::Units)],
        );
        assert_eq!(frame.selected(), SettingsPage::Units);
        assert!(
            frame
                .view_model()
                .groups
                .iter()
                .flat_map(|g| g.rows.iter())
                .all(|row| !row.muted)
        );

        let router = send(&mut frame, [E::LedgerClosed]);
        assert_eq!(
            frame.selected(),
            SettingsPage::General,
            "a This Ledger page on show gives way to the index on General"
        );
        assert_eq!(frame.focus(), SettingsFocus::Index);
        assert!(frame.view_model().index_page);
        assert!(router.pages_changed().is_empty());
    }

    #[test]
    fn a_closing_ledger_leaves_a_this_device_page_on_show() {
        let mut frame = frame();
        send(&mut frame, [E::OpenPage(SettingsPage::Display)]);

        send(&mut frame, [E::LedgerClosed]);

        assert_eq!(frame.selected(), SettingsPage::Display);
        assert_eq!(frame.focus(), SettingsFocus::Page);
    }

    #[test]
    fn a_closing_ledger_moves_an_index_cursor_on_a_this_ledger_page_to_general() {
        let mut frame = frame();
        send(&mut frame, [E::Move(Step::Last)]);

        send(&mut frame, [E::LedgerClosed]);

        assert_eq!(frame.selected(), SettingsPage::General);
        assert_eq!(frame.focus(), SettingsFocus::Index);
    }

    #[test]
    fn enter_or_l_moves_the_keyboard_into_the_page_and_h_or_back_returns_it() {
        let mut frame = frame();

        send(&mut frame, [E::Select]);
        assert_eq!(frame.focus(), SettingsFocus::Page);
        send(&mut frame, [E::FocusIndex]);
        assert_eq!(frame.focus(), SettingsFocus::Index);
        send(&mut frame, [E::EnterPage]);
        assert_eq!(frame.focus(), SettingsFocus::Page);
        assert!(frame.takes_back());
        send(&mut frame, [E::Back]);
        assert_eq!(frame.focus(), SettingsFocus::Index);
        assert!(!frame.takes_back(), "back on the index is the Shell's");
    }

    #[test]
    fn the_frame_declines_the_keys_that_belong_to_where_the_focus_is_not() {
        let mut frame = frame();
        assert!(frame.takes(&E::Move(Step::Next)));
        assert!(!frame.takes(&E::FocusIndex));
        assert!(
            !frame.takes(&E::OpenPicker),
            "only a Medium page has the picker"
        );
        assert!(!frame.takes(&E::Move(Step::HalfPageDown)));

        send(&mut frame, [E::EnterPage]);
        assert!(
            !frame.takes(&E::Move(Step::Next)),
            "the page scrolls or moves its rows"
        );
        assert!(!frame.takes(&E::Select), "enter on a page is the page's");
        assert!(frame.takes(&E::FocusIndex));
        assert!(frame.takes(&E::ClickIndex(SettingsPage::Display)));
    }

    #[test]
    fn a_page_with_nothing_to_focus_opens_like_any_other() {
        let mut frame = frame();
        send(&mut frame, [E::Move(Step::Next); 3]);
        assert_eq!(frame.selected(), SettingsPage::About);

        send(&mut frame, [E::EnterPage]);

        assert_eq!(
            frame.focus(),
            SettingsFocus::Page,
            "with the index the resting view, About shows only when opened"
        );
        assert!(!frame.view_model().index_page);
    }

    #[test]
    fn esc_h_and_left_return_a_page_to_the_index_at_every_window_size() {
        for drill_down in [false, true] {
            for leave in [E::Back, E::FocusIndex] {
                let mut frame = frame();
                send(&mut frame, [E::DrillDown(drill_down)]);
                send(&mut frame, [E::OpenPage(SettingsPage::Tags)]);

                let router = send(&mut frame, [leave]);

                let model = frame.view_model();
                assert!(model.index_page, "{leave:?} at drill-down {drill_down}");
                assert_eq!(model.selected, SettingsPage::Tags, "the cursor stays");
                assert!(router.routed.is_empty(), "the index is no history entry");
            }
        }
    }

    #[test]
    fn entering_settings_lands_on_the_index_even_from_an_open_page() {
        let mut frame = frame();
        send(&mut frame, [E::Move(Step::Next), E::Select]);
        assert_eq!(frame.focus(), SettingsFocus::Page);

        let router = send(&mut frame, [E::Entered]);

        assert_eq!(frame.focus(), SettingsFocus::Index);
        assert!(frame.view_model().index_page);
        assert_eq!(frame.selected(), SettingsPage::Display, "the cursor stays");
        assert!(router.routed.is_empty());
    }

    #[test]
    fn a_start_on_settings_lands_on_the_index_with_the_restored_page_under_the_cursor() {
        let mut frame = frame();

        let router = send(&mut frame, [E::Restore(SettingsPage::Units), E::Entered]);

        assert_eq!(frame.selected(), SettingsPage::Units);
        assert_eq!(frame.focus(), SettingsFocus::Index);
        assert!(router.routed.is_empty());
    }

    #[test]
    fn a_row_click_on_the_page_hands_it_the_keyboard() {
        let mut frame = frame();

        send(&mut frame, [E::PageClicked]);

        assert_eq!(frame.focus(), SettingsFocus::Page);
    }

    #[test]
    fn a_command_opens_its_page_with_the_keyboard_in_it_and_entering_settings_lands_on_the_index() {
        let mut frame = frame();

        let router = send(&mut frame, [E::OpenPage(SettingsPage::Tags)]);
        assert_eq!(frame.selected(), SettingsPage::Tags);
        assert_eq!(frame.focus(), SettingsFocus::Page);
        assert_eq!(router.pages_changed(), [SettingsPage::Tags]);

        send(&mut frame, [E::Entered]);
        assert_eq!(frame.focus(), SettingsFocus::Index);
        assert_eq!(
            frame.selected(),
            SettingsPage::Tags,
            "the last-visited page stays"
        );
    }

    #[test]
    fn back_and_forward_show_the_restored_page_itself_without_announcing_it() {
        for drill_down in [false, true] {
            let mut frame = frame();
            send(&mut frame, [E::DrillDown(drill_down)]);

            let router = send(&mut frame, [E::Restore(SettingsPage::Inventory)]);

            assert_eq!(frame.selected(), SettingsPage::Inventory);
            assert_eq!(
                frame.focus(),
                SettingsFocus::Page,
                "the page, not the index"
            );
            assert!(!frame.view_model().index_page);
            assert!(
                router.routed.is_empty(),
                "the Shell restored it, so nothing goes back"
            );
        }
    }

    #[test]
    fn with_no_ledger_a_restored_this_ledger_page_shows_general() {
        let mut frame = no_ledger();

        let router = send(&mut frame, [E::Restore(SettingsPage::Payees)]);

        assert_eq!(frame.selected(), SettingsPage::General);
        assert_eq!(frame.focus(), SettingsFocus::Page);
        assert!(router.routed.is_empty(), "a restore falls back quietly");
    }

    #[test]
    fn resizing_keeps_whatever_shows() {
        let mut frame = frame();
        send(&mut frame, [E::OpenPage(SettingsPage::Tags)]);
        medium(&mut frame);
        assert_eq!(frame.focus(), SettingsFocus::Page);
        send(&mut frame, [E::DrillDown(false)]);
        assert_eq!(frame.focus(), SettingsFocus::Page);

        send(&mut frame, [E::Back, E::DrillDown(true)]);
        assert!(frame.view_model().index_page);
        send(&mut frame, [E::DrillDown(false)]);
        assert!(frame.view_model().index_page);
    }

    #[test]
    fn at_medium_the_index_is_a_page_and_a_page_carries_the_breadcrumb() {
        let mut frame = frame();
        medium(&mut frame);

        let model = frame.view_model();
        assert!(model.index_page);
        assert_eq!(model.breadcrumb, None);
        assert_eq!(model.index_count, "15 pages");
        assert_eq!(model.groups[0].scope_line, "4 pages · never synced");

        send(&mut frame, [E::ClickIndexPage(SettingsPage::Display)]);
        let model = frame.view_model();
        assert!(!model.index_page);
        assert_eq!(
            model.breadcrumb,
            Some(Breadcrumb {
                settings: "Settings".to_string(),
                page: "Display".to_string(),
            })
        );

        send(&mut frame, [E::ClickCrumb]);
        assert!(frame.view_model().index_page);
        assert_eq!(
            frame.selected(),
            SettingsPage::Display,
            "the page stays selected"
        );
    }

    #[test]
    fn at_medium_the_brackets_step_through_the_pages_into_each() {
        let mut frame = frame();
        medium(&mut frame);
        send(&mut frame, [E::Select]);

        let router = send(
            &mut frame,
            [E::StepPage(Step::Next), E::StepPage(Step::Next)],
        );
        assert_eq!(frame.selected(), SettingsPage::Tracing);
        assert_eq!(frame.focus(), SettingsFocus::Page);
        assert_eq!(
            router.pages_changed(),
            [SettingsPage::Display, SettingsPage::Tracing]
        );

        send(&mut frame, [E::StepPage(Step::Prev)]);
        assert_eq!(frame.selected(), SettingsPage::Display);
    }

    #[test]
    fn the_picker_opens_on_the_page_on_show_and_moves_and_opens_the_chosen_page() {
        let mut frame = frame();
        medium(&mut frame);
        send(&mut frame, [E::Select]);

        let router = send(&mut frame, [E::OpenPicker]);
        assert_eq!(router.routed, [Routed::PickerOpened]);
        let picker = frame.view_model().picker.expect("the picker is open");
        assert_eq!(picker.chosen, Some(SettingsPage::General));
        assert_eq!(picker.groups.len(), 2, "both groups, in index order");
        assert!(
            frame.takes(&E::FocusIndex),
            "the open picker takes every key"
        );

        send(
            &mut frame,
            [E::Move(Step::Next), E::Move(Step::Next), E::FocusIndex],
        );
        assert_eq!(
            frame.view_model().picker.and_then(|picker| picker.chosen),
            Some(SettingsPage::Tracing),
            "h is swallowed"
        );

        let router = send(&mut frame, [E::Select]);
        assert_eq!(frame.selected(), SettingsPage::Tracing);
        assert_eq!(frame.focus(), SettingsFocus::Page);
        assert!(!frame.picker_open());
        assert_eq!(
            router.routed,
            [
                Routed::PickerClosed,
                Routed::PageChanged(SettingsPage::Tracing)
            ]
        );
    }

    #[test]
    fn with_no_ledger_a_this_ledger_page_in_the_picker_is_refused_and_the_picker_stays() {
        let mut frame = no_ledger();
        medium(&mut frame);
        send(&mut frame, [E::Select, E::OpenPicker]);
        send(&mut frame, [E::Move(Step::Last)]);

        let router = send(&mut frame, [E::Select]);

        assert!(frame.picker_open());
        assert_eq!(frame.selected(), SettingsPage::General);
        assert_eq!(router.statuses().len(), 1);
        let picker = frame.view_model().picker.expect("the picker is open");
        assert!(picker.groups[1].rows.iter().all(|row| row.muted));
    }

    #[test]
    fn back_closes_the_picker_and_leaves_the_page_and_the_caret_toggles_it() {
        let mut frame = frame();
        medium(&mut frame);
        send(&mut frame, [E::Select, E::OpenPicker]);

        let router = send(&mut frame, [E::Back]);
        assert!(!frame.picker_open());
        assert_eq!(
            frame.focus(),
            SettingsFocus::Page,
            "the page keeps the keyboard"
        );
        assert_eq!(router.routed, [Routed::PickerClosed]);

        send(&mut frame, [E::ClickCaret]);
        assert!(frame.picker_open());
        send(&mut frame, [E::ClickCaret]);
        assert!(!frame.picker_open());
    }

    #[test]
    fn a_picker_row_click_opens_its_page() {
        let mut frame = frame();
        medium(&mut frame);
        send(&mut frame, [E::Select, E::ClickCaret]);

        send(&mut frame, [E::ClickPicker(SettingsPage::Accounts)]);

        assert!(!frame.picker_open());
        assert_eq!(frame.selected(), SettingsPage::Accounts);
        assert_eq!(frame.focus(), SettingsFocus::Page);
    }

    #[test]
    fn leaving_medium_closes_the_picker() {
        let mut frame = frame();
        medium(&mut frame);
        send(&mut frame, [E::Select, E::OpenPicker]);

        let router = send(&mut frame, [E::DrillDown(false)]);

        assert!(!frame.picker_open());
        assert_eq!(router.routed, [Routed::PickerClosed]);
        assert!(!frame.view_model().drill_down);
    }

    #[test]
    fn the_hints_follow_the_focus_and_the_keymap() {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(MOVE_DOWN.to_string(), "n".to_string());
        config.bindings.insert(PICKER.to_string(), "P".to_string());
        let mut frame = frame_on(&config, true);
        let pairs = |hints: Vec<Hint>| -> Vec<(String, String)> {
            hints
                .into_iter()
                .map(|hint| (hint.chord, hint.action))
                .collect()
        };

        assert_eq!(
            pairs(frame.hints()),
            [
                ("n/k".to_string(), "page".to_string()),
                ("l/enter".to_string(), "open".to_string()),
            ]
        );
        assert_eq!(frame.help(), frame.hints());

        send(&mut frame, [E::EnterPage]);
        assert!(
            frame.hints().is_empty(),
            "a docked page's hints are its own"
        );

        medium(&mut frame);
        assert_eq!(
            pairs(frame.hints()),
            [
                ("[/]".to_string(), "pages".to_string()),
                ("P".to_string(), "picker".to_string()),
                ("esc".to_string(), "settings".to_string()),
            ]
        );

        send(&mut frame, [E::OpenPicker]);
        assert_eq!(
            pairs(frame.hints()),
            [
                ("n/k".to_string(), "page".to_string()),
                ("enter".to_string(), "open".to_string()),
                ("esc".to_string(), "close".to_string()),
            ]
        );
    }

    #[test]
    fn the_scope_note_follows_the_page_on_show() {
        let mut frame = frame();

        send(&mut frame, [E::ClickIndex(SettingsPage::Tracing)]);

        assert_eq!(
            frame.view_model().scope_note,
            "this device · diagnostic · last 500 entries"
        );
    }
}
