//! What the integration tests read back of the Settings pages, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

use super::Shell;
use super::inventory_ui::InventoryDialog;
use crate::{
    accounts::{self, AccountsDialog},
    categories::CategoriesDialog,
    document_types::DocumentTypesDialog,
    payees::{self, PayeesDialog},
    settings::{SettingsDialog, SettingsFocus},
    tags::{self, TagsDialog},
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
    /// The Tracing page: the chosen level (`TracingLevel` variant name) and how many log lines
    /// the viewport still holds.
    pub tracing_level: String,
    pub log_line_count: usize,
    /// The status line's message, where the stubbed buttons report.
    pub status_message: Option<String>,
}

impl Shell {
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
    pub fn settings_snapshot(&self) -> SettingsSnapshot {
        let account_names = accounts::display_order(&self.accounts)
            .into_iter()
            .filter_map(|index| self.accounts.get(index))
            .map(|account| account.name.clone())
            .collect::<Vec<_>>();
        let selected_account = self
            .selected_account_index()
            .and_then(|index| self.accounts.get(index))
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

        let selected_document_type = self.settings_documents_selected_id().and_then(|id| {
            self.document_types
                .iter()
                .find(|row| row.id == id)
                .map(|row| row.name.clone())
        });
        let selected_tag = self
            .settings_tags_selected_id()
            .and_then(|id| tags::get(&self.tags, id).map(|tag| tag.name.clone()));
        let mut merge_pair = None;
        let mut dialog = None;
        let mut dialog_name = None;
        let mut dialog_confirm = None;
        let mut unit_form = None;
        let mut institution_form = None;
        if let Some(open) = self.accounts_dialog.as_ref() {
            let (label, name, confirm) = match open {
                AccountsDialog::Add(form) => ("AddAccount", Some(form.name.clone()), None),
                AccountsDialog::Edit(_, form) => ("EditAccount", Some(form.name.clone()), None),
                AccountsDialog::Delete(_, form) => {
                    ("DeleteAccount", None, Some(form.confirm_input.clone()))
                }
            };
            dialog = Some(label);
            dialog_name = name;
            dialog_confirm = confirm;
        } else if let Some(open) = self.categories_dialog.as_ref() {
            let (label, name, confirm) = match open {
                CategoriesDialog::Add { form, .. } => {
                    ("AddCategory", Some(form.name.clone()), None)
                }
                CategoriesDialog::Edit(_, form) => ("EditCategory", Some(form.name.clone()), None),
                CategoriesDialog::Delete(_, form) => {
                    ("DeleteCategory", None, Some(form.confirmation_name.clone()))
                }
            };
            dialog = Some(label);
            dialog_name = name;
            dialog_confirm = confirm;
        } else if let Some(open) = self.payees_dialog.as_ref() {
            let (label, name, confirm) = match open {
                PayeesDialog::Add(form) => ("AddPayee", Some(form.name.clone()), None),
                PayeesDialog::Edit(_, form) => ("EditPayee", Some(form.name.clone()), None),
                PayeesDialog::Delete(_, form) => {
                    ("DeletePayee", None, Some(form.confirmation_name.clone()))
                }
            };
            dialog = Some(label);
            dialog_name = name;
            dialog_confirm = confirm;
        } else if let Some(open) = self.tags_dialog.as_ref() {
            let (label, name, confirm) = match open {
                TagsDialog::Add(form) => ("AddTag", Some(form.name.clone()), None),
                TagsDialog::Edit(_, form) => ("EditTag", Some(form.name.clone()), None),
                TagsDialog::Remove(_, form) => {
                    ("RemoveTag", None, Some(form.confirmation_name.clone()))
                }
                TagsDialog::Merge(form) => {
                    let options = self.merge_tag_options();
                    merge_pair = form.pair(&options).and_then(|(source, target)| {
                        Some((
                            tags::get(&self.tags, source)?.name.clone(),
                            tags::get(&self.tags, target)?.name.clone(),
                        ))
                    });
                    ("MergeTags", None, None)
                }
            };
            dialog = Some(label);
            dialog_name = name;
            dialog_confirm = confirm;
        } else if let Some(open) = self.settings_dialog.as_ref() {
            let (label, name, confirm) = match open {
                SettingsDialog::AddUnit(form) => {
                    unit_form = Some(form);
                    ("AddUnit", Some(form.code.clone()), None)
                }
                SettingsDialog::EditUnit(_, form) => {
                    unit_form = Some(form);
                    ("EditUnit", Some(form.code.clone()), None)
                }
                SettingsDialog::DeleteUnit(_, form) => {
                    ("DeleteUnit", None, Some(form.confirm_input.clone()))
                }
                SettingsDialog::AddInstitution(form) => {
                    institution_form = Some(form);
                    ("AddInstitution", Some(form.name.clone()), None)
                }
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
                .document_types
                .iter()
                .map(|row| row.name.clone())
                .collect(),
            selected_document_type,
            document_type_files: self.document_types.iter().map(|row| row.files).collect(),
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
            inventory_dialog: self.inventory_dialog.as_ref().map(|dialog| {
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
            document_type_dialog: self.document_types_dialog.as_ref().map(|dialog| {
                match dialog {
                    DocumentTypesDialog::Add(_) => "add",
                    DocumentTypesDialog::Edit(..) => "edit",
                    DocumentTypesDialog::Remove(..) => "remove",
                    DocumentTypesDialog::DefaultNotice => "notice",
                }
                .to_string()
            }),
            tag_names: tags::sorted_by_name(&self.tags)
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
            unit_dialog_name: unit_form.map(|form| form.name.clone()),
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
            toasts_on: self.toasts.display().toasts_on,
            tracing_level: format!("{:?}", self.settings_tracing_level),
            log_line_count: self.settings_log_lines.len(),
            status_message: self.status_message.clone(),
        }
    }
}
