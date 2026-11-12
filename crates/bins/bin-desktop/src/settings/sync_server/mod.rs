//! The Settings › Sync server page (ADR-0037's amendment, one folder per page): `lib-settings`' Sync
//! server Presenter is the [`PageView`] the generic `ModulePage` hosts (ADR-0056), drawn from its
//! view model through `PageClicks` (`render.rs`). "Sync now" is permanently out of scope for this
//! map, so its event only says so on the status line, as a Notice.

mod render;

use gpui::{AnyElement, Context, Window};
use lib_database::Domain;
use lib_settings::{SyncServerPresenter, SyncServerViewModel};
use lib_viper::keymap::KeyBindingConfig;

use crate::announcer::Announcer;
use crate::settings::SettingsPage;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::{PageStatus, StatusRight};

/// The Sync server page: the Presenter in the generic page Entity.
pub type SyncServerPage = ModulePage<SyncServerPresenter>;

/// The page, its keys built from `config`.
pub fn sync_server_page(
    config: &KeyBindingConfig,
    announcer: &gpui::Entity<Announcer>,
    cx: &mut Context<'_, SyncServerPage>,
) -> SyncServerPage {
    let presenter = SyncServerPresenter::new(config);
    ModulePage::new(presenter, (), announcer, cx)
}

impl PageView for SyncServerPresenter {
    type Setup = ();
    type Widgets = ();
    type Before = ();

    /// The page reads no Ledger domain.
    const REPAINTS: &'static [Domain] = &[];

    /// The frame scrolls the page: it has no selection.
    const FRAME_SCROLLS: bool = true;

    fn with_router<R>(
        presenter: &mut Self,
        _setup: &(),
        _cx: &mut Context<'_, SyncServerPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut crate::notices::Queue)
    }

    fn render(
        view: &SyncServerViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, SyncServerPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, cx)
    }

    fn status(view: &SyncServerViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: StatusRight::of_settings_page(SettingsPage::SyncServer),
        }
    }
}

impl SettingsPageView for SyncServerPresenter {
    const PAGE: SettingsPage = SettingsPage::SyncServer;
}
