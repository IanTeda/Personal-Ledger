//! What the integration tests read back of the Settings pages, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

use super::Shell;
use crate::{
    accounts::{self, AccountsDialog},
    categories::CategoriesDialog,
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
    /// Tag names A-Z, and the selected row's name.
    pub tag_names: Vec<String>,
    pub selected_tag: Option<String>,
    /// The Merge tags dialog's chosen `(source, target)` names, once both are set.
    pub merge_pair: Option<(String, String)>,
    /// Unit codes in table order.
    pub unit_codes: Vec<String>,
    /// The open dialog (`AddAccount`, `EditPayee`, `DeleteUnit` and so on); only one page's
    /// dialog is ever open.
    pub dialog: Option<String>,
    /// The dialog's name (or Unit code) draft, and its typed-back confirmation for a Delete.
    pub dialog_name: Option<String>,
    pub dialog_confirm: Option<String>,
}

impl Shell {
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

        let selected_tag = self
            .settings_tags_selected_id()
            .and_then(|id| tags::get(&self.tags, id).map(|tag| tag.name.clone()));
        let mut merge_pair = None;
        let mut dialog = None;
        let mut dialog_name = None;
        let mut dialog_confirm = None;
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
                SettingsDialog::AddUnit(form) => ("AddUnit", Some(form.code.clone()), None),
                SettingsDialog::EditUnit(_, form) => ("EditUnit", Some(form.code.clone()), None),
                SettingsDialog::DeleteUnit(_, form) => {
                    ("DeleteUnit", None, Some(form.confirm_input.clone()))
                }
                SettingsDialog::AddInstitution(_) => ("AddInstitution", None, None),
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
            dialog: dialog.map(str::to_string),
            dialog_name,
            dialog_confirm,
        }
    }
}
