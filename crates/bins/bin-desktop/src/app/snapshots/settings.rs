//! What the integration tests read back of the Settings pages, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

use crate::app::Shell;
use crate::inventory::form::InventoryDialog;
use crate::{
    accounts::{self, form::AccountsDialog},
    categories::form::CategoriesDialog,
    documents::types::DocumentTypesDialog,
    payees::{self, form::PayeesDialog},
    settings::{SettingsDialog, SettingsFocus},
    tags::{self, form::TagsDialog},
};

/// The Settings pages' state: plain values, so the private form types stay private.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    /// The page on show, as its `SettingsSection` variant name.
    pub page: String,
    /// Whether the page, not the index rail, holds keyboard focus.
    pub page_focused: bool,
    /// Account names in the order the page lists them, and the selected row's name.
    pub account_names: Vec<String>,
    pub selected_account: Option<String>,
    /// Category names in storage order, and the selected row's name.
    pub category_names: Vec<String>,
    pub selected_category: Option<String>,
    /// Payee names A-Z, and the selected row's name.
    pub payee_names: Vec<String>,
    pub selected_payee: Option<String>,
    /// Document type names in the user's order, and the selected row's name.
    pub document_type_names: Vec<String>,
    pub selected_document_type: Option<String>,
    /// Filed files per type in the same order, and the open dialog (`add`, `edit`, `remove` or
    /// `notice`).
    pub document_type_files: Vec<u32>,
    pub document_type_dialog: Option<String>,
    /// The Inventory page's visible rows and the selected one, as `property:<name>` or
    /// `room:<name>`, each Property's Rooms in order, and the dialog last asked for.
    pub inventory_rows: Vec<String>,
    pub selected_inventory: Option<String>,
    pub inventory_rooms: Vec<(String, Vec<String>)>,
    /// The open Property dialog (`add`, `edit` or `remove`), and each Property as
    /// `(name, unit code, insurer)`.
    pub inventory_dialog: Option<String>,
    pub inventory_properties: Vec<(String, String, Option<String>)>,
    /// Each Room's name with its Item count, in page order.
    pub inventory_room_items: Vec<(String, usize)>,
    /// Tag names A-Z, and the selected row's name.
    pub tag_names: Vec<String>,
    pub selected_tag: Option<String>,
    /// The Merge tags dialog's chosen `(source, target)` names, once both are set.
    pub merge_pair: Option<(String, String)>,
    /// Unit codes in table order.
    pub unit_codes: Vec<String>,
    /// Institution `(name, account type)` rows in table order.
    pub institutions: Vec<(String, String)>,
    /// Unit `(code, name, kind)` rows in table order.
    pub unit_rows: Vec<(String, String, String)>,
    /// The Add/Edit unit dialog's Name draft, Type (`UnitKind` variant name) and focused field
    /// (`Code` or `Name`).
    pub unit_dialog_name: Option<String>,
    pub unit_dialog_kind: Option<String>,
    pub unit_dialog_field: Option<String>,
    /// The Add institution dialog's checked Account types (`AccountType` variant names) and
    /// chosen Default unit code.
    pub institution_dialog_types: Option<Vec<String>>,
    pub institution_dialog_unit: Option<String>,
    /// The open dialog (`AddAccount`, `EditPayee`, `DeleteUnit` and so on); only one page's
    /// dialog is ever open.
    pub dialog: Option<String>,
    /// The dialog's name (or Unit code) draft, and its typed-back confirmation for a Delete.
    pub dialog_name: Option<String>,
    pub dialog_confirm: Option<String>,
    /// The Display page: the focused control above the Colour Theme grid and the focused card,
    /// then each control's value (enum variant names; `date_style` is `None` for the default).
    pub display_field: Option<usize>,
    pub colour_theme_focus: Option<usize>,
    pub date_style: Option<String>,
    pub row_density: String,
    pub status_glyphs: String,
    pub start_sidebar_minimised: bool,
    pub toasts_on: bool,
    /// The Tracing page: the chosen level (`TracingLevel` variant name), how many entries the
    /// filter shows, their bodies newest first (`subsystem: message key=value`), how many it
    /// hides, and the list's top row.
    pub tracing_level: String,
    pub log_line_count: usize,
    pub log_lines: Vec<String>,
    pub log_hidden_count: usize,
    pub log_scroll_top: usize,
    /// The status line's message, where the stubbed buttons report.
    pub status_message: Option<String>,
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
        let form = self.accounts_dialog()?.form()?;
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
    fn inventory_row_label(&self, row: crate::view::settings::inventory::InventoryRow) -> String {
        use crate::view::settings::inventory::InventoryRow;
        match row {
            InventoryRow::Property(id) => self
                .inventory
                .property(id)
                .map(|property| format!("property:{}", property.name))
                .unwrap_or_default(),
            InventoryRow::Room(id) => self
                .inventory
                .room(id)
                .map(|(_, room)| format!("room:{}", room.name))
                .unwrap_or_default(),
        }
    }

    /// The Settings pages for a test.
    #[doc(hidden)]
    pub fn settings_snapshot(&self, cx: &gpui::App) -> SettingsSnapshot {
        let account_names = accounts::display_order(self.accounts.read(cx).accounts())
            .into_iter()
            .filter_map(|index| self.accounts.read(cx).accounts().get(index))
            .map(|account| account.name.clone())
            .collect::<Vec<_>>();
        let selected_account = self
            .accounts_view
            .read(cx)
            .selected_index(cx)
            .and_then(|index| self.accounts.read(cx).accounts().get(index))
            .map(|account| account.name.clone());
        let selected_category = self.categories_selected_id.and_then(|id| {
            self.categories
                .iter()
                .find(|category| category.id == id)
                .map(|category| category.name.clone())
        });
        let selected_payee = self.settings_payees_selected_id().and_then(|id| {
            self.payees
                .iter()
                .find(|payee| payee.id == id)
                .map(|payee| payee.name.clone())
        });

        let selected_document_type = self.settings_documents_selected_id(cx).and_then(|id| {
            self.document_types(cx)
                .iter()
                .find(|row| row.id == id)
                .map(|row| row.name.clone())
        });
        let selected_tag = self
            .settings_tags_selected_id(cx)
            .and_then(|id| tags::get(self.tags_list(cx), id).map(|tag| tag.name.clone()));
        let mut merge_pair = None;
        let mut dialog = None;
        let mut dialog_name = None;
        let mut dialog_confirm = None;
        let mut unit_form = None;
        let mut institution_form = None;
        if let Some(open) = self.accounts_dialog() {
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
            dialog = Some(label);
            dialog_name = name;
            dialog_confirm = confirm;
        } else if let Some(open) = self.categories_dialog() {
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
            dialog = Some(label);
            dialog_name = name;
            dialog_confirm = confirm;
        } else if let Some(open) = self.payees_dialog() {
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
            dialog = Some(label);
            dialog_name = name;
            dialog_confirm = confirm;
        } else if let Some(open) = self.tags_dialog() {
            let (label, name, confirm) = match open {
                TagsDialog::Add(form) => ("AddTag", Some(form.name.text().to_string()), None),
                TagsDialog::Edit(_, form) => ("EditTag", Some(form.name.text().to_string()), None),
                TagsDialog::Remove(_, form) => (
                    "RemoveTag",
                    None,
                    Some(form.confirmation_name.text().to_string()),
                ),
                TagsDialog::Merge(form) => {
                    merge_pair = form.pair().and_then(|(source, target)| {
                        Some((
                            tags::get(self.tags_list(cx), source)?.name.clone(),
                            tags::get(self.tags_list(cx), target)?.name.clone(),
                        ))
                    });
                    ("MergeTags", None, None)
                }
            };
            dialog = Some(label);
            dialog_name = name;
            dialog_confirm = confirm;
        } else if let Some(open) = self.settings_dialog() {
            let (label, name, confirm) = match open {
                SettingsDialog::AddUnit(form) => {
                    unit_form = Some(form);
                    ("AddUnit", Some(form.code.text().to_string()), None)
                }
                SettingsDialog::EditUnit(_, form) => {
                    unit_form = Some(form);
                    ("EditUnit", Some(form.code.text().to_string()), None)
                }
                SettingsDialog::DeleteUnit(_, form) => (
                    "DeleteUnit",
                    None,
                    Some(form.confirm_input.text().to_string()),
                ),
                SettingsDialog::AddInstitution(form) => {
                    institution_form = Some(form);
                    ("AddInstitution", Some(form.name.text().to_string()), None)
                }
                SettingsDialog::ClearLogs => ("ClearLogs", None, None),
            };
            dialog = Some(label);
            dialog_name = name;
            dialog_confirm = confirm;
        }

        SettingsSnapshot {
            page: format!("{:?}", self.settings_selected_section),
            page_focused: self.settings_focus == SettingsFocus::Page,
            account_names,
            selected_account,
            category_names: self
                .categories
                .iter()
                .map(|category| category.name.clone())
                .collect(),
            selected_category,
            payee_names: payees::sorted_by_name(&self.payees)
                .into_iter()
                .map(|payee| payee.name.clone())
                .collect(),
            selected_payee,
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
            inventory_rows: crate::view::settings::inventory::visible_rows(
                &self.inventory,
                &self.settings_inventory_expanded,
            )
            .into_iter()
            .map(|row| self.inventory_row_label(row))
            .collect(),
            selected_inventory: self
                .settings_inventory_selected_row()
                .map(|row| self.inventory_row_label(row)),
            inventory_rooms: self
                .inventory
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
            inventory_dialog: self.inventory_dialog().map(|dialog| {
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
                .inventory
                .properties
                .iter()
                .flat_map(|p| p.rooms.iter())
                .map(|room| (room.name.clone(), self.inventory.room_items(room.id)))
                .collect(),
            inventory_properties: self
                .inventory
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
            document_type_dialog: self.document_types_dialog().map(|dialog| {
                match dialog {
                    DocumentTypesDialog::Add(_) => "add",
                    DocumentTypesDialog::Edit(..) => "edit",
                    DocumentTypesDialog::Remove(..) => "remove",
                    DocumentTypesDialog::DefaultNotice => "notice",
                }
                .to_string()
            }),
            tag_names: tags::sorted_by_name(self.tags_list(cx))
                .into_iter()
                .map(|tag| tag.name.clone())
                .collect(),
            selected_tag,
            merge_pair,
            unit_codes: self
                .settings_units
                .iter()
                .map(|unit| unit.code.clone())
                .collect(),
            institutions: self
                .settings_institutions
                .iter()
                .map(|row| (row.name.clone(), row.account_type.clone()))
                .collect(),
            unit_rows: self
                .settings_units
                .iter()
                .map(|unit| (unit.code.clone(), unit.name.clone(), unit.kind.clone()))
                .collect(),
            unit_dialog_name: unit_form.map(|form| form.name.text().to_string()),
            unit_dialog_kind: unit_form.map(|form| format!("{:?}", form.kind)),
            unit_dialog_field: unit_form.map(|form| format!("{:?}", form.focused_field)),
            institution_dialog_types: institution_form.map(|form| {
                form.account_types
                    .iter()
                    .map(|account_type| format!("{account_type:?}"))
                    .collect()
            }),
            institution_dialog_unit: institution_form
                .and_then(|form| form.default_unit_code.clone()),
            dialog: dialog.map(str::to_string),
            dialog_name,
            dialog_confirm,
            display_field: self.settings_display_field,
            colour_theme_focus: self.colour_theme_focus,
            date_style: self.settings_date_style.map(|style| format!("{style:?}")),
            row_density: format!("{:?}", self.settings_row_density),
            status_glyphs: format!("{:?}", self.settings_status_glyphs),
            start_sidebar_minimised: self.settings_start_sidebar_minimised,
            toasts_on: self.chrome.toasts.display().toasts_on,
            tracing_level: format!("{:?}", self.settings_log.level()),
            log_line_count: self.settings_log.visible().len(),
            log_lines: self
                .settings_log
                .visible()
                .iter()
                .map(|entry| crate::settings::tracing_log::body(entry))
                .collect(),
            log_hidden_count: self.settings_log.hidden_count(),
            log_scroll_top: self.settings_log_list.logical_scroll_top().item_ix,
            status_message: self.chrome.status_message.clone(),
        }
    }
}
