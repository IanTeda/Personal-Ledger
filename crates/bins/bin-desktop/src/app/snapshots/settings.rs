//! What the integration tests read back of the Settings pages, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

use crate::app::Shell;
use crate::inventory::form::InventoryDialog;
#[doc(hidden)]
pub use crate::settings::accounts::snapshot::SettingsAccountsSnapshot;
#[doc(hidden)]
pub use crate::settings::institutions::SettingsInstitutionsSnapshot;
#[doc(hidden)]
pub use crate::settings::payees::SettingsPayeesSnapshot;
#[doc(hidden)]
pub use crate::settings::snapshot::{
    SettingsDisplaySnapshot, SettingsSnapshot, SettingsTracingSnapshot,
};
#[doc(hidden)]
pub use crate::settings::units::SettingsUnitsSnapshot;
use crate::{
    categories::form::CategoriesDialog,
    documents::types::DocumentTypesDialog,
    settings::accounts::dialogs::AccountsDialog,
    settings::payees::dialogs::PayeesDialog,
    settings::{
        SettingsDialog, institutions::dialogs::InstitutionsDialog, units::dialogs::UnitsDialog,
    },
    tags::{self, form::TagsDialog},
};

/// The Categories page: names in storage order, and the selected row's name.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsCategoriesSnapshot {
    pub category_names: Vec<String>,
    pub selected_category: Option<String>,
}

/// The Document Types page: names in the user's order, the selected row's name, filed files per
/// type in the same order, and the open dialog (`add`, `edit`, `remove` or `notice`).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDocumentTypesSnapshot {
    pub document_type_names: Vec<String>,
    pub selected_document_type: Option<String>,
    pub document_type_files: Vec<u32>,
    pub document_type_dialog: Option<String>,
}

/// The Inventory page's visible rows and the selected one, as `property:<name>` or `room:<name>`,
/// each Property's Rooms in order, and the dialog last asked for.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsInventorySnapshot {
    pub inventory_rows: Vec<String>,
    pub selected_inventory: Option<String>,
    pub inventory_rooms: Vec<(String, Vec<String>)>,
    /// The open Property dialog (`add`, `edit` or `remove`), and each Property as
    /// `(name, unit code, insurer)`.
    pub inventory_dialog: Option<String>,
    pub inventory_properties: Vec<(String, String, Option<String>)>,
    /// Each Room's name with its Item count, in page order.
    pub inventory_room_items: Vec<(String, usize)>,
}

/// The Tags page: names A-Z, the selected row's name, and the Merge tags dialog's chosen
/// `(source, target)` names once both are set.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTagsSnapshot {
    pub tag_names: Vec<String>,
    pub selected_tag: Option<String>,
    pub merge_pair: Option<(String, String)>,
}

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

    /// An Inventory row as `property:<name>` or `room:<name>`.
    fn inventory_row_label(
        &self,
        row: crate::settings::inventory::InventoryRow,
        cx: &gpui::App,
    ) -> String {
        use crate::settings::inventory::InventoryRow;
        match row {
            InventoryRow::Property(id) => self
                .inventory(cx)
                .property(id)
                .map(|property| format!("property:{}", property.name))
                .unwrap_or_default(),
            InventoryRow::Room(id) => self
                .inventory(cx)
                .room(id)
                .map(|(_, room)| format!("room:{}", room.name))
                .unwrap_or_default(),
        }
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
                            tags::get(self.tags_list(cx), source)?.name.clone(),
                            tags::get(self.tags_list(cx), target)?.name.clone(),
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
        let selected_category = self.categories_view.read(cx).selected_id().and_then(|id| {
            self.categories(cx)
                .iter()
                .find(|category| category.id == id)
                .map(|category| category.name.clone())
        });
        SettingsCategoriesSnapshot {
            category_names: self
                .categories(cx)
                .iter()
                .map(|category| category.name.clone())
                .collect(),
            selected_category,
        }
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
        let selected_document_type = self.settings_documents_selected_id(cx).and_then(|id| {
            self.document_types(cx)
                .iter()
                .find(|row| row.id == id)
                .map(|row| row.name.clone())
        });
        SettingsDocumentTypesSnapshot {
            document_type_names: self
                .document_types(cx)
                .iter()
                .map(|row| row.name.clone())
                .collect(),
            selected_document_type,
            document_type_files: self
                .document_types(cx)
                .iter()
                .map(|row| row.files)
                .collect(),
            document_type_dialog: self.dialog::<DocumentTypesDialog>().map(|dialog| {
                match dialog {
                    DocumentTypesDialog::Add(_) => "add",
                    DocumentTypesDialog::Edit(..) => "edit",
                    DocumentTypesDialog::Remove(..) => "remove",
                    DocumentTypesDialog::DefaultNotice => "notice",
                }
                .to_string()
            }),
        }
    }

    /// The Settings Inventory page for a test.
    #[doc(hidden)]
    pub fn settings_inventory_snapshot(&self, cx: &gpui::App) -> SettingsInventorySnapshot {
        SettingsInventorySnapshot {
            inventory_rows: crate::settings::inventory::visible_rows(
                self.inventory(cx),
                self.views.settings.read(cx).inventory_expanded(),
            )
            .into_iter()
            .map(|row| self.inventory_row_label(row, cx))
            .collect(),
            selected_inventory: self
                .settings_inventory_selected_row(cx)
                .map(|row| self.inventory_row_label(row, cx)),
            inventory_rooms: self
                .inventory(cx)
                .properties
                .iter()
                .map(|property| {
                    (
                        property.name.clone(),
                        property
                            .rooms
                            .iter()
                            .map(|room| room.name.clone())
                            .collect(),
                    )
                })
                .collect(),
            inventory_dialog: self.dialog::<InventoryDialog>().map(|dialog| {
                match dialog {
                    InventoryDialog::Add(_) => "add",
                    InventoryDialog::Edit(..) => "edit",
                    InventoryDialog::Remove(..) => "remove",
                    InventoryDialog::AddRoom(..) => "add-room",
                    InventoryDialog::EditRoom(..) => "edit-room",
                    InventoryDialog::RemoveRoom(..) => "remove-room",
                    InventoryDialog::RoomBlocked(_) => "room-blocked",
                }
                .to_string()
            }),
            inventory_room_items: self
                .inventory(cx)
                .properties
                .iter()
                .flat_map(|p| p.rooms.iter())
                .map(|room| (room.name.clone(), self.inventory(cx).room_items(room.id)))
                .collect(),
            inventory_properties: self
                .inventory(cx)
                .properties
                .iter()
                .map(|p| {
                    (
                        p.name.clone(),
                        p.unit.clone(),
                        p.cover.as_ref().map(|c| c.insurer.clone()),
                    )
                })
                .collect(),
        }
    }

    /// The Settings Tags page for a test.
    #[doc(hidden)]
    pub fn settings_tags_snapshot(&self, cx: &gpui::App) -> SettingsTagsSnapshot {
        let selected_tag = self
            .settings_tags_selected_id(cx)
            .and_then(|id| tags::get(self.tags_list(cx), id).map(|tag| tag.name.clone()));
        SettingsTagsSnapshot {
            tag_names: tags::sorted_by_name(self.tags_list(cx))
                .into_iter()
                .map(|tag| tag.name.clone())
                .collect(),
            selected_tag,
            merge_pair: self.open_settings_dialog(cx).merge_pair,
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
