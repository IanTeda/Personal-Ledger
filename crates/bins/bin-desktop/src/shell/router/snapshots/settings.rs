//! What the integration tests read back of the Settings pages, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

#[doc(hidden)]
pub use crate::accounts::SettingsAccountsSnapshot;
#[doc(hidden)]
pub use crate::institutions::SettingsInstitutionsSnapshot;
#[doc(hidden)]
pub use crate::inventory::SettingsInventorySnapshot;
#[doc(hidden)]
pub use crate::payees::SettingsPayeesSnapshot;
#[doc(hidden)]
pub use crate::settings::documents::SettingsDocumentTypesSnapshot;
#[doc(hidden)]
pub use crate::settings::general::SettingsGeneralSnapshot;
#[doc(hidden)]
pub use crate::settings::snapshot::{PageSnapshot, SettingsDisplaySnapshot, SettingsSnapshot};
#[doc(hidden)]
pub use crate::settings::tracing::SettingsTracingSnapshot;
use crate::shell::router::Root;
#[doc(hidden)]
pub use crate::tags::SettingsTagsSnapshot;
#[doc(hidden)]
pub use crate::units::{SettingsUnitsSnapshot, UnitPricesSnapshot};

pub use crate::categories::SettingsCategoriesSnapshot;

impl Root {
    /// The Settings shell for a test: the page on show, as the View builds it, and the open
    /// dialog.
    #[doc(hidden)]
    pub fn settings_snapshot(&self, cx: &gpui::App) -> SettingsSnapshot {
        SettingsSnapshot {
            status_message: self.status_message(),
            ..self.views.settings.read(cx).snapshot()
        }
    }

    /// A Settings page's snapshot for a test, read from the page `S` names, as the View builds it.
    #[doc(hidden)]
    pub fn settings_page_snapshot<S: PageSnapshot>(&self, cx: &gpui::App) -> S {
        S::read(self.views.settings.read(cx).page::<S::Page>().read(cx), cx)
    }
}
