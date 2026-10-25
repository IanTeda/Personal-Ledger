//! The Accounts dialogs' form state: the Add, Edit and Delete account forms and the select
//! options behind them. `gpui`-free, like the rest of this domain; the page and its Dialogs render it.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_accounts::{Account, AccountDraft, GROUP_ORDER, NO_INSTITUTION, NewAccount};
use lib_core::{AccountType, InstitutionId, Money, UnitId};
use lib_locale::Label;

use crate::{
    form::{
        field::TextField,
        select::{SelectKey, SelectState},
    },
    shell::view::dialog_host::{DialogKey, DialogOutcome},
};

/// The Delete account dialog's live form state -- pure, `gpui`-free. Just the typed-back
/// confirmation: there is nothing to `Tab` between, so the input is implicitly always focused
/// (the same shape as the Settings `DeleteUnitForm`). The account's name is copied in at open so
/// the form validates without `Root`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteAccountForm {
    pub confirm_input: TextField,
    name: String,
}

impl DeleteAccountForm {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            confirm_input: TextField::default(),
            name: name.into(),
        }
    }

    /// The README's "disabled until the typed value matches the account name exactly": a plain
    /// case-sensitive `==`, no trimming or case folding, so it can't be confirmed by habit.
    pub fn is_valid(&self) -> bool {
        self.confirm_input.text() == self.name
    }
}

/// The Add account dialog's fields, in `Tab` order (the mockup's reading order: Name,
/// Institution / Type, Unit / Opening balance, Account number).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccountField {
    #[default]
    Name,
    Institution,
    Type,
    Unit,
    OpeningBalance,
    AccountNumber,
}

impl AccountField {
    const ORDER: [AccountField; 6] = [
        Self::Name,
        Self::Institution,
        Self::Type,
        Self::Unit,
        Self::OpeningBalance,
        Self::AccountNumber,
    ];

    pub fn is_select(self) -> bool {
        matches!(self, Self::Institution | Self::Type | Self::Unit)
    }
}

/// The option lists behind the three selects, all value keys: institution names and unit codes
/// come live from Settings, type labels from [`GROUP_ORDER`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountOptions {
    pub institutions: Vec<String>,
    pub types: Vec<String>,
    pub units: Vec<String>,
    /// The id of each of `institutions`, in the same order: the select stores the name, the
    /// Account the id.
    institution_ids: Vec<InstitutionId>,
    /// The id of each of `units`, in the same order: the select stores the code, the Account the id.
    unit_ids: Vec<UnitId>,
}

impl AccountOptions {
    pub fn new(institutions: Vec<(InstitutionId, String)>, units: Vec<(UnitId, String)>) -> Self {
        let (institution_ids, institutions) = institutions.into_iter().unzip();
        let (unit_ids, units) = units.into_iter().unzip();
        Self {
            institutions,
            types: GROUP_ORDER.iter().map(Label::label).collect(),
            units,
            institution_ids,
            unit_ids,
        }
    }

    /// The name the Institution select stores for `id`.
    pub fn institution_name(&self, id: InstitutionId) -> Option<&str> {
        let index = self
            .institution_ids
            .iter()
            .position(|institution| *institution == id)?;
        self.institutions.get(index).map(String::as_str)
    }

    /// The Institution the select's `name` names.
    pub fn institution_id(&self, name: &str) -> Option<InstitutionId> {
        let index = self
            .institutions
            .iter()
            .position(|institution| institution == name)?;
        self.institution_ids.get(index).copied()
    }

    /// The code the Unit select stores for `id`.
    pub fn unit_code(&self, id: UnitId) -> Option<&str> {
        let index = self.unit_ids.iter().position(|unit| *unit == id)?;
        self.units.get(index).map(String::as_str)
    }

    /// The Unit the select's `code` names.
    pub fn unit_id(&self, code: &str) -> Option<UnitId> {
        let index = self.units.iter().position(|unit| unit == code)?;
        self.unit_ids.get(index).copied()
    }

    /// Read back by the UI tests, which can't see the select's list otherwise.
    pub fn unit_codes(&self) -> &[String] {
        &self.units
    }

    /// The options behind `field`; empty for a text field.
    pub fn for_field(&self, field: AccountField) -> &[String] {
        match field {
            AccountField::Institution => &self.institutions,
            AccountField::Type => &self.types,
            AccountField::Unit => &self.units,
            _ => &[],
        }
    }
}

/// The type a [`Label::label`] names in the Locale in effect.
pub fn account_type_from_label(label: &str) -> Option<AccountType> {
    GROUP_ORDER
        .iter()
        .find(|account_type| account_type.label() == label)
        .cloned()
}

/// The Add and Edit account dialogs' live form state (`docs/ux/desktop-mockups/17-accounts/README.md`'s 3b) -- pure,
/// `gpui`-free. Name, Opening balance and Account number are typed into (append/pop only, like
/// the Settings dialogs); Institution, Type and Unit are [`SelectState`]s.
///
/// **Cash has no Institution to pick.** The glossary links a Cash account to a system-seeded
/// placeholder ([`NO_INSTITUTION`]) because the real-world thing has none, so while Type is Cash
/// the Institution select is read-only, `Tab` skips it, and the created account links to the
/// placeholder. Switching away from Cash brings the earlier pick back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountForm {
    pub name: TextField,
    pub institution: SelectState,
    pub account_type: SelectState,
    pub unit: SelectState,
    pub opening_balance: TextField,
    pub account_number: TextField,
    pub focused: AccountField,
    /// The select lists, copied from `Root` when the dialog opens so select keys and `Tab` need
    /// nothing else.
    options: AccountOptions,
    /// Edit mode: Unit and Opening balance are fixed once an account exists (FR.13), so they are
    /// shown read-only, `Tab` skips them and focus is refused.
    pub fixed_unit_and_balance: bool,
}

impl AccountForm {
    /// A fresh form: Type starts on Bank (the common case, and one that needs a real
    /// Institution), Institution on the first available, Unit on `default_unit` when it is one of
    /// the options and the first option otherwise.
    pub fn new(options: &AccountOptions, default_unit: Option<&str>) -> Self {
        let unit = default_unit
            .filter(|code| options.units.iter().any(|unit| unit == code))
            .map(str::to_string)
            .or_else(|| options.units.first().cloned());
        Self {
            name: TextField::default(),
            institution: SelectState::new(options.institutions.first().cloned()),
            account_type: SelectState::new(Some(AccountType::Bank.label())),
            unit: SelectState::new(unit),
            opening_balance: TextField::default(),
            account_number: TextField::default(),
            focused: AccountField::Name,
            options: options.clone(),
            fixed_unit_and_balance: false,
        }
    }

    /// A form pre-filled from `account` for the Edit dialog. A Cash account's Institution is the
    /// placeholder, not a pick, so that select starts on the first real Institution (which only
    /// matters if Type is switched away from Cash).
    pub fn from_account(account: &Account, options: &AccountOptions) -> Self {
        let institution = if account.institution == NO_INSTITUTION {
            options.institutions.first().cloned()
        } else {
            options
                .institution_name(account.institution)
                .map(str::to_string)
        };
        Self {
            name: TextField::new(account.name.clone()),
            institution: SelectState::new(institution),
            account_type: SelectState::new(Some(account.account_type.label())),
            unit: SelectState::new(options.unit_code(account.unit).map(str::to_string)),
            opening_balance: TextField::default(),
            account_number: TextField::new(account.account_number.clone().unwrap_or_default()),
            focused: AccountField::Name,
            options: options.clone(),
            fixed_unit_and_balance: true,
        }
    }

    /// Whether `field` cannot be focused right now: Unit and Opening balance while editing.
    fn is_fixed(&self, field: AccountField) -> bool {
        self.fixed_unit_and_balance
            && matches!(field, AccountField::Unit | AccountField::OpeningBalance)
    }

    pub fn selected_type(&self) -> Option<AccountType> {
        account_type_from_label(self.account_type.value()?)
    }

    /// Whether the chosen Type is Cash -- Institution is then the placeholder, not a pick.
    pub fn is_cash(&self) -> bool {
        self.selected_type() == Some(AccountType::Cash)
    }

    /// `field`'s select with its option list: disjoint fields, so a key can step the one through
    /// the other.
    fn select_and_list(&mut self, field: AccountField) -> Option<(&mut SelectState, &[String])> {
        let options = &self.options;
        let state = match field {
            AccountField::Institution => &mut self.institution,
            AccountField::Type => &mut self.account_type,
            AccountField::Unit => &mut self.unit,
            _ => return None,
        };
        Some((state, options.for_field(field)))
    }

    /// Whether any select's list is open.
    pub fn any_select_open(&self) -> bool {
        self.institution.is_open() || self.account_type.is_open() || self.unit.is_open()
    }

    /// Closes whichever list is open, discarding its highlight -- the first `Esc`. Returns whether
    /// one was open; if not, `Esc` should close the dialog.
    pub fn close_open_select(&mut self) -> bool {
        let was_open = self.any_select_open();
        self.institution.cancel();
        self.account_type.cancel();
        self.unit.cancel();
        was_open
    }

    /// Moves focus to `field`, closing any list open on another field. Focusing the read-only
    /// Cash Institution is refused.
    pub fn focus(&mut self, field: AccountField) {
        if (field == AccountField::Institution && self.is_cash()) || self.is_fixed(field) {
            return;
        }
        if field != self.focused {
            self.close_open_select();
        }
        self.focused = field;
    }

    /// `Tab` / `Shift-Tab`: commits an open list's highlight, then moves to the next field,
    /// wrapping and skipping the read-only Cash Institution.
    pub fn cycle_focus(&mut self, backward: bool) {
        let focused = self.focused;
        if let Some((state, list)) = self.select_and_list(focused) {
            state.commit(list);
        }
        let count = AccountField::ORDER.len();
        let mut index = AccountField::ORDER
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        for _ in 0..count {
            index = if backward {
                (index + count - 1) % count
            } else {
                (index + 1) % count
            };
            let candidate = AccountField::ORDER[index];
            if (candidate == AccountField::Institution && self.is_cash())
                || self.is_fixed(candidate)
            {
                continue;
            }
            self.focused = candidate;
            return;
        }
    }

    /// A key on the focused select. Returns whether the focused field is a select (and so took
    /// the key).
    pub fn handle_select_key(&mut self, key: SelectKey) -> bool {
        let focused = self.focused;
        let Some((state, list)) = self.select_and_list(focused) else {
            return false;
        };
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

    /// A click on a select's closed field: focuses it and toggles its list.
    pub fn click_select(&mut self, field: AccountField) {
        if !field.is_select()
            || (field == AccountField::Institution && self.is_cash())
            || self.is_fixed(field)
        {
            return;
        }
        self.focus(field);
        if let Some((state, list)) = self.select_and_list(field) {
            if state.is_open() {
                state.cancel();
            } else {
                state.open(list);
            }
        }
    }

    /// A click on row `index` of an open list.
    pub fn choose_option(&mut self, field: AccountField, index: usize) {
        if let Some((state, list)) = self.select_and_list(field) {
            state.choose(list, index);
        }
    }

    /// The text field typing and `Backspace` edit, `None` while a select has focus.
    pub(super) fn focused_text(&mut self) -> Option<&mut TextField> {
        match self.focused {
            AccountField::Name => Some(&mut self.name),
            AccountField::AccountNumber => Some(&mut self.account_number),
            AccountField::OpeningBalance => Some(&mut self.opening_balance),
            _ => None,
        }
    }

    /// The keys that are not plain typing: select stepping and opening, `Shift-Tab`, and the
    /// opening balance's own filter -- it only takes what can be part of a decimal amount: digits,
    /// one `.`, and a `-` at the start. A select swallows typed characters rather than letting
    /// them fall through to the shell.
    pub(super) fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let on_select = self.focused.is_select();
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
            DialogKey::Char(_) if on_select => {}
            DialogKey::Char(ch) if self.focused == AccountField::OpeningBalance => {
                let typed = self.opening_balance.text();
                let allowed = ch.is_ascii_digit()
                    || (ch == '.' && !typed.contains('.'))
                    || (ch == '-' && typed.is_empty());
                if allowed {
                    self.opening_balance.push(ch);
                }
            }
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    /// The typed opening balance: `0` when left empty (the placeholder is `0.00`), `None` when
    /// what was typed is not a decimal amount.
    pub fn opening_balance_money(&self) -> Option<Money> {
        match self.opening_balance.text().trim() {
            "" => Some(Money(BigDecimal::from(0))),
            text => text.parse().ok(),
        }
    }

    /// The Unit codes the Unit select offers, for a test to read back.
    pub fn unit_choices(&self) -> &[String] {
        self.options.unit_codes()
    }

    /// The README's dialog lifecycle: Name, Type, Unit and Opening balance are required (Type and
    /// Unit always hold a value once options exist; an empty balance means zero), Institution
    /// too unless the account is Cash, and Account number is optional.
    pub fn is_valid(&self) -> bool {
        !self.name.is_blank()
            && self.selected_type().is_some()
            && self.unit.value().is_some()
            && (self.is_cash() || self.institution.value().is_some())
            && self.opening_balance_money().is_some()
    }

    /// The Institution the account links to: the placeholder for Cash, else the picked one.
    fn resolved_institution(&self, account_type: &AccountType) -> Option<InstitutionId> {
        if *account_type == AccountType::Cash {
            Some(NO_INSTITUTION)
        } else {
            self.options.institution_id(self.institution.value()?)
        }
    }

    /// The Edit dialog's Save: the Name, Institution, Type and Account number to write, leaving
    /// Unit, balance and opened date as they were. `None` while the form is invalid.
    pub fn draft(&self) -> Option<AccountDraft> {
        let account_type = self.selected_type().filter(|_| self.is_valid())?;
        let institution = self.resolved_institution(&account_type)?;
        let account_number = self.account_number.text().trim();
        Some(AccountDraft {
            name: self.name.text().trim().to_string(),
            institution,
            account_type,
            account_number: (!account_number.is_empty()).then(|| account_number.to_string()),
        })
    }

    /// The Account this form describes, for the Ledger to add. `is_currency` pads a currency
    /// amount to two places (`1000` becomes `1000.00`); other Units keep what was typed. `None`
    /// while invalid.
    pub fn into_new_account(self, opened_at: NaiveDate, is_currency: bool) -> Option<NewAccount> {
        let unit = self.options.unit_id(self.unit.value()?)?;
        let draft = self.draft()?;
        let mut balance = self.opening_balance_money()?;
        if is_currency && balance.0.fractional_digit_count() < 2 {
            balance = Money(balance.0.with_scale(2));
        }
        Some(NewAccount {
            draft,
            unit,
            balance,
            opened_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use lib_accounts::edit_account;

    use super::*;
    use lib_accounts::{default_accounts, group_accounts};
    use lib_core::AccountId;

    use super::super::dialogs::AccountsDialog;
    use crate::shell::view::dialog_host::Dialog;

    fn date(year: i32, month: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, 1).expect("test dates are valid")
    }

    fn add_options() -> AccountOptions {
        AccountOptions::new(
            lib_institutions::default_institutions()
                .into_iter()
                .map(|institution| (institution.id, institution.name))
                .collect(),
            lib_units::default_units(chrono::Utc::now())
                .into_iter()
                .map(|unit| (unit.id, unit.code))
                .collect(),
        )
    }

    fn valid_form(options: &AccountOptions) -> AccountForm {
        let mut form = AccountForm::new(options, Some("aud"));
        form.name = TextField::new("Savings Maximiser");
        form
    }

    #[test]
    fn a_fresh_form_starts_on_bank_with_the_first_institution_and_default_unit() {
        let options = add_options();
        let form = AccountForm::new(&options, Some("btc"));
        assert_eq!(form.focused, AccountField::Name);
        assert_eq!(form.selected_type(), Some(AccountType::Bank));
        assert_eq!(form.institution.value(), Some("ANZ Banking Group"));
        assert_eq!(form.unit.value(), Some("btc"));
        assert!(!form.is_valid(), "name is required");
    }

    #[test]
    fn an_unknown_default_unit_falls_back_to_the_first_option() {
        let options = add_options();
        assert_eq!(
            AccountForm::new(&options, Some("zzz")).unit.value(),
            Some("aud")
        );
        assert_eq!(AccountForm::new(&options, None).unit.value(), Some("aud"));
    }

    #[test]
    fn type_options_follow_the_fixed_group_order() {
        assert_eq!(
            add_options().types,
            vec!["Cash", "Bank", "Credit card", "Loan", "Investment"]
        );
    }

    #[test]
    fn tab_walks_the_fields_in_reading_order_and_wraps() {
        let options = add_options();
        let mut form = valid_form(&options);
        let mut seen = vec![form.focused];
        for _ in 0..6 {
            form.cycle_focus(false);
            seen.push(form.focused);
        }
        assert_eq!(
            seen,
            vec![
                AccountField::Name,
                AccountField::Institution,
                AccountField::Type,
                AccountField::Unit,
                AccountField::OpeningBalance,
                AccountField::AccountNumber,
                AccountField::Name,
            ]
        );
        form.cycle_focus(true);
        assert_eq!(form.focused, AccountField::AccountNumber);
    }

    #[test]
    fn tab_commits_an_open_lists_highlight_before_moving_on() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::Type);
        form.handle_select_key(SelectKey::Activate);
        form.handle_select_key(SelectKey::Down);
        assert!(form.account_type.is_open());
        form.cycle_focus(false);
        assert!(!form.account_type.is_open());
        assert_eq!(form.selected_type(), Some(AccountType::CreditCard));
        assert_eq!(form.focused, AccountField::Unit);
    }

    #[test]
    fn cash_makes_institution_read_only_and_tab_skips_it() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::Type);
        form.handle_select_key(SelectKey::Up);
        assert!(form.is_cash());

        form.focus(AccountField::Institution);
        assert_eq!(form.focused, AccountField::Type, "focus is refused");
        form.focus(AccountField::Name);
        form.cycle_focus(false);
        assert_eq!(form.focused, AccountField::Type, "tab skips Institution");
        form.click_select(AccountField::Institution);
        assert!(!form.institution.is_open());
    }

    #[test]
    fn a_cash_account_links_to_the_placeholder_institution() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::Type);
        form.handle_select_key(SelectKey::Up);
        let draft = form.draft().expect("valid");
        assert_eq!(draft.account_type, AccountType::Cash);
        assert_eq!(draft.institution, NO_INSTITUTION);
    }

    #[test]
    fn leaving_cash_brings_the_earlier_institution_pick_back() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::Institution);
        form.handle_select_key(SelectKey::Down);
        assert_eq!(form.institution.value(), Some("American Express"));
        form.focus(AccountField::Type);
        form.handle_select_key(SelectKey::Up);
        form.handle_select_key(SelectKey::Down);
        assert!(!form.is_cash());
        assert_eq!(form.institution.value(), Some("American Express"));
    }

    #[test]
    fn a_focused_select_steps_opens_navigates_and_commits() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::Unit);
        assert!(form.handle_select_key(SelectKey::Down));
        assert_eq!(form.unit.value(), Some("btc"));
        form.handle_select_key(SelectKey::Activate);
        assert!(form.unit.is_open());
        form.handle_select_key(SelectKey::Down);
        form.handle_select_key(SelectKey::Activate);
        assert!(!form.unit.is_open());
        assert_eq!(form.unit.value(), Some("vas"));
    }

    #[test]
    fn a_select_key_on_a_text_field_is_not_taken() {
        let options = add_options();
        let mut form = valid_form(&options);
        assert!(!form.handle_select_key(SelectKey::Down));
    }

    #[test]
    fn close_open_select_reports_whether_one_was_open() {
        let options = add_options();
        let mut form = valid_form(&options);
        assert!(!form.close_open_select());
        form.focus(AccountField::Type);
        form.handle_select_key(SelectKey::Activate);
        assert!(form.close_open_select());
        assert!(!form.any_select_open());
        assert!(!form.close_open_select());
    }

    #[test]
    fn moving_focus_closes_a_list_open_on_another_field() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.click_select(AccountField::Unit);
        assert!(form.unit.is_open());
        form.focus(AccountField::Name);
        assert!(!form.unit.is_open());
    }

    #[test]
    fn clicking_a_select_toggles_it_and_a_row_click_chooses() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.click_select(AccountField::Type);
        assert!(form.account_type.is_open());
        assert_eq!(form.focused, AccountField::Type);
        form.choose_option(AccountField::Type, 3);
        assert!(!form.account_type.is_open());
        assert_eq!(form.selected_type(), Some(AccountType::Loan));
        form.click_select(AccountField::Type);
        form.click_select(AccountField::Type);
        assert!(!form.account_type.is_open());
    }

    #[test]
    fn validity_needs_a_name_and_a_parseable_balance() {
        let options = add_options();
        let mut form = valid_form(&options);
        assert!(form.is_valid());
        form.name = TextField::new("   ");
        assert!(!form.is_valid());
        form.name = TextField::new("Ok");
        form.opening_balance = TextField::new("-");
        assert!(!form.is_valid());
        form.opening_balance = TextField::default();
        assert!(form.is_valid(), "empty means zero");
    }

    #[test]
    fn into_new_account_builds_a_trimmed_row() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.name = TextField::new("  Rainy Day  ");
        form.opening_balance = TextField::new("1500");
        form.account_number = TextField::new("  ");
        let new = form.into_new_account(date(2026, 9), true).expect("valid");
        assert_eq!(new.draft.name, "Rainy Day");
        assert_eq!(new.draft.account_type, AccountType::Bank);
        assert_eq!(new.draft.institution, InstitutionId::SEED_ANZ);
        assert_eq!(new.unit, UnitId::SEED_AUD);
        assert_eq!(new.draft.account_number, None);
        assert_eq!(crate::view::format::amount(&new.balance).1, "1,500.00");
    }

    #[test]
    fn a_non_currency_amount_keeps_what_was_typed() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.unit = SelectState::new(Some("vas".to_string()));
        form.opening_balance = TextField::new("10");
        let new = form.into_new_account(date(2026, 9), false).expect("valid");
        assert_eq!(crate::view::format::amount(&new.balance).1, "10");
    }

    #[test]
    fn a_negative_opening_balance_is_kept_and_a_longer_scale_is_not_rounded() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.opening_balance = TextField::new("-250.005");
        let new = form.into_new_account(date(2026, 9), true).expect("valid");
        assert_eq!(
            crate::view::format::amount(&new.balance),
            (true, "\u{2212}250.005".to_string())
        );
    }

    #[test]
    fn an_invalid_form_builds_nothing() {
        let options = add_options();
        let form = AccountForm::new(&options, None);
        assert_eq!(form.into_new_account(date(2026, 9), true), None);
    }

    /// The Edit dialog's Save against one row, as the Ledger's `edit_account` runs it.
    fn apply_to(form: &AccountForm, account: &mut Account) -> bool {
        let Some(draft) = form.draft() else {
            return false;
        };
        let id = account.id;
        edit_account(std::slice::from_mut(account), id, &draft).is_ok()
    }

    fn seeded(name: &str) -> Account {
        default_accounts()
            .into_iter()
            .find(|account| account.name == name)
            .expect("seeded")
    }

    #[test]
    fn from_account_prefills_every_field_and_marks_unit_and_balance_fixed() {
        let options = add_options();
        let form = AccountForm::from_account(&seeded("ANZ Everyday"), &options);
        assert_eq!(form.name.text(), "ANZ Everyday");
        assert_eq!(form.institution.value(), Some("ANZ Banking Group"));
        assert_eq!(form.selected_type(), Some(AccountType::Bank));
        assert_eq!(form.unit.value(), Some("aud"));
        assert_eq!(form.account_number.text(), "1234 5678");
        assert!(form.fixed_unit_and_balance);
        assert_eq!(form.focused, AccountField::Name);
        assert!(form.is_valid());
    }

    #[test]
    fn a_cash_account_edits_with_the_placeholder_and_a_real_institution_ready() {
        let options = add_options();
        let form = AccountForm::from_account(&seeded("Wallet"), &options);
        assert!(form.is_cash());
        assert_eq!(form.institution.value(), Some("ANZ Banking Group"));
        assert_eq!(form.account_number.text(), "");
    }

    #[test]
    fn tab_in_edit_mode_skips_the_fixed_unit_and_opening_balance() {
        let options = add_options();
        let mut form = AccountForm::from_account(&seeded("ANZ Everyday"), &options);
        let mut seen = vec![form.focused];
        for _ in 0..4 {
            form.cycle_focus(false);
            seen.push(form.focused);
        }
        assert_eq!(
            seen,
            vec![
                AccountField::Name,
                AccountField::Institution,
                AccountField::Type,
                AccountField::AccountNumber,
                AccountField::Name,
            ]
        );
    }

    #[test]
    fn edit_mode_refuses_focus_and_clicks_on_the_fixed_fields() {
        let options = add_options();
        let mut form = AccountForm::from_account(&seeded("ANZ Everyday"), &options);
        form.focus(AccountField::Unit);
        assert_eq!(form.focused, AccountField::Name);
        form.focus(AccountField::OpeningBalance);
        assert_eq!(form.focused, AccountField::Name);
        form.click_select(AccountField::Unit);
        assert!(!form.unit.is_open());
        assert_eq!(form.unit.value(), Some("aud"));
    }

    #[test]
    fn apply_to_changes_name_institution_type_and_number_only() {
        let options = add_options();
        let mut account = seeded("ANZ Everyday");
        let before = account.clone();
        let mut form = AccountForm::from_account(&account, &options);
        form.name = TextField::new("  Daily  ");
        form.institution = SelectState::new(Some("Westpac Banking".to_string()));
        form.account_type = SelectState::new(Some("Credit card".to_string()));
        form.account_number = TextField::new("  ");

        assert!(apply_to(&form, &mut account));
        assert_eq!(account.name, "Daily");
        assert_eq!(account.institution, InstitutionId::SEED_WESTPAC);
        assert_eq!(account.account_type, AccountType::CreditCard);
        assert_eq!(account.account_number, None);
        assert_eq!(account.id, before.id);
        assert_eq!(account.unit, before.unit);
        assert_eq!(account.balance, before.balance);
        assert_eq!(account.opened_at, before.opened_at);
        assert_eq!(account.transaction_count, before.transaction_count);
        assert_eq!(account.budget_count, before.budget_count);
    }

    #[test]
    fn changing_type_regroups_the_account() {
        let options = add_options();
        let mut accounts = default_accounts();
        let index = accounts
            .iter()
            .position(|account| account.name == "ANZ Everyday")
            .expect("seeded");
        let mut form = AccountForm::from_account(&accounts[index], &options);
        form.account_type = SelectState::new(Some("Loan".to_string()));
        assert!(apply_to(&form, &mut accounts[index]));

        let loans = group_accounts(&accounts)
            .into_iter()
            .find(|group| group.account_type == AccountType::Loan)
            .expect("loan group");
        assert!(loans.indices.contains(&index));
    }

    #[test]
    fn editing_to_cash_links_the_placeholder_and_back_restores_a_real_institution() {
        let options = add_options();
        let mut account = seeded("ANZ Everyday");
        let mut form = AccountForm::from_account(&account, &options);
        form.account_type = SelectState::new(Some("Cash".to_string()));
        assert!(apply_to(&form, &mut account));
        assert_eq!(account.institution, NO_INSTITUTION);

        let mut form = AccountForm::from_account(&account, &options);
        form.account_type = SelectState::new(Some("Bank".to_string()));
        assert!(apply_to(&form, &mut account));
        assert_eq!(account.institution, InstitutionId::SEED_ANZ);
    }

    #[test]
    fn apply_to_leaves_the_account_alone_while_invalid() {
        let options = add_options();
        let mut account = seeded("ANZ Everyday");
        let before = account.clone();
        let mut form = AccountForm::from_account(&account, &options);
        form.name = TextField::new("   ");
        assert!(!apply_to(&form, &mut account));
        assert_eq!(account, before);
    }

    #[test]
    fn the_dialog_exposes_its_form_for_add_and_edit_but_not_delete() {
        let options = add_options();
        let form = AccountForm::new(&options, None);
        assert!(AccountsDialog::Add(form.clone()).form().is_some());
        assert!(
            AccountsDialog::Edit(AccountId::seed(1), form)
                .form()
                .is_some()
        );
        assert!(
            AccountsDialog::Delete(AccountId::seed(1), DeleteAccountForm::new("Wallet"))
                .form()
                .is_none()
        );
    }

    use crate::shell::view::dialog_host::{
        DialogKey, DialogOutcome, OpenDialog, handle_key, type_text,
    };

    fn account_options() -> AccountOptions {
        AccountOptions::new(
            vec![
                (InstitutionId::seed(1), "ANZ".to_string()),
                (InstitutionId::seed(2), "CBA".to_string()),
            ],
            vec![
                (UnitId::SEED_AUD, "aud".to_string()),
                (UnitId::SEED_BTC, "btc".to_string()),
            ],
        )
    }

    fn add_account() -> OpenDialog {
        OpenDialog::Accounts(AccountsDialog::Add(AccountForm::new(
            &account_options(),
            Some("aud"),
        )))
    }

    fn account_form(dialog: &OpenDialog) -> &AccountForm {
        match dialog {
            OpenDialog::Accounts(AccountsDialog::Add(form) | AccountsDialog::Edit(_, form)) => form,
            other => panic!("expected an account form, got {other:?}"),
        }
    }

    #[test]
    fn account_typing_edits_the_focused_text_field_and_tab_moves_on() {
        let mut dialog = add_account();
        type_text(&mut dialog, "Everyday");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(account_form(&dialog).name.text(), "Everyda");

        // Name -> Institution (a select): typed characters are swallowed, not typed.
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(account_form(&dialog).focused, AccountField::Institution);
        type_text(&mut dialog, "x");
        assert_eq!(account_form(&dialog).name.text(), "Everyda");
        assert_eq!(account_form(&dialog).account_number.text(), "");

        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(account_form(&dialog).focused, AccountField::Name);
    }

    #[test]
    fn account_opening_balance_only_takes_a_decimal_amount() {
        let mut dialog = add_account();
        if let OpenDialog::Accounts(AccountsDialog::Add(form)) = &mut dialog {
            form.focus(AccountField::OpeningBalance);
        }
        type_text(&mut dialog, "-12a3.4.5e-");
        assert_eq!(account_form(&dialog).opening_balance.text(), "-123.45");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(account_form(&dialog).opening_balance.text(), "-123.4");
    }

    #[test]
    fn account_enter_confirms_only_with_a_name_and_activates_a_focused_select() {
        let mut dialog = add_account();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "Everyday");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );

        // On a select, Enter opens its list instead of confirming.
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        assert!(account_form(&dialog).institution.is_open());
    }

    #[test]
    fn account_select_keys_step_and_the_first_esc_closes_only_the_open_list() {
        let mut dialog = add_account();
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(account_form(&dialog).institution.value(), Some("ANZ"));
        handle_key(&mut dialog, DialogKey::Down);
        assert_eq!(account_form(&dialog).institution.value(), Some("CBA"));

        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(account_form(&dialog).institution.is_open());
        assert!(dialog.close_open_select(), "first Esc closes the list");
        assert!(!dialog.close_open_select(), "second Esc closes the Dialog");
    }

    #[test]
    fn delete_account_confirms_only_on_the_exact_case_sensitive_name() {
        let mut dialog = OpenDialog::Accounts(AccountsDialog::Delete(
            AccountId::seed(1),
            DeleteAccountForm::new("Amex Platinum"),
        ));
        type_text(&mut dialog, "amex platinum");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        for _ in 0.."amex platinum".len() {
            handle_key(&mut dialog, DialogKey::Backspace);
        }
        type_text(&mut dialog, "Amex Platinum");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
        type_text(&mut dialog, " ");
        assert!(!dialog.is_valid(), "no trimming");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );
    }
}
