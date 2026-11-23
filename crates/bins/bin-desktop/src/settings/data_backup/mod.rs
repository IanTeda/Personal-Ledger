//! The Settings › Data & backup page (ADR-0037's amendment, one folder per page): `lib-settings`'
//! Data & backup Presenter is the [`PageView`] the generic `ModulePage` hosts (ADR-0056), drawn
//! from its view model through `PageClicks` (`render.rs`). Both buttons are permanently out of
//! scope for this map, so each only says so on the status line, as a Notice. The page also shows
//! the encryption key's fingerprint and opens the Remove all API keys confirm (`dialogs/`, #1392).

pub mod dialogs;
mod render;

use std::rc::Rc;

use gpui::{AnyElement, Context, Window};
use lib_database::Domain;
use lib_settings::{ApiKeys, DataBackupPresenter, DataBackupSetup, DataBackupViewModel};
use lib_viper::keymap::KeyBindingConfig;

use crate::announcer::Announcer;
use crate::settings::SettingsPage;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{
    ModulePage, PageDraw, PageLayout, PageView, SettingsPageView,
};
use crate::shell::view::page_router::page_router;
use crate::shell::view::statusline::{PageStatus, StatusRight};

/// The Data & backup page: the Presenter in the generic page Entity.
pub type DataBackupPage = ModulePage<DataBackupPresenter>;

/// The page over `keys`, its keys built from `config`.
pub fn data_backup_page(
    keys: Rc<dyn ApiKeys>,
    config: &KeyBindingConfig,
    announcer: &gpui::Entity<Announcer>,
    cx: &mut Context<'_, DataBackupPage>,
) -> DataBackupPage {
    let presenter = DataBackupPresenter::new(DataBackupSetup { keys }, config);
    ModulePage::new(presenter, (), announcer, cx)
}

impl PageView for DataBackupPresenter {
    type Setup = ();
    type Widgets = ();
    type Before = ();

    /// The page reads no Ledger domain.
    const REPAINTS: &'static [Domain] = &[];

    /// The frame scrolls the page: it has no selection.
    fn layout() -> PageLayout {
        PageLayout {
            frame_scrolls: true,
            ..PageLayout::default()
        }
    }

    page_router!();

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
            right: StatusRight::of_settings_page(SettingsPage::DataBackup),
        }
    }
}

impl SettingsPageView for DataBackupPresenter {
    const PAGE: SettingsPage = SettingsPage::DataBackup;
}

impl PageSnapshot for SettingsDataBackupSnapshot {
    type Presenter = DataBackupPresenter;

    /// The page's rows as drawn: the encryption key's fact is read from the keys port.
    fn read(page: &DataBackupPage, _cx: &gpui::App) -> Self {
        let view = page.view_model();
        SettingsDataBackupSnapshot {
            rows: view
                .rows
                .into_iter()
                .map(|row| (row.label, row.value))
                .collect(),
        }
    }
}

/// The Data & backup page: each row's label and value, as drawn.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDataBackupSnapshot {
    pub rows: Vec<(String, String)>,
}

impl SettingsDataBackupSnapshot {
    /// The value of the row labelled `label`.
    #[must_use]
    pub fn value(&self, label: &str) -> Option<&str> {
        self.rows
            .iter()
            .find_map(|(row, value)| (row == label).then_some(value.as_str()))
    }
}
