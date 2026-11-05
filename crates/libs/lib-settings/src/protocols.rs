//! What the Settings module asks of the Shell and of the device, through ports each Client
//! implements. [`SettingsRouter`] is the frame's and [`DeviceRouter`] the This device pages', each
//! lent to every `handle` call; a refusal or a saved setting goes to the status line through
//! [`Notify`] (ADR-0048). [`DeviceConfiguration`] writes a device's settings, Configuration in the
//! user's configuration file (ADR-0053), which the Client answers over its own configuration writer.

use lib_colour_theme::ColourAppearance;
use lib_core::{DateStyle, OnStart, StartView};
use lib_toast::Notify;

use crate::page::SettingsPage;
use crate::presenter::DisplayOutput;

/// What only the Shell does for the Settings frame.
pub trait SettingsRouter: Notify {
    /// The page on show changed by the person's hand (a key, a click, a command), so the Shell
    /// records it in its history (`NavInput::SettingsPage`) for Back and Forward. A restored page
    /// is not announced, since the Shell restored it.
    fn page_changed(&mut self, page: SettingsPage);

    /// The page picker opened, so the Shell's input mode becomes the picker's and every key
    /// reaches the frame until it closes.
    fn picker_opened(&mut self);

    /// The page picker closed, so the Shell's input mode returns to normal.
    fn picker_closed(&mut self);
}

/// What only the Shell does once a This device page has changed a setting: the Toasts, the colours
/// and the Views that draw with the Display output are the Client's.
pub trait DeviceRouter: Notify {
    /// Toasts were turned on or off and saved, so the Shell's Toasts take it.
    fn toasts_changed(&mut self, on: bool);

    /// The Display output changed, so the Shell hands it to every View that draws with it.
    fn display_changed(&mut self, display: DisplayOutput);

    /// A Colour Theme or Appearance was saved, so the Client applies it to every window.
    fn colour_changed(&mut self, colour: ColourChoice);
}

/// A Colour Theme or Appearance chosen on Display or by the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourChoice {
    /// A built-in Colour Theme, by its id.
    Theme(&'static str),
    Appearance(ColourAppearance),
}

/// A device's setting that is Configuration, by the key the configuration file spells it with,
/// which the status line and a control's shadowed note name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKey {
    OnStart,
    StartView,
    Toasts,
    DateStyle,
    ColourTheme,
    ColourAppearance,
}

impl DeviceKey {
    pub const ALL: [DeviceKey; 6] = [
        Self::OnStart,
        Self::StartView,
        Self::Toasts,
        Self::DateStyle,
        Self::ColourTheme,
        Self::ColourAppearance,
    ];

    /// The key as the configuration file spells it, the TUI's `:set` and the environment variable
    /// naming it too.
    pub fn name(self) -> &'static str {
        match self {
            Self::OnStart => "on_start",
            Self::StartView => "start_view",
            Self::Toasts => "toasts",
            Self::DateStyle => "date_style",
            Self::ColourTheme => "colour_theme",
            Self::ColourAppearance => "colour_appearance",
        }
    }
}

/// How a write of a device's setting went: an answer for the person, never a failure for the
/// caller, so the port stays infallible (ADR-0046).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceAnswer<T> {
    /// Written: the value now in the file, and the file, as the status line names it.
    Saved { value: T, file: String },
    /// Not written, since a source above the file sets the key: an environment variable or a
    /// higher configuration file, named by `source`.
    Shadowed { source: String },
    /// Not written, for the reason given.
    Refused(DeviceRefusal),
}

/// Why a device's setting the file would take wasn't written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceRefusal {
    /// There is no home folder to keep the file in.
    NoFile,
    /// The file or its folder couldn't be written; the Client logs the cause.
    WriteFailed,
}

/// Writes a device's settings to the user's configuration file, one typed method per setting, and
/// says which keys a source above the file sets, so a control can say so before it is changed.
pub trait DeviceConfiguration {
    /// What sets `key` above the file, by name, or `None` when a write to the file applies.
    fn shadow(&self, key: DeviceKey) -> Option<String>;

    fn set_on_start(&self, on_start: OnStart) -> DeviceAnswer<OnStart>;

    fn set_start_view(&self, start_view: StartView) -> DeviceAnswer<StartView>;

    fn set_toasts(&self, on: bool) -> DeviceAnswer<bool>;

    /// `None` is the Locale's default.
    fn set_date_style(&self, style: Option<DateStyle>) -> DeviceAnswer<Option<DateStyle>>;

    /// A built-in Colour Theme, by its id.
    fn set_colour_theme(&self, id: &'static str) -> DeviceAnswer<&'static str>;

    fn set_colour_appearance(&self, appearance: ColourAppearance)
    -> DeviceAnswer<ColourAppearance>;
}
