//! Pure Accounts-surface domain types (`docs/ux/desktop/Accounts/README.md`, section 3) --
//! `gpui`-free, the same "pure state, chrome renders it" split `settings.rs` uses. `view::accounts`
//! and its dialogs are the chrome; this module only knows what an account row holds, the fixed
//! order the page groups them in, and the stub rows the page is seeded with.
//!
//! All data is stubbed and in-memory (the Desktop Accounts Surface map's Destination): nothing
//! here reads `lib_database`.

use chrono::NaiveDate;
use lib_core::{AccountType, Money};

use crate::select::SelectState;

/// The Institution a Cash account links to. The glossary gives every Account a mandatory
/// Institution, and Cash has no real one, so it points at a system-seeded placeholder. It is
/// deliberately absent from `settings::default_institutions()`: it is not a user-managed row.
pub const NO_INSTITUTION: &str = "No institution";

/// The order the Accounts page groups accounts in -- fixed, never alphabetised or sorted by
/// balance (the README's implementation note 3). Not `AccountType::all()`, whose order puts
/// Investment before Loan; the desktop design lists Loan first.
pub const GROUP_ORDER: [AccountType; 5] = [
    AccountType::Cash,
    AccountType::Bank,
    AccountType::CreditCard,
    AccountType::Loan,
    AccountType::Investment,
];

/// The group heading and Type control label for `account_type`.
pub fn type_label(account_type: &AccountType) -> &'static str {
    match account_type {
        AccountType::Cash => "Cash",
        AccountType::Bank => "Bank",
        AccountType::CreditCard => "Credit card",
        AccountType::Loan => "Loan",
        AccountType::Investment => "Investment",
    }
}

/// One row of the Accounts page. `institution` and `unit` are the Institution's name and the
/// Unit's code -- the same value keys the select control stores (issue: select control for the
/// Add/Edit dialogs), so a row survives the Settings lists changing underneath it.
#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: u32,
    pub name: String,
    pub institution: String,
    pub account_type: AccountType,
    /// Unit code (e.g. `"aud"`). Fixed at creation.
    pub unit: String,
    /// In `unit`, at whatever precision the Unit carries (`0.4120` for btc). Its opening value is
    /// fixed at creation; nothing in this map moves it afterwards.
    pub balance: Money,
    /// UI-only: the design has the field, the domain model does not, so it is never persisted.
    pub account_number: Option<String>,
    pub opened_at: NaiveDate,
    /// Stub count shown by the Edit and Delete dialogs. Loan and Investment accounts take
    /// Repayments and Trades rather than Transactions, but this map shows one uniform figure.
    pub transaction_count: u32,
    /// Stub count of Budgets referencing the account, shown by the Delete dialog.
    pub budget_count: u32,
}

/// One type's block on the page: the indices (into the `Vec<Account>` it was built from) of its
/// accounts, in the order they were added.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountGroup {
    pub account_type: AccountType,
    pub indices: Vec<usize>,
}

/// Groups `accounts` by type in [`GROUP_ORDER`], omitting types with no accounts.
pub fn group_accounts(accounts: &[Account]) -> Vec<AccountGroup> {
    GROUP_ORDER
        .iter()
        .filter_map(|account_type| {
            let indices: Vec<usize> = accounts
                .iter()
                .enumerate()
                .filter(|(_, account)| &account.account_type == account_type)
                .map(|(index, _)| index)
                .collect();
            (!indices.is_empty()).then(|| AccountGroup {
                account_type: account_type.clone(),
                indices,
            })
        })
        .collect()
}

/// Indices into `accounts` in the order the page shows them top to bottom -- what `j`/`k` step
/// through.
pub fn display_order(accounts: &[Account]) -> Vec<usize> {
    group_accounts(accounts)
        .into_iter()
        .flat_map(|group| group.indices)
        .collect()
}

/// The next unused account id.
pub fn next_account_id(accounts: &[Account]) -> u32 {
    accounts
        .iter()
        .map(|account| account.id)
        .max()
        .map_or(1, |max| max + 1)
}

/// Every Accounts dialog `Shell` can have open, `None` when none is -- the README's own `State`
/// block (`dialog: Option<Dialog>`). The `u32` is the account's [`Account::id`]. Each dialog
/// ticket attaches its own form state to its variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountsDialog {
    Add(AddAccountForm),
    Edit(u32),
    Delete(u32),
}

/// The Add account dialog's fields, in `Tab` order (the mockup's reading order: Name,
/// Institution / Type, Unit / Opening balance, Account number).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddAccountField {
    #[default]
    Name,
    Institution,
    Type,
    Unit,
    OpeningBalance,
    AccountNumber,
}

impl AddAccountField {
    const ORDER: [AddAccountField; 6] = [
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
pub struct AddAccountOptions {
    pub institutions: Vec<String>,
    pub types: Vec<String>,
    pub units: Vec<String>,
}

impl AddAccountOptions {
    pub fn new(institutions: Vec<String>, units: Vec<String>) -> Self {
        Self {
            institutions,
            types: GROUP_ORDER
                .iter()
                .map(|account_type| type_label(account_type).to_string())
                .collect(),
            units,
        }
    }

    /// The options behind `field`; empty for a text field.
    pub fn for_field(&self, field: AddAccountField) -> &[String] {
        match field {
            AddAccountField::Institution => &self.institutions,
            AddAccountField::Type => &self.types,
            AddAccountField::Unit => &self.units,
            _ => &[],
        }
    }
}

/// A key a focused select understands, parsed from a keystroke by `Shell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectKey {
    Up,
    Down,
    /// `Enter` or `Space`: open a closed list, commit an open one.
    Activate,
}

/// The type a [`type_label`] names.
pub fn account_type_from_label(label: &str) -> Option<AccountType> {
    GROUP_ORDER
        .iter()
        .find(|account_type| type_label(account_type) == label)
        .cloned()
}

/// The Add account dialog's live form state (`docs/ux/desktop/Accounts/README.md`'s 3b) -- pure,
/// `gpui`-free. Name, Opening balance and Account number are typed into (append/pop only, like
/// the Settings dialogs); Institution, Type and Unit are [`SelectState`]s.
///
/// **Cash has no Institution to pick.** The glossary links a Cash account to a system-seeded
/// placeholder ([`NO_INSTITUTION`]) because the real-world thing has none, so while Type is Cash
/// the Institution select is read-only, `Tab` skips it, and the created account links to the
/// placeholder. Switching away from Cash brings the earlier pick back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddAccountForm {
    pub name: String,
    pub institution: SelectState,
    pub account_type: SelectState,
    pub unit: SelectState,
    pub opening_balance: String,
    pub account_number: String,
    pub focused: AddAccountField,
}

impl AddAccountForm {
    /// A fresh form: Type starts on Bank (the common case, and one that needs a real
    /// Institution), Institution on the first available, Unit on `default_unit` when it is one of
    /// the options and the first option otherwise.
    pub fn new(options: &AddAccountOptions, default_unit: Option<&str>) -> Self {
        let unit = default_unit
            .filter(|code| options.units.iter().any(|unit| unit == code))
            .map(str::to_string)
            .or_else(|| options.units.first().cloned());
        Self {
            name: String::new(),
            institution: SelectState::new(options.institutions.first().cloned()),
            account_type: SelectState::new(Some(type_label(&AccountType::Bank).to_string())),
            unit: SelectState::new(unit),
            opening_balance: String::new(),
            account_number: String::new(),
            focused: AddAccountField::Name,
        }
    }

    pub fn selected_type(&self) -> Option<AccountType> {
        account_type_from_label(self.account_type.value()?)
    }

    /// Whether the chosen Type is Cash -- Institution is then the placeholder, not a pick.
    pub fn is_cash(&self) -> bool {
        self.selected_type() == Some(AccountType::Cash)
    }

    fn select_mut(&mut self, field: AddAccountField) -> Option<&mut SelectState> {
        match field {
            AddAccountField::Institution => Some(&mut self.institution),
            AddAccountField::Type => Some(&mut self.account_type),
            AddAccountField::Unit => Some(&mut self.unit),
            _ => None,
        }
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
    pub fn focus(&mut self, field: AddAccountField) {
        if field == AddAccountField::Institution && self.is_cash() {
            return;
        }
        if field != self.focused {
            self.close_open_select();
        }
        self.focused = field;
    }

    /// `Tab` / `Shift-Tab`: commits an open list's highlight, then moves to the next field,
    /// wrapping and skipping the read-only Cash Institution.
    pub fn cycle_focus(&mut self, backward: bool, options: &AddAccountOptions) {
        let focused = self.focused;
        if let Some(state) = self.select_mut(focused) {
            state.commit(options.for_field(focused));
        }
        let count = AddAccountField::ORDER.len();
        let mut index = AddAccountField::ORDER
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        for _ in 0..count {
            index = if backward {
                (index + count - 1) % count
            } else {
                (index + 1) % count
            };
            let candidate = AddAccountField::ORDER[index];
            if candidate == AddAccountField::Institution && self.is_cash() {
                continue;
            }
            self.focused = candidate;
            return;
        }
    }

    /// A key on the focused select. Returns whether the focused field is a select (and so took
    /// the key).
    pub fn handle_select_key(&mut self, key: SelectKey, options: &AddAccountOptions) -> bool {
        let focused = self.focused;
        let list = options.for_field(focused);
        let Some(state) = self.select_mut(focused) else {
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
    pub fn click_select(&mut self, field: AddAccountField, options: &AddAccountOptions) {
        if !field.is_select() || (field == AddAccountField::Institution && self.is_cash()) {
            return;
        }
        self.focus(field);
        let list = options.for_field(field);
        if let Some(state) = self.select_mut(field) {
            if state.is_open() {
                state.cancel();
            } else {
                state.open(list);
            }
        }
    }

    /// A click on row `index` of an open list.
    pub fn choose_option(
        &mut self,
        field: AddAccountField,
        index: usize,
        options: &AddAccountOptions,
    ) {
        let list = options.for_field(field);
        if let Some(state) = self.select_mut(field) {
            state.choose(list, index);
        }
    }

    /// Types `ch` into the focused text field. Opening balance only takes what can be part of a
    /// decimal amount: digits, one `.`, and a `-` at the start.
    pub fn push_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        match self.focused {
            AddAccountField::Name => self.name.push(ch),
            AddAccountField::AccountNumber => self.account_number.push(ch),
            AddAccountField::OpeningBalance => {
                let allowed = ch.is_ascii_digit()
                    || (ch == '.' && !self.opening_balance.contains('.'))
                    || (ch == '-' && self.opening_balance.is_empty());
                if allowed {
                    self.opening_balance.push(ch);
                }
            }
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.focused {
            AddAccountField::Name => {
                self.name.pop();
            }
            AddAccountField::AccountNumber => {
                self.account_number.pop();
            }
            AddAccountField::OpeningBalance => {
                self.opening_balance.pop();
            }
            _ => {}
        }
    }

    /// The typed opening balance: `0` when left empty (the placeholder is `0.00`), `None` when
    /// what was typed is not a decimal amount.
    pub fn opening_balance_money(&self) -> Option<Money> {
        match self.opening_balance.trim() {
            "" => Some(money("0")),
            text => text.parse().ok(),
        }
    }

    /// The README's dialog lifecycle: Name, Type, Unit and Opening balance are required (Type and
    /// Unit always hold a value once options exist; an empty balance means zero), Institution
    /// too unless the account is Cash, and Account number is optional.
    pub fn is_valid(&self) -> bool {
        !self.name.trim().is_empty()
            && self.selected_type().is_some()
            && self.unit.value().is_some()
            && (self.is_cash() || self.institution.value().is_some())
            && self.opening_balance_money().is_some()
    }

    /// Builds the account this form describes. `is_currency` pads a currency amount to two
    /// places (`1000` becomes `1000.00`); other Units keep what was typed. `None` while invalid.
    pub fn into_account(self, id: u32, opened_at: NaiveDate, is_currency: bool) -> Option<Account> {
        if !self.is_valid() {
            return None;
        }
        let account_type = self.selected_type()?;
        let institution = if account_type == AccountType::Cash {
            NO_INSTITUTION.to_string()
        } else {
            self.institution.value()?.to_string()
        };
        let mut balance = self.opening_balance_money()?;
        if is_currency && balance.0.fractional_digit_count() < 2 {
            balance = Money(balance.0.with_scale(2));
        }
        let account_number = self.account_number.trim();
        Some(Account {
            id,
            name: self.name.trim().to_string(),
            institution,
            account_type,
            unit: self.unit.value()?.to_string(),
            balance,
            account_number: (!account_number.is_empty()).then(|| account_number.to_string()),
            opened_at,
            transaction_count: 0,
            budget_count: 0,
        })
    }
}

/// Splits `money` into its sign and its display text: thousands-grouped integer part, fractional
/// digits kept exactly as the amount carries them (`0.4120` stays four places -- the Unit's own
/// precision, never a hard-coded two). Negative is a separate flag so the caller can apply the
/// negative-balance rule (an ink token plus the U+2212 minus, never colour alone); a zero amount
/// is never negative, even if its text is `-0.00`.
pub fn format_amount(money: &Money) -> (bool, String) {
    let text = money.0.to_string();
    let (signed, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.as_str()),
    };
    let negative = signed && digits.chars().any(|c| matches!(c, '1'..='9'));
    let (integer, fraction) = match digits.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction.to_string())),
        // `BigDecimal` prints a zero as a bare `0` whatever scale it carries, so an empty
        // `0.00` account would lose its places; put them back from the amount's own scale.
        None => match usize::try_from(money.0.fractional_digit_count()) {
            Ok(scale) if scale > 0 => (digits, Some("0".repeat(scale))),
            _ => (digits, None),
        },
    };

    let grouped = if integer.chars().all(|c| c.is_ascii_digit()) {
        let mut out = String::with_capacity(integer.len() + integer.len() / 3);
        for (index, digit) in integer.chars().enumerate() {
            if index > 0 && (integer.len() - index) % 3 == 0 {
                out.push(',');
            }
            out.push(digit);
        }
        out
    } else {
        integer.to_string()
    };

    let mut display = String::new();
    if negative {
        display.push('\u{2212}');
    }
    display.push_str(&grouped);
    if let Some(fraction) = fraction {
        display.push('.');
        display.push_str(&fraction);
    }
    (negative, display)
}

/// The page header's figures under the no-cross-Unit rule: one net figure in the base Unit
/// only, with every other Unit held named rather than summed or silently dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct NetWorth {
    pub account_count: usize,
    /// `None` when Settings has no base Unit, in which case there is no net figure at all.
    pub base_unit: Option<String>,
    /// Signed sum of the base-Unit accounts' balances (loans and cards negative).
    pub base_net: Money,
    /// Other Unit codes held, in first-seen order, each once.
    pub held_separately: Vec<String>,
}

/// Computes the header's [`NetWorth`] for `accounts` against the Settings base Unit.
pub fn net_worth(accounts: &[Account], base_unit: Option<&str>) -> NetWorth {
    let mut base_net = money("0");
    let mut held_separately: Vec<String> = Vec::new();
    for account in accounts {
        if base_unit == Some(account.unit.as_str()) {
            base_net = Money(base_net.0 + account.balance.0.clone());
        } else if !held_separately.contains(&account.unit) {
            held_separately.push(account.unit.clone());
        }
    }
    NetWorth {
        account_count: accounts.len(),
        base_unit: base_unit.map(str::to_string),
        base_net,
        held_separately,
    }
}

impl NetWorth {
    /// `"7 accounts"`, or `"1 account"`.
    pub fn count_text(&self) -> String {
        match self.account_count {
            1 => "1 account".to_string(),
            count => format!("{count} accounts"),
        }
    }

    /// `"vas, btc held separately"`, or `None` when every account is in the base Unit.
    pub fn held_separately_text(&self) -> Option<String> {
        (!self.held_separately.is_empty())
            .then(|| format!("{} held separately", self.held_separately.join(", ")))
    }
}

/// Moves a selection (a position in [`display_order`]) by `delta` rows, clamped to `0..len`. An
/// empty list always selects `0`.
pub fn step_selection(selected: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    selected.saturating_add_signed(delta).min(len - 1)
}

/// The zero-based group position of the account at `accounts[index]` -- which group block on the
/// page holds it, for scrolling that block into view.
pub fn group_position(accounts: &[Account], index: usize) -> Option<usize> {
    group_accounts(accounts)
        .iter()
        .position(|group| group.indices.contains(&index))
}

fn money(text: &str) -> Money {
    text.parse()
        .expect("seed amounts are valid decimals (see seed_rows_parse_and_link)")
}

fn date(year: i32, month: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, 1).expect("seed dates are valid calendar dates")
}

/// The mockup's rows (Bank, Credit card, Loan, Investment) plus one Cash row, so all five groups
/// exist. The mockup says "7 accounts" but draws six; Wallet is the seventh.
///
/// Two balances differ from the mockup so the header's net figure is *computed* rather than
/// hard-coded: ANZ Offset is 463,203.10 (not 61,204.10), and Wallet is 240.00. The mockup's own
/// aud rows sum to a large negative (the loan dwarfs the deposits) and its printed 83,995.87
/// matches no sum of them; with these two changes the aud rows sum to exactly 83,995.87.
pub fn default_accounts() -> Vec<Account> {
    vec![
        Account {
            id: 1,
            name: "Wallet".to_string(),
            institution: NO_INSTITUTION.to_string(),
            account_type: AccountType::Cash,
            unit: "aud".to_string(),
            balance: money("240.00"),
            account_number: None,
            opened_at: date(2021, 2),
            transaction_count: 96,
            budget_count: 0,
        },
        Account {
            id: 2,
            name: "ANZ Everyday".to_string(),
            institution: "ANZ Banking Group".to_string(),
            account_type: AccountType::Bank,
            unit: "aud".to_string(),
            balance: money("4182.55"),
            account_number: Some("1234 5678".to_string()),
            opened_at: date(2019, 3),
            transaction_count: 312,
            budget_count: 3,
        },
        Account {
            id: 3,
            name: "ANZ Offset".to_string(),
            institution: "ANZ Banking Group".to_string(),
            account_type: AccountType::Bank,
            unit: "aud".to_string(),
            balance: money("463203.10"),
            account_number: None,
            opened_at: date(2019, 3),
            transaction_count: 88,
            budget_count: 0,
        },
        Account {
            id: 4,
            name: "Amex Platinum".to_string(),
            institution: "American Express".to_string(),
            account_type: AccountType::CreditCard,
            unit: "aud".to_string(),
            balance: money("-2318.44"),
            account_number: None,
            opened_at: date(2020, 8),
            transaction_count: 204,
            budget_count: 2,
        },
        Account {
            id: 5,
            name: "Home Loan".to_string(),
            institution: "Westpac Banking".to_string(),
            account_type: AccountType::Loan,
            unit: "aud".to_string(),
            balance: money("-381311.34"),
            account_number: None,
            opened_at: date(2019, 6),
            transaction_count: 36,
            budget_count: 0,
        },
        Account {
            id: 6,
            name: "Vanguard VAS".to_string(),
            institution: "Vanguard Investments".to_string(),
            account_type: AccountType::Investment,
            unit: "vas".to_string(),
            balance: money("1240"),
            account_number: None,
            opened_at: date(2022, 1),
            transaction_count: 27,
            budget_count: 0,
        },
        Account {
            id: 7,
            name: "Bitcoin".to_string(),
            institution: "Cryptocurrency Exchange".to_string(),
            account_type: AccountType::Investment,
            unit: "btc".to_string(),
            balance: money("0.4120"),
            account_number: None,
            opened_at: date(2021, 11),
            transaction_count: 9,
            budget_count: 0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings;

    fn account(id: u32, name: &str, account_type: AccountType) -> Account {
        Account {
            id,
            name: name.to_string(),
            institution: NO_INSTITUTION.to_string(),
            account_type,
            unit: "aud".to_string(),
            balance: money("0"),
            account_number: None,
            opened_at: date(2024, 1),
            transaction_count: 0,
            budget_count: 0,
        }
    }

    #[test]
    fn group_order_puts_loan_before_investment() {
        let labels: Vec<_> = GROUP_ORDER.iter().map(type_label).collect();
        assert_eq!(
            labels,
            vec!["Cash", "Bank", "Credit card", "Loan", "Investment"]
        );
    }

    #[test]
    fn group_order_covers_every_account_type_once() {
        for account_type in AccountType::all() {
            assert_eq!(
                GROUP_ORDER.iter().filter(|t| *t == account_type).count(),
                1,
                "{account_type} must appear exactly once"
            );
        }
    }

    #[test]
    fn groups_follow_fixed_order_not_insertion_order() {
        let accounts = vec![
            account(1, "Fund", AccountType::Investment),
            account(2, "Mortgage", AccountType::Loan),
            account(3, "Card", AccountType::CreditCard),
            account(4, "Savings", AccountType::Bank),
        ];
        let types: Vec<_> = group_accounts(&accounts)
            .into_iter()
            .map(|group| group.account_type)
            .collect();
        assert_eq!(
            types,
            vec![
                AccountType::Bank,
                AccountType::CreditCard,
                AccountType::Loan,
                AccountType::Investment,
            ]
        );
    }

    #[test]
    fn empty_types_are_omitted() {
        let accounts = vec![account(1, "Savings", AccountType::Bank)];
        let groups = group_accounts(&accounts);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].account_type, AccountType::Bank);
        assert!(group_accounts(&[]).is_empty());
    }

    #[test]
    fn within_a_group_accounts_keep_insertion_order() {
        let accounts = vec![
            account(1, "Zebra", AccountType::Bank),
            account(2, "Apple", AccountType::Bank),
        ];
        assert_eq!(group_accounts(&accounts)[0].indices, vec![0, 1]);
    }

    #[test]
    fn display_order_visits_every_account_exactly_once() {
        let accounts = default_accounts();
        let mut order = display_order(&accounts);
        assert_eq!(order.len(), accounts.len());
        order.sort_unstable();
        order.dedup();
        assert_eq!(order.len(), accounts.len());
    }

    #[test]
    fn next_account_id_is_one_past_the_max_and_starts_at_one() {
        assert_eq!(next_account_id(&[]), 1);
        assert_eq!(next_account_id(&default_accounts()), 8);
    }

    #[test]
    fn seed_rows_parse_and_link() {
        let accounts = default_accounts();
        assert_eq!(accounts.len(), 7);

        let mut ids: Vec<_> = accounts.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), accounts.len(), "ids must be unique");

        let units = settings::default_units();
        let institutions = settings::default_institutions();
        for account in &accounts {
            assert!(
                units.iter().any(|unit| unit.code == account.unit),
                "{} names an unknown unit {}",
                account.name,
                account.unit
            );
            assert!(
                account.institution == NO_INSTITUTION
                    || institutions.iter().any(|i| i.name == account.institution),
                "{} names an unknown institution {}",
                account.name,
                account.institution
            );
        }
    }

    #[test]
    fn seed_covers_all_five_groups() {
        assert_eq!(group_accounts(&default_accounts()).len(), 5);
    }

    #[test]
    fn seeded_base_unit_rows_sum_to_the_mockups_net_worth() {
        let base = settings::default_units()
            .into_iter()
            .find(|unit| unit.is_base)
            .map(|unit| unit.code)
            .unwrap_or_default();
        let net = default_accounts()
            .into_iter()
            .filter(|account| account.unit == base)
            .fold(money("0"), |sum, account| Money(sum.0 + account.balance.0));
        assert_eq!(net, money("83995.87"));
    }

    #[test]
    fn format_amount_groups_thousands_and_keeps_the_amounts_own_precision() {
        assert_eq!(
            format_amount(&money("463203.10")),
            (false, "463,203.10".to_string())
        );
        assert_eq!(
            format_amount(&money("4182.55")),
            (false, "4,182.55".to_string())
        );
        assert_eq!(format_amount(&money("1240")), (false, "1,240".to_string()));
        assert_eq!(
            format_amount(&money("0.4120")),
            (false, "0.4120".to_string())
        );
        assert_eq!(format_amount(&money("999")), (false, "999".to_string()));
        assert_eq!(
            format_amount(&money("1000000")),
            (false, "1,000,000".to_string())
        );
    }

    #[test]
    fn format_amount_marks_negatives_with_a_real_minus_sign() {
        assert_eq!(
            format_amount(&money("-381311.34")),
            (true, "\u{2212}381,311.34".to_string())
        );
        assert_eq!(
            format_amount(&money("-2318.44")),
            (true, "\u{2212}2,318.44".to_string())
        );
    }

    #[test]
    fn format_amount_never_treats_zero_as_negative() {
        let (negative, text) = format_amount(&money("-0.00"));
        assert!(!negative);
        assert_eq!(text, "0.00");
    }

    #[test]
    fn net_worth_sums_base_unit_only_and_names_the_rest() {
        let net = net_worth(&default_accounts(), Some("aud"));
        assert_eq!(net.account_count, 7);
        assert_eq!(net.base_net, money("83995.87"));
        assert_eq!(
            net.held_separately,
            vec!["vas".to_string(), "btc".to_string()]
        );
        assert_eq!(net.count_text(), "7 accounts");
        assert_eq!(
            net.held_separately_text().as_deref(),
            Some("vas, btc held separately")
        );
    }

    #[test]
    fn net_worth_with_no_base_unit_accounts_is_zero_and_lists_every_unit() {
        let accounts = vec![Account {
            unit: "btc".to_string(),
            ..account(1, "Coins", AccountType::Investment)
        }];
        let net = net_worth(&accounts, Some("aud"));
        assert_eq!(net.base_net, money("0"));
        assert_eq!(net.held_separately, vec!["btc".to_string()]);
    }

    #[test]
    fn net_worth_with_no_base_unit_sums_nothing() {
        let net = net_worth(&default_accounts(), None);
        assert_eq!(net.base_net, money("0"));
        assert_eq!(net.base_unit, None);
        assert_eq!(net.held_separately.len(), 3);
    }

    #[test]
    fn net_worth_count_text_is_singular_for_one_account() {
        let net = net_worth(&[account(1, "Only", AccountType::Cash)], Some("aud"));
        assert_eq!(net.count_text(), "1 account");
        assert_eq!(net.held_separately_text(), None);
    }

    #[test]
    fn step_selection_clamps_at_both_ends_and_tolerates_an_empty_list() {
        assert_eq!(step_selection(0, 7, -1), 0);
        assert_eq!(step_selection(3, 7, 1), 4);
        assert_eq!(step_selection(6, 7, 1), 6);
        assert_eq!(step_selection(2, 7, -5), 0);
        assert_eq!(step_selection(2, 7, 100), 6);
        assert_eq!(step_selection(0, 0, 1), 0);
    }

    #[test]
    fn group_position_finds_the_block_holding_an_account() {
        let accounts = default_accounts();
        // Wallet (Cash) is the first block; Bitcoin (Investment) is the last of five.
        assert_eq!(group_position(&accounts, 0), Some(0));
        assert_eq!(group_position(&accounts, 6), Some(4));
        assert_eq!(group_position(&accounts, 99), None);
    }

    fn add_options() -> AddAccountOptions {
        AddAccountOptions::new(
            settings::default_institutions()
                .into_iter()
                .map(|institution| institution.name)
                .collect(),
            settings::default_units()
                .into_iter()
                .map(|unit| unit.code)
                .collect(),
        )
    }

    fn valid_form(options: &AddAccountOptions) -> AddAccountForm {
        let mut form = AddAccountForm::new(options, Some("aud"));
        form.name = "Savings Maximiser".to_string();
        form
    }

    #[test]
    fn a_fresh_form_starts_on_bank_with_the_first_institution_and_default_unit() {
        let options = add_options();
        let form = AddAccountForm::new(&options, Some("btc"));
        assert_eq!(form.focused, AddAccountField::Name);
        assert_eq!(form.selected_type(), Some(AccountType::Bank));
        assert_eq!(form.institution.value(), Some("ANZ Banking Group"));
        assert_eq!(form.unit.value(), Some("btc"));
        assert!(!form.is_valid(), "name is required");
    }

    #[test]
    fn an_unknown_default_unit_falls_back_to_the_first_option() {
        let options = add_options();
        assert_eq!(
            AddAccountForm::new(&options, Some("zzz")).unit.value(),
            Some("aud")
        );
        assert_eq!(
            AddAccountForm::new(&options, None).unit.value(),
            Some("aud")
        );
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
            form.cycle_focus(false, &options);
            seen.push(form.focused);
        }
        assert_eq!(
            seen,
            vec![
                AddAccountField::Name,
                AddAccountField::Institution,
                AddAccountField::Type,
                AddAccountField::Unit,
                AddAccountField::OpeningBalance,
                AddAccountField::AccountNumber,
                AddAccountField::Name,
            ]
        );
        form.cycle_focus(true, &options);
        assert_eq!(form.focused, AddAccountField::AccountNumber);
    }

    #[test]
    fn tab_commits_an_open_lists_highlight_before_moving_on() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AddAccountField::Type);
        form.handle_select_key(SelectKey::Activate, &options);
        form.handle_select_key(SelectKey::Down, &options);
        assert!(form.account_type.is_open());
        form.cycle_focus(false, &options);
        assert!(!form.account_type.is_open());
        assert_eq!(form.selected_type(), Some(AccountType::CreditCard));
        assert_eq!(form.focused, AddAccountField::Unit);
    }

    #[test]
    fn cash_makes_institution_read_only_and_tab_skips_it() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AddAccountField::Type);
        form.handle_select_key(SelectKey::Up, &options);
        assert!(form.is_cash());

        form.focus(AddAccountField::Institution);
        assert_eq!(form.focused, AddAccountField::Type, "focus is refused");
        form.focus(AddAccountField::Name);
        form.cycle_focus(false, &options);
        assert_eq!(form.focused, AddAccountField::Type, "tab skips Institution");
        form.click_select(AddAccountField::Institution, &options);
        assert!(!form.institution.is_open());
    }

    #[test]
    fn a_cash_account_links_to_the_placeholder_institution() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AddAccountField::Type);
        form.handle_select_key(SelectKey::Up, &options);
        let account = form.into_account(9, date(2026, 9), true).expect("valid");
        assert_eq!(account.account_type, AccountType::Cash);
        assert_eq!(account.institution, NO_INSTITUTION);
    }

    #[test]
    fn leaving_cash_brings_the_earlier_institution_pick_back() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AddAccountField::Institution);
        form.handle_select_key(SelectKey::Down, &options);
        assert_eq!(form.institution.value(), Some("American Express"));
        form.focus(AddAccountField::Type);
        form.handle_select_key(SelectKey::Up, &options);
        form.handle_select_key(SelectKey::Down, &options);
        assert!(!form.is_cash());
        assert_eq!(form.institution.value(), Some("American Express"));
    }

    #[test]
    fn a_focused_select_steps_opens_navigates_and_commits() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AddAccountField::Unit);
        assert!(form.handle_select_key(SelectKey::Down, &options));
        assert_eq!(form.unit.value(), Some("btc"));
        form.handle_select_key(SelectKey::Activate, &options);
        assert!(form.unit.is_open());
        form.handle_select_key(SelectKey::Down, &options);
        form.handle_select_key(SelectKey::Activate, &options);
        assert!(!form.unit.is_open());
        assert_eq!(form.unit.value(), Some("vas"));
    }

    #[test]
    fn a_select_key_on_a_text_field_is_not_taken() {
        let options = add_options();
        let mut form = valid_form(&options);
        assert!(!form.handle_select_key(SelectKey::Down, &options));
    }

    #[test]
    fn close_open_select_reports_whether_one_was_open() {
        let options = add_options();
        let mut form = valid_form(&options);
        assert!(!form.close_open_select());
        form.focus(AddAccountField::Type);
        form.handle_select_key(SelectKey::Activate, &options);
        assert!(form.close_open_select());
        assert!(!form.any_select_open());
        assert!(!form.close_open_select());
    }

    #[test]
    fn moving_focus_closes_a_list_open_on_another_field() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.click_select(AddAccountField::Unit, &options);
        assert!(form.unit.is_open());
        form.focus(AddAccountField::Name);
        assert!(!form.unit.is_open());
    }

    #[test]
    fn clicking_a_select_toggles_it_and_a_row_click_chooses() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.click_select(AddAccountField::Type, &options);
        assert!(form.account_type.is_open());
        assert_eq!(form.focused, AddAccountField::Type);
        form.choose_option(AddAccountField::Type, 3, &options);
        assert!(!form.account_type.is_open());
        assert_eq!(form.selected_type(), Some(AccountType::Loan));
        form.click_select(AddAccountField::Type, &options);
        form.click_select(AddAccountField::Type, &options);
        assert!(!form.account_type.is_open());
    }

    #[test]
    fn opening_balance_only_takes_a_decimal_amount() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AddAccountField::OpeningBalance);
        for ch in "-12a3.4.5e-".chars() {
            form.push_char(ch);
        }
        assert_eq!(form.opening_balance, "-123.45");
        form.backspace();
        assert_eq!(form.opening_balance, "-123.4");
    }

    #[test]
    fn typing_goes_to_the_focused_text_field_only() {
        let options = add_options();
        let mut form = AddAccountForm::new(&options, None);
        form.push_char('A');
        form.focus(AddAccountField::AccountNumber);
        form.push_char('1');
        form.focus(AddAccountField::Unit);
        form.push_char('x');
        assert_eq!(form.name, "A");
        assert_eq!(form.account_number, "1");
        assert_eq!(form.unit.value(), Some("aud"));
    }

    #[test]
    fn validity_needs_a_name_and_a_parseable_balance() {
        let options = add_options();
        let mut form = valid_form(&options);
        assert!(form.is_valid());
        form.name = "   ".to_string();
        assert!(!form.is_valid());
        form.name = "Ok".to_string();
        form.opening_balance = "-".to_string();
        assert!(!form.is_valid());
        form.opening_balance = String::new();
        assert!(form.is_valid(), "empty means zero");
    }

    #[test]
    fn into_account_builds_a_trimmed_zero_count_row() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.name = "  Rainy Day  ".to_string();
        form.opening_balance = "1500".to_string();
        form.account_number = "  ".to_string();
        let account = form.into_account(9, date(2026, 9), true).expect("valid");
        assert_eq!(account.id, 9);
        assert_eq!(account.name, "Rainy Day");
        assert_eq!(account.account_type, AccountType::Bank);
        assert_eq!(account.institution, "ANZ Banking Group");
        assert_eq!(account.unit, "aud");
        assert_eq!(account.account_number, None);
        assert_eq!((account.transaction_count, account.budget_count), (0, 0));
        assert_eq!(format_amount(&account.balance).1, "1,500.00");
    }

    #[test]
    fn a_non_currency_amount_keeps_what_was_typed() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.unit = SelectState::new(Some("vas".to_string()));
        form.opening_balance = "10".to_string();
        let account = form.into_account(9, date(2026, 9), false).expect("valid");
        assert_eq!(format_amount(&account.balance).1, "10");
    }

    #[test]
    fn a_negative_opening_balance_is_kept_and_a_longer_scale_is_not_rounded() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.opening_balance = "-250.005".to_string();
        let account = form.into_account(9, date(2026, 9), true).expect("valid");
        assert_eq!(
            format_amount(&account.balance),
            (true, "\u{2212}250.005".to_string())
        );
    }

    #[test]
    fn an_invalid_form_builds_nothing() {
        let options = add_options();
        let form = AddAccountForm::new(&options, None);
        assert_eq!(form.into_account(9, date(2026, 9), true), None);
    }
}
