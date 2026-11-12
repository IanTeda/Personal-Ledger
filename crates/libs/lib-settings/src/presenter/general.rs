//! Settings › General's Presenter (DESKTOP-177, DESKTOP-178): On start, Start view, Toasts, the
//! Locale and the Data folder, Configuration in the user's configuration file (ADR-0053, ADR-0057)
//! written through [`DeviceConfiguration`], and the read-only last Ledger and configuration file,
//! which the Client hands it. Everything here belongs to this device and works with no Ledger open
//! (ADR-0051).
//!
//! The fields are the device settings form's ([`DeviceForm`]), which walks them, steps them, holds
//! the one open dropdown and writes each change, showing it only once saved. General declares them
//! ([`GeneralField`]) and keeps what is its own: the Locale's notes, the Locale and the Data
//! folder being written even while shadowed with a note naming the override (DESKTOP-186), Start
//! view's unlisted line, Toasts' test and the THIS DEVICE box. A chosen Locale applies at the next
//! start, so a note says so while the file's differs from the one in effect (DESKTOP-185).
//!
//! The Data folder is picked in the Shell's file explorer (DESKTOP-187): **Change…** asks the
//! Router for it, and what it picks comes back to [`GeneralPresenter::set_data_folder`], the one
//! place the Data folder is written, which answers how the write went as a [`DataFolderOutcome`].
//! It applies at once, with a Success Toast (DESKTOP-188).

use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use lib_core::configuration::{ConfigKey, ConfigValue};
use lib_core::{OnStart, StartView};
use lib_list_cursor::Step;
use lib_locale::Locale;
use lib_toast::{Notice, ToastKind};
use lib_viper::keymap::{KeyBindingConfig, Keymap, SELECT};
use lib_viper::presenter::{PagePresenter, Presenter};
use lib_viper::view::{Choice, Hint};

use super::form::{
    DeviceForm, FieldDecl, FieldValue, FieldViews, FormAnswer, FormInput, Presentation,
};
use crate::keymap::TEST_TOASTS;
use crate::msg;
use crate::protocols::{DeviceConfiguration, DeviceRouter, DeviceSnapshot};
use crate::summary::SummaryRow;

/// On start's choices, in the segmented control's order.
pub const ON_START_CHOICES: [OnStart; 2] = [OnStart::ReopenLastLedger, OnStart::NoLedger];

/// Toasts' choices, On then Off as on the control.
const TOASTS_CHOICES: [bool; 2] = [true, false];

/// General's fields, top to bottom, which is the `j`/`k` walk's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneralField {
    OnStart,
    StartView,
    Toasts,
    Locale,
    DataFolder,
}

impl GeneralField {
    pub const ALL: [GeneralField; 5] = [
        Self::OnStart,
        Self::StartView,
        Self::Toasts,
        Self::Locale,
        Self::DataFolder,
    ];
}

/// The Locale's choices, in the radio group's order: **System** (`None`), then each supported
/// Locale [`Locale::choice`] takes, by tag, so the list and every read of the file agree.
pub fn locale_choices() -> Vec<Option<Locale>> {
    let mut locales: Vec<Locale> = Locale::SUPPORTED
        .into_iter()
        .filter_map(|locale| Locale::choice(locale.tag()))
        .collect();
    locales.sort_by_key(|locale| locale.tag());
    std::iter::once(None)
        .chain(locales.into_iter().map(Some))
        .collect()
}

/// What General is told and the person asks of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneralEvent {
    /// `j`/`k`: the field below or above, or an open list's highlight.
    Move(Step),
    /// `h`/`l`: the focused field's choice after or before.
    Change(Step),
    /// `enter`: on Start view or the Locale, opens its list, or picks an open list's highlight; on
    /// the Data folder, its picker.
    Select,
    /// `esc` while a list is open: closes it, changing nothing.
    CloseList,
    /// A click on a field's choice, by its index.
    Choose { field: GeneralField, index: usize },
    /// A click on a field: opens a dropdown's list, or closes it while open.
    Press(GeneralField),
    /// `:toasts on|off`, which writes the setting even when it holds already.
    SetToasts(bool),
    /// `t`: the Toasts test, only while Toasts has the keyboard.
    TestToasts,
    /// A click on **Test**, wherever the keyboard is.
    PressTest,
    /// A click on **Change…**, wherever the keyboard is.
    PressChange,
}

impl GeneralEvent {
    /// The event as the form takes it, `None` for one that is General's own.
    fn form_input(&self) -> Option<FormInput<GeneralField>> {
        Some(match *self {
            Self::Move(step) => FormInput::Move(step),
            Self::Change(step) => FormInput::Change(step),
            Self::Select => FormInput::Select,
            Self::CloseList => FormInput::CloseList,
            Self::Choose { field, index } => FormInput::Choose { field, index },
            Self::Press(field) => FormInput::Press(field),
            Self::SetToasts(_) | Self::TestToasts | Self::PressTest | Self::PressChange => {
                return None;
            }
        })
    }
}

/// How writing the folder the picker picked went, which the Shell needs to know where `:open` and
/// `:new` start: [`GeneralPresenter::set_data_folder`] answers it rather than the Client comparing
/// the folder in effect before and after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFolderOutcome {
    /// The folder is the one the page already shows, so nothing was written.
    Unchanged,
    /// Written, and now the Data folder in effect.
    Applied,
    /// Written, but a source above the configuration file keeps the old folder in effect.
    SavedOverridden,
    /// Not written: there is no file to keep it in, or the write failed.
    NotWritten,
}

impl DataFolderOutcome {
    /// Whether the folder picked is now the Data folder in effect, the `bool` the Shell's Router
    /// answers: only once applied.
    pub fn in_effect(self) -> bool {
        self == Self::Applied
    }
}

/// A device's settings General writes, as the device read them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralSettings {
    pub on_start: OnStart,
    pub start_view: StartView,
    pub toasts_on: bool,
    /// The Locale the user's configuration file names, `None` for **System**: not the one in
    /// effect, which a flag, a variable or a higher file may set.
    pub locale: Option<Locale>,
    /// The Data folder the user's configuration file names, `None` when it names none: not the one
    /// in effect ([`GeneralPresenter::data_folder`]), which a flag, a variable or a higher file may
    /// set.
    pub data: Option<PathBuf>,
}

impl GeneralSettings {
    /// What each field shows of `snapshot` ([`DeviceSnapshot::shown`]): the value in force, but
    /// the file's own Locale and Data folder, each one of the choices or the default.
    pub fn read(snapshot: &DeviceSnapshot) -> Self {
        let mut settings = Self::default();
        for key in [
            ConfigKey::OnStart,
            ConfigKey::StartView,
            ConfigKey::Toasts,
            ConfigKey::Locale,
            ConfigKey::Data,
        ] {
            if let Some(value) = snapshot.shown(key) {
                settings.take(value);
            }
        }
        settings
    }

    fn take(&mut self, value: &ConfigValue) {
        match value {
            ConfigValue::OnStart(on_start) => self.on_start = *on_start,
            ConfigValue::StartView(start_view) => self.start_view = start_view.clone(),
            ConfigValue::Toasts(on) => self.toasts_on = *on,
            // One of the choices, else **System**, as the TUI reads it: an unsupported tag would
            // otherwise show as en-US checked, and choosing en-US would then write nothing.
            ConfigValue::Locale(tag) => self.locale = Locale::choice(tag),
            ConfigValue::Data(folder) => self.data = Some(folder.clone()),
            _ => {}
        }
    }
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
    /// Where the settings are read from, once, and written: the configuration file start-up read.
    pub device: Rc<dyn DeviceConfiguration>,
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

/// What the Data folder shows beside its field: the folder the file names (else the one in
/// effect) and the **Change…** button with the key that presses it. Its note while a source above
/// the file sets it is the field's own note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFolderView {
    pub value: String,
    pub change: Hint,
}

/// General as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralViewModel {
    /// Each field as the form draws it, in [`GeneralField::ALL`]'s order. The Locale's note while
    /// its choice waits on the next start or a source above the file overrides it is the field's
    /// own note.
    pub fields: FieldViews<GeneralField>,
    /// The line naming a Start view from the file that isn't one of the choices.
    pub start_view_unlisted: Option<String>,
    /// The **Test** button beside Toasts: its label and the key that presses it from Toasts.
    pub toasts_test: Hint,
    /// The note when the Locale asked for wasn't supported.
    pub locale_fallback: Option<String>,
    pub data_folder: DataFolderView,
    pub this_device: ThisDeviceView,
    /// The page's keys, for the status line and the `?` cheat sheet.
    pub hints: Vec<Hint>,
}

/// Settings › General's Presenter.
pub struct GeneralPresenter {
    /// The device as last read: what sets each key above the file and the file itself.
    snapshot: DeviceSnapshot,
    keymap: Keymap<GeneralEvent>,
    back_chord: String,
    locale: LocaleLine,
    /// The fields and the values they show, as each write's answer keeps them.
    form: DeviceForm<GeneralField>,
    /// The Data folder in effect, where `:open` and `:new` start: Data's value in force, until a
    /// write that applies at once replaces it. Not the field's value, which is the file's.
    data_folder: PathBuf,
    /// The Ledger file open now, which the Client remembers at quit; `None` with none open.
    last_ledger: Option<PathBuf>,
    page_focused: bool,
}

impl GeneralPresenter {
    /// The page's keymap and the back chord its legend shows under `config`, as [`Self::new`] and
    /// [`PagePresenter::rebind`] build them.
    fn keys(config: &KeyBindingConfig) -> (Keymap<GeneralEvent>, String) {
        (
            crate::keymap::general_keymap(config),
            crate::keymap::back_chord(config),
        )
    }

    /// General on what `setup`'s device reads.
    pub fn new(setup: GeneralSetup, config: &KeyBindingConfig) -> Self {
        let (keymap, back_chord) = Self::keys(config);
        let snapshot = setup.device.read();
        let data_folder = snapshot
            .data_folder()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let fields = GeneralField::ALL
            .into_iter()
            .map(|field| declare(field, &setup.start_views, &setup.locale, &data_folder))
            .collect();
        Self {
            form: DeviceForm::new(setup.device, &snapshot, fields),
            data_folder,
            last_ledger: None,
            snapshot,
            keymap,
            back_chord,
            locale: setup.locale,
            page_focused: false,
        }
    }

    /// The Ledger file the Client will remember at quit, once one has opened or closed.
    pub fn set_last_ledger(&mut self, last_ledger: Option<PathBuf>) {
        self.last_ledger = last_ledger;
    }

    /// Whether the page, not Settings' index, has the keyboard. Arriving puts it on On start, and
    /// leaving takes it off the fields; only a change moves it. Answers whether it changed, so
    /// the View redraws only then: the frame tells the page on every draw.
    pub fn set_page_focused(&mut self, focused: bool) -> bool {
        if focused == self.page_focused {
            return false;
        }
        self.page_focused = focused;
        self.form.set_focused(focused);
        true
    }

    /// The settings the fields show, as start-up read them or as the page last wrote them.
    pub fn settings(&self) -> GeneralSettings {
        let mut settings = GeneralSettings::default();
        for field in GeneralField::ALL {
            if let Some(FieldValue::Set(value)) = self.form.value(field) {
                settings.take(value);
            }
        }
        settings
    }

    /// The Ledger file the Client will remember at quit, `None` with none open.
    pub fn last_ledger(&self) -> Option<&Path> {
        self.last_ledger.as_deref()
    }

    /// The user's configuration file, as the device read it; `None` with no home folder.
    pub fn configuration_file(&self) -> Option<&Path> {
        self.snapshot.file.as_deref()
    }

    /// The Locale in effect as the Client described it at start-up.
    pub fn locale(&self) -> &LocaleLine {
        &self.locale
    }

    /// The Data folder in effect, where `:open` and `:new` start.
    pub fn data_folder(&self) -> &Path {
        &self.data_folder
    }

    /// What the Settings index's detail pane quotes of General: its fields as they stand, with
    /// the Locale and the Data folder in effect rather than the file's.
    pub fn summary_rows(&self) -> Vec<SummaryRow> {
        let settings = self.settings();
        vec![
            SummaryRow::new(
                msg::settings_general_on_start(),
                on_start_label(settings.on_start),
            ),
            SummaryRow::new(
                lib_locale::msg::toast_setting_label(),
                toasts_label(settings.toasts_on),
            ),
            SummaryRow::new(msg::settings_summary_locale(), self.locale.line.clone()),
            SummaryRow::new(
                msg::settings_general_data_folder(),
                self.data_folder.display().to_string(),
            ),
        ]
    }

    /// The folder the picker picked, from **Change…** or `:data`, written as the Data folder: the
    /// one place it is written. The folder the page shows already writes nothing. Without an
    /// override it applies at once, with a Success Toast in place of the status line; with one it
    /// is saved for when the override is gone, on the status line, as is a refused or failed write.
    ///
    /// A method, not an event, since `handle` answers nothing and the Shell needs the outcome
    /// (ADR-0056's 2026-11-11 amendment).
    pub fn set_data_folder(
        &mut self,
        folder: PathBuf,
        router: &mut dyn DeviceRouter,
    ) -> DataFolderOutcome {
        // As any event but a list's own keys, it closes an open list first.
        self.form.close_list();
        let shadowed = self.snapshot.get(ConfigKey::Data).shadow.is_some();
        // The folder in effect is unchanged only while nothing overrides it: under an override,
        // the file naming no folder, it is saved for when the override is gone.
        let in_file = match self.form.value(GeneralField::DataFolder) {
            Some(FieldValue::Set(ConfigValue::Data(saved))) => Some(saved.as_path()),
            _ => None,
        };
        if in_file == Some(folder.as_path())
            || (in_file.is_none() && !shadowed && folder == self.data_folder)
        {
            return DataFolderOutcome::Unchanged;
        }
        // The status line's word waits here, since an applied folder says so with a Toast instead.
        let mut notices = Vec::new();
        let answer = self.form.write(
            GeneralField::DataFolder,
            FieldValue::Set(ConfigValue::Data(folder)),
            &mut notices,
        );
        let saved = match answer {
            Some(FormAnswer::Saved(_, FieldValue::Set(ConfigValue::Data(saved)))) => Some(saved),
            _ => None,
        };
        match saved {
            Some(saved) if !shadowed => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::settings_general_data_folder_now(&saved.display().to_string()),
                ));
                self.data_folder = saved;
                DataFolderOutcome::Applied
            }
            saved => {
                notices.into_iter().for_each(|notice| router.notify(notice));
                match saved {
                    Some(_) => DataFolderOutcome::SavedOverridden,
                    None => DataFolderOutcome::NotWritten,
                }
            }
        }
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
        let Some(input) = event.form_input() else {
            // As any input the form doesn't take, General's own closes an open list first.
            self.form.close_list();
            match event {
                GeneralEvent::SetToasts(on) => {
                    let saved = self.form.write(
                        GeneralField::Toasts,
                        FieldValue::Set(ConfigValue::Toasts(on)),
                        router,
                    );
                    self.answered(saved, router);
                }
                // The test touches no setting: the Shell's Toasts raise it.
                GeneralEvent::TestToasts | GeneralEvent::PressTest => router.test_toasts(),
                GeneralEvent::PressChange => router.pick_data_folder(),
                _ => {}
            }
            return;
        };
        let answer = self.form.handle(input, router);
        self.answered(answer, router);
    }

    fn view_model(&self) -> GeneralViewModel {
        let path = |path: Option<&Path>| {
            path.map_or_else(msg::settings_general_none, |path| {
                path.display().to_string()
            })
        };
        let fields = self.form.views(GeneralField::ALL);
        let start_view_listed = fields
            .view(GeneralField::StartView)
            .is_some_and(|view| view.chosen.is_some());
        GeneralViewModel {
            fields,
            start_view_unlisted: (!start_view_listed).then(|| {
                msg::settings_general_start_view_unlisted(self.settings().start_view.key())
            }),
            toasts_test: Hint::new(
                self.keymap.hint_chord(TEST_TOASTS),
                lib_locale::msg::toast_setting_test(),
            ),
            locale_fallback: self.locale.fallback.clone(),
            data_folder: DataFolderView {
                value: self.shown_data_folder().display().to_string(),
                change: Hint::new(
                    self.keymap.hint_chord(SELECT),
                    msg::settings_general_data_folder_change(),
                ),
            },
            this_device: ThisDeviceView {
                heading: msg::settings_general_this_device(),
                rows: vec![
                    (
                        msg::settings_general_last_ledger(),
                        path(self.last_ledger.as_deref()),
                    ),
                    (
                        msg::settings_general_configuration(),
                        path(self.snapshot.file.as_deref()),
                    ),
                ],
                note: msg::settings_general_note(),
            },
            hints: self.hints(),
        }
    }

    /// Whether General takes `event` as things stand, so a key it declines goes on to the frame:
    /// the form's keys as the form says, and `t` only on Toasts with no list open.
    fn takes(&self, event: &GeneralEvent) -> bool {
        match event {
            GeneralEvent::TestToasts => {
                !self.form.list_open() && self.form.focused() == Some(GeneralField::Toasts)
            }
            event => event
                .form_input()
                .is_none_or(|input| self.form.takes(&input)),
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

    /// The keymap and the `back` chord its legend shows.
    fn rebind(&mut self, keys: &KeyBindingConfig) {
        (self.keymap, self.back_chord) = Self::keys(keys);
    }
}

impl GeneralPresenter {
    /// The form's keys, and on Toasts, `t` for the test.
    fn hints(&self) -> Vec<Hint> {
        let mut hints = self.form.hints(&self.keymap, &self.back_chord);
        if !self.form.list_open() && self.form.focused() == Some(GeneralField::Toasts) {
            hints.push(Hint::new(
                self.keymap.hint_chord(TEST_TOASTS),
                msg::settings_hint_test(),
            ));
        }
        hints
    }

    /// What the form answered: saved Toasts reach the Shell, and the Data folder's **Change…** asks
    /// for the picker.
    fn answered(
        &mut self,
        answer: Option<FormAnswer<GeneralField>>,
        router: &mut dyn DeviceRouter,
    ) {
        match answer {
            // Start view, the Locale and On start take effect at the next start, which reads them
            // from the file, so only Toasts reach the Shell now.
            Some(FormAnswer::Saved(_, FieldValue::Set(ConfigValue::Toasts(on)))) => {
                router.toasts_changed(on);
            }
            Some(FormAnswer::Activated(GeneralField::DataFolder)) => router.pick_data_folder(),
            // General's walk stops at either end, and remembers nothing.
            Some(
                FormAnswer::Saved(..)
                | FormAnswer::Activated(_)
                | FormAnswer::Changed(..)
                | FormAnswer::LeftEnd(_),
            )
            | None => {}
        }
    }

    /// The Data folder the field shows: the file's, else the one in effect.
    fn shown_data_folder(&self) -> &Path {
        match self.form.value(GeneralField::DataFolder) {
            Some(FieldValue::Set(ConfigValue::Data(folder))) => folder,
            _ => &self.data_folder,
        }
    }
}

/// `field` as the form takes it: its key, label, choices, how it is drawn, the line under it and
/// the Locale's and the Data folder's own notes. `data_folder` is the one in effect, which stays
/// as start-up read it while a source above the file sets it, the only time its note shows.
fn declare(
    field: GeneralField,
    start_views: &[Choice<StartView>],
    locale: &LocaleLine,
    data_folder: &Path,
) -> FieldDecl<GeneralField> {
    let set = |value| FieldValue::Set(value);
    let defaults = GeneralSettings::default();
    match field {
        GeneralField::OnStart => FieldDecl::configuration(
            field,
            ConfigKey::OnStart,
            msg::settings_general_on_start(),
            Presentation::Segmented,
            ON_START_CHOICES
                .into_iter()
                .map(|value| Choice {
                    value: set(ConfigValue::OnStart(value)),
                    label: on_start_label(value),
                })
                .collect(),
        )
        .when_unset(set(ConfigValue::OnStart(defaults.on_start)))
        .note(msg::settings_general_on_start_hint()),
        GeneralField::StartView => FieldDecl::configuration(
            field,
            ConfigKey::StartView,
            msg::settings_general_start_view(),
            Presentation::Dropdown,
            start_views
                .iter()
                .map(|choice| Choice {
                    value: set(ConfigValue::StartView(choice.value.clone())),
                    label: choice.label.clone(),
                })
                .collect(),
        )
        .when_unset(set(ConfigValue::StartView(defaults.start_view)))
        // A noun from the file that isn't a choice steps from the Dashboard.
        .step_from(set(ConfigValue::StartView(StartView::Dashboard)))
        .note(msg::settings_general_start_view_hint()),
        GeneralField::Toasts => FieldDecl::configuration(
            field,
            ConfigKey::Toasts,
            lib_locale::msg::toast_setting_label(),
            Presentation::Segmented,
            TOASTS_CHOICES
                .into_iter()
                .map(|value| Choice {
                    value: set(ConfigValue::Toasts(value)),
                    label: toasts_label(value),
                })
                .collect(),
        )
        .when_unset(set(ConfigValue::Toasts(defaults.toasts_on)))
        .change_hint(msg::settings_hint_on_off())
        .note(lib_locale::msg::toast_setting_test_note()),
        // **System** is no value: choosing it takes the key out. Each choice reads as its label
        // then its code (`System · now en-AU`).
        GeneralField::Locale => FieldDecl::configuration(
            field,
            ConfigKey::Locale,
            msg::settings_general_locale_label(),
            Presentation::Dropdown,
            locale_choices()
                .into_iter()
                .map(|choice| {
                    let code = match choice {
                        None => msg::settings_general_locale_system_now(locale.system.tag()),
                        Some(locale) => locale.tag().to_string(),
                    };
                    Choice {
                        value: locale_value(choice),
                        label: msg::settings_general_locale_option(&locale_label(choice), &code),
                    }
                })
                .collect(),
        )
        .reading(|value| match value {
            ConfigValue::Locale(tag) => locale_value(Locale::choice(tag)),
            _ => FieldValue::Unset,
        })
        .own_note_replacing_shadowed(locale_note(locale.effective, locale.system))
        .note(msg::settings_general_locale_hint()),
        GeneralField::DataFolder => FieldDecl::configuration(
            field,
            ConfigKey::Data,
            msg::settings_general_data_folder(),
            Presentation::Action,
            Vec::new(),
        )
        .own_note_replacing_shadowed({
            let in_effect = data_folder.display().to_string();
            move |_, source| {
                source
                    .map(|source| msg::settings_general_data_folder_overridden(source, &in_effect))
            }
        })
        .note(msg::settings_general_data_folder_hint()),
    }
}

/// The Locale's own note: the override, while a source above the file sets it; else the pending
/// Locale, while the one chosen (**System** being `system`) isn't `effective`, the one this run
/// reads.
fn locale_note(
    effective: Locale,
    system: Locale,
) -> impl Fn(&FieldValue, Option<&str>) -> Option<String> {
    move |value, source| {
        if let Some(source) = source {
            return Some(msg::settings_general_locale_overridden(
                source,
                effective.tag(),
            ));
        }
        let chosen = match value {
            FieldValue::Set(ConfigValue::Locale(tag)) => Locale::choice(tag),
            _ => None,
        }
        .unwrap_or(system);
        (chosen != effective)
            .then(|| msg::settings_general_locale_pending(chosen.tag(), effective.tag()))
    }
}

/// A Locale choice as the form holds it: its tag, or no value for **System**.
fn locale_value(choice: Option<Locale>) -> FieldValue {
    choice.map_or(FieldValue::Unset, |locale| {
        FieldValue::Set(ConfigValue::Locale(locale.tag().to_string()))
    })
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

fn on_start_label(choice: OnStart) -> String {
    match choice {
        OnStart::ReopenLastLedger => msg::settings_general_on_start_reopen(),
        OnStart::NoLedger => msg::settings_general_on_start_none(),
    }
}

/// Toasts' choice as its control and the index's summary both name it.
fn toasts_label(on: bool) -> String {
    if on {
        lib_locale::msg::toast_setting_on()
    } else {
        lib_locale::msg::toast_setting_off()
    }
}

#[cfg(test)]
mod tests {
    use lib_viper::keymap::KeyBindingConfig;

    use super::*;
    use crate::fake::{InMemoryDeviceConfiguration, RecordingRouter, Routed, StoredDevice};

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
        GeneralPresenter::new(
            GeneralSetup {
                device,
                start_views: start_views(),
                locale: LocaleLine {
                    line: "en-AU, from config".to_string(),
                    fallback: Some("en-XX isn't supported".to_string()),
                    effective,
                    system: Locale::EnAu,
                },
            },
            &KeyBindingConfig::default(),
        )
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

    /// A click on the Locale's choice `index`: System, en-AU, en-GB, en-US.
    fn choose_locale(index: usize) -> GeneralEvent {
        GeneralEvent::Choose {
            field: GeneralField::Locale,
            index,
        }
    }

    fn pairs(general: &GeneralPresenter) -> Vec<(String, String)> {
        general
            .view_model()
            .hints
            .into_iter()
            .map(|hint| (hint.chord, hint.action))
            .collect()
    }

    #[test]
    fn general_shows_the_values_in_force_but_the_files_locale_and_data_folder() {
        let device = Rc::new(
            InMemoryDeviceConfiguration::shadowing(ConfigKey::OnStart, "--no-ledger")
                .reading([
                    ConfigValue::OnStart(OnStart::NoLedger),
                    ConfigValue::StartView(StartView::Last),
                    ConfigValue::Toasts(false),
                    ConfigValue::Locale("en-US".to_string()),
                    ConfigValue::Data(PathBuf::from("/from/above")),
                ])
                .storing([
                    ConfigValue::OnStart(OnStart::ReopenLastLedger),
                    ConfigValue::Locale("en-gb".to_string()),
                    ConfigValue::Data(PathBuf::from("/in/the/file")),
                ]),
        );

        let general = general_on(device);

        assert_eq!(
            general.settings(),
            GeneralSettings {
                on_start: OnStart::NoLedger,
                start_view: StartView::Last,
                toasts_on: false,
                locale: Some(Locale::EnGb),
                data: Some(PathBuf::from("/in/the/file")),
            }
        );
        let view = general.view_model();
        assert_eq!(
            view.fields
                .view(GeneralField::Locale)
                .and_then(|view| view.chosen),
            Some(2),
            "en-GB, whatever the file's case"
        );
    }

    #[test]
    fn general_summarises_its_four_fields_with_the_data_folder_in_effect() {
        let device = Rc::new(
            InMemoryDeviceConfiguration::writable()
                .reading([
                    ConfigValue::OnStart(OnStart::ReopenLastLedger),
                    ConfigValue::Toasts(false),
                    ConfigValue::Data(PathBuf::from("/from/above")),
                ])
                .storing([ConfigValue::Data(PathBuf::from("/in/the/file"))]),
        );

        let rows = general_on(device).summary_rows();

        let pairs: Vec<_> = rows
            .iter()
            .map(|row| (row.label.as_str(), row.value.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("On start", "Reopen last Ledger"),
                ("Toasts", "Off"),
                ("Locale", "en-AU, from config"),
                ("Data folder", "/from/above"),
            ]
        );
    }

    #[test]
    fn a_files_locale_that_isnt_a_choice_shows_as_system() {
        let device = Rc::new(
            InMemoryDeviceConfiguration::writable()
                .storing([ConfigValue::Locale("en-XA".to_string())]),
        );

        let general = general_on(device);

        assert_eq!(general.settings().locale, None);
        assert_eq!(
            general
                .view_model()
                .fields
                .view(GeneralField::Locale)
                .and_then(|view| view.chosen),
            Some(0)
        );
    }

    #[test]
    fn choosing_system_takes_out_a_files_locale_that_isnt_a_choice() {
        let device = Rc::new(
            InMemoryDeviceConfiguration::writable()
                .storing([ConfigValue::Locale("en-XA".to_string())]),
        );
        let mut general = general_on(device.clone());

        let router = send(&mut general, [choose_locale(0)]);

        assert_eq!(device.stored().get(ConfigKey::Locale), None);
        assert_eq!(router.statuses().len(), 1, "the removal is reported");

        let again = send(&mut general, [choose_locale(0)]);
        assert!(again.routed.is_empty(), "System is now the file's too");
    }

    #[test]
    fn an_unlisted_start_view_is_named_under_its_field() {
        let device = Rc::new(
            InMemoryDeviceConfiguration::writable()
                .reading([ConfigValue::StartView(StartView::Noun("units".into()))]),
        );
        let mut general = general_on(device);
        assert!(
            general
                .view_model()
                .start_view_unlisted
                .is_some_and(|line| line.contains("units"))
        );

        send(
            &mut general,
            [GeneralEvent::Choose {
                field: GeneralField::StartView,
                index: 1,
            }],
        );

        assert_eq!(general.settings().start_view, StartView::Last);
        assert_eq!(general.view_model().start_view_unlisted, None);
    }

    #[test]
    fn saved_toasts_reach_the_shell_and_the_command_writes_even_when_they_hold() {
        let (_, mut general) = writable();
        let off = GeneralEvent::Choose {
            field: GeneralField::Toasts,
            index: 1,
        };
        let router = send(&mut general, [off.clone()]);
        assert!(!general.settings().toasts_on);
        assert!(router.routed.contains(&Routed::ToastsChanged(false)));

        let router = send(&mut general, [off]);
        assert!(
            router.routed.is_empty(),
            "a click on the choice held does nothing"
        );
        let router = send(&mut general, [GeneralEvent::SetToasts(false)]);
        assert!(router.routed.contains(&Routed::ToastsChanged(false)));
    }

    #[test]
    fn refused_toasts_arent_handed_the_shell() {
        let mut general = general_on(Rc::new(InMemoryDeviceConfiguration {
            fail_writes: true,
            ..InMemoryDeviceConfiguration::writable()
        }));
        let router = send(&mut general, [GeneralEvent::SetToasts(false)]);
        assert!(general.settings().toasts_on);
        assert!(!router.routed.contains(&Routed::ToastsChanged(false)));
    }

    #[test]
    fn t_on_toasts_asks_for_the_test_and_touches_no_setting() {
        let (device, mut general) = writable();
        general.set_page_focused(true);
        assert!(!general.takes(&GeneralEvent::TestToasts), "on On start");
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
        assert_eq!(device.stored(), StoredDevice::default());
        assert!(general.settings().toasts_on);
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
        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Move(Step::Next), 2),
        );
        assert_eq!(
            general.view_model().fields.focused(),
            Some(GeneralField::Toasts)
        );
        assert_eq!(
            pairs(&general),
            [
                ("j/k".to_string(), "field".to_string()),
                ("h/l".to_string(), "on / off".to_string()),
                ("t".to_string(), "test".to_string()),
            ]
        );
        let view = general.view_model();
        assert_eq!(
            view.toasts_test,
            Hint::new("t".to_string(), "Test".to_string())
        );
        assert_eq!(
            view.fields
                .view(GeneralField::Toasts)
                .and_then(|view| view.hint.clone())
                .as_deref(),
            Some("Errors still show. Test raises one of each kind.")
        );
    }

    #[test]
    fn the_locale_offers_system_then_the_supported_locales_with_their_codes() {
        let (_, general) = writable();
        let view = general.view_model();
        let locale = view.fields.view(GeneralField::Locale);

        assert_eq!(
            locale.map(|view| view.options.clone()),
            Some(vec![
                "System · now en-AU".to_string(),
                "English (Australia) · en-AU".to_string(),
                "English (United Kingdom) · en-GB".to_string(),
                "English (United States) · en-US".to_string(),
            ])
        );
        assert_eq!(locale.and_then(|view| view.chosen), Some(0), "System");
        assert_eq!(locale.and_then(|view| view.own_note.clone()), None);
        assert_eq!(
            view.locale_fallback.as_deref(),
            Some("en-XX isn't supported")
        );
    }

    /// The Locale's own note as drawn.
    fn locale_note(general: &GeneralPresenter) -> Option<String> {
        general
            .view_model()
            .fields
            .view(GeneralField::Locale)
            .and_then(|view| view.own_note.clone())
    }

    #[test]
    fn choosing_system_reads_back_as_no_locale() {
        let (_, mut general) = writable();
        send(&mut general, [choose_locale(2)]);
        assert_eq!(general.settings().locale, Some(Locale::EnGb));

        send(&mut general, [choose_locale(0)]);

        assert_eq!(general.settings().locale, None);
    }

    #[test]
    fn a_locale_other_than_the_one_in_effect_is_noted_until_the_next_start() {
        let (_, mut general) = writable();

        send(&mut general, [choose_locale(3)]);
        assert_eq!(
            locale_note(&general).as_deref(),
            Some(
                "Saved. en-US applies when Personal Ledger next starts; until then it reads en-AU."
            )
        );

        send(&mut general, [choose_locale(1)]);
        assert_eq!(locale_note(&general), None, "en-AU is in effect");
        send(&mut general, [choose_locale(0)]);
        assert_eq!(
            locale_note(&general),
            None,
            "System reads en-AU on this computer"
        );
    }

    #[test]
    fn a_shadowed_locale_names_the_override_and_reads_back_its_choice() {
        let device = Rc::new(InMemoryDeviceConfiguration::shadowing(
            ConfigKey::Locale,
            "--locale en-GB",
        ));
        let mut general = general_reading(device, Locale::EnGb);
        let overridden = "--locale en-GB overrides this for this run, so Personal Ledger reads \
                          en-GB. Your choice is kept for when it's gone.";
        assert_eq!(locale_note(&general).as_deref(), Some(overridden));

        send(&mut general, [choose_locale(3)]);

        assert_eq!(general.settings().locale, Some(Locale::EnUs));
        assert_eq!(locale_note(&general).as_deref(), Some(overridden));
    }

    /// `device` with the Data folder in effect `/home/me/ledgers`.
    fn with_data_folder(device: InMemoryDeviceConfiguration) -> Rc<InMemoryDeviceConfiguration> {
        Rc::new(device.reading([ConfigValue::Data(PathBuf::from("/home/me/ledgers"))]))
    }

    /// The picker picking `folder`: what General answered and what it routed.
    fn pick(general: &mut GeneralPresenter, folder: &str) -> (DataFolderOutcome, RecordingRouter) {
        let mut router = RecordingRouter::default();
        let outcome = general.set_data_folder(PathBuf::from(folder), &mut router);
        (outcome, router)
    }

    #[test]
    fn enter_on_the_data_folder_and_change_ask_for_the_picker() {
        let mut general = general_on(with_data_folder(InMemoryDeviceConfiguration::writable()));
        general.set_page_focused(true);
        send(
            &mut general,
            std::iter::repeat_n(GeneralEvent::Move(Step::Next), 4),
        );
        let view = general.view_model();
        assert_eq!(view.fields.focused(), Some(GeneralField::DataFolder));
        assert_eq!(
            view.fields
                .view(GeneralField::DataFolder)
                .map(|view| view.label.as_str()),
            Some("Data folder")
        );
        assert_eq!(view.data_folder.value, "/home/me/ledgers");
        assert_eq!(
            (
                view.data_folder.change.chord,
                view.data_folder.change.action
            ),
            ("enter".to_string(), "Change…".to_string())
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
        let device = with_data_folder(InMemoryDeviceConfiguration::writable());
        let mut general = general_on(device.clone());

        let (outcome, router) = pick(&mut general, "/home/me/finance");

        assert_eq!(outcome, DataFolderOutcome::Applied);
        assert_eq!(
            device.stored().get(ConfigKey::Data),
            Some(&ConfigValue::Data(PathBuf::from("/home/me/finance")))
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
        let device = with_data_folder(InMemoryDeviceConfiguration::writable());
        let mut general = general_on(device.clone());

        let (outcome, router) = pick(&mut general, "/home/me/ledgers");

        assert_eq!(outcome, DataFolderOutcome::Unchanged);
        assert_eq!(device.stored(), StoredDevice::default());
        assert!(router.routed.is_empty());
    }

    #[test]
    fn the_overridden_data_folder_is_saved_when_the_file_names_none() {
        let device = with_data_folder(InMemoryDeviceConfiguration::shadowing(
            ConfigKey::Data,
            "--data",
        ));
        let mut general = general_on(device.clone());

        let (outcome, _) = pick(&mut general, "/home/me/ledgers");

        assert_eq!(outcome, DataFolderOutcome::SavedOverridden);
        assert_eq!(
            device.stored().get(ConfigKey::Data),
            Some(&ConfigValue::Data(PathBuf::from("/home/me/ledgers")))
        );
    }

    #[test]
    fn a_refused_data_folder_says_why_and_changes_nothing() {
        let mut general = general_on(with_data_folder(InMemoryDeviceConfiguration::default()));

        let (outcome, router) = pick(&mut general, "/home/me/finance");

        assert_eq!(outcome, DataFolderOutcome::NotWritten);
        assert_eq!(general.data_folder(), Path::new("/home/me/ledgers"));
        assert_eq!(router.statuses().len(), 1);
        assert!(router.statuses()[0].starts_with("data can't be saved"));
    }

    #[test]
    fn a_shadowed_data_folder_is_written_and_names_the_override() {
        let device = with_data_folder(InMemoryDeviceConfiguration::shadowing(
            ConfigKey::Data,
            "PERSONAL_LEDGER_GENERAL__DATA",
        ));
        let mut general = general_on(device.clone());
        let overridden = "PERSONAL_LEDGER_GENERAL__DATA sets the Data folder to /home/me/ledgers for \
                          now, so :open and :new start there. Change… keeps your choice for when \
                          it's gone.";
        let note = |general: &GeneralPresenter| {
            general
                .view_model()
                .fields
                .view(GeneralField::DataFolder)
                .and_then(|view| view.own_note.clone())
        };
        assert_eq!(note(&general).as_deref(), Some(overridden));

        let (outcome, router) = pick(&mut general, "/home/me/finance");

        assert_eq!(outcome, DataFolderOutcome::SavedOverridden);
        assert_eq!(
            device.stored().get(ConfigKey::Data),
            Some(&ConfigValue::Data(PathBuf::from("/home/me/finance")))
        );
        assert_eq!(
            general.data_folder(),
            Path::new("/home/me/ledgers"),
            "the override still decides the folder in effect"
        );
        assert_eq!(
            general.view_model().data_folder.value,
            "/home/me/finance",
            "the file's value"
        );
        assert_eq!(note(&general).as_deref(), Some(overridden));
        assert_eq!(router.statuses(), ["data saved to personal-ledger.conf"]);
    }

    #[test]
    fn only_an_applied_data_folder_is_in_effect() {
        assert!(DataFolderOutcome::Applied.in_effect());
        assert!(!DataFolderOutcome::Unchanged.in_effect());
        assert!(!DataFolderOutcome::SavedOverridden.in_effect());
        assert!(!DataFolderOutcome::NotWritten.in_effect());
    }

    #[test]
    fn the_data_folder_and_this_device_show_what_the_device_reads() {
        let device = InMemoryDeviceConfiguration {
            file: Some(PathBuf::from("/home/me/.config/personal-ledger.conf")),
            ..InMemoryDeviceConfiguration::writable()
        }
        .reading([ConfigValue::Data(PathBuf::from("/home/me/Ledgers"))]);
        let mut general = general_on(Rc::new(device));
        general.set_last_ledger(Some(PathBuf::from("/home/me/Ledgers/home.ledger")));

        let view = general.view_model();
        assert_eq!(general.locale().line, "en-AU, from config");
        assert_eq!(general.data_folder(), Path::new("/home/me/Ledgers"));
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
}
