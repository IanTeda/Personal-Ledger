//! The Settings › Data & backup page (ADR-0037's amendment, one folder per page): `lib-settings`'
//! Data & backup Presenter is the [`PageView`] the generic `ModulePage` hosts (ADR-0056), drawn
//! from its view model through `PageClicks` (`render.rs`). Both buttons are permanently out of
//! scope for this map, so each only says so on the status line, as a Notice.

mod render;

use gpui::{AnyElement, Context, Window};
use lib_database::Domain;
use lib_settings::{DataBackupPresenter, DataBackupViewModel, SettingsFrameEvent};
use lib_viper::keymap::Keymap;

use crate::announcer::Announcer;
use crate::settings::SettingsPage;
use crate::settings::hints::settings_plain_page_hints;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::PageStatus;

/// The Data & backup page: the Presenter in the generic page Entity.
pub type DataBackupPage = ModulePage<DataBackupPresenter>;

/// The page, its legend read from the frame's `keymap`.
pub fn data_backup_page(
    keymap: &Keymap<SettingsFrameEvent>,
    announcer: &gpui::Entity<Announcer>,
    cx: &mut Context<'_, DataBackupPage>,
) -> DataBackupPage {
    let mut presenter = DataBackupPresenter::new();
    presenter.set_legend(settings_plain_page_hints(keymap));
    ModulePage::new(presenter, (), announcer, cx)
}

impl PageView for DataBackupPresenter {
    type Setup = ();
    type Widgets = ();
    type Before = ();

    /// The page reads no Ledger domain.
    const REPAINTS: &'static [Domain] = &[];

    fn with_router<R>(
        presenter: &mut Self,
        _setup: &(),
        _cx: &mut Context<'_, DataBackupPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut crate::notices::Queue)
    }

    fn render(
        view: &DataBackupViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, DataBackupPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, cx)
    }

    fn status(view: &DataBackupViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: crate::settings::scope_note(SettingsPage::DataBackup),
        }
    }
}

impl SettingsPageView for DataBackupPresenter {
    const PAGE: SettingsPage = SettingsPage::DataBackup;

    /// The frame scrolls the page: it has no selection.
    const FRAME_SCROLLS: bool = true;
}
