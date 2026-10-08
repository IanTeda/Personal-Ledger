//! The Payees dialogs' form state: the Add, Edit and Delete payee forms, the Default category
//! select's options and the dialog they sit behind. `gpui`-free, like the rest of this domain;
//! `view::payees` renders it.

use super::{
    Payee, PayeeDraft, PayeeError, alias_owner, clean_name, is_referenced, normalise_alias,
};
use crate::{
    accounts::form::SelectKey,
    categories::{self, Category},
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    form::field::TextField,
    form::select::SelectState,
    transactions::Transaction,
};

/// The Default category select's options: "none" first, then every leaf Category. `labels` are
/// what the select shows and stores; `ids` runs parallel to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeOptions {
    pub labels: Vec<String>,
    pub ids: Vec<Option<u32>>,
}

impl PayeeOptions {
    /// `none_label` heads the list. A leaf whose name another leaf shares is shown under its
    /// parent (`Parent › Leaf`), so the stored label always names exactly one Category.
    pub fn new(categories: &[Category], none_label: String) -> Self {
        let leaves: Vec<&Category> = categories
            .iter()
            .filter(|category| categories::is_leaf(categories, category.id))
            .collect();
        let mut labels = vec![none_label];
        let mut ids = vec![None];
        for leaf in &leaves {
            let shared = leaves
                .iter()
                .filter(|other| other.name == leaf.name)
                .count()
                > 1;
            let parent = leaf
                .parent
                .and_then(|id| categories.iter().find(|c| c.id == id));
            labels.push(match parent {
                Some(parent) if shared => format!("{} \u{203a} {}", parent.name, leaf.name),
                _ => leaf.name.clone(),
            });
            ids.push(Some(leaf.id));
        }
        Self { labels, ids }
    }

    fn label_for(&self, id: Option<u32>) -> Option<String> {
        self.ids
            .iter()
            .position(|candidate| *candidate == id)
            .map(|index| self.labels[index].clone())
    }

    fn id_for(&self, label: &str) -> Option<u32> {
        self.labels
            .iter()
            .position(|candidate| candidate == label)
            .and_then(|index| self.ids[index])
    }
}

/// The Add and Edit payee dialogs' fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PayeeField {
    #[default]
    Name,
    DefaultCategory,
    /// The match-rule input beside **+ add**.
    Rule,
}

impl PayeeField {
    const ORDER: [PayeeField; 3] = [Self::Name, Self::DefaultCategory, Self::Rule];
}

/// The Add and Edit payee dialogs' live form state -- pure, `gpui`-free. Name and the rule input
/// are [`TextField`]s; Default category is a [`SelectState`]; the rules are chips, added from the
/// input and removed by their `✕`. The select's options and the Payees the checks read are
/// copied in when the dialog opens, so the form validates without `Shell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeForm {
    pub name: TextField,
    pub default_category: SelectState,
    /// The chips, normalised by [`normalise_alias`] as they are added.
    pub rules: Vec<String>,
    pub rule_input: TextField,
    pub focused: PayeeField,
    /// The last rejected rule or submit, shown inline until the offending text changes.
    pub error: Option<PayeeError>,
    options: PayeeOptions,
    /// Every Payee when the dialog opened, what names and rules are checked against.
    payees: Vec<Payee>,
    /// The Payee being edited (`None` for Add), so its own name and rules don't count as taken.
    own_id: Option<u32>,
}

impl PayeeForm {
    /// A fresh Add form: no name, no default Category, no rules.
    pub fn new(options: &PayeeOptions, payees: &[Payee]) -> Self {
        Self {
            name: TextField::default(),
            default_category: SelectState::new(options.label_for(None)),
            rules: Vec::new(),
            rule_input: TextField::default(),
            focused: PayeeField::Name,
            error: None,
            options: options.clone(),
            payees: payees.to_vec(),
            own_id: None,
        }
    }

    /// A form pre-filled from `payee`, for the Edit dialog.
    pub fn from_payee(payee: &Payee, options: &PayeeOptions, payees: &[Payee]) -> Self {
        Self {
            name: TextField::new(payee.name.as_str()),
            default_category: SelectState::new(
                options
                    .label_for(payee.default_category)
                    .or_else(|| options.label_for(None)),
            ),
            rules: payee.aliases.clone(),
            own_id: Some(payee.id),
            ..Self::new(options, payees)
        }
    }

    /// The Default category select's options, for the view to list.
    pub fn options(&self) -> &PayeeOptions {
        &self.options
    }

    /// What the dialog submits.
    pub fn draft(&self) -> PayeeDraft {
        PayeeDraft {
            name: self.name.text().to_string(),
            default_category: self
                .default_category
                .value()
                .and_then(|label| self.options.id_for(label)),
            aliases: self.rules.clone(),
        }
    }

    /// The name's problem, if any, checked live against every other Payee. An empty name is not
    /// reported, only kept from submitting.
    pub fn name_error(&self) -> Option<PayeeError> {
        if self.name.is_blank() {
            return None;
        }
        clean_name(&self.payees, self.own_id, self.name.text()).err()
    }

    pub fn is_valid(&self) -> bool {
        clean_name(&self.payees, self.own_id, self.name.text()).is_ok()
    }

    /// Turns the rule input into a chip: normalised, ignored when blank or already a chip, and
    /// refused (kept in the input, with [`Self::error`] set) when another Payee owns it.
    pub fn add_rule(&mut self) {
        let Some(rule) = normalise_alias(self.rule_input.text()) else {
            self.rule_input = TextField::default();
            return;
        };
        if let Some(owner) = alias_owner(&self.payees, &rule).filter(|p| Some(p.id) != self.own_id)
        {
            self.error = Some(PayeeError::AliasTaken {
                alias: rule,
                owner: owner.name.clone(),
            });
            return;
        }
        if !self.rules.contains(&rule) {
            self.rules.push(rule);
        }
        self.rule_input = TextField::default();
        self.error = None;
    }

    pub fn remove_rule(&mut self, index: usize) {
        if index < self.rules.len() {
            self.rules.remove(index);
        }
    }

    /// Moves focus to `field`, closing the select's list when leaving it.
    pub fn focus(&mut self, field: PayeeField) {
        if field != self.focused {
            self.default_category.cancel();
        }
        self.focused = field;
    }

    /// `Tab` / `Shift-Tab`: commits an open list's highlight, then moves to the next field.
    pub fn cycle_focus(&mut self, backward: bool) {
        self.default_category.commit(&self.options.labels);
        let count = PayeeField::ORDER.len();
        let index = PayeeField::ORDER
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        let next = if backward {
            (index + count - 1) % count
        } else {
            (index + 1) % count
        };
        self.focused = PayeeField::ORDER[next];
    }

    /// A key on the Default category select. Returns whether it is focused (and so took the key).
    pub fn handle_select_key(&mut self, key: SelectKey) -> bool {
        if self.focused != PayeeField::DefaultCategory {
            return false;
        }
        let list = &self.options.labels;
        let state = &mut self.default_category;
        match (key, state.is_open()) {
            (SelectKey::Up, true) => state.move_highlight(list, -1),
            (SelectKey::Down, true) => state.move_highlight(list, 1),
            (SelectKey::Up, false) => state.step(list, -1),
            (SelectKey::Down, false) => state.step(list, 1),
            (SelectKey::Activate, true) => state.commit(list),
            (SelectKey::Activate, false) => state.open(list),
        }
        true
    }

    /// A click on the select's closed field: focuses it and toggles its list.
    pub fn click_select(&mut self) {
        self.focus(PayeeField::DefaultCategory);
        if self.default_category.is_open() {
            self.default_category.cancel();
        } else {
            self.default_category.open(&self.options.labels);
        }
    }

    /// Picks option `index` from the select's list.
    pub fn choose_category(&mut self, index: usize) {
        self.default_category.choose(&self.options.labels, index);
    }

    /// Closes the select's list if open -- the first `Esc`. Returns whether it was open.
    pub fn close_open_select(&mut self) -> bool {
        let was_open = self.default_category.is_open();
        self.default_category.cancel();
        was_open
    }

    /// The text field typing and `Backspace` edit; the select has none.
    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self.focused {
            PayeeField::Name => Some(&mut self.name),
            PayeeField::Rule => Some(&mut self.rule_input),
            PayeeField::DefaultCategory => None,
        }
    }

    /// The keys the form takes before `dialog_host`'s shared typing: the select's, `Enter` adding
    /// the typed rule as a chip, and `Backspace` in an empty rule input removing the last chip.
    /// Typing and `Backspace` that fall through to the shared handling clear a shown error first.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let on_select = self.focused == PayeeField::DefaultCategory;
        match key {
            DialogKey::Up => {
                self.handle_select_key(SelectKey::Up);
            }
            DialogKey::Down => {
                self.handle_select_key(SelectKey::Down);
            }
            DialogKey::BackTab => self.cycle_focus(true),
            DialogKey::Enter | DialogKey::Char(' ') if on_select => {
                self.handle_select_key(SelectKey::Activate);
            }
            DialogKey::Enter
                if self.focused == PayeeField::Rule && !self.rule_input.text().is_empty() =>
            {
                self.add_rule();
            }
            // A select swallows typing rather than letting it fall through to the shell.
            DialogKey::Char(_) | DialogKey::Backspace if on_select => {}
            DialogKey::Backspace
                if self.focused == PayeeField::Rule && self.rule_input.text().is_empty() =>
            {
                self.rules.pop();
                self.error = None;
            }
            DialogKey::Char(_) | DialogKey::Backspace => {
                self.error = None;
                return None;
            }
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }
}

/// Which Payees dialog is open, following `AccountsDialog`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeesDialog {
    Add(PayeeForm),
    /// Editing the Payee with this [`Payee::id`].
    Edit(u32, PayeeForm),
    /// Deleting (or, when referenced, deactivating) the Payee with this [`Payee::id`].
    Delete(u32, DeletePayeeForm),
}

impl PayeesDialog {
    /// The form behind the Add and Edit dialogs.
    pub fn form_mut(&mut self) -> Option<&mut PayeeForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Delete(..) => None,
        }
    }
}

impl Dialog for PayeesDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.form_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.focused_text(),
            Self::Delete(_, form) => Some(&mut form.confirmation_name),
        }
    }

    fn cycle_field(&mut self) {
        if let Some(form) = self.form_mut() {
            form.cycle_focus(false);
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.is_valid(),
            Self::Delete(_, form) => form.is_valid(),
        }
    }

    fn close_open_select(&mut self) -> bool {
        self.form_mut().is_some_and(PayeeForm::close_open_select)
    }
}

/// What the 6d dialog does to a Payee (#283): an unreferenced one is hard-deleted, a referenced
/// active one can only be deactivated, and a referenced inactive one is offered reactivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteAction {
    Delete,
    Deactivate,
    Reactivate,
}

impl DeleteAction {
    pub fn for_payee(payee: &Payee, transactions: &[Transaction]) -> Self {
        if !is_referenced(transactions, payee.id) {
            Self::Delete
        } else if payee.is_active {
            Self::Deactivate
        } else {
            Self::Reactivate
        }
    }

    /// Reactivating loses nothing, so it skips the typed-name gate and the destructive chrome.
    pub fn is_destructive(self) -> bool {
        self != Self::Reactivate
    }
}

/// The 6d dialog's typed-name confirmation (exact and case-sensitive, as in Accounts and
/// Categories). The Payee's name and [`DeleteAction`] are copied in at open so the form validates
/// without `Shell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletePayeeForm {
    pub confirmation_name: TextField,
    name: String,
    action: DeleteAction,
}

impl DeletePayeeForm {
    pub fn new(payee: &Payee, action: DeleteAction) -> Self {
        Self {
            confirmation_name: TextField::default(),
            name: payee.name.clone(),
            action,
        }
    }

    pub fn action(&self) -> DeleteAction {
        self.action
    }

    /// Whether the confirm button is live: the typed name matches for a destructive action.
    pub fn is_valid(&self) -> bool {
        !self.action.is_destructive() || self.confirmation_name.text() == self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        categories::default_categories,
        payees::{default_payees, find_by_name, get},
    };

    fn id_of(payees: &[Payee], name: &str) -> u32 {
        find_by_name(payees, name).unwrap()
    }

    #[test]
    fn the_delete_form_needs_the_exact_name_unless_reactivating() {
        let payees = default_payees();
        let j_smith = get(&payees, id_of(&payees, "J Smith")).unwrap();
        let mut form = DeletePayeeForm::new(j_smith, DeleteAction::Delete);
        for ch in "j smith".chars() {
            form.confirmation_name.push(ch);
        }
        assert!(!form.is_valid());
        assert!(!DeletePayeeForm::new(j_smith, DeleteAction::Deactivate).is_valid());
        assert!(DeletePayeeForm::new(j_smith, DeleteAction::Reactivate).is_valid());

        form.confirmation_name = TextField::new("J Smith");
        assert!(form.is_valid());
        form.confirmation_name.backspace();
        assert!(!form.is_valid());
    }

    fn options() -> PayeeOptions {
        PayeeOptions::new(&default_categories(), "none".to_string())
    }

    #[test]
    fn options_lead_with_none_then_only_leaves() {
        let categories = default_categories();
        let options = options();
        assert_eq!(options.labels[0], "none");
        assert_eq!(options.ids[0], None);
        assert_eq!(options.labels.len(), options.ids.len());
        for id in options.ids.iter().skip(1).flatten() {
            assert!(categories::is_leaf(&categories, *id));
        }
        assert!(options.labels.contains(&"Groceries".to_string()));
    }

    #[test]
    fn a_fresh_form_drafts_no_category_and_no_rules() {
        let mut form = PayeeForm::new(&options(), &default_payees());
        assert_eq!(form.default_category.value(), Some("none"));
        form.name = TextField::new("Aussie Candle Co");
        assert_eq!(
            form.draft(),
            PayeeDraft {
                name: "Aussie Candle Co".to_string(),
                default_category: None,
                aliases: Vec::new(),
            }
        );
    }

    #[test]
    fn the_select_picks_a_leaf_category_into_the_draft() {
        let options = options();
        let mut form = PayeeForm::new(&options, &default_payees());
        form.focus(PayeeField::DefaultCategory);
        assert!(form.handle_select_key(SelectKey::Down));
        assert_eq!(form.draft().default_category, options.ids[1]);
        assert!(form.handle_select_key(SelectKey::Activate));
        assert!(form.default_category.is_open());
        assert!(form.close_open_select());
        assert!(!form.close_open_select());
    }

    #[test]
    fn select_keys_are_ignored_off_the_select() {
        let mut form = PayeeForm::new(&options(), &default_payees());
        assert!(!form.handle_select_key(SelectKey::Down));
        assert_eq!(form.default_category.value(), Some("none"));
    }

    #[test]
    fn tab_cycles_the_three_fields_both_ways() {
        let mut form = PayeeForm::new(&options(), &default_payees());
        form.cycle_focus(false);
        assert_eq!(form.focused, PayeeField::DefaultCategory);
        form.cycle_focus(false);
        assert_eq!(form.focused, PayeeField::Rule);
        form.cycle_focus(false);
        assert_eq!(form.focused, PayeeField::Name);
        form.cycle_focus(true);
        assert_eq!(form.focused, PayeeField::Rule);
    }

    #[test]
    fn add_rule_normalises_dedupes_and_ignores_blank() {
        let mut form = PayeeForm::new(&options(), &default_payees());
        form.rule_input = TextField::new(" aussie candle ");
        form.add_rule();
        form.rule_input = TextField::new("AUSSIE CANDLE");
        form.add_rule();
        form.rule_input = TextField::new("   ");
        form.add_rule();
        assert_eq!(form.rules, vec!["AUSSIE CANDLE".to_string()]);
        assert_eq!(form.rule_input.text(), "");
        assert_eq!(form.error, None);
    }

    #[test]
    fn add_rule_refuses_another_payees_alias_and_keeps_the_input() {
        let payees = default_payees();
        let mut form = PayeeForm::new(&options(), &payees);
        form.rule_input = TextField::new("woolies");
        form.add_rule();
        assert!(form.rules.is_empty());
        assert_eq!(form.rule_input.text(), "woolies");
        assert_eq!(
            form.error,
            Some(PayeeError::AliasTaken {
                alias: "WOOLIES".to_string(),
                owner: "Woolworths".to_string(),
            })
        );

        // Editing Woolworths itself, its own alias is fine.
        let woolworths = get(&payees, id_of(&payees, "Woolworths")).unwrap();
        let mut form = PayeeForm::from_payee(woolworths, &options(), &payees);
        form.rules.clear();
        form.rule_input = TextField::new("woolies");
        form.add_rule();
        assert_eq!(form.rules, vec!["WOOLIES".to_string()]);
    }

    #[test]
    fn remove_rule_ignores_an_index_past_the_end() {
        let mut form = PayeeForm::new(&options(), &default_payees());
        form.rules = vec!["ONE".to_string(), "TWO".to_string()];
        form.remove_rule(0);
        form.remove_rule(5);
        assert_eq!(form.rules, vec!["TWO".to_string()]);
    }

    #[test]
    fn the_name_is_checked_live_but_blank_is_only_invalid() {
        let payees = default_payees();
        let mut form = PayeeForm::new(&options(), &payees);
        assert_eq!(form.name_error(), None);
        assert!(!form.is_valid());
        form.name = TextField::new("coles");
        assert_eq!(
            form.name_error(),
            Some(PayeeError::DuplicateName("Coles".to_string()))
        );
        assert!(!form.is_valid());
        let coles = get(&payees, id_of(&payees, "Coles")).unwrap();
        let mut editing = PayeeForm::from_payee(coles, &options(), &payees);
        editing.name = TextField::new("coles");
        assert!(editing.is_valid());
        form.name = TextField::new("Aussie Candle Co");
        assert!(form.is_valid());
    }

    #[test]
    fn from_payee_prefills_name_category_and_rules() {
        let payees = default_payees();
        let options = options();
        let woolworths = get(&payees, id_of(&payees, "Woolworths")).unwrap();
        let form = PayeeForm::from_payee(woolworths, &options, &payees);
        assert_eq!(form.name.text(), "Woolworths");
        assert_eq!(form.default_category.value(), Some("Groceries"));
        assert_eq!(form.rules, woolworths.aliases);
        assert_eq!(form.draft().default_category, Some(7));
    }
}
