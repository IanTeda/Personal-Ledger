//! What the integration tests read back of the Settings pages, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

use crate::app::Shell;
#[doc(hidden)]
pub use crate::settings::accounts::snapshot::SettingsAccountsSnapshot;
#[doc(hidden)]
pub use crate::settings::documents::SettingsDocumentTypesSnapshot;
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
#[doc(hidden)]
pub use crate::settings::units::SettingsUnitsSnapshot;
use crate::{
    settings::accounts::dialogs::AccountsDialog, settings::documents::dialogs::DocumentTypesDialog,
    settings::inventory::dialogs::InventoryDialog, settings::payees::dialogs::PayeesDialog,
    settings::tracing::dialogs::TracingDialog,
};

pub use crate::settings::categories::SettingsCategoriesSnapshot;

/// What the open Settings dialog shows, read once for the snapshots that report it.
#[derive(Default)]
struct OpenDialog {
    label: Option<&'static str>,
    name: Option<String>,
    confirm: Option<String>,
}

/// The open Add or Edit account dialog's form: plain values, so the private form type stays
/// private.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountFormSnapshot {
    /// The focused field, as its `AccountField` variant name.
    pub focused: String,
    pub name: String,
    pub institution: Option<String>,
    pub account_type: Option<String>,
    pub unit: Option<String>,
    pub opening_balance: String,
    pub account_number: String,
    /// Whether any select's option list is showing.
    pub select_open: bool,
    /// Whether the form would confirm.
    pub valid: bool,
}

impl Shell {
    /// The open Add or Edit account dialog's form for a test; `None` for any other dialog.
    #[doc(hidden)]
    pub fn account_form_snapshot(&self) -> Option<AccountFormSnapshot> {
        let form = self.dialog::<AccountsDialog>()?.form()?;
        Some(AccountFormSnapshot {
            focused: format!("{:?}", form.focused),
            name: form.name.text().to_string(),
            institution: form.institution.value().map(str::to_string),
            account_type: form.account_type.value().map(str::to_string),
            unit: form.unit.value().map(str::to_string),
            opening_balance: form.opening_balance.text().to_string(),
            account_number: form.account_number.text().to_string(),
            select_open: form.any_select_open(),
            valid: form.is_valid(),
        })
    }

    /// The open Settings dialog, whichever page owns it.
    fn open_settings_dialog(&self) -> OpenDialog {
        let mut open_dialog = OpenDialog::default();
        if let Some(open) = self.dialog::<AccountsDialog>() {
            let (label, name, confirm) = match open {
                AccountsDialog::Add(form) => {
                    ("AddAccount", Some(form.name.text().to_string()), None)
                }
                AccountsDialog::Edit(_, form) => {
                    ("EditAccount", Some(form.name.text().to_string()), None)
                }
                AccountsDialog::Delete(_, form) => (
                    "DeleteAccount",
                    None,
                    Some(form.confirm_input.text().to_string()),
                ),
            };
            open_dialog.label = Some(label);
            open_dialog.name = name;
            open_dialog.confirm = confirm;
        } else if let Some(open) = self.dialog::<PayeesDialog>() {
            let (label, name, confirm) = match open {
                PayeesDialog::Add(form) => ("AddPayee", Some(form.name.text().to_string()), None),
                PayeesDialog::Edit(_, form) => {
                    ("EditPayee", Some(form.name.text().to_string()), None)
                }
                PayeesDialog::Delete(_, form) => (
                    "DeletePayee",
                    None,
                    Some(form.confirmation_name.text().to_string()),
                ),
            };
            open_dialog.label = Some(label);
            open_dialog.name = name;
            open_dialog.confirm = confirm;
        } else if let Some(open) = self.dialog::<TracingDialog>() {
            let (label, name, confirm) = match open {
                TracingDialog::ClearLogs => ("ClearLogs", None, None),
            };
            open_dialog.label = Some(label);
            open_dialog.name = name;
            open_dialog.confirm = confirm;
        }
        open_dialog
    }

    /// The Settings shell for a test: the page on show, as the View builds it, and the open
    /// dialog.
    #[doc(hidden)]
    pub fn settings_snapshot(&self, cx: &gpui::App) -> SettingsSnapshot {
        let dialog = self.open_settings_dialog();
        SettingsSnapshot {
            dialog: dialog.label.map(str::to_string),
            dialog_name: dialog.name,
            dialog_confirm: dialog.confirm,
            status_message: self.chrome.status_message.clone(),
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
        let page = self
            .views
            .settings
            .read(cx)
            .documents()
            .read(cx)
            .snapshot(cx);
        SettingsDocumentTypesSnapshot {
            document_type_dialog: self
                .dialog::<DocumentTypesDialog>()
                .map(|dialog| dialog.kind().to_string()),
            ..page
        }
    }

    /// The Settings Inventory page for a test.
    #[doc(hidden)]
    pub fn settings_inventory_snapshot(&self, cx: &gpui::App) -> SettingsInventorySnapshot {
        SettingsInventorySnapshot {
            inventory_dialog: self
                .dialog::<InventoryDialog>()
                .map(|dialog| dialog.kind().to_string()),
            ..self.inventory_page(cx).read(cx).snapshot(cx)
        }
    }

    /// The Settings Tags page for a test.
    #[doc(hidden)]
    pub fn settings_tags_snapshot(&self, cx: &gpui::App) -> SettingsTagsSnapshot {
        self.views.settings.read(cx).tags().read(cx).snapshot(cx)
    }

    /// The Settings Units page for a test, as the View builds it.
    #[doc(hidden)]
    pub fn settings_units_snapshot(&self, cx: &gpui::App) -> SettingsUnitsSnapshot {
        self.views.settings.read(cx).units().read(cx).snapshot(cx)
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
