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

/// The Settings shell's state: the page on show and the dialog open over it. Each page's own state
/// is its own snapshot (`settings_<page>_snapshot`), so a change to one domain edits one struct.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    /// The page on show, as its `SettingsSection` variant name.
    pub page: String,
    /// Whether the page, not the index rail, holds keyboard focus.
    pub page_focused: bool,
    /// The open dialog (`AddAccount`, `EditPayee`, `DeleteUnit` and so on); only one page's
    /// dialog is ever open.
    pub dialog: Option<String>,
    /// The dialog's name (or Unit code) draft, and its typed-back confirmation for a Delete.
    pub dialog_name: Option<String>,
    pub dialog_confirm: Option<String>,
    /// The status line's message, where the stubbed buttons report.
    pub status_message: Option<String>,
}

/// The Accounts page: names in the order the page lists them, and the selected row's name.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsAccountsSnapshot {
    pub account_names: Vec<String>,
    pub selected_account: Option<String>,
}

/// The Categories page: names in storage order, and the selected row's name.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsCategoriesSnapshot {
    pub category_names: Vec<String>,
    pub selected_category: Option<String>,
}

/// The Payees page: names A-Z, and the selected row's name.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPayeesSnapshot {
    pub payee_names: Vec<String>,
    pub selected_payee: Option<String>,
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

/// The Units page: codes in table order, `(code, name, kind)` rows, and the Add/Edit unit dialog's
/// Name draft, Type (`UnitKind` variant name) and focused field (`Code` or `Name`).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsUnitsSnapshot {
    pub unit_codes: Vec<String>,
    pub unit_rows: Vec<(String, String, String)>,
    pub unit_dialog_name: Option<String>,
    pub unit_dialog_kind: Option<String>,
    pub unit_dialog_field: Option<String>,
}

/// The Institutions page: `(name, account type)` rows in table order, and the Add institution
/// dialog's checked products (`InstitutionProduct` variant names) and chosen Default unit code.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsInstitutionsSnapshot {
    pub institutions: Vec<(String, String)>,
    pub institution_dialog_types: Option<Vec<String>>,
    pub institution_dialog_unit: Option<String>,
}

/// The Display page: the focused control above the Colour Theme grid and the focused card, then
/// each control's value (enum variant names; `date_style` is `None` for the default).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDisplaySnapshot {
    pub display_field: Option<usize>,
    pub colour_theme_focus: Option<usize>,
    pub date_style: Option<String>,
    pub row_density: String,
    pub status_glyphs: String,
    pub start_sidebar_minimised: bool,
    pub toasts_on: bool,
}

/// The Tracing page: the chosen level (`TracingLevel` variant name), how many entries the filter
/// shows, their bodies newest first (`subsystem: message key=value`), how many it hides, and the
/// list's top row.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTracingSnapshot {
    pub tracing_level: String,
    pub log_line_count: usize,
    pub log_lines: Vec<String>,
    pub log_hidden_count: usize,
    pub log_scroll_top: usize,
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
        row: crate::view::settings::inventory::InventoryRow,
        cx: &gpui::App,
    ) -> String {
        use crate::view::settings::inventory::InventoryRow;
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
                SettingsDialog::AddUnit(form) => {
                    open_dialog.unit = Some((
                        form.name.text().to_string(),
                        format!("{:?}", form.kind),
                        format!("{:?}", form.focused_field),
                    ));
                    ("AddUnit", Some(form.code.text().to_string()), None)
                }
                SettingsDialog::EditUnit(_, form) => {
                    open_dialog.unit = Some((
                        form.name.text().to_string(),
                        format!("{:?}", form.kind),
                        format!("{:?}", form.focused_field),
                    ));
                    ("EditUnit", Some(form.code.text().to_string()), None)
                }
                SettingsDialog::DeleteUnit(_, form) => (
                    "DeleteUnit",
                    None,
                    Some(form.confirm_input.text().to_string()),
                ),
                SettingsDialog::AddInstitution(form) => {
                    open_dialog.institution = Some((
                        form.account_types
                            .iter()
                            .map(|product| format!("{product:?}"))
                            .collect(),
                        form.default_unit_code.clone(),
                    ));
                    ("AddInstitution", Some(form.name.text().to_string()), None)
                }
                SettingsDialog::ClearLogs => ("ClearLogs", None, None),
            };
            open_dialog.label = Some(label);
            open_dialog.name = name;
            open_dialog.confirm = confirm;
        }
        open_dialog
    }

    /// The Settings shell for a test: the page on show and the open dialog.
    #[doc(hidden)]
    pub fn settings_snapshot(&self, cx: &gpui::App) -> SettingsSnapshot {
        let dialog = self.open_settings_dialog(cx);
        SettingsSnapshot {
            page: format!("{:?}", self.settings_view.read(cx).selected_section()),
            page_focused: self.settings_view.read(cx).focus() == SettingsFocus::Page,
            dialog: dialog.label.map(str::to_string),
            dialog_name: dialog.name,
            dialog_confirm: dialog.confirm,
            status_message: self.chrome.status_message.clone(),
        }
    }

    /// The Settings Accounts page for a test.
    #[doc(hidden)]
    pub fn settings_accounts_snapshot(&self, cx: &gpui::App) -> SettingsAccountsSnapshot {
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
        SettingsAccountsSnapshot {
            account_names,
            selected_account,
        }
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
        let selected_payee = self.settings_payees_selected_id(cx).and_then(|id| {
            self.payees_list(cx)
                .iter()
                .find(|payee| payee.id == id)
                .map(|payee| payee.name.clone())
        });
        SettingsPayeesSnapshot {
            payee_names: payees::sorted_by_name(self.payees_list(cx))
                .into_iter()
                .map(|payee| payee.name.clone())
                .collect(),
            selected_payee,
        }
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
            inventory_rows: crate::view::settings::inventory::visible_rows(
                self.inventory(cx),
                self.settings_view.read(cx).inventory_expanded(),
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

    /// The Settings Units page for a test.
    #[doc(hidden)]
    pub fn settings_units_snapshot(&self, cx: &gpui::App) -> SettingsUnitsSnapshot {
        let unit_form = self.open_settings_dialog(cx).unit;
        SettingsUnitsSnapshot {
            unit_codes: self
                .units_store
                .read(cx)
                .units()
                .iter()
                .map(|unit| unit.code.clone())
                .collect(),
            unit_rows: self
                .units_store
                .read(cx)
                .units()
                .iter()
                .map(|unit| (unit.code.clone(), unit.name.clone(), unit.kind.clone()))
                .collect(),
            unit_dialog_name: unit_form.as_ref().map(|(name, ..)| name.clone()),
            unit_dialog_kind: unit_form.as_ref().map(|(_, kind, _)| kind.clone()),
            unit_dialog_field: unit_form.map(|(.., field)| field),
        }
    }

    /// The Settings Institutions page for a test.
    #[doc(hidden)]
    pub fn settings_institutions_snapshot(&self, cx: &gpui::App) -> SettingsInstitutionsSnapshot {
        let institution_form = self.open_settings_dialog(cx).institution;
        SettingsInstitutionsSnapshot {
            institutions: self
                .institutions_store
                .read(cx)
                .institutions()
                .iter()
                .map(|row| (row.name.clone(), row.account_type.clone()))
                .collect(),
            institution_dialog_types: institution_form.as_ref().map(|(types, _)| types.clone()),
            institution_dialog_unit: institution_form.and_then(|(_, unit)| unit),
        }
    }

    /// The Settings Display page for a test.
    #[doc(hidden)]
    pub fn settings_display_snapshot(&self, cx: &gpui::App) -> SettingsDisplaySnapshot {
        SettingsDisplaySnapshot {
            display_field: self.settings_view.read(cx).display_field(),
            colour_theme_focus: self.settings_view.read(cx).colour_theme_focus(),
            date_style: self.settings_date_style.map(|style| format!("{style:?}")),
            row_density: format!("{:?}", self.settings_row_density),
            status_glyphs: format!("{:?}", self.settings_status_glyphs),
            start_sidebar_minimised: self.settings_start_sidebar_minimised,
            toasts_on: self.chrome.toasts.display().toasts_on,
        }
    }

    /// The Settings Tracing page for a test.
    #[doc(hidden)]
    pub fn settings_tracing_snapshot(&self, cx: &gpui::App) -> SettingsTracingSnapshot {
        SettingsTracingSnapshot {
            tracing_level: format!("{:?}", self.settings_view.read(cx).log().level()),
            log_line_count: self.settings_view.read(cx).log().visible().len(),
            log_lines: self
                .settings_view
                .read(cx)
                .log()
                .visible()
                .iter()
                .map(|entry| crate::settings::tracing_log::body(entry))
                .collect(),
            log_hidden_count: self.settings_view.read(cx).log().hidden_count(),
            log_scroll_top: self
                .settings_view
                .read(cx)
                .log_list()
                .logical_scroll_top()
                .item_ix,
        }
    }
}
