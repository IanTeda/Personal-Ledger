//! Settings › General's Presenter (DESKTOP-177, DESKTOP-178): On start, Start view and Toasts,
//! Configuration in the user's configuration file (ADR-0053) written through
//! [`DeviceConfiguration`], and the read-only Locale, Ledger folder, last Ledger and configuration
//! file, which the Client hands it. Everything here belongs to this device and works with no
//! Ledger open (ADR-0051).
//!
//! A control shows a change only once it is saved: a shadowed key or a refused write leaves it as
//! it was and says why on the status line.

use std::convert::Infallible;
use std::path::PathBuf;
use std::rc::Rc;

use lib_core::{OnStart, StartView};
use lib_list_cursor::Step;
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP};
use lib_viper::presenter::{PagePresenter, Presenter};
use lib_viper::view::{Choice, Hint, SelectView};

use super::device::{ControlView, chords, settle, shadowed_note, step_choice};
use crate::keymap::{NEXT_CHOICE, PREVIOUS_CHOICE};
use crate::msg;
use crate::protocols::{DeviceConfiguration, DeviceKey, DeviceRouter};

/// On start's choices, in the segmented control's order.
pub const ON_START_CHOICES: [OnStart; 2] = [OnStart::ReopenLastLedger, OnStart::NoLedger];

/// Toasts' choices, On then Off as on the control.
const TOASTS_CHOICES: [bool; 2] = [true, false];

/// The page's controls, top to bottom. The Locale and Ledger folder are read-only, so the keyboard
/// passes them by.
pub const GENERAL_FIELD_ON_START: usize = 0;
pub const GENERAL_FIELD_START_VIEW: usize = 1;
pub const GENERAL_FIELD_TOASTS: usize = 2;
const GENERAL_FIELD_COUNT: usize = 3;

/// What General is told and the person asks of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneralEvent {
    /// `j`/`k`: the control below or above.
    Move(Step),
    /// `h`/`l`: the focused control's choice after or before.
    Change(Step),
    ChooseOnStart(OnStart),
    ChooseStartView(StartView),
    ChooseToasts(bool),
    /// `:toasts on|off`, which writes the setting even when it holds already.
    SetToasts(bool),
}

/// A device's settings General writes, as start-up read them from Configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralSettings {
    pub on_start: OnStart,
    pub start_view: StartView,
    pub toasts_on: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            on_start: OnStart::default(),
            start_view: StartView::default(),
            toasts_on: lib_toast::Display::default().toasts_on,
        }
    }
}

/// What start-up knows about this device that General shows read-only: the Ledger folder (the
/// `data` setting), the Ledger On start would reopen and the configuration file Settings writes to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThisDevice {
    pub ledger_folder: PathBuf,
    /// The Ledger file open now, which the Client remembers at quit; `None` with none open.
    pub last_ledger: Option<PathBuf>,
    /// The user's configuration file; `None` with no home directory to keep one in.
    pub configuration: Option<PathBuf>,
}

/// The effective Locale as the Client describes it, with the note when the one asked for wasn't
/// supported. The Locale is Configuration read at start-up (ADR-0021), so it is shown, not set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocaleLine {
    pub line: String,
    pub fallback: Option<String>,
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

/// A read-only line: its label, its value and the hint under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOnlyView {
    pub label: String,
    pub value: String,
    pub hint: String,
}

/// The THIS DEVICE box: its heading, its `(label, value)` rows and the page's footnote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThisDeviceView {
    pub heading: String,
    pub rows: Vec<(String, String)>,
    pub note: String,
}

/// Start view's radio group: the choices, the one held (`None` for a value from the file that
/// isn't a choice), and the line naming that value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartViewControl {
    pub control: ControlView<StartView>,
    pub unlisted: Option<String>,
}

/// General as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralViewModel {
    /// The control the keyboard is on, by its `GENERAL_FIELD_*` index.
    pub field: Option<usize>,
    pub on_start: ControlView<OnStart>,
    pub start_view: StartViewControl,
    pub toasts: ControlView<bool>,
    pub locale: ReadOnlyView,
    /// The note when the Locale asked for wasn't supported.
    pub locale_fallback: Option<String>,
    pub ledger_folder: ReadOnlyView,
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
        true
    }

    pub fn settings(&self) -> &GeneralSettings {
        &self.settings
    }

    pub fn this_device(&self) -> &ThisDevice {
        &self.this_device
    }

    /// The effective Locale as the Client described it at start-up.
    pub fn locale(&self) -> &LocaleLine {
        &self.locale
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
        match event {
            GeneralEvent::Move(step) => self.move_field(step),
            GeneralEvent::Change(step) => self.change(step, router),
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
                hint: Some(lib_locale::msg::toast_setting_note()),
                shadowed: shadowed_note(device, DeviceKey::Toasts),
            },
            locale: ReadOnlyView {
                label: msg::settings_general_locale_label(),
                value: self.locale.line.clone(),
                hint: msg::settings_general_locale_hint(),
            },
            locale_fallback: self.locale.fallback.clone(),
            ledger_folder: ReadOnlyView {
                label: msg::settings_general_ledger_folder(),
                value: self.this_device.ledger_folder.display().to_string(),
                hint: msg::settings_general_ledger_folder_hint(),
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
            hints: vec![
                Hint::new(
                    chords(&self.keymap, &[MOVE_DOWN, MOVE_UP]),
                    msg::settings_hint_field(),
                ),
                Hint::new(
                    chords(&self.keymap, &[PREVIOUS_CHOICE, NEXT_CHOICE]),
                    msg::settings_hint_change(),
                ),
                Hint::new(self.back_chord.clone(), msg::settings_hint_index()),
            ],
        }
    }

    /// Whether General takes `event` as things stand, so a key it declines goes on to the frame:
    /// a move or change only while a control has the keyboard.
    fn takes(&self, event: &GeneralEvent) -> bool {
        match event {
            GeneralEvent::Move(_) | GeneralEvent::Change(_) => self.field.is_some(),
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

    fn write_toasts(&mut self, on: bool, router: &mut dyn DeviceRouter) {
        let answer = self.device.set_toasts(on);
        if let Some(saved) = settle(answer, DeviceKey::Toasts, router) {
            self.settings.toasts_on = saved;
            router.toasts_changed(saved);
        }
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
    use crate::fake::{InMemoryDeviceConfiguration, RecordingRouter, Routed};
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
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        let mut general = GeneralPresenter::new(GeneralSetup {
            device: device.clone(),
            keymap: general_keymap(&KeyBindingConfig::default()),
            back_chord: "esc".to_string(),
            start_views: start_views(),
            locale: LocaleLine {
                line: "en-AU, from config".to_string(),
                fallback: None,
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
    fn j_and_k_walk_the_three_controls_and_stop_at_the_ends() {
        let (_, mut general) = writable();
        general.set_page_focused(true);

        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Move(Step::Next), 3),
        );
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_TOASTS));
        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Move(Step::Prev), 4),
        );
        assert_eq!(general.view_model().field, Some(GENERAL_FIELD_ON_START));
        assert_eq!(
            general.keymap().event("enter"),
            None,
            "enter is the frame's"
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
    fn the_read_only_lines_show_what_the_client_told_it() {
        let (_, mut general) = writable();
        general.set_this_device(ThisDevice {
            ledger_folder: PathBuf::from("/home/me/Ledgers"),
            last_ledger: None,
            configuration: Some(PathBuf::from("/home/me/.config/personal-ledger.conf")),
        });
        general.set_last_ledger(Some(PathBuf::from("/home/me/Ledgers/home.ledger")));

        let view = general.view_model();
        assert_eq!(view.locale.value, "en-AU, from config");
        assert_eq!(view.ledger_folder.value, "/home/me/Ledgers");
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
        let pairs = |general: &GeneralPresenter| -> Vec<(String, String)> {
            general
                .view_model()
                .hints
                .into_iter()
                .map(|hint| (hint.chord, hint.action))
                .collect()
        };
        assert_eq!(
            pairs(&general),
            [
                ("j/k".to_string(), "field".to_string()),
                ("h/l".to_string(), "change".to_string()),
                ("esc".to_string(), "index".to_string()),
            ]
        );

        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(NEXT_CHOICE.to_string(), "ctrl+l".to_string());
        general.set_keymap(general_keymap(&config), "esc".to_string());
        assert_eq!(pairs(&general)[1].0, "h/ctrl+l");
    }
}
