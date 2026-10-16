//! What the integration tests read back of the Settings pages, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

use crate::app::Shell;
#[doc(hidden)]
pub use crate::settings::accounts::snapshot::SettingsAccountsSnapshot;
use crate::settings::categories::dialogs::CategoriesDialog;
#[doc(hidden)]
pub use crate::settings::documents::SettingsDocumentTypesSnapshot;
#[doc(hidden)]
pub use crate::settings::institutions::SettingsInstitutionsSnapshot;
#[doc(hidden)]
pub use crate::settings::inventory::SettingsInventorySnapshot;
#[doc(hidden)]
pub use crate::settings::payees::SettingsPayeesSnapshot;
#[doc(hidden)]
pub use crate::settings::snapshot::{
    SettingsDisplaySnapshot, SettingsSnapshot, SettingsTracingSnapshot,
};
#[doc(hidden)]
pub use crate::settings::tags::SettingsTagsSnapshot;
#[doc(hidden)]
pub use crate::settings::units::SettingsUnitsSnapshot;
use crate::{
    settings::accounts::dialogs::AccountsDialog,
    settings::documents::dialogs::DocumentTypesDialog,
    settings::inventory::dialogs::InventoryDialog,
    settings::payees::dialogs::PayeesDialog,
    settings::tags::dialogs::TagsDialog,
    settings::{
        SettingsDialog, institutions::dialogs::InstitutionsDialog, units::dialogs::UnitsDialog,
    },
};

pub use crate::settings::categories::SettingsCategoriesSnapshot;

/// What the open Settings dialog shows, read once for the snapshots that report it.
#[derive(Default)]
struct OpenDialog {
    label: Option<&'static str>,
    name: Option<String>,
    confirm: Option<String>,
    merge_pair: Option<(String, String)>,
    /// The Add/Edit unit dialog's Name, Type and focused field.
    unit: Option<(String, String, String)>,
    /// The Add institution dialog's checked products and Default unit code.
    institution: Option<(Vec<String>, Option<String>)>,
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
    fn open_settings_dialog(&self, cx: &gpui::App) -> OpenDialog {
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
        } else if let Some(open) = self.dialog::<CategoriesDialog>() {
            let (label, name, confirm) = match open {
                CategoriesDialog::Add { form, .. } => {
                    ("AddCategory", Some(form.name.text().to_string()), None)
                }
                CategoriesDialog::Edit(_, form) => {
                    ("EditCategory", Some(form.name.text().to_string()), None)
                }
                CategoriesDialog::Delete(_, form) => (
                    "DeleteCategory",
                    None,
                    Some(form.confirmation_name.text().to_string()),
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
        } else if let Some(open) = self.dialog::<TagsDialog>() {
            let (label, name, confirm) = match open {
                TagsDialog::Add(form) => ("AddTag", Some(form.name.text().to_string()), None),
                TagsDialog::Edit(_, form) => ("EditTag", Some(form.name.text().to_string()), None),
                TagsDialog::Remove(_, form) => (
                    "RemoveTag",
                    None,
                    Some(form.confirmation_name.text().to_string()),
                ),
                TagsDialog::Merge(form) => {
                    open_dialog.merge_pair = form.pair().and_then(|(source, target)| {
                        Some((
                            lib_tags::get(self.ledger.read(cx).ledger().tags(), source)?
                                .name
                                .clone(),
                            lib_tags::get(self.ledger.read(cx).ledger().tags(), target)?
                                .name
                                .clone(),
                        ))
                    });
                    ("MergeTags", None, None)
                }
            };
            open_dialog.label = Some(label);
            open_dialog.name = name;
            open_dialog.confirm = confirm;
        } else if let Some(open) = self.dialog::<SettingsDialog>() {
            let (label, name, confirm) = match open {
                SettingsDialog::ClearLogs => ("ClearLogs", None, None),
            };
            open_dialog.label = Some(label);
            open_dialog.name = name;
            open_dialog.confirm = confirm;
        } else if let Some(open) = self.dialog::<InstitutionsDialog>() {
            let open = open.snapshot();
            open_dialog.label = Some(open.label);
            open_dialog.name = Some(open.name);
            open_dialog.institution = Some(open.form);
        } else if let Some(open) = self.dialog::<UnitsDialog>() {
            let open = open.snapshot();
            open_dialog.label = Some(open.label);
            open_dialog.name = open.code;
            open_dialog.confirm = open.confirm;
            open_dialog.unit = open.form;
        }
        open_dialog
    }

    /// The Settings shell for a test: the page on show, as the View builds it, and the open
    /// dialog.
    #[doc(hidden)]
    pub fn settings_snapshot(&self, cx: &gpui::App) -> SettingsSnapshot {
        let dialog = self.open_settings_dialog(cx);
        let parent_id = match self.dialog::<CategoriesDialog>() {
            Some(CategoriesDialog::Add { parent_id, .. }) => *parent_id,
            _ => None,
        };
        let dialog_parent = parent_id.and_then(|id| {
            self.categories(cx)
                .iter()
                .find(|category| category.id == id)
                .map(|category| category.name.clone())
        });
        SettingsSnapshot {
            dialog: dialog.label.map(str::to_string),
            dialog_name: dialog.name,
            dialog_confirm: dialog.confirm,
            dialog_parent,
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
        let page = self.views.settings.read(cx).tags().read(cx).snapshot(cx);
        SettingsTagsSnapshot {
            merge_pair: self.open_settings_dialog(cx).merge_pair,
            ..page
        }
    }

    /// The Settings Units page for a test, as the View builds it, plus the open Unit dialog.
    #[doc(hidden)]
    pub fn settings_units_snapshot(&self, cx: &gpui::App) -> SettingsUnitsSnapshot {
        let unit_form = self.open_settings_dialog(cx).unit;
        SettingsUnitsSnapshot {
            unit_dialog_name: unit_form.as_ref().map(|(name, ..)| name.clone()),
            unit_dialog_kind: unit_form.as_ref().map(|(_, kind, _)| kind.clone()),
            unit_dialog_field: unit_form.map(|(.., field)| field),
            ..self.views.settings.read(cx).units().read(cx).snapshot(cx)
        }
    }

    /// The Settings Institutions page for a test, as the View builds it, plus the open Add
    /// institution dialog.
    #[doc(hidden)]
    pub fn settings_institutions_snapshot(&self, cx: &gpui::App) -> SettingsInstitutionsSnapshot {
        let institution_form = self.open_settings_dialog(cx).institution;
        SettingsInstitutionsSnapshot {
            institution_dialog_types: institution_form.as_ref().map(|(types, _)| types.clone()),
            institution_dialog_unit: institution_form.and_then(|(_, unit)| unit),
            ..self
                .views
                .settings
                .read(cx)
                .institutions()
                .read(cx)
                .snapshot(cx)
        }
    }

    /// The Settings Display page for a test, as the View builds it.
    #[doc(hidden)]
    pub fn settings_display_snapshot(&self, cx: &gpui::App) -> SettingsDisplaySnapshot {
        self.views.settings.read(cx).display_snapshot(cx)
    }

    /// The Settings Tracing page for a test, as the View builds it.
    #[doc(hidden)]
    pub fn settings_tracing_snapshot(&self, cx: &gpui::App) -> SettingsTracingSnapshot {
        self.views.settings.read(cx).tracing_snapshot()
    }
}
