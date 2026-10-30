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
pub use crate::settings::snapshot::{SettingsDisplaySnapshot, SettingsSnapshot};
#[doc(hidden)]
pub use crate::settings::tracing::SettingsTracingSnapshot;
use crate::shell::router::Root;
#[doc(hidden)]
pub use crate::tags::SettingsTagsSnapshot;
#[doc(hidden)]
pub use crate::units::{SettingsUnitsSnapshot, UnitPricesSnapshot};
use crate::{
    accounts::AccountsPage, categories::CategoriesPage, institutions::InstitutionsPage,
    inventory::InventoryPage, payees::PayeesPage, settings::documents::DocumentsPage,
    settings::general::GeneralPage, settings::tracing::TracingPage, tags::TagsPage,
    units::UnitsPage,
};

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

    /// The Settings Accounts page for a test.
    #[doc(hidden)]
    pub fn settings_accounts_snapshot(&self, cx: &gpui::App) -> SettingsAccountsSnapshot {
        self.views
            .settings
            .read(cx)
            .page::<AccountsPage>()
            .read(cx)
            .snapshot(cx)
    }

    /// The Settings Categories page for a test.
    #[doc(hidden)]
    pub fn settings_categories_snapshot(&self, cx: &gpui::App) -> SettingsCategoriesSnapshot {
        self.views
            .settings
            .read(cx)
            .page::<CategoriesPage>()
            .read(cx)
            .snapshot(cx)
    }

    /// The Settings Payees page for a test.
    #[doc(hidden)]
    pub fn settings_payees_snapshot(&self, cx: &gpui::App) -> SettingsPayeesSnapshot {
        self.views
            .settings
            .read(cx)
            .page::<PayeesPage>()
            .read(cx)
            .snapshot(cx)
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
            .page::<DocumentsPage>()
            .read(cx)
            .snapshot(cx)
    }

    /// The Settings Inventory page for a test.
    #[doc(hidden)]
    pub fn settings_inventory_snapshot(&self, cx: &gpui::App) -> SettingsInventorySnapshot {
        self.views
            .settings
            .read(cx)
            .page::<InventoryPage>()
            .read(cx)
            .snapshot(cx)
    }

    /// The Settings Tags page for a test.
    #[doc(hidden)]
    pub fn settings_tags_snapshot(&self, cx: &gpui::App) -> SettingsTagsSnapshot {
        self.views
            .settings
            .read(cx)
            .page::<TagsPage>()
            .read(cx)
            .snapshot(cx)
    }

    /// The Settings General page for a test.
    #[doc(hidden)]
    pub fn settings_general_snapshot(&self, cx: &gpui::App) -> SettingsGeneralSnapshot {
        self.views
            .settings
            .read(cx)
            .page::<GeneralPage>()
            .read(cx)
            .snapshot(cx)
    }

    /// The Settings Units page for a test, as the View builds it.
    #[doc(hidden)]
    pub fn settings_units_snapshot(&self, cx: &gpui::App) -> SettingsUnitsSnapshot {
        self.views
            .settings
            .read(cx)
            .page::<UnitsPage>()
            .read(cx)
            .view_model(cx)
    }

    /// The Settings Units page's Unit prices view for a test; `None` while it is closed.
    #[doc(hidden)]
    pub fn unit_prices_snapshot(&self, cx: &gpui::App) -> Option<UnitPricesSnapshot> {
        self.views
            .settings
            .read(cx)
            .page::<UnitsPage>()
            .read(cx)
            .prices_view_model()
    }

    /// The Settings Institutions page for a test, as the View builds it.
    #[doc(hidden)]
    pub fn settings_institutions_snapshot(&self, cx: &gpui::App) -> SettingsInstitutionsSnapshot {
        self.views
            .settings
            .read(cx)
            .page::<InstitutionsPage>()
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
        self.views
            .settings
            .read(cx)
            .page::<TracingPage>()
            .read(cx)
            .snapshot()
    }
}
