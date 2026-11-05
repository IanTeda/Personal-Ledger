//! The Settings frame's Presenter: which page shows, whether the index or the page has the
//! keyboard, the Medium drill-down and its page picker. The pages themselves are drawn by their
//! own Presenters; the frame only chooses among them.
//!
//! With no Ledger open the This Ledger pages stay on the index, drawn muted, and choosing one by
//! any route (a key, a click, the picker, a command) is refused on the status line and leaves the
//! page on show (`docs/settings.md`). A This Ledger page is never the page on show then: a restore
//! or a closing Ledger falls back to General.

use lib_list_cursor::Step;
use lib_toast::Notice;
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP, SELECT};
use lib_viper::view::{Hint, count};

use crate::keymap::{ENTER_PAGE, NEXT_PAGE, PICKER, PREVIOUS_PAGE};
use crate::msg;
use crate::page::{ScopeFacts, ScopeGroup, SettingsPage};
use crate::protocols::SettingsRouter;

/// What the frame is told: its keys (from [`crate::keymap`]), its clicks, and what the Shell
/// tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFrameEvent {
    /// On the index, step the page on show; in the picker, step its highlight.
    Move(Step),
    /// `enter`: into the page from the index, or the picker's chosen page.
    Select,
    /// From the index into the page on show.
    EnterPage,
    /// From the page back to the index.
    FocusIndex,
    /// At Medium, on a page: the page before or after in index order.
    StepPage(Step),
    /// At Medium, on a page: open the page picker.
    OpenPicker,
    /// The Shell's `back` (`esc`): close the picker, else leave the page for the index.
    Back,
    /// A character typed into the open picker's query.
    Type(char),
    /// `Backspace` in the open picker's query.
    Backspace,
    /// A click on a docked index row: that page shows, and the index keeps the keyboard.
    ClickIndex(SettingsPage),
    /// A click on a Medium index page row: that page opens in the index's place.
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
    /// Back, Forward or the launch restore `page`, without moving focus or telling the Shell back.
    Restore(SettingsPage),
    /// Settings has just become the destination, which lands on the index.
    Entered,
    /// A Ledger opened, so the This Ledger pages can show.
    LedgerOpened,
    /// The Ledger closed, so the This Ledger pages can't.
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
                | Self::Type(_)
                | Self::Backspace
        )
    }
}

/// Where the keyboard sits in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsFocus {
    #[default]
    Index,
    Page,
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
    /// The line under the label on the Medium index page.
    pub description: String,
    /// The page on show.
    pub selected: bool,
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
    /// The page on show.
    pub selected: SettingsPage,
    pub focus: SettingsFocus,
    pub ledger_open: bool,
    pub drill_down: bool,
    /// At Medium with the index focused, the index is a page of its own, in the page's place.
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
    /// The frame on General with the index focused, docked rather than drilled down.
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

    /// The page on show, also the last-visited page the Client remembers.
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

    /// Whether the frame takes `event` as things stand, so a key it declines goes on to the page
    /// or the Shell (the page's keys go first, so its `h` wins over [`SettingsFrameEvent::FocusIndex`]).
    /// The open picker takes every key. Anything but a key is always taken.
    pub fn takes(&self, event: &SettingsFrameEvent) -> bool {
        if !event.is_key() || self.picker.is_some() {
            return true;
        }
        let index = self.focus == SettingsFocus::Index;
        let medium_page = self.drill_down && !index;
        match event {
            SettingsFrameEvent::Move(step) => {
                index && matches!(step, Step::Next | Step::Prev | Step::First | Step::Last)
            }
            SettingsFrameEvent::Select | SettingsFrameEvent::EnterPage => index,
            SettingsFrameEvent::FocusIndex => !index,
            SettingsFrameEvent::StepPage(_) | SettingsFrameEvent::OpenPicker => medium_page,
            SettingsFrameEvent::Back => self.takes_back(),
            _ => false,
        }
    }

    /// Whether `back` has something to back out of: the picker, or a page with the keyboard.
    pub fn takes_back(&self) -> bool {
        self.picker.is_some() || self.focus == SettingsFocus::Page
    }

    pub fn handle(&mut self, event: SettingsFrameEvent, router: &mut dyn SettingsRouter) {
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
                self.select(SettingsPage::ALL[next], router);
            }
            SettingsFrameEvent::Select
            | SettingsFrameEvent::EnterPage
            | SettingsFrameEvent::PageClicked => {
                self.focus_page();
            }
            SettingsFrameEvent::FocusIndex | SettingsFrameEvent::Entered => {
                self.focus = SettingsFocus::Index;
            }
            SettingsFrameEvent::Back => self.focus = SettingsFocus::Index,
            SettingsFrameEvent::StepPage(step) => {
                let next =
                    lib_list_cursor::step(self.selected.index(), SettingsPage::ALL.len(), step);
                if self.select(SettingsPage::ALL[next], router) {
                    self.focus_page();
                }
            }
            SettingsFrameEvent::OpenPicker => self.open_picker(router),
            SettingsFrameEvent::ClickIndex(page) => {
                if self.select(page, router) {
                    self.focus = SettingsFocus::Index;
                }
            }
            SettingsFrameEvent::ClickIndexPage(page) | SettingsFrameEvent::OpenPage(page) => {
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
            }
            SettingsFrameEvent::LedgerOpened => self.ledger_open = true,
            SettingsFrameEvent::LedgerClosed => {
                self.ledger_open = false;
                if !self.can_show(self.selected) {
                    self.selected = SettingsPage::General;
                }
            }
            SettingsFrameEvent::DrillDown(drill_down) => {
                self.drill_down = drill_down;
                if !drill_down {
                    self.close_picker(router);
                }
            }
            SettingsFrameEvent::Type(_) | SettingsFrameEvent::Backspace => {}
        }
    }

    pub fn view_model(&self) -> SettingsFrameViewModel {
        let settings = lib_locale::msg::nav_settings();
        let page_label = self.selected.label();
        SettingsFrameViewModel {
            selected: self.selected,
            focus: self.focus,
            ledger_open: self.ledger_open,
            drill_down: self.drill_down,
            index_page: self.drill_down && self.focus == SettingsFocus::Index,
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
                            selected: page == self.selected,
                            muted: !self.can_show(page),
                        })
                        .collect(),
                })
                .collect(),
            scope_note: self.selected.scope_note(self.facts),
            picker: self.picker.as_ref().map(|picker| self.picker_view(picker)),
        }
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
            SettingsFocus::Index => self.index_hints(),
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

impl SettingsFramePresenter {
    /// Makes `page` the page on show, announcing a change. With no Ledger open a This Ledger page
    /// is refused on the status line and the page on show stays. Whether `page` now shows.
    fn select(&mut self, page: SettingsPage, router: &mut dyn SettingsRouter) -> bool {
        if !self.can_show(page) {
            router.notify(Notice::Status(msg::settings_status_refused_no_ledger(
                ":open", ":new",
            )));
            return false;
        }
        if self.selected != page {
            self.selected = page;
            router.page_changed(page);
        }
        true
    }

    /// Hands the page the keyboard. A page with nothing to focus keeps it on the docked index as a
    /// quiet no-op; at Medium the page is what's on screen, so it opens all the same.
    fn focus_page(&mut self) {
        if self.selected.has_controls() || self.drill_down {
            self.focus = SettingsFocus::Page;
        }
    }

    fn open_page(&mut self, page: SettingsPage, router: &mut dyn SettingsRouter) {
        if self.select(page, router) {
            self.focus_page();
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
        if !self.can_show(page) {
            self.select(page, router);
            return;
        }
        self.close_picker(router);
        self.open_page(page, router);
    }

    /// A key while the picker is open: the moves step its highlight through the matches, `enter`
    /// opens it, typing narrows the query, `back` closes it, and every other key is swallowed.
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
            SettingsFrameEvent::Type(character) => picker.query.push(character),
            SettingsFrameEvent::Backspace => {
                picker.query.pop();
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

    #[test]
    fn opens_on_general_with_the_index_focused() {
        let frame = frame();
        let model = frame.view_model();

        assert_eq!(model.selected, SettingsPage::General);
        assert_eq!(model.focus, SettingsFocus::Index);
        assert_eq!(model.heading, "Settings - General");
        assert_eq!(model.scope_note, "this device · never synced");
        assert!(!model.index_page);
        assert_eq!(model.picker, None);
    }

    #[test]
    fn moving_on_the_index_selects_each_page_and_announces_it() {
        let mut frame = frame();

        let router = send(&mut frame, [E::Move(Step::Next), E::Move(Step::Next)]);

        assert_eq!(frame.selected(), SettingsPage::Tracing);
        assert_eq!(
            router.pages_changed(),
            [SettingsPage::Display, SettingsPage::Tracing]
        );
        assert_eq!(
            frame.focus(),
            SettingsFocus::Index,
            "the index keeps the keyboard"
        );
        let selected: Vec<_> = frame
            .view_model()
            .groups
            .iter()
            .flat_map(|group| group.rows.iter())
            .filter(|row| row.selected)
            .map(|row| row.page)
            .collect();
        assert_eq!(selected, [SettingsPage::Tracing]);
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
    fn a_click_on_the_docked_index_shows_the_page_and_keeps_the_index_focused() {
        let mut frame = frame();

        let router = send(&mut frame, [E::ClickIndex(SettingsPage::Payees)]);

        assert_eq!(frame.selected(), SettingsPage::Payees);
        assert_eq!(frame.focus(), SettingsFocus::Index);
        assert_eq!(router.pages_changed(), [SettingsPage::Payees]);
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
        send(&mut frame, [E::ClickIndex(SettingsPage::About)]);

        let router = send(&mut frame, [E::Move(Step::Next)]);

        assert_eq!(frame.selected(), SettingsPage::About);
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
            "a This Ledger page on show gives way to General"
        );
        assert!(router.pages_changed().is_empty());
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
    fn a_page_with_nothing_to_focus_keeps_the_keyboard_on_the_docked_index() {
        let mut frame = frame();
        send(
            &mut frame,
            [E::ClickIndex(SettingsPage::About), E::EnterPage],
        );
        assert_eq!(frame.focus(), SettingsFocus::Index);

        medium(&mut frame);
        send(&mut frame, [E::EnterPage]);
        assert_eq!(
            frame.focus(),
            SettingsFocus::Page,
            "at Medium the page opens all the same"
        );
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
    fn a_restored_page_shows_without_being_announced_or_moving_focus() {
        let mut frame = frame();
        send(&mut frame, [E::EnterPage]);

        let router = send(&mut frame, [E::Restore(SettingsPage::Inventory)]);

        assert_eq!(frame.selected(), SettingsPage::Inventory);
        assert_eq!(frame.focus(), SettingsFocus::Page);
        assert!(
            router.routed.is_empty(),
            "the Shell restored it, so nothing goes back"
        );
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
    fn the_picker_finds_a_page_by_name() {
        let mut frame = frame();
        medium(&mut frame);
        send(&mut frame, [E::Select, E::OpenPicker]);

        send(&mut frame, "pay".chars().map(E::Type));
        let picker = frame.view_model().picker.expect("the picker is open");
        assert_eq!(picker.query, "pay");
        let shown: Vec<_> = picker
            .groups
            .iter()
            .flat_map(|group| group.rows.iter().map(|row| row.page))
            .collect();
        assert_eq!(shown, [SettingsPage::Payees]);
        assert_eq!(picker.chosen, Some(SettingsPage::Payees));

        send(
            &mut frame,
            [E::Backspace, E::Backspace, E::Backspace, E::Type('z')],
        );
        let picker = frame.view_model().picker.expect("the picker is open");
        assert!(picker.groups.is_empty());
        assert_eq!(picker.chosen, None);
        send(&mut frame, [E::Select]);
        assert!(frame.picker_open(), "nothing to open");

        send(&mut frame, [E::Backspace]);
        send(&mut frame, "DIS".chars().map(E::Type));
        send(&mut frame, [E::Select]);
        assert_eq!(
            frame.selected(),
            SettingsPage::Display,
            "matching ignores case"
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
