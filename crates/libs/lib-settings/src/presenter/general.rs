//! Settings › General's Presenter (DESKTOP-177, DESKTOP-178): On start, Start view, Toasts, the
//! Locale and the Data folder, Configuration in the user's configuration file (ADR-0053, ADR-0057)
//! written through [`DeviceConfiguration`], and the read-only last Ledger and configuration file,
//! which the Client hands it. Everything here belongs to this device and works with no Ledger open
//! (ADR-0051).
//!
//! A control shows a change only once it is saved: a shadowed key or a refused write leaves it as
//! it was and says why on the status line. The Locale and the Data folder are the exceptions
//! (DESKTOP-186): each is written even while shadowed and shows the file's value, with a note
//! naming the override. A chosen Locale applies at the next start, so a note says so while the
//! file's differs from the one in effect (DESKTOP-185).
//!
//! The Data folder is picked in the Shell's file explorer (DESKTOP-187): **Change…** asks the
//! Router for it, and what it picks comes back as [`GeneralEvent::SetDataFolder`], this the one
//! place the Data folder is written. It applies at once, with a Success Toast (DESKTOP-188).
//!
//! Start view and the Locale are dropdowns (#1156): each holds its open list as a [`SelectList`],
//! as a Dialog's Presenter does, but driven by the page's keymap. While a list is open its keys
//! (`j`/`k`, `enter`, `esc`) are the list's, and the rest of the page waits; at most one is open.

use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use lib_core::{OnStart, StartView};
use lib_list_cursor::Step;
use lib_locale::Locale;
use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::DialogInput;
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP, SELECT};
use lib_viper::presenter::{PagePresenter, Presenter};
use lib_viper::view::{Choice, Hint, SelectList, SelectView};

use super::device::{ControlView, chords, settle, shadowed_note, step_choice};
use crate::keymap::{NEXT_CHOICE, PREVIOUS_CHOICE, TEST_TOASTS};
use crate::msg;
use crate::protocols::{DeviceAnswer, DeviceConfiguration, DeviceKey, DeviceRouter};

/// On start's choices, in the segmented control's order.
pub const ON_START_CHOICES: [OnStart; 2] = [OnStart::ReopenLastLedger, OnStart::NoLedger];

/// Toasts' choices, On then Off as on the control.
const TOASTS_CHOICES: [bool; 2] = [true, false];

/// The page's controls, top to bottom.
pub const GENERAL_FIELD_ON_START: usize = 0;
pub const GENERAL_FIELD_START_VIEW: usize = 1;
pub const GENERAL_FIELD_TOASTS: usize = 2;
pub const GENERAL_FIELD_LOCALE: usize = 3;
pub const GENERAL_FIELD_DATA_FOLDER: usize = 4;
const GENERAL_FIELD_COUNT: usize = 5;

/// The Locale's choices, in the radio group's order: **System** (`None`), then `lib-locale`'s
/// supported Locales by tag, never the pseudo-Locale.
pub fn locale_choices() -> Vec<Option<Locale>> {
    let mut locales: Vec<Locale> = Locale::SUPPORTED
        .into_iter()
        .filter(|locale| !locale.is_pseudo())
        .collect();
    locales.sort_by_key(|locale| locale.tag());
    std::iter::once(None)
        .chain(locales.into_iter().map(Some))
        .collect()
}

/// What General is told and the person asks of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneralEvent {
    /// `j`/`k`: the control below or above, or an open list's highlight.
    Move(Step),
    /// `h`/`l`: the focused control's choice after or before.
    Change(Step),
    ChooseOnStart(OnStart),
    ChooseStartView(StartView),
    ChooseToasts(bool),
    /// `:toasts on|off`, which writes the setting even when it holds already.
    SetToasts(bool),
    /// A Locale, `None` for **System**.
    ChooseLocale(Option<Locale>),
    /// `t`: the Toasts test, only while Toasts has the keyboard.
    TestToasts,
    /// A click on **Test**, wherever the keyboard is.
    PressTest,
    /// `enter`: on Start view or the Locale, opens its list, or picks an open list's highlight; on
    /// the Data folder, its picker.
    Select,
    /// `esc` while a list is open: closes it, changing nothing.
    CloseList,
    /// A click on Start view's field: opens its list, or closes it while open.
    PressStartView,
    /// A click on the Locale's field: opens its list, or closes it while open.
    PressLocale,
    /// A click on **Change…**, wherever the keyboard is.
    PressChange,
    /// The folder the picker picked, from **Change…** or `:data`: written as the Data folder.
    SetDataFolder(PathBuf),
}

/// A device's settings General writes, as start-up read them from Configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralSettings {
    pub on_start: OnStart,
    pub start_view: StartView,
    pub toasts_on: bool,
    /// The Locale the user's configuration file names, `None` for **System**: not the one in
    /// effect, which a flag, a variable or a higher file may set.
    pub locale: Option<Locale>,
    /// The Data folder the user's configuration file names, `None` when it names none: not the one
    /// in effect ([`ThisDevice::data_folder`]), which a flag, a variable or a higher file may set.
    pub data: Option<PathBuf>,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            on_start: OnStart::default(),
            start_view: StartView::default(),
            toasts_on: lib_toast::Display::default().toasts_on,
            locale: None,
            data: None,
        }
    }
}

/// What start-up knows about this device: the Data folder in effect (the `data` setting, from
/// wherever it was set), and, shown read-only, the Ledger On start would reopen and the
/// configuration file Settings writes to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThisDevice {
    pub data_folder: PathBuf,
    /// The Ledger file open now, which the Client remembers at quit; `None` with none open.
    pub last_ledger: Option<PathBuf>,
    /// The user's configuration file; `None` with no home directory to keep one in.
    pub configuration: Option<PathBuf>,
}

/// The Locale in effect as the Client describes it, with the note when the one asked for wasn't
/// supported. The Locale is Configuration read at start-up (ADR-0021) and applies only at the next
/// start (ADR-0057), so this run's stays as it began.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleLine {
    pub line: String,
    pub fallback: Option<String>,
    /// The Locale this run reads.
    pub effective: Locale,
    /// The Locale **System** resolves to on this computer.
    pub system: Locale,
}

impl Default for LocaleLine {
    fn default() -> Self {
        Self {
            line: String::new(),
            fallback: None,
            effective: Locale::DEFAULT,
            system: Locale::DEFAULT,
        }
    }
}

/// What the Client hands General as it builds it.
pub struct GeneralSetup {
    /// Where the settings are written; the Client swaps in its configuration file's once start-up
    /// has read it ([`GeneralPresenter::configure`]).
    pub device: Rc<dyn DeviceConfiguration>,
    pub keymap: Keymap<GeneralEvent>,
    /// The chord of the Shell's `back`, for the legend.
    pub back_chord: String,
    /// Start view's choices, labelled, in the order the radio group shows them: the Client's own
    /// Views, since each Client lands on its own.
    pub start_views: Vec<Choice<StartView>>,
    pub locale: LocaleLine,
}

/// The THIS DEVICE box: its heading, its `(label, value)` rows and the page's footnote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThisDeviceView {
    pub heading: String,
    pub rows: Vec<(String, String)>,
    pub note: String,
}

/// Start view's dropdown: the choices, the one held (`None` for a value from the file that isn't a
/// choice), its list, and the line naming that value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartViewControl {
    pub control: ControlView<StartView>,
    pub list: SelectList,
    pub unlisted: Option<String>,
}

/// The Locale's dropdown: the choices with the file's value chosen, its list, each choice as the
/// dropdown shows it, its label then its code (`System · now en-AU`), and the note while the
/// file's Locale isn't the one in effect or a source above the file overrides it.
/// `control.shadowed` stays `None`: the note says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleControl {
    pub control: ControlView<Option<Locale>>,
    pub list: SelectList,
    /// Each option's label with its code, in the options' order.
    pub lines: Vec<String>,
    pub note: Option<String>,
    /// The note when the Locale asked for wasn't supported.
    pub fallback: Option<String>,
}

/// The Data folder: the folder the file names (else the one in effect), the **Change…** button with
/// the key that presses it, the hint, and the note while a source above the file sets it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFolderView {
    pub label: String,
    pub value: String,
    pub change: Hint,
    pub focused: bool,
    pub hint: String,
    pub note: Option<String>,
}

/// General as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralViewModel {
    /// The control the keyboard is on, by its `GENERAL_FIELD_*` index.
    pub field: Option<usize>,
    pub on_start: ControlView<OnStart>,
    pub start_view: StartViewControl,
    pub toasts: ControlView<bool>,
    /// The **Test** button beside Toasts: its label and the key that presses it from Toasts.
    pub toasts_test: Hint,
    pub locale: LocaleControl,
    pub data_folder: DataFolderView,
    pub this_device: ThisDeviceView,
    /// The page's keys, for the status line and the `?` cheat sheet.
    pub hints: Vec<Hint>,
}

/// Settings › General's Presenter.
pub struct GeneralPresenter {
    device: Rc<dyn DeviceConfiguration>,
    keymap: Keymap<GeneralEvent>,
    back_chord: String,
    start_views: Vec<Choice<StartView>>,
    locale: LocaleLine,
    settings: GeneralSettings,
    this_device: ThisDevice,
    field: Option<usize>,
    page_focused: bool,
    start_view_list: SelectList,
    locale_list: SelectList,
}

impl GeneralPresenter {
    /// General on the defaults, until the Client tells it what start-up read.
    pub fn new(setup: GeneralSetup) -> Self {
        Self {
            device: setup.device,
            keymap: setup.keymap,
            back_chord: setup.back_chord,
            start_views: setup.start_views,
            locale: setup.locale,
            settings: GeneralSettings::default(),
            this_device: ThisDevice::default(),
            field: None,
            page_focused: false,
            start_view_list: SelectList::default(),
            locale_list: SelectList::default(),
        }
    }

    /// Replaces the keymap and the `back` chord, for a `[keybindings]` change.
    pub fn set_keymap(&mut self, keymap: Keymap<GeneralEvent>, back_chord: String) {
        self.keymap = keymap;
        self.back_chord = back_chord;
    }

    /// Writes to `device` from now on, and shows `settings` as start-up read them.
    pub fn configure(&mut self, device: Rc<dyn DeviceConfiguration>, settings: GeneralSettings) {
        self.device = device;
        self.settings = settings;
    }

    pub fn set_this_device(&mut self, this_device: ThisDevice) {
        self.this_device = this_device;
    }

    /// The Ledger file the Client will remember at quit, once one has opened or closed.
    pub fn set_last_ledger(&mut self, last_ledger: Option<PathBuf>) {
        self.this_device.last_ledger = last_ledger;
    }

    /// Whether the page, not Settings' index, has the keyboard. Arriving puts it on On start, and
    /// leaving takes it off the controls; only a change moves it. Answers whether it changed, so
    /// the View redraws only then: the frame tells the page on every draw.
    pub fn set_page_focused(&mut self, focused: bool) -> bool {
        if focused == self.page_focused {
            return false;
        }
        self.page_focused = focused;
        self.field = focused.then_some(GENERAL_FIELD_ON_START);
        self.close_lists();
        true
    }

    pub fn settings(&self) -> &GeneralSettings {
        &self.settings
    }

    pub fn this_device(&self) -> &ThisDevice {
        &self.this_device
    }

    /// The Locale in effect as the Client described it at start-up.
    pub fn locale(&self) -> &LocaleLine {
        &self.locale
    }

    /// The Data folder in effect, where `:open` and `:new` start.
    pub fn data_folder(&self) -> &Path {
        &self.this_device.data_folder
    }
}

impl Presenter for GeneralPresenter {
    type Event = GeneralEvent;
    type ViewModel = GeneralViewModel;
    type Router<'r> = dyn DeviceRouter + 'r;

    fn handle(&mut self, event: GeneralEvent, router: &mut Self::Router<'_>) {
        if !self.takes(&event) {
            return;
        }
        // Which list was open, so a second click on its field closes it rather than reopening it.
        let (start_view_was_open, locale_was_open) =
            (self.start_view_list.is_open(), self.locale_list.is_open());
        if self.list_open() {
            match event {
                GeneralEvent::Move(step) => return self.move_highlight(step),
                // `h`/`l` wait while a list is open, rather than step the value under it.
                GeneralEvent::Change(_) => return,
                GeneralEvent::Select => return self.pick_highlight(router),
                // Another click, a picked row or a Command closes the open list first.
                _ => {
                    self.close_lists();
                }
            }
        }
        match event {
            GeneralEvent::Move(step) => self.move_field(step),
            GeneralEvent::Change(step) => self.change(step, router),
            GeneralEvent::Select => match self.field {
                Some(GENERAL_FIELD_START_VIEW) => self.open_start_view(),
                Some(GENERAL_FIELD_LOCALE) => self.open_locale(),
                _ => router.pick_data_folder(),
            },
            // The open list closed above.
            GeneralEvent::CloseList => {}
            GeneralEvent::PressStartView => {
                if !start_view_was_open {
                    self.open_start_view();
                    self.focus_if_on_page(GENERAL_FIELD_START_VIEW);
                }
            }
            GeneralEvent::PressLocale => {
                if !locale_was_open {
                    self.open_locale();
                    self.focus_if_on_page(GENERAL_FIELD_LOCALE);
                }
            }
            GeneralEvent::ChooseOnStart(on_start) => self.choose_on_start(on_start, router),
            GeneralEvent::ChooseStartView(start_view) => {
                self.choose_start_view(start_view, router);
            }
            GeneralEvent::ChooseToasts(on) => {
                if on != self.settings.toasts_on {
                    self.write_toasts(on, router);
                }
            }
            GeneralEvent::SetToasts(on) => self.write_toasts(on, router),
            GeneralEvent::ChooseLocale(locale) => self.choose_locale(locale, router),
            // The test touches no setting: the Shell's Toasts raise it.
            GeneralEvent::TestToasts | GeneralEvent::PressTest => router.test_toasts(),
            GeneralEvent::PressChange => router.pick_data_folder(),
            GeneralEvent::SetDataFolder(folder) => self.set_data_folder(folder, router),
        }
    }

    fn view_model(&self) -> GeneralViewModel {
        let device = self.device.as_ref();
        let listed = self
            .start_views
            .iter()
            .any(|choice| choice.value == self.settings.start_view);
        let path = |path: Option<&PathBuf>| {
            path.map_or_else(msg::settings_general_none, |path| {
                path.display().to_string()
            })
        };
        GeneralViewModel {
            field: self.field,
            on_start: ControlView {
                select: SelectView {
                    label: msg::settings_general_on_start(),
                    options: ON_START_CHOICES
                        .into_iter()
                        .map(|value| Choice {
                            value,
                            label: on_start_label(value),
                        })
                        .collect(),
                    chosen: Some(self.settings.on_start),
                    focused: self.field == Some(GENERAL_FIELD_ON_START),
                },
                hint: Some(msg::settings_general_on_start_hint()),
                shadowed: shadowed_note(device, DeviceKey::OnStart),
            },
            start_view: StartViewControl {
                control: ControlView {
                    select: SelectView {
                        label: msg::settings_general_start_view(),
                        options: self.start_views.clone(),
                        chosen: listed.then(|| self.settings.start_view.clone()),
                        focused: self.field == Some(GENERAL_FIELD_START_VIEW),
                    },
                    hint: Some(msg::settings_general_start_view_hint()),
                    shadowed: shadowed_note(device, DeviceKey::StartView),
                },
                list: self.start_view_list,
                unlisted: (!listed).then(|| {
                    msg::settings_general_start_view_unlisted(self.settings.start_view.key())
                }),
            },
            toasts: ControlView {
                select: SelectView {
                    label: lib_locale::msg::toast_setting_label(),
                    options: TOASTS_CHOICES
                        .into_iter()
                        .map(|value| Choice {
                            value,
                            label: if value {
                                lib_locale::msg::toast_setting_on()
                            } else {
                                lib_locale::msg::toast_setting_off()
                            },
                        })
                        .collect(),
                    chosen: Some(self.settings.toasts_on),
                    focused: self.field == Some(GENERAL_FIELD_TOASTS),
                },
                hint: Some(lib_locale::msg::toast_setting_test_note()),
                shadowed: shadowed_note(device, DeviceKey::Toasts),
            },
            toasts_test: Hint::new(
                chords(&self.keymap, &[TEST_TOASTS]),
                lib_locale::msg::toast_setting_test(),
            ),
            locale: self.locale_control(),
            data_folder: DataFolderView {
                label: msg::settings_general_data_folder(),
                value: self.shown_data_folder().display().to_string(),
                change: Hint::new(
                    chords(&self.keymap, &[SELECT]),
                    msg::settings_general_data_folder_change(),
                ),
                focused: self.field == Some(GENERAL_FIELD_DATA_FOLDER),
                hint: msg::settings_general_data_folder_hint(),
                note: self.device.shadow(DeviceKey::Data).map(|source| {
                    msg::settings_general_data_folder_overridden(
                        &source,
                        &self.this_device.data_folder.display().to_string(),
                    )
                }),
            },
            this_device: ThisDeviceView {
                heading: msg::settings_general_this_device(),
                rows: vec![
                    (
                        msg::settings_general_last_ledger(),
                        path(self.this_device.last_ledger.as_ref()),
                    ),
                    (
                        msg::settings_general_configuration(),
                        path(self.this_device.configuration.as_ref()),
                    ),
                ],
                note: msg::settings_general_note(),
            },
            hints: self.hints(),
        }
    }

    /// Whether General takes `event` as things stand, so a key it declines goes on to the frame:
    /// a move or change only while a control has the keyboard, and every list key while a list is
    /// open, so neither the frame's `h` nor the Shell's `back` sees one.
    fn takes(&self, event: &GeneralEvent) -> bool {
        let open = self.list_open();
        match event {
            GeneralEvent::Move(_) | GeneralEvent::Change(_) => open || self.field.is_some(),
            GeneralEvent::TestToasts => !open && self.field == Some(GENERAL_FIELD_TOASTS),
            GeneralEvent::Select => {
                open || matches!(
                    self.field,
                    Some(
                        GENERAL_FIELD_START_VIEW | GENERAL_FIELD_LOCALE | GENERAL_FIELD_DATA_FOLDER
                    )
                )
            }
            GeneralEvent::CloseList => open,
            _ => true,
        }
    }
}

impl PagePresenter for GeneralPresenter {
    /// General opens no Dialog.
    type Outcome = Infallible;

    /// General opens no Dialog, so there is nothing to tell it.
    fn dialog_closed(_outcome: Option<Infallible>) -> Option<GeneralEvent> {
        None
    }

    fn keymap(&self) -> &Keymap<GeneralEvent> {
        &self.keymap
    }
}

impl GeneralPresenter {
    /// The page's keys: an open list's own while one is open; on Toasts, `h`/`l` read on / off and
    /// `t` raises the test; on Start view and the Locale, `enter` opens the list beside `h`/`l`; on
    /// the Data folder, `enter` changes it in place of `h`/`l`.
    fn hints(&self) -> Vec<Hint> {
        if self.list_open() {
            return vec![
                Hint::new(chords(&self.keymap, &[SELECT]), msg::settings_hint_pick()),
                Hint::new(
                    chords(&self.keymap, &[MOVE_DOWN, MOVE_UP]),
                    msg::settings_hint_move(),
                ),
                Hint::new(self.back_chord.clone(), msg::settings_hint_close()),
            ];
        }
        let on_toasts = self.field == Some(GENERAL_FIELD_TOASTS);
        let field = Hint::new(
            chords(&self.keymap, &[MOVE_DOWN, MOVE_UP]),
            msg::settings_hint_field(),
        );
        if self.field == Some(GENERAL_FIELD_DATA_FOLDER) {
            return vec![
                field,
                Hint::new(chords(&self.keymap, &[SELECT]), msg::settings_hint_change()),
            ];
        }
        let mut hints = vec![
            field,
            Hint::new(
                chords(&self.keymap, &[PREVIOUS_CHOICE, NEXT_CHOICE]),
                if on_toasts {
                    msg::settings_hint_on_off()
                } else {
                    msg::settings_hint_change()
                },
            ),
        ];
        if on_toasts {
            hints.push(Hint::new(
                chords(&self.keymap, &[TEST_TOASTS]),
                msg::settings_hint_test(),
            ));
        }
        if matches!(
            self.field,
            Some(GENERAL_FIELD_START_VIEW | GENERAL_FIELD_LOCALE)
        ) {
            hints.push(Hint::new(
                chords(&self.keymap, &[SELECT]),
                msg::settings_hint_open(),
            ));
        }
        hints
    }

    /// Whether Start view's or the Locale's list is open.
    pub fn list_open(&self) -> bool {
        self.start_view_list.is_open() || self.locale_list.is_open()
    }

    fn close_lists(&mut self) {
        self.start_view_list.close();
        self.locale_list.close();
    }

    /// A click on a dropdown puts the keyboard on it, while the page has the keyboard, so its list
    /// keys follow.
    fn focus_if_on_page(&mut self, field: usize) {
        if self.field.is_some() {
            self.field = Some(field);
        }
    }

    /// The Start view held, `None` for a value from the file that isn't a choice.
    fn start_view_chosen(&self) -> Option<&StartView> {
        self.start_views
            .iter()
            .any(|choice| choice.value == self.settings.start_view)
            .then_some(&self.settings.start_view)
    }

    fn open_start_view(&mut self) {
        self.locale_list.close();
        let chosen = self.start_view_chosen().cloned();
        self.start_view_list
            .open(&self.start_views, chosen.as_ref());
    }

    fn open_locale(&mut self) {
        self.start_view_list.close();
        self.locale_list
            .open(&locale_options(), Some(&self.settings.locale));
    }

    /// `j`/`k` on an open list: its highlight, wrapping at either end.
    fn move_highlight(&mut self, step: Step) {
        let input = match step {
            Step::Next => DialogInput::SelectNext,
            Step::Prev => DialogInput::SelectPrevious,
            _ => return,
        };
        if self.start_view_list.is_open() {
            let chosen = self.start_view_chosen().cloned();
            self.start_view_list
                .input(input, &self.start_views, chosen.as_ref());
        } else {
            let chosen = self.settings.locale;
            self.locale_list
                .input(input, &locale_options(), Some(&chosen));
        }
    }

    /// `enter` on an open list: picks its highlight and closes it.
    fn pick_highlight(&mut self, router: &mut dyn DeviceRouter) {
        if let Some(start_view) = self.start_view_list.commit(&self.start_views) {
            self.choose_start_view(start_view, router);
        } else if let Some(locale) = self.locale_list.commit(&locale_options()) {
            self.choose_locale(locale, router);
        }
    }

    /// `j`/`k` walk the controls, stopping at either end.
    fn move_field(&mut self, step: Step) {
        if let Some(field) = self.field {
            self.field = Some(lib_list_cursor::step(field, GENERAL_FIELD_COUNT, step));
        }
    }

    /// `h`/`l` step the focused control's choice.
    fn change(&mut self, step: Step, router: &mut dyn DeviceRouter) {
        let Some(field) = self.field else {
            return;
        };
        match field {
            GENERAL_FIELD_ON_START => {
                let current = self.settings.on_start;
                if let Some(next) = step_choice(&ON_START_CHOICES, &current, &current, step) {
                    self.choose_on_start(next, router);
                }
            }
            GENERAL_FIELD_START_VIEW => {
                let choices: Vec<StartView> = self
                    .start_views
                    .iter()
                    .map(|choice| choice.value.clone())
                    .collect();
                // A noun from the file that isn't a choice steps from the Dashboard.
                if let Some(next) = step_choice(
                    &choices,
                    &self.settings.start_view,
                    &StartView::Dashboard,
                    step,
                ) {
                    self.choose_start_view(next, router);
                }
            }
            GENERAL_FIELD_LOCALE => {
                let current = self.settings.locale;
                if let Some(next) = step_choice(&locale_choices(), &current, &None, step) {
                    self.choose_locale(next, router);
                }
            }
            // A path has no choices to step through: `enter` opens the picker.
            GENERAL_FIELD_DATA_FOLDER => {}
            _ => {
                let current = self.settings.toasts_on;
                if let Some(on) = step_choice(&TOASTS_CHOICES, &current, &current, step)
                    && on != current
                {
                    self.write_toasts(on, router);
                }
            }
        }
    }

    fn choose_on_start(&mut self, on_start: OnStart, router: &mut dyn DeviceRouter) {
        if on_start == self.settings.on_start {
            return;
        }
        let answer = self.device.set_on_start(on_start);
        if let Some(saved) = settle(answer, DeviceKey::OnStart, router) {
            self.settings.on_start = saved;
        }
    }

    /// Start view takes effect at the next start, which reads it from the file, so nothing else
    /// follows the write.
    fn choose_start_view(&mut self, start_view: StartView, router: &mut dyn DeviceRouter) {
        if start_view == self.settings.start_view {
            return;
        }
        let answer = self.device.set_start_view(start_view);
        if let Some(saved) = settle(answer, DeviceKey::StartView, router) {
            self.settings.start_view = saved;
        }
    }

    /// The Locale takes effect at the next start, so nothing else follows the write; it is written
    /// even while shadowed.
    fn choose_locale(&mut self, locale: Option<Locale>, router: &mut dyn DeviceRouter) {
        if locale == self.settings.locale {
            return;
        }
        let answer = self.device.set_locale(locale);
        if let Some(saved) = settle(answer, DeviceKey::Locale, router) {
            self.settings.locale = saved;
        }
    }

    fn locale_control(&self) -> LocaleControl {
        let options = locale_options();
        let lines = options
            .iter()
            .map(|choice| {
                let code = match choice.value {
                    None => msg::settings_general_locale_system_now(self.locale.system.tag()),
                    Some(locale) => locale.tag().to_string(),
                };
                msg::settings_general_locale_option(&choice.label, &code)
            })
            .collect();
        LocaleControl {
            list: self.locale_list,
            control: ControlView {
                select: SelectView {
                    label: msg::settings_general_locale_label(),
                    options,
                    chosen: Some(self.settings.locale),
                    focused: self.field == Some(GENERAL_FIELD_LOCALE),
                },
                hint: Some(msg::settings_general_locale_hint()),
                shadowed: None,
            },
            lines,
            note: self.locale_note(),
            fallback: self.locale.fallback.clone(),
        }
    }

    /// The override, while a source above the file sets the Locale; else the pending Locale, while
    /// the one the file names isn't the one this run reads.
    fn locale_note(&self) -> Option<String> {
        let effective = self.locale.effective.tag();
        if let Some(source) = self.device.shadow(DeviceKey::Locale) {
            return Some(msg::settings_general_locale_overridden(&source, effective));
        }
        let chosen = self.settings.locale.unwrap_or(self.locale.system);
        (chosen != self.locale.effective)
            .then(|| msg::settings_general_locale_pending(chosen.tag(), effective))
    }

    /// The Data folder the field shows: the file's, else the one in effect.
    fn shown_data_folder(&self) -> &Path {
        self.settings
            .data
            .as_deref()
            .unwrap_or(&self.this_device.data_folder)
    }

    /// Writes `folder` as the Data folder, unless the field shows it already. It applies at once,
    /// with a Success Toast in place of the status line's word; under an override it is written
    /// for when the override is gone, and the folder in effect stays as it was.
    fn set_data_folder(&mut self, folder: PathBuf, router: &mut dyn DeviceRouter) {
        if folder == self.shown_data_folder() {
            return;
        }
        let shadowed = self.device.shadow(DeviceKey::Data).is_some();
        match self.device.set_data(&folder) {
            DeviceAnswer::Saved { value, .. } if !shadowed => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::settings_general_data_folder_now(&value.display().to_string()),
                ));
                self.this_device.data_folder = value.clone();
                self.settings.data = Some(value);
            }
            answer => {
                if let Some(saved) = settle(answer, DeviceKey::Data, router) {
                    self.settings.data = Some(saved);
                }
            }
        }
    }

    fn write_toasts(&mut self, on: bool, router: &mut dyn DeviceRouter) {
        let answer = self.device.set_toasts(on);
        if let Some(saved) = settle(answer, DeviceKey::Toasts, router) {
            self.settings.toasts_on = saved;
            router.toasts_changed(saved);
        }
    }
}

/// The Locale's choices, labelled.
fn locale_options() -> Vec<Choice<Option<Locale>>> {
    locale_choices()
        .into_iter()
        .map(|value| Choice {
            value,
            label: locale_label(value),
        })
        .collect()
}

/// A Locale choice's label, **System** for `None`.
fn locale_label(choice: Option<Locale>) -> String {
    match choice {
        None => msg::settings_general_locale_system(),
        Some(Locale::EnAu) => msg::settings_general_locale_en_au(),
        Some(Locale::EnGb) => msg::settings_general_locale_en_gb(),
        Some(Locale::EnUs) => msg::settings_general_locale_en_us(),
        // Never offered; its tag is all there is to call it.
        Some(Locale::EnXa) => Locale::EnXa.tag().to_string(),
    }
}

pub(crate) fn on_start_label(choice: OnStart) -> String {
    match choice {
        OnStart::ReopenLastLedger => msg::settings_general_on_start_reopen(),
        OnStart::NoLedger => msg::settings_general_on_start_none(),
    }
}

#[cfg(test)]
mod tests {
    use lib_toast::Notice;
    use lib_viper::keymap::KeyBindingConfig;

    use super::*;
    use crate::fake::{InMemoryDeviceConfiguration, RecordingRouter, Routed, StoredDevice};
    use crate::keymap::general_keymap;

    fn start_views() -> Vec<Choice<StartView>> {
        ["dashboard", "last", "settings", "bills"]
            .into_iter()
            .map(|key| Choice {
                value: StartView::from_key(key),
                label: key.to_string(),
            })
            .collect()
    }

    fn general_on(device: Rc<InMemoryDeviceConfiguration>) -> GeneralPresenter {
        general_reading(device, Locale::EnAu)
    }

    /// General on `device` in a run reading `effective`, with en-AU the system's Locale.
    fn general_reading(
        device: Rc<InMemoryDeviceConfiguration>,
        effective: Locale,
    ) -> GeneralPresenter {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        let mut general = GeneralPresenter::new(GeneralSetup {
            device: device.clone(),
            keymap: general_keymap(&KeyBindingConfig::default()),
            back_chord: "esc".to_string(),
            start_views: start_views(),
            locale: LocaleLine {
                line: "en-AU, from config".to_string(),
                fallback: None,
                effective,
                system: Locale::EnAu,
            },
        });
        general.configure(device, GeneralSettings::default());
        general
    }

    fn writable() -> (Rc<InMemoryDeviceConfiguration>, GeneralPresenter) {
        let device = Rc::new(InMemoryDeviceConfiguration::writable());
        (device.clone(), general_on(device))
    }

    fn send(
        general: &mut GeneralPresenter,
        events: impl IntoIterator<Item = GeneralEvent>,
    ) -> RecordingRouter {
        let mut router = RecordingRouter::default();
        for event in events {
            general.handle(event, &mut router);
        }
        router
    }

    #[test]
    fn focus_arrives_on_on_start_and_leaves_the_controls() {
        let (_, mut general) = writable();
        assert_eq!(general.view_model().field, None);
        assert!(!general.takes(&GeneralEvent::Move(Step::Next)));

        general.set_page_focused(true);
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_ON_START));
        assert!(general.view_model().on_start.select.focused);
        general.set_page_focused(false);
        assert_eq!(general.view_model().field, None);
    }

    #[test]
    fn telling_the_page_its_focus_again_is_no_change() {
        let (_, mut general) = writable();
        assert!(general.set_page_focused(true));
        assert!(!general.set_page_focused(true));
        assert!(general.set_page_focused(false));
        assert!(!general.set_page_focused(false));
    }

    #[test]
    fn j_and_k_walk_the_five_controls_and_stop_at_the_ends() {
        let (_, mut general) = writable();
        general.set_page_focused(true);

        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Move(Step::Next), 2),
        );
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_TOASTS));
        send(&mut general, [GeneralEvent::Move(Step::Next)]);
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_LOCALE));
        assert!(general.view_model().locale.control.select.focused);
        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Move(Step::Next), 2),
        );
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_DATA_FOLDER));
        assert!(general.view_model().data_folder.focused);
        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Move(Step::Prev), 5),
        );
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_ON_START));
        assert!(
            !general.takes(&GeneralEvent::Select),
            "enter is the frame's off the Data folder"
        );
    }

    #[test]
    fn on_start_is_saved_then_shown_and_the_file_named() {
        let (device, mut general) = writable();
        general.set_page_focused(true);

        let router = send(&mut general, [GeneralEvent::Change(Step::Next)]);

        assert_eq!(general.settings().on_start, OnStart::NoLedger);
        assert_eq!(device.stored().on_start, Some(OnStart::NoLedger));
        assert_eq!(
            router.statuses(),
            ["on_start saved to personal-ledger.conf"]
        );
        let router = send(&mut general, [GeneralEvent::Change(Step::Next)]);
        assert!(router.routed.is_empty(), "the last choice stays, unwritten");
    }

    #[test]
    fn a_shadowed_setting_is_noted_and_left_unwritten() {
        let device = Rc::new(InMemoryDeviceConfiguration::shadowing(
            DeviceKey::OnStart,
            "PERSONAL_LEDGER_GENERAL__ON_START",
        ));
        let mut general = general_on(device.clone());
        assert_eq!(
            general.view_model().on_start.shadowed.as_deref(),
            Some("Set by PERSONAL_LEDGER_GENERAL__ON_START")
        );
        assert_eq!(general.view_model().toasts.shadowed, None);

        let router = send(
            &mut general,
            [GeneralEvent::ChooseOnStart(OnStart::NoLedger)],
        );

        assert_eq!(general.settings().on_start, OnStart::ReopenLastLedger);
        assert_eq!(device.stored(), Default::default());
        assert!(router.statuses()[0].contains("PERSONAL_LEDGER_GENERAL__ON_START"));
    }

    #[test]
    fn a_refused_write_says_why_and_changes_nothing() {
        let mut general = general_on(Rc::new(InMemoryDeviceConfiguration::default()));
        let router = send(
            &mut general,
            [GeneralEvent::ChooseStartView(StartView::Last)],
        );
        assert_eq!(general.settings().start_view, StartView::Dashboard);
        assert!(router.statuses()[0].contains("start_view can't be saved"));

        let failing = Rc::new(InMemoryDeviceConfiguration {
            fail_writes: true,
            ..InMemoryDeviceConfiguration::writable()
        });
        let mut general = general_on(failing);
        let router = send(&mut general, [GeneralEvent::ChooseToasts(false)]);
        assert!(general.settings().toasts_on);
        assert_eq!(
            router.routed,
            [Routed::Notice(Notice::Status(
                "Couldn't save toasts to personal-ledger.conf: the log says why".to_string()
            ))],
            "the Toasts aren't told"
        );
    }

    #[test]
    fn start_view_steps_through_its_choices_and_an_unlisted_one_steps_from_the_dashboard() {
        let (device, mut general) = writable();
        general.configure(
            device.clone(),
            GeneralSettings {
                start_view: StartView::Noun("units".into()),
                ..GeneralSettings::default()
            },
        );
        let view = general.view_model().start_view;
        assert_eq!(view.control.select.chosen, None);
        assert!(view.unlisted.is_some_and(|line| line.contains("units")));

        general.set_page_focused(true);
        send(
            &mut general,
            [
                GeneralEvent::Move(Step::Next),
                GeneralEvent::Change(Step::Next),
            ],
        );
        assert_eq!(general.settings().start_view, StartView::Last);
        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Change(Step::Next), 5),
        );
        assert_eq!(
            general.settings().start_view,
            StartView::Noun("bills".into())
        );
        assert_eq!(
            device.stored().start_view,
            Some(StartView::Noun("bills".into()))
        );
    }

    #[test]
    fn saved_toasts_reach_the_shell_and_the_command_writes_even_when_they_hold() {
        let (_, mut general) = writable();
        let router = send(&mut general, [GeneralEvent::ChooseToasts(false)]);
        assert!(!general.settings().toasts_on);
        assert!(router.routed.contains(&Routed::ToastsChanged(false)));

        let router = send(&mut general, [GeneralEvent::ChooseToasts(false)]);
        assert!(
            router.routed.is_empty(),
            "a click on the choice held does nothing"
        );
        let router = send(&mut general, [GeneralEvent::SetToasts(false)]);
        assert!(router.routed.contains(&Routed::ToastsChanged(false)));
    }

    #[test]
    fn t_on_toasts_asks_for_the_test_and_touches_no_setting() {
        let (device, mut general) = writable();
        general.set_page_focused(true);
        let router = send(
            &mut general,
            [
                GeneralEvent::Move(Step::Next),
                GeneralEvent::Move(Step::Next),
            ],
        );
        assert!(router.routed.is_empty());
        assert_eq!(general.keymap().event("t"), Some(GeneralEvent::TestToasts));

        let router = send(&mut general, [GeneralEvent::TestToasts]);

        assert_eq!(router.routed, [Routed::TestToasts]);
        assert_eq!(device.stored(), Default::default());
        assert!(general.settings().toasts_on);
    }

    #[test]
    fn t_on_another_field_does_nothing() {
        let (_, mut general) = writable();
        assert!(
            !general.takes(&GeneralEvent::TestToasts),
            "no field focused"
        );
        general.set_page_focused(true);
        assert!(!general.takes(&GeneralEvent::TestToasts), "on On start");
        let router = send(
            &mut general,
            [GeneralEvent::Move(Step::Next), GeneralEvent::TestToasts],
        );
        assert!(router.routed.is_empty(), "on Start view");
    }

    #[test]
    fn a_click_on_test_raises_it_wherever_the_keyboard_is() {
        let (_, mut general) = writable();
        let router = send(&mut general, [GeneralEvent::PressTest]);
        assert_eq!(router.routed, [Routed::TestToasts]);
    }

    #[test]
    fn on_toasts_the_hints_read_on_off_and_test() {
        let (_, mut general) = writable();
        general.set_page_focused(true);
        assert_eq!(pairs(&general)[1].1, "change");
        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Move(Step::Next), 2),
        );
        assert_eq!(
            pairs(&general),
            [
                ("j/k".to_string(), "field".to_string()),
                ("h/l".to_string(), "on / off".to_string()),
                ("t".to_string(), "test".to_string()),
            ]
        );
        assert_eq!(
            general.view_model().toasts_test,
            Hint::new("t".to_string(), "Test".to_string())
        );
        assert_eq!(
            general.view_model().toasts.hint.as_deref(),
            Some("Errors still show. Test raises one of each kind.")
        );
    }

    fn on_locale(general: &mut GeneralPresenter) {
        general.set_page_focused(true);
        send(
            general,
            std::iter::repeat_n(GeneralEvent::Move(Step::Next), 3),
        );
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_LOCALE));
    }

    #[test]
    fn the_locale_offers_system_then_the_supported_locales_never_the_pseudo_locale() {
        let (_, general) = writable();
        let view = general.view_model().locale;

        let labels: Vec<_> = view
            .control
            .select
            .options
            .iter()
            .map(|choice| choice.label.as_str())
            .collect();
        assert_eq!(
            labels,
            [
                "System",
                "English (Australia)",
                "English (United Kingdom)",
                "English (United States)"
            ]
        );
        assert_eq!(
            view.lines,
            [
                "System · now en-AU",
                "English (Australia) · en-AU",
                "English (United Kingdom) · en-GB",
                "English (United States) · en-US"
            ]
        );
        assert_eq!(
            view.control.select.chosen,
            Some(None),
            "System, with no key"
        );
        assert_eq!(view.note, None);
    }

    #[test]
    fn l_steps_the_locale_and_each_choice_is_written_with_no_toast() {
        let (device, mut general) = writable();
        on_locale(&mut general);

        let router = send(&mut general, [GeneralEvent::Change(Step::Next)]);

        assert_eq!(general.settings().locale, Some(Locale::EnAu));
        assert_eq!(device.stored().locale, Some(Locale::EnAu));
        assert_eq!(router.statuses(), ["locale saved to personal-ledger.conf"]);
        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Change(Step::Next), 3),
        );
        assert_eq!(
            device.stored().locale,
            Some(Locale::EnUs),
            "stops at the end"
        );
    }

    #[test]
    fn choosing_system_takes_the_locale_out_of_the_file() {
        let (device, mut general) = writable();
        send(
            &mut general,
            [GeneralEvent::ChooseLocale(Some(Locale::EnGb))],
        );
        assert_eq!(device.stored().locale, Some(Locale::EnGb));

        send(&mut general, [GeneralEvent::ChooseLocale(None)]);

        assert_eq!(device.stored().locale, None);
        assert_eq!(general.settings().locale, None);
        assert_eq!(
            general.view_model().locale.control.select.chosen,
            Some(None)
        );
    }

    #[test]
    fn a_locale_other_than_the_one_in_effect_is_noted_until_the_next_start() {
        let (_, mut general) = writable();

        send(
            &mut general,
            [GeneralEvent::ChooseLocale(Some(Locale::EnUs))],
        );
        assert_eq!(
            general.view_model().locale.note.as_deref(),
            Some(
                "Saved. en-US applies when Personal Ledger next starts; until then it reads en-AU."
            )
        );

        send(
            &mut general,
            [GeneralEvent::ChooseLocale(Some(Locale::EnAu))],
        );
        assert_eq!(general.view_model().locale.note, None, "en-AU is in effect");
        send(&mut general, [GeneralEvent::ChooseLocale(None)]);
        assert_eq!(
            general.view_model().locale.note,
            None,
            "System reads en-AU on this computer"
        );
    }

    #[test]
    fn a_shadowed_locale_is_still_written_and_names_the_override() {
        let device = Rc::new(InMemoryDeviceConfiguration::shadowing(
            DeviceKey::Locale,
            "--locale en-GB",
        ));
        let mut general = general_reading(device.clone(), Locale::EnGb);
        let overridden = "--locale en-GB overrides this for this run, so Personal Ledger reads \
                          en-GB. Your choice is kept for when it's gone.";
        let view = general.view_model().locale;
        assert_eq!(view.note.as_deref(), Some(overridden));
        assert_eq!(view.control.shadowed, None, "the note says it, not Set by");
        assert_eq!(view.control.select.chosen, Some(None), "the file's value");

        let router = send(
            &mut general,
            [GeneralEvent::ChooseLocale(Some(Locale::EnUs))],
        );

        assert_eq!(device.stored().locale, Some(Locale::EnUs));
        assert_eq!(general.settings().locale, Some(Locale::EnUs));
        assert_eq!(router.statuses(), ["locale saved to personal-ledger.conf"]);
        assert_eq!(
            general.view_model().locale.note.as_deref(),
            Some(overridden)
        );
    }

    /// General on `device` with the Data folder in effect `/home/me/ledgers`.
    fn with_data_folder(device: Rc<InMemoryDeviceConfiguration>) -> GeneralPresenter {
        let mut general = general_on(device);
        general.set_this_device(ThisDevice {
            data_folder: PathBuf::from("/home/me/ledgers"),
            ..ThisDevice::default()
        });
        general
    }

    #[test]
    fn enter_on_the_data_folder_and_change_ask_for_the_picker() {
        let mut general = with_data_folder(Rc::new(InMemoryDeviceConfiguration::writable()));
        general.set_page_focused(true);
        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Move(Step::Next), 4),
        );
        let view = general.view_model();
        assert_eq!(view.data_folder.label, "Data folder");
        assert_eq!(view.data_folder.value, "/home/me/ledgers");
        assert_eq!(
            (
                view.data_folder.change.chord,
                view.data_folder.change.action
            ),
            ("enter".to_string(), "Change…".to_string())
        );
        assert_eq!(
            view.hints
                .iter()
                .map(|hint| (hint.chord.as_str(), hint.action.as_str()))
                .collect::<Vec<_>>(),
            [("j/k", "field"), ("enter", "change")]
        );

        let router = send(
            &mut general,
            [GeneralEvent::Select, GeneralEvent::PressChange],
        );

        assert_eq!(
            router.routed,
            [Routed::PickDataFolder, Routed::PickDataFolder]
        );
    }

    #[test]
    fn a_picked_data_folder_is_written_applies_at_once_and_raises_a_toast() {
        let device = Rc::new(InMemoryDeviceConfiguration::writable());
        let mut general = with_data_folder(device.clone());

        let router = send(
            &mut general,
            [GeneralEvent::SetDataFolder(PathBuf::from(
                "/home/me/finance",
            ))],
        );

        assert_eq!(
            device.stored().data,
            Some(PathBuf::from("/home/me/finance"))
        );
        assert_eq!(general.data_folder(), Path::new("/home/me/finance"));
        assert_eq!(
            general.settings().data,
            Some(PathBuf::from("/home/me/finance"))
        );
        assert_eq!(general.view_model().data_folder.value, "/home/me/finance");
        assert_eq!(
            router.routed,
            [Routed::Notice(Notice::Toast(
                ToastKind::Success,
                "Data folder is now /home/me/finance".to_string()
            ))]
        );
    }

    #[test]
    fn picking_the_data_folder_already_set_writes_nothing_and_raises_nothing() {
        let device = Rc::new(InMemoryDeviceConfiguration::writable());
        let mut general = with_data_folder(device.clone());

        let router = send(
            &mut general,
            [GeneralEvent::SetDataFolder(PathBuf::from(
                "/home/me/ledgers",
            ))],
        );

        assert_eq!(device.stored(), StoredDevice::default());
        assert!(router.routed.is_empty());
    }

    #[test]
    fn a_refused_data_folder_says_why_and_changes_nothing() {
        let device = Rc::new(InMemoryDeviceConfiguration::default());
        let mut general = with_data_folder(device);

        let router = send(
            &mut general,
            [GeneralEvent::SetDataFolder(PathBuf::from(
                "/home/me/finance",
            ))],
        );

        assert_eq!(general.data_folder(), Path::new("/home/me/ledgers"));
        assert_eq!(router.statuses().len(), 1);
        assert!(router.statuses()[0].starts_with("data can't be saved"));
    }

    #[test]
    fn a_shadowed_data_folder_is_written_and_names_the_override() {
        let device = Rc::new(InMemoryDeviceConfiguration::shadowing(
            DeviceKey::Data,
            "PERSONAL_LEDGER_GENERAL__DATA",
        ));
        let mut general = with_data_folder(device.clone());
        let overridden = "PERSONAL_LEDGER_GENERAL__DATA sets the Data folder to /home/me/ledgers for \
                          now, so :open and :new start there. Change… keeps your choice for when \
                          it's gone.";
        assert_eq!(
            general.view_model().data_folder.note.as_deref(),
            Some(overridden)
        );

        let router = send(
            &mut general,
            [GeneralEvent::SetDataFolder(PathBuf::from(
                "/home/me/finance",
            ))],
        );

        assert_eq!(
            device.stored().data,
            Some(PathBuf::from("/home/me/finance"))
        );
        assert_eq!(
            general.data_folder(),
            Path::new("/home/me/ledgers"),
            "the override still decides the folder in effect"
        );
        let view = general.view_model().data_folder;
        assert_eq!(view.value, "/home/me/finance", "the file's value");
        assert_eq!(view.note.as_deref(), Some(overridden));
        assert_eq!(router.statuses(), ["data saved to personal-ledger.conf"]);
    }

    #[test]
    fn the_data_folder_and_this_device_show_what_the_client_told_it() {
        let (_, mut general) = writable();
        general.set_this_device(ThisDevice {
            data_folder: PathBuf::from("/home/me/Ledgers"),
            last_ledger: None,
            configuration: Some(PathBuf::from("/home/me/.config/personal-ledger.conf")),
        });
        general.set_last_ledger(Some(PathBuf::from("/home/me/Ledgers/home.ledger")));

        let view = general.view_model();
        assert_eq!(general.locale().line, "en-AU, from config");
        assert_eq!(view.data_folder.value, "/home/me/Ledgers");
        assert_eq!(
            view.this_device.rows,
            [
                (
                    "last ledger".to_string(),
                    "/home/me/Ledgers/home.ledger".to_string()
                ),
                (
                    "configuration".to_string(),
                    "/home/me/.config/personal-ledger.conf".to_string()
                ),
            ]
        );
    }

    #[test]
    fn the_legend_reads_its_chords_from_the_keymap() {
        let (_, mut general) = writable();
        assert_eq!(
            pairs(&general),
            [
                ("j/k".to_string(), "field".to_string()),
                ("h/l".to_string(), "change".to_string()),
            ],
            "the frame's esc is the only esc"
        );

        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(NEXT_CHOICE.to_string(), "ctrl+l".to_string());
        general.set_keymap(general_keymap(&config), "esc".to_string());
        assert_eq!(pairs(&general)[1].0, "h/ctrl+l");
    }

    fn pairs(general: &GeneralPresenter) -> Vec<(String, String)> {
        general
            .view_model()
            .hints
            .into_iter()
            .map(|hint| (hint.chord, hint.action))
            .collect()
    }

    /// General focused on Start view.
    fn on_start_view(general: &mut GeneralPresenter) {
        general.set_page_focused(true);
        send(general, [GeneralEvent::Move(Step::Next)]);
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_START_VIEW));
    }

    #[test]
    fn enter_opens_start_views_list_on_the_chosen_option_and_offers_open_while_closed() {
        let (device, mut general) = writable();
        on_start_view(&mut general);
        assert_eq!(
            pairs(&general),
            [
                ("j/k".to_string(), "field".to_string()),
                ("h/l".to_string(), "change".to_string()),
                ("enter".to_string(), "open".to_string()),
            ]
        );

        let router = send(&mut general, [GeneralEvent::Select]);

        let view = general.view_model();
        assert_eq!(view.start_view.list.highlight(), Some(0), "the Dashboard");
        assert!(router.routed.is_empty());
        assert_eq!(
            device.stored(),
            StoredDevice::default(),
            "opening writes nothing"
        );
        assert_eq!(
            pairs(&general),
            [
                ("enter".to_string(), "pick".to_string()),
                ("j/k".to_string(), "move".to_string()),
                ("esc".to_string(), "close".to_string()),
            ]
        );
    }

    #[test]
    fn j_and_k_move_the_open_lists_highlight_not_the_focus_and_enter_picks_it() {
        let (device, mut general) = writable();
        on_start_view(&mut general);
        send(
            &mut general,
            [
                GeneralEvent::Select,
                GeneralEvent::Move(Step::Next),
                GeneralEvent::Move(Step::Next),
                GeneralEvent::Move(Step::Prev),
            ],
        );
        let view = general.view_model();
        assert_eq!(view.field, Some(GENERAL_FIELD_START_VIEW));
        assert_eq!(view.start_view.list.highlight(), Some(1));
        assert_eq!(
            general.settings().start_view,
            StartView::Dashboard,
            "not yet"
        );

        let router = send(&mut general, [GeneralEvent::Select]);

        assert!(!general.list_open());
        assert_eq!(general.settings().start_view, StartView::Last);
        assert_eq!(device.stored().start_view, Some(StartView::Last));
        assert_eq!(
            router.statuses(),
            ["start_view saved to personal-ledger.conf"]
        );
    }

    #[test]
    fn esc_closes_the_open_list_changing_nothing_and_is_the_frames_once_closed() {
        let (device, mut general) = writable();
        on_start_view(&mut general);
        assert!(!general.takes(&GeneralEvent::CloseList), "nothing open");
        send(
            &mut general,
            [GeneralEvent::Select, GeneralEvent::Move(Step::Next)],
        );
        assert!(general.takes(&GeneralEvent::CloseList));

        let router = send(&mut general, [GeneralEvent::CloseList]);

        assert!(!general.list_open());
        assert!(router.routed.is_empty());
        assert_eq!(device.stored(), StoredDevice::default());
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_START_VIEW));
        assert!(!general.takes(&GeneralEvent::CloseList));
    }

    #[test]
    fn h_and_l_wait_while_a_list_is_open() {
        let (device, mut general) = writable();
        on_start_view(&mut general);
        send(
            &mut general,
            [GeneralEvent::Select, GeneralEvent::Change(Step::Next)],
        );
        assert!(
            general.takes(&GeneralEvent::Change(Step::Prev)),
            "swallowed"
        );
        assert!(general.list_open());
        assert_eq!(device.stored(), StoredDevice::default());
    }

    #[test]
    fn an_unlisted_start_view_opens_its_list_on_the_first_choice() {
        let (device, mut general) = writable();
        general.configure(
            device.clone(),
            GeneralSettings {
                start_view: StartView::Noun("units".into()),
                ..GeneralSettings::default()
            },
        );
        on_start_view(&mut general);
        send(&mut general, [GeneralEvent::Select]);
        let view = general.view_model().start_view;
        assert_eq!(view.list.highlight(), Some(0));
        assert_eq!(view.control.select.chosen, None);
        assert!(view.unlisted.is_some());
    }

    #[test]
    fn the_locales_list_opens_moves_picks_and_cancels() {
        let (device, mut general) = writable();
        on_locale(&mut general);
        assert_eq!(
            pairs(&general)[2],
            ("enter".to_string(), "open".to_string())
        );

        send(&mut general, [GeneralEvent::Select]);
        assert_eq!(
            general.view_model().locale.list.highlight(),
            Some(0),
            "System"
        );
        send(
            &mut general,
            [GeneralEvent::Move(Step::Prev), GeneralEvent::CloseList],
        );
        assert!(!general.list_open());
        assert_eq!(device.stored().locale, None, "cancelled");

        let router = send(
            &mut general,
            [
                GeneralEvent::Select,
                GeneralEvent::Move(Step::Prev),
                GeneralEvent::Select,
            ],
        );

        assert_eq!(
            device.stored().locale,
            Some(Locale::EnUs),
            "wrapped to the last"
        );
        assert_eq!(router.statuses(), ["locale saved to personal-ledger.conf"]);
        assert_eq!(
            general.view_model().locale.note.as_deref(),
            Some(
                "Saved. en-US applies when Personal Ledger next starts; until then it reads en-AU."
            )
        );
    }

    #[test]
    fn h_and_l_still_step_a_closed_dropdown() {
        let (device, mut general) = writable();
        on_start_view(&mut general);
        send(&mut general, [GeneralEvent::Change(Step::Next)]);
        assert_eq!(device.stored().start_view, Some(StartView::Last));
        assert!(!general.list_open());

        send(
            &mut general,
            [
                GeneralEvent::Move(Step::Next),
                GeneralEvent::Move(Step::Next),
            ],
        );
        send(&mut general, [GeneralEvent::Change(Step::Next)]);
        assert_eq!(device.stored().locale, Some(Locale::EnAu));
        assert!(!general.list_open());
    }

    #[test]
    fn a_click_on_a_field_opens_its_list_closes_the_other_and_a_second_click_closes_it() {
        let (_, mut general) = writable();
        send(&mut general, [GeneralEvent::PressStartView]);
        assert!(general.view_model().start_view.list.is_open());

        send(&mut general, [GeneralEvent::PressLocale]);
        let view = general.view_model();
        assert!(!view.start_view.list.is_open(), "one list at a time");
        assert!(view.locale.list.is_open());

        send(&mut general, [GeneralEvent::PressLocale]);
        assert!(!general.list_open());
    }

    #[test]
    fn a_click_on_a_field_puts_the_keyboard_on_it_while_the_page_has_it() {
        let (_, mut general) = writable();
        send(&mut general, [GeneralEvent::PressLocale]);
        assert_eq!(general.view_model().field, None, "the index keeps it");

        general.set_page_focused(true);
        send(&mut general, [GeneralEvent::PressLocale]);
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_LOCALE));
    }

    #[test]
    fn a_click_on_a_row_picks_it_and_closes_the_list() {
        let (device, mut general) = writable();
        send(
            &mut general,
            [
                GeneralEvent::PressStartView,
                GeneralEvent::ChooseStartView(StartView::Settings),
            ],
        );
        assert!(!general.list_open());
        assert_eq!(device.stored().start_view, Some(StartView::Settings));

        send(
            &mut general,
            [
                GeneralEvent::PressLocale,
                GeneralEvent::ChooseLocale(Some(Locale::EnGb)),
            ],
        );
        assert!(!general.list_open());
        assert_eq!(device.stored().locale, Some(Locale::EnGb));
    }

    #[test]
    fn leaving_the_page_closes_an_open_list() {
        let (_, mut general) = writable();
        on_start_view(&mut general);
        send(&mut general, [GeneralEvent::Select]);
        general.set_page_focused(false);
        assert!(!general.list_open());
    }
}
