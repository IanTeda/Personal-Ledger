//! Settings › General's View: `lib-settings`' General Presenter in a gpui Entity, which the
//! Settings View hosts as any other page. It looks up each key the Shell passes on in General's
//! keymap, turns clicks into events through `PageClicks` and draws the view model
//! (`render.rs`). On start, Start view and Toasts are written through the Presenter's
//! `DeviceConfiguration` (`super::device`); saved Toasts reach the Shell as
//! `ViewEvent::ToastsChanged` (`super::router`). Each key and click is one input
//! (`crate::input::send`).

mod render;

use std::path::PathBuf;
use std::rc::Rc;

use gpui::{App, Context, EventEmitter};
use lib_core::StartView;
use lib_settings::keymap::general_keymap;
use lib_settings::{
    DeviceConfiguration, GeneralEvent, GeneralPresenter, GeneralSettings, GeneralSetup,
    GeneralViewModel, LocaleLine, ThisDevice,
};
use lib_shell::Hint;
use lib_viper::keymap::KeyBindingConfig;
use lib_viper::presenter::{PagePresenter, Presenter};
use lib_viper::view::Choice;

use crate::{
    settings::{
        SettingsPage,
        pages::{Page, PageRowClicked},
        router::DeviceRouter,
        snapshot::PageSnapshot,
    },
    shell::view::page_clicks::PresenterPage,
    start_view,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for GeneralPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for GeneralPage {}

/// The General page: its Presenter, which holds the device's settings, what it shows of this
/// device and the keyboard focus on its controls.
pub struct GeneralPage {
    presenter: GeneralPresenter,
}

impl GeneralPage {
    /// General writing to `device` under `config`'s keys, with the Shell's `back` on
    /// `back_chord`. Start view's choices are the Desktop's own Views.
    pub fn new(
        device: Rc<dyn DeviceConfiguration>,
        config: &KeyBindingConfig,
        back_chord: String,
    ) -> Self {
        let (line, fallback) = crate::locale::describe(&crate::locale::info());
        Self {
            presenter: GeneralPresenter::new(GeneralSetup {
                device,
                keymap: general_keymap(config),
                back_chord,
                start_views: start_view::CHOICES
                    .into_iter()
                    .map(|key| Choice {
                        value: StartView::from_key(key),
                        label: start_view::label(key),
                    })
                    .collect(),
                locale: LocaleLine { line, fallback },
            }),
        }
    }

    /// The page's keys under `config`'s overrides.
    pub fn set_keymaps(&mut self, config: &KeyBindingConfig, back_chord: String) {
        self.presenter
            .set_keymap(general_keymap(config), back_chord);
    }

    /// Writes to `device` from now on, showing `settings` as start-up read them.
    pub fn configure(
        &mut self,
        device: Rc<dyn DeviceConfiguration>,
        settings: GeneralSettings,
        cx: &mut Context<'_, Self>,
    ) {
        self.presenter.configure(device, settings);
        cx.notify();
    }

    pub fn set_this_device(&mut self, device: ThisDevice, cx: &mut Context<'_, Self>) {
        self.presenter.set_this_device(device);
        cx.notify();
    }

    /// Sets the Ledger file the state file will remember at quit, once `:open` has opened one.
    pub fn set_last_ledger(&mut self, last_ledger: Option<PathBuf>, cx: &mut Context<'_, Self>) {
        self.presenter.set_last_ledger(last_ledger);
        cx.notify();
    }

    /// `:toasts on|off`, written as General writes it, inside the command's input.
    pub fn set_toasts(&mut self, on: bool, cx: &mut Context<'_, Self>) {
        self.send(GeneralEvent::SetToasts(on), cx);
    }

    pub fn view_model(&self) -> GeneralViewModel {
        self.presenter.view_model()
    }

    /// The settings, this device's facts and the Locale the X-Large index pane quotes.
    pub fn summary_sources(&self) -> (&GeneralSettings, &ThisDevice, &LocaleLine) {
        (
            self.presenter.settings(),
            self.presenter.this_device(),
            self.presenter.locale(),
        )
    }

    /// A key, click or command as one input, committed once it is handled; one sent inside
    /// another input (the palette's) runs in it.
    fn send(&mut self, event: GeneralEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
    }

    fn handle(&mut self, event: GeneralEvent, cx: &mut Context<'_, Self>) {
        self.presenter.handle(event, &mut DeviceRouter { cx });
        cx.notify();
    }
}

/// The page's clicks on its options.
impl PresenterPage<GeneralEvent> for GeneralPage {
    fn send(&mut self, event: GeneralEvent, cx: &mut Context<'_, Self>) {
        GeneralPage::send(self, event, cx);
    }
}

impl Page for GeneralPage {
    const PAGE: SettingsPage = SettingsPage::General;

    /// The keymap's chords while a control has the keyboard, ahead of the frame's, so its `h`/`l`
    /// win; the rest go on to the frame.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.presenter.keymap().event(chord) else {
            return false;
        };
        if !self.presenter.takes(&event) {
            return false;
        }
        self.send(event, cx);
        true
    }

    /// Focus arriving puts the keyboard on On start; focus leaving takes it off the controls.
    fn set_page_focused(&mut self, focused: bool, cx: &mut Context<'_, Self>) {
        if self.presenter.set_page_focused(focused) {
            cx.notify();
        }
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        self.presenter.view_model().hints
    }
}

/// The General page for a test: the focused control, each setting, and what it shows read-only.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsGeneralSnapshot {
    pub field: Option<usize>,
    pub on_start: lib_core::OnStart,
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
        let settings = page.presenter.settings();
        let device = page.presenter.this_device();
        SettingsGeneralSnapshot {
            field: page.view_model().field,
            on_start: settings.on_start,
            start_view: settings.start_view.key().to_string(),
            toasts_on: settings.toasts_on,
            ledger_folder: path(&device.ledger_folder),
            last_ledger: device.last_ledger.as_ref().map(path),
            configuration: device.configuration.as_ref().map(path),
        }
    }
}
