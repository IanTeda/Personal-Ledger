//! Settings › General's View: `lib-settings`' General Presenter is the [`PageView`] the generic
//! `ModulePage` hosts (ADR-0056), which the Settings View hosts as any other page. It names the
//! page's Router and how it draws the view model (`render.rs`); the keymap, the take, send and
//! handle path and the clicks' path are the generic page's. On start, Start view and Toasts are
//! written through the Presenter's `DeviceConfiguration` (`super::device`); saved Toasts reach the
//! Shell as `ViewEvent::ToastsChanged` (`super::router`). Each key and click is one input
//! (`crate::input::send`).

mod render;

use std::path::PathBuf;
use std::rc::Rc;

use gpui::{AnyElement, App, Context, Window};
use lib_core::StartView;
use lib_database::Domain;
use lib_settings::keymap::general_keymap;
use lib_settings::{
    DeviceConfiguration, GeneralPresenter, GeneralSetup, GeneralViewModel, LocaleLine,
};
use lib_viper::keymap::KeyBindingConfig;
use lib_viper::view::Choice;

use crate::settings::SettingsPage;
use crate::settings::router::DeviceRouter;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::{PageStatus, StatusRight};
use crate::start_view;

/// The General page: the Presenter in the generic page Entity. The Presenter holds the device's
/// settings, what it shows of this device and the keyboard focus on its controls.
pub type GeneralPage = ModulePage<GeneralPresenter>;

/// General writing to `device` under `config`'s keys, with the Shell's `back` on `back_chord`.
/// Start view's choices are the Desktop's own Views. General reads no Ledger, so it stands before
/// one is open.
pub fn general_page(
    device: Rc<dyn DeviceConfiguration>,
    config: &KeyBindingConfig,
    back_chord: String,
    cx: &mut Context<'_, GeneralPage>,
) -> GeneralPage {
    let (line, fallback) = crate::locale::describe(&crate::locale::info());
    let presenter = GeneralPresenter::new(GeneralSetup {
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
    });
    ModulePage::detached(presenter, (), cx)
}

impl PageView for GeneralPresenter {
    type Setup = ();
    type Widgets = ();
    type Before = ();

    /// General reads Configuration, not the Ledger.
    const REPAINTS: &'static [Domain] = &[];

    /// With no control focused, the frame scrolls the page.
    const FRAME_SCROLLS: bool = true;

    fn with_router<R>(
        presenter: &mut Self,
        _setup: &(),
        cx: &mut Context<'_, GeneralPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut DeviceRouter { cx })
    }

    fn render(
        view: &GeneralViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, GeneralPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, cx)
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn status(view: &GeneralViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: StatusRight::of_settings_page(SettingsPage::General),
        }
    }

    /// Focus arriving puts the keyboard on On start; focus leaving takes it off the controls.
    fn focus_changed(&mut self, focused: bool) -> bool {
        self.set_page_focused(focused)
    }
}

impl SettingsPageView for GeneralPresenter {
    const PAGE: SettingsPage = SettingsPage::General;
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
    type Presenter = GeneralPresenter;

    fn read(page: &GeneralPage, _cx: &App) -> Self {
        let path = |path: &PathBuf| path.display().to_string();
        let settings = page.presenter().settings();
        let device = page.presenter().this_device();
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
