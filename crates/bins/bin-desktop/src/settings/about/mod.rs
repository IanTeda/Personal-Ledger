//! The Settings › About page (ADR-0037's amendment, one folder per page): static project info.
//! `lib-settings`' About Presenter, given the version from this crate's `env!`, is the
//! [`PageView`] the generic `ModulePage` hosts (ADR-0056). It has no events, so the page only
//! draws (`render.rs`).

mod render;

use gpui::{AnyElement, Context, Window};
use lib_database::Domain;
use lib_settings::{AboutPresenter, AboutSetup, AboutViewModel};
use lib_viper::keymap::KeyBindingConfig;

use crate::settings::SettingsPage;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::{PageStatus, StatusRight};

/// The About page: the Presenter in the generic page Entity.
pub type AboutPage = ModulePage<AboutPresenter>;

/// The About page for a build of `version`, its keys built from `config`.
pub fn about_page(
    version: &str,
    config: &KeyBindingConfig,
    cx: &mut Context<'_, AboutPage>,
) -> AboutPage {
    let presenter = AboutPresenter::new(
        AboutSetup {
            version: version.to_string(),
        },
        config,
    );
    ModulePage::detached(presenter, (), cx)
}

impl PageView for AboutPresenter {
    type Setup = ();
    type Widgets = ();
    type Before = ();

    /// About reads no Ledger domain.
    const REPAINTS: &'static [Domain] = &[];

    /// The frame scrolls the page: it has no selection.
    const FRAME_SCROLLS: bool = true;

    /// About takes no event, so its Router is the status line's queue, which nothing uses.
    fn with_router<R>(
        presenter: &mut Self,
        _setup: &(),
        _cx: &mut Context<'_, AboutPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut crate::notices::Queue)
    }

    fn render(
        view: &AboutViewModel,
        _page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, AboutPage>,
    ) -> AnyElement {
        render::page(view, cx)
    }

    fn status(view: &AboutViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: StatusRight::of_settings_page(SettingsPage::About),
        }
    }
}

impl SettingsPageView for AboutPresenter {
    const PAGE: SettingsPage = SettingsPage::About;
}
