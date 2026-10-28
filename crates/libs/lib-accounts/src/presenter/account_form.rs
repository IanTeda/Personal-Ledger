//! The Add and Edit account Dialog (`docs/ux/desktop-mockups/17-accounts/README.md`'s 3b and 3c):
//! one form, Edit pre-filled from the Account.
//!
//! - The fields, in `Tab` order: Name, Institution, Type, Unit, Opening balance and Account number.
//! - **The Institution follows the Type until one is picked** (ADR-0049). A new Cash Account
//!   starts on the Personal Institution when it is on the Ledger, any other Type on the first
//!   Institution that isn't it, and either otherwise on no choice. Any Institution can be picked
//!   for any Type, Cash included, and a pick stays through a change of Type.
//! - **Edit fixes Unit and Opening balance**: they are shown read-only, `Tab` skips them and focus
//!   is refused. The Edit's notice says how many Transactions the Account carries.
//! - Opening balance only takes what can be part of a decimal amount: digits, one `.` and a `-` at
//!   the start. An empty balance means zero; a currency's is padded to two places when added.
//!
//! The Institution, Type and Unit choices and the Account's use are read when the Dialog opens, as
//! it is modal and nothing changes them under it.

use bigdecimal::BigDecimal;
use chrono::{Datelike, NaiveDate};
use lib_core::{AccountId, AccountType, InstitutionId, Money, UnitId};
use lib_locale::Label;
use lib_locale::format::{AmountStyle, format_amount};
use lib_viper::view::{Choice, DialogFrame, SelectView, TextFieldView};

use super::count;
use crate::interactor::{AddAccount, EditAccount};
use crate::protocols::{AccountsOutcome, AccountsRouter, SharedPorts};
use crate::{AccountDraft, AccountError, GROUP_ORDER, NewAccount, msg};

/// The form's fields, in `Tab` order.
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
    const ORDER: [Self; 6] = [
        Self::Name,
        Self::Institution,
        Self::Type,
        Self::Unit,
        Self::OpeningBalance,
        Self::AccountNumber,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountFormEvent {
    /// A text field's whole new text, as its View's input holds it.
    Name(String),
    /// Ignored unless the text could be part of a decimal amount.
    OpeningBalance(String),
    AccountNumber(String),
    /// A select's chosen option. Ignored when it isn't one of the options.
    Institution(InstitutionId),
    Type(AccountType),
    Unit(UnitId),
    /// `Tab`/`Shift-Tab`, wrapping and skipping the fields that can't take focus.
    NextField,
    PrevField,
    /// A click on a field. Ignored for a field that can't take focus.
    Focus(AccountField),
    Confirm,
    Cancel,
}

/// The use the Edit form quotes, read when it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    /// `Opened Mar 2019 · 312 transactions. Renaming is safe. …`
    pub notice: String,
    pub transactions: usize,
    pub budgets: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountFormViewModel {
    pub frame: DialogFrame,
    pub name: TextFieldView,
    pub institution: SelectView<InstitutionId>,
    pub account_type: SelectView<AccountType>,
    pub unit: SelectView<UnitId>,
    /// Edit: the Unit's code, which can no longer change.
    pub unit_read_only: Option<String>,
    pub opening_balance: TextFieldView,
    /// Edit: the opening balance as written, which can no longer change.
    pub balance_read_only: Option<String>,
    pub account_number: TextFieldView,
    /// Edit only.
    pub usage: Option<Usage>,
    /// Why the last confirm was refused.
    pub error: Option<String>,
    pub editing: bool,
    pub focused: AccountField,
}

/// The Account an Edit saves, with what its fixed fields show.
struct Editing {
    id: AccountId,
    unit: String,
    balance: Money,
    usage: Usage,
}

pub struct AccountFormPresenter {
    ports: SharedPorts,
    today: NaiveDate,
    editing: Option<Editing>,
    institutions: Vec<Choice<InstitutionId>>,
    types: Vec<Choice<AccountType>>,
    units: Vec<Choice<UnitId>>,
    name: String,
    institution: Option<InstitutionId>,
    /// Whether the Institution is the person's pick (or an Edit's own), which a change of Type
    /// leaves alone, rather than the Type's default.
    institution_picked: bool,
    account_type: AccountType,
    unit: Option<UnitId>,
    opening_balance: String,
    account_number: String,
    focused: AccountField,
    error: Option<AccountError>,
}

impl AccountFormPresenter {
    /// A fresh Add form: Type on Bank (the common case), Institution on Bank's default, Unit on the
    /// base Unit when it is one of the options and the first option otherwise. The Account opens
    /// `today`.
    pub fn add(ports: SharedPorts, name: &str, today: NaiveDate) -> Self {
        let (institutions, units, base) = {
            let ports = ports.borrow();
            (
                ports.institutions().account_institutions(),
                ports.units().account_units(),
                ports.units().base_unit(),
            )
        };
        let units: Vec<Choice<UnitId>> = units
            .into_iter()
            .filter(|unit| unit.is_active)
            .map(|unit| Choice {
                value: unit.id,
                label: unit.code,
            })
            .collect();
        let unit = base
            .filter(|base| units.iter().any(|choice| choice.value == *base))
            .or_else(|| units.first().map(|choice| choice.value));
        let institutions: Vec<Choice<InstitutionId>> = institutions
            .into_iter()
            .map(|institution| Choice {
                value: institution.id,
                label: institution.name,
            })
            .collect();
        let mut form = Self {
            ports,
            today,
            editing: None,
            institution: None,
            institution_picked: false,
            institutions,
            types: GROUP_ORDER
                .iter()
                .map(|account_type| Choice {
                    value: account_type.clone(),
                    label: account_type.label(),
                })
                .collect(),
            units,
            name: name.trim().to_string(),
            account_type: AccountType::Bank,
            unit,
            opening_balance: String::new(),
            account_number: String::new(),
            focused: AccountField::Name,
            error: None,
        };
        form.institution = form.default_institution();
        form
    }

    /// The Edit form pre-filled from Account `id`, or `None` when it is gone. Its Institution counts
    /// as picked, so a change of Type leaves it; one no longer among the choices falls back to the
    /// Type's default, so the form still saves.
    pub fn edit(ports: SharedPorts, id: AccountId, today: NaiveDate) -> Option<Self> {
        let (account, usage, unit_code) = {
            let ports = ports.borrow();
            let account = ports.accounts().account(id)?;
            let usage = ports.usage().account_usage(id);
            let unit_code = ports
                .units()
                .account_units()
                .into_iter()
                .find(|unit| unit.id == account.unit)
                .map(|unit| unit.code);
            (account, usage, unit_code)
        };
        let mut form = Self::add(ports, &account.name, today);
        // A deactivated Unit takes no new Account, but the one already on it keeps showing.
        if !form.units.iter().any(|choice| choice.value == account.unit)
            && let Some(code) = &unit_code
        {
            form.units.push(Choice {
                value: account.unit,
                label: code.clone(),
            });
        }
        form.account_type = account.account_type.clone();
        // An Institution that is no longer a choice opens as no choice, never as the Type's default,
        // so Save needs a pick instead of silently re-pointing the Account. It counts as picked so
        // a change of Type doesn't fill it in either.
        form.institution = form
            .institutions
            .iter()
            .any(|choice| choice.value == account.institution)
            .then_some(account.institution);
        form.institution_picked = true;
        form.unit = Some(account.unit);
        form.account_number = account.account_number.clone().unwrap_or_default();
        form.editing = Some(Editing {
            id,
            unit: unit_code.unwrap_or_default(),
            balance: account.balance.clone(),
            usage: Usage {
                notice: msg::accounts_edit_usage_notice(
                    &lib_locale::format::format_year_month(
                        account.opened_at.year(),
                        account.opened_at.month(),
                    ),
                    count(usage.transactions),
                ),
                transactions: usage.transactions,
                budgets: usage.budgets,
            },
        });
        Some(form)
    }

    pub fn handle(&mut self, event: AccountFormEvent, router: &mut dyn AccountsRouter) {
        self.error = None;
        match event {
            AccountFormEvent::Name(text) => self.name = text,
            AccountFormEvent::OpeningBalance(text) => {
                if self.editing.is_none() && accepts_balance_text(&text) {
                    self.opening_balance = text;
                }
            }
            AccountFormEvent::AccountNumber(text) => self.account_number = text,
            AccountFormEvent::Institution(id) => {
                if self.institutions.iter().any(|choice| choice.value == id) {
                    self.institution = Some(id);
                    self.institution_picked = true;
                }
            }
            AccountFormEvent::Type(account_type) => {
                if self.types.iter().any(|choice| choice.value == account_type) {
                    self.account_type = account_type;
                    if !self.institution_picked {
                        self.institution = self.default_institution();
                    }
                }
            }
            AccountFormEvent::Unit(id) => {
                if self.editing.is_none() && self.units.iter().any(|choice| choice.value == id) {
                    self.unit = Some(id);
                }
            }
            AccountFormEvent::NextField => self.cycle(false),
            AccountFormEvent::PrevField => self.cycle(true),
            AccountFormEvent::Focus(field) => {
                if !self.is_skipped(field) {
                    self.focused = field;
                }
            }
            AccountFormEvent::Confirm => self.confirm(router),
            AccountFormEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> AccountFormViewModel {
        let (title, submit) = if self.editing.is_some() {
            (msg::accounts_edit_title(), msg::accounts_edit_submit())
        } else {
            (msg::accounts_add_title(), msg::accounts_add_submit())
        };
        let focus = |field: AccountField| self.focused == field;
        AccountFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.is_valid(),
                destructive: false,
            },
            name: TextFieldView {
                label: lib_locale::msg::column_name(),
                text: self.name.clone(),
                placeholder: msg::accounts_name_placeholder(),
                focused: focus(AccountField::Name),
            },
            institution: SelectView {
                label: lib_locale::msg::column_institution(),
                options: self.institutions.clone(),
                chosen: self.institution,
                focused: focus(AccountField::Institution),
            },
            account_type: SelectView {
                label: lib_locale::msg::column_type(),
                options: self.types.clone(),
                chosen: Some(self.account_type.clone()),
                focused: focus(AccountField::Type),
            },
            unit: SelectView {
                label: lib_locale::msg::column_unit(),
                options: self.units.clone(),
                chosen: self.unit,
                focused: focus(AccountField::Unit),
            },
            unit_read_only: self.editing.as_ref().map(|editing| editing.unit.clone()),
            opening_balance: TextFieldView {
                label: msg::accounts_field_opening_balance(),
                text: self.opening_balance.clone(),
                placeholder: msg::accounts_balance_placeholder(),
                focused: focus(AccountField::OpeningBalance),
            },
            balance_read_only: self
                .editing
                .as_ref()
                .map(|editing| format_amount(&editing.balance, AmountStyle::own()).text),
            account_number: TextFieldView {
                label: msg::accounts_field_number(),
                text: self.account_number.clone(),
                placeholder: msg::accounts_number_placeholder(),
                focused: focus(AccountField::AccountNumber),
            },
            usage: self.editing.as_ref().map(|editing| editing.usage.clone()),
            error: self.error.map(refusal_text),
            editing: self.editing.is_some(),
            focused: self.focused,
        }
    }

    /// The Institution the Type starts on: for Cash the Personal Institution, for any other Type
    /// the first Institution that isn't it, and `None` when the Ledger has no such Institution.
    fn default_institution(&self) -> Option<InstitutionId> {
        let personal = |id: InstitutionId| id == InstitutionId::SEED_PERSONAL;
        let is_cash = self.account_type == AccountType::Cash;
        self.institutions
            .iter()
            .map(|choice| choice.value)
            .find(|id| personal(*id) == is_cash)
    }

    /// Whether `field` can't take focus right now: the Unit and Opening balance of an Account that
    /// exists.
    fn is_skipped(&self, field: AccountField) -> bool {
        self.editing.is_some() && matches!(field, AccountField::Unit | AccountField::OpeningBalance)
    }

    fn cycle(&mut self, backward: bool) {
        let order = AccountField::ORDER;
        let count = order.len();
        let mut index = order
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        for _ in 0..count {
            index = if backward {
                (index + count - 1) % count
            } else {
                (index + 1) % count
            };
            if !self.is_skipped(order[index]) {
                self.focused = order[index];
                return;
            }
        }
    }

    /// The typed opening balance: zero when left empty, `None` when what was typed is not a
    /// decimal amount.
    fn balance(&self) -> Option<Money> {
        match self.opening_balance.trim() {
            "" => Some(Money(BigDecimal::from(0))),
            text => text.parse().ok(),
        }
    }

    /// Name, Institution, Type, Unit and Opening balance are required (an empty balance means
    /// zero), and Account number is optional.
    fn is_valid(&self) -> bool {
        !self.name.trim().is_empty()
            && self.unit.is_some()
            && self.institution.is_some()
            && self.balance().is_some()
    }

    /// The Name, Institution, Type and Account number to write.
    fn draft(&self) -> Option<AccountDraft> {
        let institution = self.institution?;
        let number = self.account_number.trim();
        Some(AccountDraft {
            name: self.name.trim().to_string(),
            institution,
            account_type: self.account_type.clone(),
            account_number: (!number.is_empty()).then(|| number.to_string()),
        })
    }

    fn confirm(&mut self, router: &mut dyn AccountsRouter) {
        if !self.is_valid() {
            return;
        }
        let Some(draft) = self.draft() else {
            return;
        };
        let saved = match &self.editing {
            Some(editing) => {
                let mut ports = self.ports.borrow_mut();
                EditAccount {
                    id: editing.id,
                    draft,
                }
                .run(ports.accounts_mut())
            }
            None => {
                let (Some(unit), Some(mut balance)) = (self.unit, self.balance()) else {
                    return;
                };
                let mut ports = self.ports.borrow_mut();
                let is_currency = ports
                    .units()
                    .account_units()
                    .iter()
                    .find(|row| row.id == unit)
                    .is_none_or(|row| row.is_currency);
                if is_currency && balance.0.fractional_digit_count() < 2 {
                    balance = Money(balance.0.with_scale(2));
                }
                Ok(AddAccount(NewAccount {
                    draft,
                    unit,
                    balance,
                    opened_at: self.today,
                })
                .run(ports.accounts_mut()))
            }
        };
        match saved {
            Ok(account) => router.close_dialog(Some(AccountsOutcome { id: account.id })),
            Err(refusal) => self.error = Some(refusal),
        }
    }
}

/// Whether `text` could be the start of a decimal amount: digits, at most one `.`, and a `-` only
/// at the start.
fn accepts_balance_text(text: &str) -> bool {
    text.chars()
        .enumerate()
        .all(|(at, ch)| ch.is_ascii_digit() || ch == '.' || (ch == '-' && at == 0))
        && text.matches('.').count() <= 1
}

/// A refusal as the Dialog shows it. An incomplete form never confirms, so only a gone Account is
/// ever shown.
fn refusal_text(error: AccountError) -> String {
    match error {
        AccountError::NotFound | AccountError::Incomplete => msg::accounts_error_gone(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::fake::{RecordingRouter, RouterCall};
    use crate::{AccountRepository as _, AccountUse};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 26).expect("a date")
    }

    fn send(form: &mut AccountFormPresenter, events: Vec<AccountFormEvent>) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            form.handle(event, &mut router);
        }
        router.calls
    }

    fn name(text: &str) -> AccountFormEvent {
        AccountFormEvent::Name(text.to_string())
    }

    fn balance(text: &str) -> AccountFormEvent {
        AccountFormEvent::OpeningBalance(text.to_string())
    }

    fn add(shared: SharedPorts) -> AccountFormPresenter {
        AccountFormPresenter::add(shared, "", today())
    }

    #[test]
    fn a_fresh_form_starts_on_bank_with_the_first_institution_and_the_base_unit() {
        let (_, shared) = stub();
        let view = add(shared).view_model();

        assert_eq!(view.frame.title, "Add account");
        assert_eq!(view.frame.submit, "Add account");
        assert_eq!(view.focused, AccountField::Name);
        assert_eq!(view.account_type.chosen, Some(AccountType::Bank));
        assert_eq!(view.institution.chosen, Some(InstitutionId::SEED_ANZ));
        assert_eq!(view.unit.chosen, Some(UnitId::SEED_AUD));
        assert!(!view.frame.can_confirm, "a name is required");
        let types: Vec<&str> = view
            .account_type
            .options
            .iter()
            .map(|choice| choice.label.as_str())
            .collect();
        assert_eq!(types, ["Cash", "Bank", "Credit card", "Loan", "Investment"]);
    }

    #[test]
    fn a_base_unit_that_is_not_an_option_falls_back_to_the_first_and_inactive_units_are_left_out() {
        let (ports, shared) = stub();
        ports.borrow_mut().units.base = None;
        ports.borrow_mut().units.units[0].is_active = false;
        let view = add(shared).view_model();

        let codes: Vec<&str> = view
            .unit
            .options
            .iter()
            .map(|choice| choice.label.as_str())
            .collect();
        assert_eq!(codes, ["vas", "btc"]);
        assert_eq!(view.unit.chosen, Some(UnitId::SEED_VAS));
    }

    #[test]
    fn a_typed_new_prefills_the_name() {
        let (_, shared) = stub();
        let form = AccountFormPresenter::add(shared, " Rainy Day ", today());

        assert_eq!(form.view_model().name.text, "Rainy Day");
        assert!(form.view_model().frame.can_confirm);
    }

    #[test]
    fn tab_walks_the_fields_in_reading_order_and_wraps_both_ways() {
        let (_, shared) = stub();
        let mut form = add(shared);
        let mut seen = vec![form.view_model().focused];
        for _ in 0..6 {
            send(&mut form, vec![AccountFormEvent::NextField]);
            seen.push(form.view_model().focused);
        }
        assert_eq!(
            seen,
            [
                AccountField::Name,
                AccountField::Institution,
                AccountField::Type,
                AccountField::Unit,
                AccountField::OpeningBalance,
                AccountField::AccountNumber,
                AccountField::Name,
            ]
        );
        send(&mut form, vec![AccountFormEvent::PrevField]);
        assert_eq!(form.view_model().focused, AccountField::AccountNumber);
    }

    #[test]
    fn cash_starts_on_the_personal_institution_and_another_type_on_the_first_other_one() {
        let (ports, shared) = stub();
        let mut form = add(shared);
        send(
            &mut form,
            vec![name("Tin"), AccountFormEvent::Type(AccountType::Cash)],
        );
        assert_eq!(
            form.view_model().institution.chosen,
            Some(InstitutionId::SEED_PERSONAL)
        );

        send(&mut form, vec![AccountFormEvent::NextField]);
        assert_eq!(
            form.view_model().focused,
            AccountField::Institution,
            "Tab no longer skips it"
        );
        send(&mut form, vec![AccountFormEvent::Type(AccountType::Loan)]);
        assert_eq!(
            form.view_model().institution.chosen,
            Some(InstitutionId::SEED_ANZ)
        );

        send(
            &mut form,
            vec![
                AccountFormEvent::Type(AccountType::Cash),
                AccountFormEvent::Confirm,
            ],
        );
        let tin = ports.borrow().accounts.accounts().last().cloned();
        assert_eq!(
            tin.map(|tin| tin.institution),
            Some(InstitutionId::SEED_PERSONAL)
        );
    }

    #[test]
    fn a_cash_account_may_name_any_institution_and_a_pick_stays_through_a_change_of_type() {
        let (ports, shared) = stub();
        let mut form = add(shared);
        send(
            &mut form,
            vec![
                name("Float"),
                AccountFormEvent::Institution(InstitutionId::SEED_AMEX),
                AccountFormEvent::Type(AccountType::Cash),
            ],
        );
        assert_eq!(
            form.view_model().institution.chosen,
            Some(InstitutionId::SEED_AMEX)
        );

        send(&mut form, vec![AccountFormEvent::Confirm]);
        let float = ports.borrow().accounts.accounts().last().cloned();
        assert_eq!(
            float.map(|float| (float.institution, float.account_type)),
            Some((InstitutionId::SEED_AMEX, AccountType::Cash))
        );
    }

    #[test]
    fn with_no_personal_institution_a_cash_account_starts_on_no_choice() {
        let (ports, shared) = stub();
        ports
            .borrow_mut()
            .institutions
            .institutions
            .retain(|row| row.id != InstitutionId::SEED_PERSONAL);
        let mut form = add(shared);
        send(
            &mut form,
            vec![name("Tin"), AccountFormEvent::Type(AccountType::Cash)],
        );

        let view = form.view_model();
        assert_eq!(view.institution.chosen, None);
        assert!(!view.frame.can_confirm, "an Institution is required");
        send(
            &mut form,
            vec![AccountFormEvent::Institution(InstitutionId::SEED_WESTPAC)],
        );
        assert!(form.view_model().frame.can_confirm);
    }

    #[test]
    fn a_form_confirms_only_with_a_name_a_unit_and_a_parseable_balance() {
        let (_, shared) = stub();
        let mut form = add(shared);
        assert!(!form.view_model().frame.can_confirm);
        send(&mut form, vec![name("   ")]);
        assert!(
            !form.view_model().frame.can_confirm,
            "a blank name only waits"
        );
        send(&mut form, vec![name("Savings")]);
        assert!(form.view_model().frame.can_confirm);
        send(&mut form, vec![balance("-")]);
        assert!(
            !form.view_model().frame.can_confirm,
            "a lone minus isn't an amount"
        );
        assert_eq!(send(&mut form, vec![AccountFormEvent::Confirm]), []);
        send(&mut form, vec![balance("-5")]);
        assert!(form.view_model().frame.can_confirm);
    }

    #[test]
    fn the_opening_balance_only_takes_what_can_be_part_of_a_decimal_amount() {
        let (_, shared) = stub();
        let mut form = add(shared);
        for text in ["1a", "1.2.3", "1-", "--1", "x"] {
            send(&mut form, vec![balance("12"), balance(text)]);
            assert_eq!(form.view_model().opening_balance.text, "12", "{text}");
        }
        send(&mut form, vec![balance("-12.50")]);
        assert_eq!(form.view_model().opening_balance.text, "-12.50");
    }

    #[test]
    fn an_add_pads_a_currency_balance_to_two_places_and_keeps_another_units() {
        let (ports, shared) = stub();
        let mut form = add(shared.clone());
        let calls = send(
            &mut form,
            vec![
                name("Rainy Day"),
                balance("1500"),
                AccountFormEvent::AccountNumber(" 42 ".into()),
                AccountFormEvent::Confirm,
            ],
        );

        let [RouterCall::CloseDialog(Some(outcome))] = calls.as_slice() else {
            panic!("expected one close, got {calls:?}");
        };
        let added = ports.borrow().accounts.account(outcome.id).expect("added");
        assert_eq!(added.name, "Rainy Day");
        assert_eq!(added.balance.to_string(), "1500.00");
        assert_eq!(added.account_number.as_deref(), Some("42"));
        assert_eq!(added.opened_at, today());
        assert_eq!(added.unit, UnitId::SEED_AUD);

        let mut fund = add(shared);
        send(
            &mut fund,
            vec![
                name("Coins"),
                AccountFormEvent::Unit(UnitId::SEED_BTC),
                balance("0.4"),
                AccountFormEvent::Confirm,
            ],
        );
        let coins = ports
            .borrow()
            .accounts
            .accounts()
            .into_iter()
            .find(|account| account.name == "Coins")
            .expect("added");
        assert_eq!(
            coins.balance.to_string(),
            "0.4",
            "not a currency, so as typed"
        );
    }

    #[test]
    fn edit_prefills_the_account_fixes_the_unit_and_balance_and_quotes_its_use() {
        let (ports, shared) = stub();
        ports.borrow_mut().usage.uses.insert(
            AccountId::seed(2),
            AccountUse {
                transactions: 312,
                budgets: 3,
            },
        );
        let mut form =
            AccountFormPresenter::edit(shared.clone(), AccountId::seed(2), today()).expect("stub");
        let view = form.view_model();

        assert_eq!(view.frame.title, "Edit account");
        assert_eq!(view.frame.submit, "Save");
        assert_eq!(view.name.text, "ANZ Everyday");
        assert_eq!(view.account_number.text, "1234 5678");
        assert_eq!(view.unit_read_only.as_deref(), Some("aud"));
        assert_eq!(view.balance_read_only.as_deref(), Some("4,182.55"));
        let usage = view.usage.expect("an Edit quotes its use");
        assert!(
            usage
                .notice
                .starts_with("Opened Mar 2019 \u{b7} 312 transactions.")
        );
        assert_eq!((usage.transactions, usage.budgets), (312, 3));

        // Tab skips the fixed Unit and Opening balance; focus and typing are refused.
        send(
            &mut form,
            vec![
                AccountFormEvent::Focus(AccountField::Type),
                AccountFormEvent::NextField,
            ],
        );
        assert_eq!(form.view_model().focused, AccountField::AccountNumber);
        send(
            &mut form,
            vec![
                AccountFormEvent::Focus(AccountField::Unit),
                balance("9"),
                AccountFormEvent::Unit(UnitId::SEED_BTC),
            ],
        );
        let view = form.view_model();
        assert_eq!(view.focused, AccountField::AccountNumber);
        assert_eq!(view.opening_balance.text, "");
        assert_eq!(view.unit.chosen, Some(UnitId::SEED_AUD));

        send(&mut form, vec![name("Everyday"), AccountFormEvent::Confirm]);
        let saved = ports
            .borrow()
            .accounts
            .account(AccountId::seed(2))
            .expect("kept");
        assert_eq!(saved.name, "Everyday");
        assert_eq!(saved.balance.to_string(), "4182.55");
        assert!(AccountFormPresenter::edit(shared, AccountId::new(), today()).is_none());
    }

    #[test]
    fn edit_keeps_an_accounts_deactivated_unit_among_the_choices() {
        let (ports, shared) = stub();
        ports.borrow_mut().units.units[1].is_active = false;
        let view = AccountFormPresenter::edit(shared, AccountId::seed(6), today())
            .expect("stub")
            .view_model();

        assert_eq!(view.unit.chosen, Some(UnitId::SEED_VAS));
        assert!(view.unit.options.iter().any(|c| c.label == "vas"));
        assert_eq!(view.unit_read_only.as_deref(), Some("vas"));
    }

    #[test]
    fn edit_of_a_cash_account_shows_its_institution_and_a_change_of_type_keeps_it() {
        let (_, shared) = stub();
        let mut form =
            AccountFormPresenter::edit(shared, AccountId::seed(1), today()).expect("stub");
        assert_eq!(
            form.view_model().institution.chosen,
            Some(InstitutionId::SEED_PERSONAL)
        );

        send(&mut form, vec![AccountFormEvent::Type(AccountType::Bank)]);
        assert_eq!(
            form.view_model().institution.chosen,
            Some(InstitutionId::SEED_PERSONAL)
        );
    }

    #[test]
    fn edit_of_an_account_whose_institution_is_gone_opens_on_no_choice_until_one_is_picked() {
        let (ports, shared) = stub();
        ports
            .borrow_mut()
            .institutions
            .institutions
            .retain(|row| row.id != InstitutionId::SEED_PERSONAL);
        let view = AccountFormPresenter::edit(shared, AccountId::seed(1), today())
            .expect("stub")
            .view_model();

        assert_eq!(view.institution.chosen, None);
        assert!(!view.frame.can_confirm, "an Institution is required");
        let (ports, shared) = stub();
        ports
            .borrow_mut()
            .institutions
            .institutions
            .retain(|row| row.id != InstitutionId::SEED_AMEX);
        let mut form =
            AccountFormPresenter::edit(shared, AccountId::seed(4), today()).expect("stub");
        let view = form.view_model();
        assert_eq!(view.institution.chosen, None, "not the Type's default");
        assert!(!view.frame.can_confirm);
        send(&mut form, vec![AccountFormEvent::Type(AccountType::Cash)]);
        assert_eq!(form.view_model().institution.chosen, None);
        send(
            &mut form,
            vec![AccountFormEvent::Institution(InstitutionId::SEED_ANZ)],
        );
        assert!(form.view_model().frame.can_confirm);
    }

    #[test]
    fn an_account_gone_under_the_edit_is_refused_onto_the_form() {
        let (ports, shared) = stub();
        let mut form =
            AccountFormPresenter::edit(shared, AccountId::seed(3), today()).expect("stub");
        ports
            .borrow_mut()
            .accounts
            .delete_account(AccountId::seed(3));

        assert_eq!(send(&mut form, vec![AccountFormEvent::Confirm]), []);
        assert_eq!(
            form.view_model().error.as_deref(),
            Some("This account has since been deleted.")
        );
        send(&mut form, vec![name("Typing clears it")]);
        assert_eq!(form.view_model().error, None);
    }

    #[test]
    fn cancel_closes_with_nothing_and_writes_nothing() {
        let (ports, shared) = stub();
        let before = ports.borrow().clone();
        let mut form = add(shared);

        assert_eq!(
            send(&mut form, vec![name("Draft"), AccountFormEvent::Cancel]),
            [RouterCall::CloseDialog(None)]
        );
        assert_eq!(*ports.borrow(), before);
    }
}
