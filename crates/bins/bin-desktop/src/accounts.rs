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
    Add(AccountForm),
    /// Editing the account with this [`Account::id`].
    Edit(u32, AccountForm),
    /// Deleting the account with this [`Account::id`], once its name has been typed back.
    Delete(u32, DeleteAccountForm),
}

impl AccountsDialog {
    /// The form behind the Add and Edit dialogs; Delete has its own, single-field form.
    pub fn form(&self) -> Option<&AccountForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Delete(..) => None,
        }
    }

    pub fn form_mut(&mut self) -> Option<&mut AccountForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Delete(..) => None,
        }
    }
}

/// The Delete account dialog's live form state -- pure, `gpui`-free. Just the typed-back
/// confirmation: there is nothing to `Tab` between, so the input is implicitly always focused
/// (the same shape as the Settings `DeleteUnitForm`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeleteAccountForm {
    pub confirm_input: String,
}

impl DeleteAccountForm {
    pub fn push_char(&mut self, ch: char) {
        if !ch.is_control() {
            self.confirm_input.push(ch);
        }
    }

    pub fn backspace(&mut self) {
        self.confirm_input.pop();
    }

    /// The README's "disabled until the typed value matches the account name exactly": a plain
    /// case-sensitive `==`, no trimming or case folding, so it can't be confirmed by habit.
    pub fn matches(&self, name: &str) -> bool {
        self.confirm_input == name
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
}

impl AccountOptions {
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
    pub fn for_field(&self, field: AccountField) -> &[String] {
        match field {
            AccountField::Institution => &self.institutions,
            AccountField::Type => &self.types,
            AccountField::Unit => &self.units,
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

/// The Add and Edit account dialogs' live form state (`docs/ux/desktop/Accounts/README.md`'s 3b) -- pure,
/// `gpui`-free. Name, Opening balance and Account number are typed into (append/pop only, like
/// the Settings dialogs); Institution, Type and Unit are [`SelectState`]s.
///
/// **Cash has no Institution to pick.** The glossary links a Cash account to a system-seeded
/// placeholder ([`NO_INSTITUTION`]) because the real-world thing has none, so while Type is Cash
/// the Institution select is read-only, `Tab` skips it, and the created account links to the
/// placeholder. Switching away from Cash brings the earlier pick back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountForm {
    pub name: String,
    pub institution: SelectState,
    pub account_type: SelectState,
    pub unit: SelectState,
    pub opening_balance: String,
    pub account_number: String,
    pub focused: AccountField,
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
            name: String::new(),
            institution: SelectState::new(options.institutions.first().cloned()),
            account_type: SelectState::new(Some(type_label(&AccountType::Bank).to_string())),
            unit: SelectState::new(unit),
            opening_balance: String::new(),
            account_number: String::new(),
            focused: AccountField::Name,
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
            Some(account.institution.clone())
        };
        Self {
            name: account.name.clone(),
            institution: SelectState::new(institution),
            account_type: SelectState::new(Some(type_label(&account.account_type).to_string())),
            unit: SelectState::new(Some(account.unit.clone())),
            opening_balance: String::new(),
            account_number: account.account_number.clone().unwrap_or_default(),
            focused: AccountField::Name,
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

    fn select_mut(&mut self, field: AccountField) -> Option<&mut SelectState> {
        match field {
            AccountField::Institution => Some(&mut self.institution),
            AccountField::Type => Some(&mut self.account_type),
            AccountField::Unit => Some(&mut self.unit),
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
    pub fn cycle_focus(&mut self, backward: bool, options: &AccountOptions) {
        let focused = self.focused;
        if let Some(state) = self.select_mut(focused) {
            state.commit(options.for_field(focused));
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
    pub fn handle_select_key(&mut self, key: SelectKey, options: &AccountOptions) -> bool {
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
    pub fn click_select(&mut self, field: AccountField, options: &AccountOptions) {
        if !field.is_select()
            || (field == AccountField::Institution && self.is_cash())
            || self.is_fixed(field)
        {
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
    pub fn choose_option(&mut self, field: AccountField, index: usize, options: &AccountOptions) {
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
            AccountField::Name => self.name.push(ch),
            AccountField::AccountNumber => self.account_number.push(ch),
            AccountField::OpeningBalance => {
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
            AccountField::Name => {
                self.name.pop();
            }
            AccountField::AccountNumber => {
                self.account_number.pop();
            }
            AccountField::OpeningBalance => {
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

    /// The Institution the account links to: the placeholder for Cash, else the picked one.
    fn resolved_institution(&self, account_type: &AccountType) -> Option<String> {
        if *account_type == AccountType::Cash {
            Some(NO_INSTITUTION.to_string())
        } else {
            self.institution.value().map(str::to_string)
        }
    }

    /// The Edit dialog's Save: writes Name, Institution, Type and Account number onto `account`,
    /// leaving Unit, balance, opened date and the stub counts untouched. `false` (and no change)
    /// while the form is invalid.
    pub fn apply_to(&self, account: &mut Account) -> bool {
        let Some(account_type) = self.selected_type().filter(|_| self.is_valid()) else {
            return false;
        };
        let Some(institution) = self.resolved_institution(&account_type) else {
            return false;
        };
        let account_number = self.account_number.trim();
        account.name = self.name.trim().to_string();
        account.institution = institution;
        account.account_type = account_type;
        account.account_number = (!account_number.is_empty()).then(|| account_number.to_string());
        true
    }

    /// Builds the account this form describes. `is_currency` pads a currency amount to two
    /// places (`1000` becomes `1000.00`); other Units keep what was typed. `None` while invalid.
    pub fn into_account(self, id: u32, opened_at: NaiveDate, is_currency: bool) -> Option<Account> {
        if !self.is_valid() {
            return None;
        }
        let account_type = self.selected_type()?;
        let institution = self.resolved_institution(&account_type)?;
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

/// The outcome of resolving a typed account name (`accounts delete Home Loan`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameLookup {
    /// The account's index in the slice searched.
    Found(usize),
    NotFound,
    /// More than one account fits; their names, in list order.
    Ambiguous(Vec<String>),
}

/// Finds the account `text` names, case-insensitively, trying an exact name first, then a name
/// that starts with `text`, then one that merely contains it. The first tier with any hits
/// decides: one hit is the account, several are [`NameLookup::Ambiguous`] rather than a guess.
/// Account names are not unique (the schema has no such rule), so even an exact name can be
/// ambiguous.
pub fn find_by_name(accounts: &[Account], text: &str) -> NameLookup {
    let needle = text.trim().to_lowercase();
    if needle.is_empty() {
        return NameLookup::NotFound;
    }
    let tiers: [fn(&str, &str) -> bool; 3] = [
        |name, needle| name == needle,
        |name, needle| name.starts_with(needle),
        |name, needle| name.contains(needle),
    ];
    for tier in tiers {
        let hits: Vec<usize> = accounts
            .iter()
            .enumerate()
            .filter(|(_, account)| tier(&account.name.to_lowercase(), &needle))
            .map(|(index, _)| index)
            .collect();
        match hits.as_slice() {
            [] => continue,
            [index] => return NameLookup::Found(*index),
            many => {
                return NameLookup::Ambiguous(
                    many.iter().map(|&i| accounts[i].name.clone()).collect(),
                );
            }
        }
    }
    NameLookup::NotFound
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

    fn add_options() -> AccountOptions {
        AccountOptions::new(
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

    fn valid_form(options: &AccountOptions) -> AccountForm {
        let mut form = AccountForm::new(options, Some("aud"));
        form.name = "Savings Maximiser".to_string();
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
            form.cycle_focus(false, &options);
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
        form.cycle_focus(true, &options);
        assert_eq!(form.focused, AccountField::AccountNumber);
    }

    #[test]
    fn tab_commits_an_open_lists_highlight_before_moving_on() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::Type);
        form.handle_select_key(SelectKey::Activate, &options);
        form.handle_select_key(SelectKey::Down, &options);
        assert!(form.account_type.is_open());
        form.cycle_focus(false, &options);
        assert!(!form.account_type.is_open());
        assert_eq!(form.selected_type(), Some(AccountType::CreditCard));
        assert_eq!(form.focused, AccountField::Unit);
    }

    #[test]
    fn cash_makes_institution_read_only_and_tab_skips_it() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::Type);
        form.handle_select_key(SelectKey::Up, &options);
        assert!(form.is_cash());

        form.focus(AccountField::Institution);
        assert_eq!(form.focused, AccountField::Type, "focus is refused");
        form.focus(AccountField::Name);
        form.cycle_focus(false, &options);
        assert_eq!(form.focused, AccountField::Type, "tab skips Institution");
        form.click_select(AccountField::Institution, &options);
        assert!(!form.institution.is_open());
    }

    #[test]
    fn a_cash_account_links_to_the_placeholder_institution() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::Type);
        form.handle_select_key(SelectKey::Up, &options);
        let account = form.into_account(9, date(2026, 9), true).expect("valid");
        assert_eq!(account.account_type, AccountType::Cash);
        assert_eq!(account.institution, NO_INSTITUTION);
    }

    #[test]
    fn leaving_cash_brings_the_earlier_institution_pick_back() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::Institution);
        form.handle_select_key(SelectKey::Down, &options);
        assert_eq!(form.institution.value(), Some("American Express"));
        form.focus(AccountField::Type);
        form.handle_select_key(SelectKey::Up, &options);
        form.handle_select_key(SelectKey::Down, &options);
        assert!(!form.is_cash());
        assert_eq!(form.institution.value(), Some("American Express"));
    }

    #[test]
    fn a_focused_select_steps_opens_navigates_and_commits() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::Unit);
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
        form.focus(AccountField::Type);
        form.handle_select_key(SelectKey::Activate, &options);
        assert!(form.close_open_select());
        assert!(!form.any_select_open());
        assert!(!form.close_open_select());
    }

    #[test]
    fn moving_focus_closes_a_list_open_on_another_field() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.click_select(AccountField::Unit, &options);
        assert!(form.unit.is_open());
        form.focus(AccountField::Name);
        assert!(!form.unit.is_open());
    }

    #[test]
    fn clicking_a_select_toggles_it_and_a_row_click_chooses() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.click_select(AccountField::Type, &options);
        assert!(form.account_type.is_open());
        assert_eq!(form.focused, AccountField::Type);
        form.choose_option(AccountField::Type, 3, &options);
        assert!(!form.account_type.is_open());
        assert_eq!(form.selected_type(), Some(AccountType::Loan));
        form.click_select(AccountField::Type, &options);
        form.click_select(AccountField::Type, &options);
        assert!(!form.account_type.is_open());
    }

    #[test]
    fn opening_balance_only_takes_a_decimal_amount() {
        let options = add_options();
        let mut form = valid_form(&options);
        form.focus(AccountField::OpeningBalance);
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
        let mut form = AccountForm::new(&options, None);
        form.push_char('A');
        form.focus(AccountField::AccountNumber);
        form.push_char('1');
        form.focus(AccountField::Unit);
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
        let form = AccountForm::new(&options, None);
        assert_eq!(form.into_account(9, date(2026, 9), true), None);
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
        assert_eq!(form.name, "ANZ Everyday");
        assert_eq!(form.institution.value(), Some("ANZ Banking Group"));
        assert_eq!(form.selected_type(), Some(AccountType::Bank));
        assert_eq!(form.unit.value(), Some("aud"));
        assert_eq!(form.account_number, "1234 5678");
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
        assert_eq!(form.account_number, "");
    }

    #[test]
    fn tab_in_edit_mode_skips_the_fixed_unit_and_opening_balance() {
        let options = add_options();
        let mut form = AccountForm::from_account(&seeded("ANZ Everyday"), &options);
        let mut seen = vec![form.focused];
        for _ in 0..4 {
            form.cycle_focus(false, &options);
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
        form.click_select(AccountField::Unit, &options);
        assert!(!form.unit.is_open());
        assert_eq!(form.unit.value(), Some("aud"));
    }

    #[test]
    fn apply_to_changes_name_institution_type_and_number_only() {
        let options = add_options();
        let mut account = seeded("ANZ Everyday");
        let before = account.clone();
        let mut form = AccountForm::from_account(&account, &options);
        form.name = "  Daily  ".to_string();
        form.institution = SelectState::new(Some("Westpac Banking".to_string()));
        form.account_type = SelectState::new(Some("Credit card".to_string()));
        form.account_number = "  ".to_string();

        assert!(form.apply_to(&mut account));
        assert_eq!(account.name, "Daily");
        assert_eq!(account.institution, "Westpac Banking");
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
        assert!(form.apply_to(&mut accounts[index]));

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
        assert!(form.apply_to(&mut account));
        assert_eq!(account.institution, NO_INSTITUTION);

        let mut form = AccountForm::from_account(&account, &options);
        form.account_type = SelectState::new(Some("Bank".to_string()));
        assert!(form.apply_to(&mut account));
        assert_eq!(account.institution, "ANZ Banking Group");
    }

    #[test]
    fn apply_to_leaves_the_account_alone_while_invalid() {
        let options = add_options();
        let mut account = seeded("ANZ Everyday");
        let before = account.clone();
        let mut form = AccountForm::from_account(&account, &options);
        form.name = "   ".to_string();
        assert!(!form.apply_to(&mut account));
        assert_eq!(account, before);
    }

    #[test]
    fn the_dialog_exposes_its_form_for_add_and_edit_but_not_delete() {
        let options = add_options();
        let form = AccountForm::new(&options, None);
        assert!(AccountsDialog::Add(form.clone()).form().is_some());
        assert!(AccountsDialog::Edit(1, form).form().is_some());
        assert!(
            AccountsDialog::Delete(1, DeleteAccountForm::default())
                .form()
                .is_none()
        );
    }

    #[test]
    fn delete_confirmation_is_an_exact_case_sensitive_match() {
        let mut form = DeleteAccountForm::default();
        assert!(!form.matches("Amex Platinum"));
        for ch in "Amex Platinum".chars() {
            form.push_char(ch);
        }
        assert!(form.matches("Amex Platinum"));
        assert!(!form.matches("amex platinum"));
        form.push_char(' ');
        assert!(!form.matches("Amex Platinum"), "no trimming");
        form.backspace();
        assert!(form.matches("Amex Platinum"));
    }

    #[test]
    fn delete_confirmation_ignores_control_characters_and_backspace_on_empty() {
        let mut form = DeleteAccountForm::default();
        form.push_char('\n');
        form.push_char('\t');
        form.backspace();
        assert_eq!(form.confirm_input, "");
    }

    #[test]
    fn find_by_name_prefers_an_exact_name_case_insensitively() {
        let accounts = default_accounts();
        assert_eq!(find_by_name(&accounts, "home loan"), NameLookup::Found(4));
        assert_eq!(
            find_by_name(&accounts, "  HOME LOAN "),
            NameLookup::Found(4)
        );
    }

    #[test]
    fn find_by_name_falls_back_to_a_unique_prefix_then_a_unique_substring() {
        let accounts = default_accounts();
        assert_eq!(find_by_name(&accounts, "amex"), NameLookup::Found(3));
        assert_eq!(find_by_name(&accounts, "platinum"), NameLookup::Found(3));
        assert_eq!(find_by_name(&accounts, "bitc"), NameLookup::Found(6));
    }

    #[test]
    fn find_by_name_reports_ambiguity_instead_of_guessing() {
        let accounts = default_accounts();
        assert_eq!(
            find_by_name(&accounts, "anz"),
            NameLookup::Ambiguous(vec!["ANZ Everyday".to_string(), "ANZ Offset".to_string()])
        );
    }

    #[test]
    fn an_exact_name_wins_over_longer_names_that_contain_it() {
        let mut accounts = default_accounts();
        accounts.push(Account {
            name: "ANZ".to_string(),
            ..account(20, "ANZ", AccountType::Bank)
        });
        assert_eq!(find_by_name(&accounts, "anz"), NameLookup::Found(7));
    }

    #[test]
    fn duplicate_exact_names_are_ambiguous() {
        let accounts = vec![
            account(1, "Savings", AccountType::Bank),
            account(2, "Savings", AccountType::Bank),
        ];
        assert_eq!(
            find_by_name(&accounts, "savings"),
            NameLookup::Ambiguous(vec!["Savings".to_string(), "Savings".to_string()])
        );
    }

    #[test]
    fn find_by_name_finds_nothing_for_an_unknown_or_empty_name() {
        let accounts = default_accounts();
        assert_eq!(find_by_name(&accounts, "zzz"), NameLookup::NotFound);
        assert_eq!(find_by_name(&accounts, "   "), NameLookup::NotFound);
        assert_eq!(find_by_name(&[], "wallet"), NameLookup::NotFound);
    }
}
