//! Settings › General (DESKTOP-177, DESKTOP-178), the first THIS DEVICE page: On start, Start view
//! (#983), Toasts (moved from Display), the read-only Locale and Ledger folder, and the THIS DEVICE
//! box.
//! Everything here belongs to this device and works with no Ledger open (ADR-0051).
//!
//! On start, Start view and Toasts are Configuration in the user's `personal-ledger.conf`
//! (ADR-0053): a control asks `Root` for the change as [`ViewEvent::DeviceSetting`], and `Root` saves it, then
//! sets it back on the page. A key a source above the file sets says so under its control. The
//! Locale and `data` are Configuration read at start-up, so they are shown, not set.

mod render;

use std::path::PathBuf;

use gpui::{App, Context, EventEmitter, Keystroke};
use lib_core::{OnStart, StartView};
use lib_shell::Hint;

use crate::{
    settings::{
        SettingsPage,
        device::DeviceSetting,
        hints::settings_general_hints,
        pages::{Page, PageRowClicked},
        snapshot::PageSnapshot,
        step_choice,
    },
    shell::view::statusline::owned_hints,
    start_view,
    view::event::ViewEvent,
};

/// On start's choices, in the segmented control's order.
pub const ON_START_CHOICES: [OnStart; 2] = [OnStart::ReopenLastLedger, OnStart::NoLedger];

/// Toasts' choices, On then Off as on the control.
pub(super) const TOASTS_CHOICES: [bool; 2] = [true, false];

/// The page's controls, top to bottom. The Locale and Ledger folder are read-only, so the
/// keyboard passes them by.
pub const GENERAL_FIELD_ON_START: usize = 0;
pub const GENERAL_FIELD_START_VIEW: usize = 1;
pub const GENERAL_FIELD_TOASTS: usize = 2;
const GENERAL_FIELD_COUNT: usize = 3;

/// What start-up knows about this device that General shows read-only: the Ledger folder (the
/// `data` setting), the Ledger On start would reopen and the config file Settings writes to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThisDevice {
    pub ledger_folder: PathBuf,
    /// The Ledger file open now, which the state file remembers at quit; `None` with none open.
    pub last_ledger: Option<PathBuf>,
    /// The user's `personal-ledger.conf`; `None` with no home directory to keep one in.
    pub configuration: Option<PathBuf>,
}

impl EventEmitter<ViewEvent> for GeneralPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for GeneralPage {}

/// The General page: the device's settings its controls set, what it shows of this device, and the
/// keyboard focus on its controls.
#[derive(Debug)]
pub struct GeneralPage {
    on_start: OnStart,
    start_view: StartView,
    toasts_on: bool,
    device: ThisDevice,
    /// The control the keyboard is on, by its `GENERAL_FIELD_*` index.
    field_focus: Option<usize>,
    /// Whether the page holds keyboard focus, told by `set_page_focused`.
    page_focused: bool,
}

impl Default for GeneralPage {
    fn default() -> Self {
        Self {
            on_start: OnStart::default(),
            start_view: StartView::default(),
            toasts_on: lib_toast::Display::default().toasts_on,
            device: ThisDevice::default(),
            field_focus: None,
            page_focused: false,
        }
    }
}

impl GeneralPage {
    pub fn on_start(&self) -> OnStart {
        self.on_start
    }

    pub fn toasts_on(&self) -> bool {
        self.toasts_on
    }

    /// Sets On start once `Root` has saved it, or as start-up read it.
    pub fn set_on_start(&mut self, on_start: OnStart, cx: &mut Context<'_, Self>) {
        self.on_start = on_start;
        cx.notify();
    }

    /// Sets Start view once `Root` has saved it, or as start-up read it.
    pub fn set_start_view(&mut self, start_view: StartView, cx: &mut Context<'_, Self>) {
        self.start_view = start_view;
        cx.notify();
    }

    /// Sets Toasts once `Root` has saved them, from this page or `:toasts`, or as start-up read
    /// them.
    pub fn set_toasts_on(&mut self, on: bool, cx: &mut Context<'_, Self>) {
        self.toasts_on = on;
        cx.notify();
    }

    pub fn set_this_device(&mut self, device: ThisDevice, cx: &mut Context<'_, Self>) {
        self.device = device;
        cx.notify();
    }

    /// Sets the Ledger file the state file will remember at quit, once `:open` has opened one.
    pub fn set_last_ledger(&mut self, last_ledger: Option<PathBuf>, cx: &mut Context<'_, Self>) {
        self.device.last_ledger = last_ledger;
        cx.notify();
    }

    fn click_on_start(&mut self, on_start: OnStart, cx: &mut Context<'_, Self>) {
        if on_start != self.on_start {
            cx.emit(ViewEvent::DeviceSetting(DeviceSetting::OnStart(on_start)));
        }
    }

    /// Asks for the Start view `key` names, one of [`start_view::CHOICES`].
    fn click_start_view(&mut self, key: &'static str, cx: &mut Context<'_, Self>) {
        if key != self.start_view.key() {
            cx.emit(ViewEvent::DeviceSetting(DeviceSetting::StartView(key)));
        }
    }

    fn click_toasts(&mut self, on: bool, cx: &mut Context<'_, Self>) {
        if on != self.toasts_on {
            cx.emit(ViewEvent::DeviceSetting(DeviceSetting::Toasts(on)));
        }
    }

    /// The page's form keys: `j`/`k` walk the controls, stopping at either end, and `h`/`l` step
    /// the focused one's choice.
    fn field_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let Some(field) = self.field_focus else {
            return false;
        };
        if keystroke.modifiers.shift {
            return false;
        }
        let delta = match keystroke.key.as_str() {
            "j" | "down" => {
                self.field_focus = Some((field + 1).min(GENERAL_FIELD_COUNT - 1));
                cx.notify();
                return true;
            }
            "k" | "up" => {
                self.field_focus = Some(field.saturating_sub(1));
                cx.notify();
                return true;
            }
            "h" | "left" => -1,
            "l" | "right" => 1,
            _ => return false,
        };
        match field {
            GENERAL_FIELD_ON_START => {
                self.click_on_start(step_choice(&ON_START_CHOICES, self.on_start, delta), cx);
            }
            GENERAL_FIELD_START_VIEW => {
                // A noun from the file that isn't a choice steps from the Dashboard.
                let current = start_view::CHOICES
                    .into_iter()
                    .find(|choice| *choice == self.start_view.key())
                    .unwrap_or(StartView::DASHBOARD);
                self.click_start_view(step_choice(&start_view::CHOICES, current, delta), cx);
            }
            _ => self.click_toasts(step_choice(&TOASTS_CHOICES, self.toasts_on, delta), cx),
        }
        true
    }
}

impl Page for GeneralPage {
    const PAGE: SettingsPage = SettingsPage::General;

    /// The page's keys while it has focus, ahead of Settings' focus keys so its `h`/`l` win.
    fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        self.field_key(keystroke, cx)
    }

    /// Focus arriving puts the keyboard on On start; focus leaving takes it off the controls. The
    /// View tells the page on every draw, so only a change moves the focus.
    fn set_page_focused(&mut self, focused: bool, cx: &mut Context<'_, Self>) {
        if focused == self.page_focused {
            return;
        }
        self.page_focused = focused;
        self.field_focus = focused.then_some(GENERAL_FIELD_ON_START);
        cx.notify();
    }

    fn hints(&self, _cx: &App) -> Vec<Hint> {
        owned_hints(settings_general_hints())
    }
}

/// The General page for a test: the focused control, each Preference, and what it shows
/// read-only.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsGeneralSnapshot {
    pub field: Option<usize>,
    pub on_start: OnStart,
    /// Start view as the file spells it.
    pub start_view: String,
    pub toasts_on: bool,
    pub ledger_folder: String,
    pub last_ledger: Option<String>,
    pub configuration: Option<String>,
}

impl PageSnapshot for SettingsGeneralSnapshot {
    type Page = GeneralPage;

    fn read(page: &GeneralPage, _cx: &App) -> Self {
        let path = |path: &PathBuf| path.display().to_string();
        SettingsGeneralSnapshot {
            field: page.field_focus,
            on_start: page.on_start,
            start_view: page.start_view.key().to_string(),
            toasts_on: page.toasts_on,
            ledger_folder: path(&page.device.ledger_folder),
            last_ledger: page.device.last_ledger.as_ref().map(path),
            configuration: page.device.configuration.as_ref().map(path),
        }
    }
}
