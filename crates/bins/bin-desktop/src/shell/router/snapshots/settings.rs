//! What the integration tests read back of the Settings pages, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

#[doc(hidden)]
pub use crate::settings::accounts::snapshot::SettingsAccountsSnapshot;
#[doc(hidden)]
pub use crate::settings::documents::SettingsDocumentTypesSnapshot;
#[doc(hidden)]
pub use crate::settings::general::SettingsGeneralSnapshot;
#[doc(hidden)]
pub use crate::settings::institutions::SettingsInstitutionsSnapshot;
#[doc(hidden)]
pub use crate::settings::inventory::SettingsInventorySnapshot;
#[doc(hidden)]
pub use crate::settings::payees::SettingsPayeesSnapshot;
#[doc(hidden)]
pub use crate::settings::snapshot::{SettingsDisplaySnapshot, SettingsSnapshot};
#[doc(hidden)]
pub use crate::settings::tags::SettingsTagsSnapshot;
#[doc(hidden)]
pub use crate::settings::tracing::SettingsTracingSnapshot;
use crate::shell::router::Root;
#[doc(hidden)]
pub use crate::units::SettingsUnitsSnapshot;

pub use crate::settings::categories::SettingsCategoriesSnapshot;

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

    /// The Settings Accounts page for a test.
    #[doc(hidden)]
    pub fn settings_accounts_snapshot(&self, cx: &gpui::App) -> SettingsAccountsSnapshot {
        self.views
            .settings
            .read(cx)
            .accounts()
            .read(cx)
            .snapshot(cx)
    }

    /// The Settings Categories page for a test.
    #[doc(hidden)]
    pub fn settings_categories_snapshot(&self, cx: &gpui::App) -> SettingsCategoriesSnapshot {
        self.views
            .settings
            .read(cx)
            .categories()
            .read(cx)
            .snapshot(cx)
    }

    /// The Settings Payees page for a test.
    #[doc(hidden)]
    pub fn settings_payees_snapshot(&self, cx: &gpui::App) -> SettingsPayeesSnapshot {
        self.views.settings.read(cx).payees().read(cx).snapshot(cx)
    }

    /// The Settings Document Types page for a test.
    #[doc(hidden)]
    pub fn settings_document_types_snapshot(
        &self,
        cx: &gpui::App,
    ) -> SettingsDocumentTypesSnapshot {
        self.views
            .settings
            .read(cx)
            .documents()
            .read(cx)
            .snapshot(cx)
    }

    /// The Settings Inventory page for a test.
    #[doc(hidden)]
    pub fn settings_inventory_snapshot(&self, cx: &gpui::App) -> SettingsInventorySnapshot {
        self.inventory_page(cx).read(cx).snapshot(cx)
    }

    /// The Settings Tags page for a test.
    #[doc(hidden)]
    pub fn settings_tags_snapshot(&self, cx: &gpui::App) -> SettingsTagsSnapshot {
        self.views.settings.read(cx).tags().read(cx).snapshot(cx)
    }

    /// The Settings General page for a test.
    #[doc(hidden)]
    pub fn settings_general_snapshot(&self, cx: &gpui::App) -> SettingsGeneralSnapshot {
        self.views.settings.read(cx).general().read(cx).snapshot(cx)
    }

    /// The Settings Units page for a test, as the View builds it.
    #[doc(hidden)]
    pub fn settings_units_snapshot(&self, cx: &gpui::App) -> SettingsUnitsSnapshot {
        self.views.settings.read(cx).units().read(cx).view_model(cx)
    }

    /// The Settings Institutions page for a test, as the View builds it.
    #[doc(hidden)]
    pub fn settings_institutions_snapshot(&self, cx: &gpui::App) -> SettingsInstitutionsSnapshot {
        self.views
            .settings
            .read(cx)
            .institutions()
            .read(cx)
            .snapshot(cx)
    }

    /// The Settings Display page for a test, as the View builds it.
    #[doc(hidden)]
    pub fn settings_display_snapshot(&self, cx: &gpui::App) -> SettingsDisplaySnapshot {
        self.views.settings.read(cx).display_snapshot(cx)
    }

    /// The Settings Tracing page for a test, as the View builds it.
    #[doc(hidden)]
    pub fn settings_tracing_snapshot(&self, cx: &gpui::App) -> SettingsTracingSnapshot {
        self.views.settings.read(cx).tracing().read(cx).snapshot()
    }
}
