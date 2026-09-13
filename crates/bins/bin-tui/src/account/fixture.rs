//! [`AccountFixture`]: the in-memory [`AccountStore`] every ticket in the "Accounts screen,
//! views and popup" map builds against, seeded with `docs/ux/tui/accounts/README.md`'s own
//! mock accounts and amounts (`§7a`'s CASH/BANK/CREDIT CARD/INVESTMENT/LOAN rows, `§7c`'s
//! Everyday Spending summary-box example, `§7d`'s delete-into-Mortgage-Offset example).
//!
//! **One deliberate divergence from the handoff's own drawing**: `§7a` draws `CASH` with a
//! printed subtotal (`320.40`) even though its two rows (`Wallet` AUD, `Travel cash` USD) are
//! different Units — which the handoff's own rule says should print `mixed units`, never a
//! summed number (the same rule its `INVESTMENT` row correctly follows). Rather than seed a
//! fixture that reproduces a rule violation, `Travel cash` is seeded **inactive** here, so the
//! default (active-only) view shows `CASH` as a clean single-Unit group and the rule is never
//! actually broken on screen; `za` still reveals the mixed pair, correctly printing `mixed
//! units` once `Travel cash` is visible. Matches `crate::category::fixture`'s own precedent of
//! not forcing a fixture to match a hand-drawn total that doesn't actually hold up — "it's
//! hand-drawn wireframe, not a spreadsheet".
//!
//! Every account's Balance lands exactly on the handoff's own displayed figure: Everyday
//! Spending `4 210.65`, Mortgage Offset `24 429.50` (so `§7d`'s `24 429.50 → 28 640.15` preview
//! is exact), Amex Platinum `-1 284.30`, Vanguard VDHG `412.4800`, Cold wallet `0.18400000`,
//! Home Loan `-612 400.00`. `Wallet` is seeded with zero Transactions (exercises `§7d`'s
//! skip-transfer/confirm-only path); `Everyday Spending` is seeded with `1 284` Transactions,
//! `12` open and `8` Balance Checks (exercises the refuse-on-non-empty/transfer-preview path,
//! and matches `§7d`'s own "holds 1 284 txns ... 8 balance checks" example exactly).

use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{Datelike, Months, NaiveDate};

use super::{
    Account, AccountBalanceCheck, AccountError, AccountStore, AccountTransaction, AccountUnit,
    TransactionStatus,
};
use lib_core::{AccountType, Money, RowID};

/// The fixture's fixed "now" — matches `crate::category::fixture`'s own `2026-09-08`, so
/// anything cross-referencing both screens (the shell's status line, "today") stays
/// consistent. `pub` so `view::accounts` can pin its own balance-chart window to the same
/// date rather than drifting onto the real `Utc::now()` and disagreeing with this fixture's
/// own seeded history.
pub const FIXTURE_NOW: NaiveDate = match NaiveDate::from_ymd_opt(2026, 9, 8) {
    Some(date) => date,
    None => panic!("fixed literal is a valid date"),
};

const FAKE_PAYEES: &[&str] = &[
    "Woolworths",
    "Coles",
    "BP",
    "Netflix",
    "Employer Pty Ltd",
    "Amazon",
    "Spotify",
    "Telco Co",
    "Electricity Co",
    "Medicare",
];

/// Builds an exact `Money` amount from a whole part and a fractional numerator over
/// `10^scale` (e.g. `money(412, 4800, 4)` -> `412.4800`, `money(-1284, 30, 2)` -> `-1284.30`) —
/// avoids both `f64` rounding error and `unwrap`/`expect` on parsed string literals for seed
/// data that's obviously always well-formed (mirrors `crate::category::fixture`'s own `money`
/// helper, generalised to a variable scale for Units other than 2dp currency).
fn money(whole: i64, frac: i64, scale: u32) -> Money {
    let sign = if whole < 0 { -1 } else { 1 };
    let magnitude = BigDecimal::from(whole.abs())
        + BigDecimal::from(frac.abs()) / BigDecimal::from(10i64.pow(scale));
    Money(magnitude * BigDecimal::from(sign))
}

fn unit(code: &str, decimal_places: i64) -> AccountUnit {
    AccountUnit {
        code: code.to_string(),
        decimal_places,
    }
}

fn money_to_f64(value: &Money) -> f64 {
    value.0.to_string().parse().unwrap_or(0.0)
}

fn round_money(amount: f64, decimal_places: i64) -> Money {
    Money(
        BigDecimal::from_f64(amount)
            .unwrap_or_default()
            .with_scale(decimal_places),
    )
}

fn earliest(a: Option<NaiveDate>, b: Option<NaiveDate>) -> Option<NaiveDate> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

fn latest(a: Option<NaiveDate>, b: Option<NaiveDate>) -> Option<NaiveDate> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

/// A tiny xorshift PRNG step — deterministic across runs/platforms, the same technique
/// `view::categories`' own fake data already uses (no `rand` dependency needed).
fn xorshift(seed: u64) -> u64 {
    let mut seed = seed;
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}

/// A deterministic seed derived from a `RowID`, so the same account always generates the same
/// fake ledger.
fn seed_from_id(id: RowID) -> u64 {
    let uuid = id.into_uuid();
    let bytes = uuid.as_bytes();
    u64::from_be_bytes(
        bytes[8..16]
            .try_into()
            .expect("a uuid's byte array is always at least 16 bytes long"),
    )
}

/// Generates `account`'s ledger rows, deterministically seeded from its id — see [`Account`]'s
/// own doc on why these aren't stored. The first and last generated rows are forced onto
/// `account.first_posted`/`last_posted` exactly; the rest land pseudo-randomly in between. A
/// residual is folded into the last row so the rows' `BigDecimal` sum is exactly
/// `account.transactions_sum`, regardless of each row's own `f64`-rounded amount.
pub(super) fn ledger_for(account: &Account) -> Vec<AccountTransaction> {
    let count = account.transaction_count as usize;
    let (Some(first), Some(last)) = (account.first_posted, account.last_posted) else {
        return Vec::new();
    };
    if count == 0 {
        return Vec::new();
    }

    let span_days = (last - first).num_days().max(0) as u64;
    let avg = money_to_f64(&account.transactions_sum) / count as f64;
    let mut seed = seed_from_id(account.id);
    let mut rows = Vec::with_capacity(count);
    let mut running = BigDecimal::from(0);

    for index in 0..count {
        let offset_days = if index == 0 {
            0
        } else if index == count - 1 {
            span_days
        } else {
            seed = xorshift(seed);
            seed % (span_days + 1)
        };
        let date = first + chrono::Duration::days(offset_days as i64);

        seed = xorshift(seed);
        let wobble = 0.4 + (seed % 120) as f64 / 100.0; // 0.4..1.6
        seed = xorshift(seed);
        let sign: f64 = if seed.is_multiple_of(6) { -1.0 } else { 1.0 };
        let amount = round_money(avg * wobble * sign, account.unit.decimal_places);
        running += amount.0.clone();

        seed = xorshift(seed);
        let payee = FAKE_PAYEES[(seed as usize) % FAKE_PAYEES.len()];

        rows.push(AccountTransaction {
            date,
            payee,
            amount,
            status: TransactionStatus::Reconciled,
        });
    }

    if let Some(last_row) = rows.last_mut() {
        let residual = account.transactions_sum.0.clone() - running;
        last_row.amount.0 += residual;
    }

    rows.sort_by_key(|row| row.date);
    let open_count = (account.open_count as usize).min(rows.len());
    let newest_start = rows.len() - open_count;
    for row in rows[newest_start..].iter_mut() {
        row.status = TransactionStatus::Open;
    }

    rows.sort_by_key(|row| std::cmp::Reverse(row.date));
    rows
}

fn balance_for(account: &Account) -> Money {
    Money(account.starting_balance.0.clone() + account.transactions_sum.0.clone())
}

fn balance_as_of_for(account: &Account, date: NaiveDate) -> Money {
    let mut total = account.starting_balance.0.clone();
    for row in ledger_for(account).iter().filter(|row| row.date <= date) {
        total += row.amount.0.clone();
    }
    Money(total)
}

/// `as_of`'s own month-end, `months_back` months earlier (`0` is `as_of`'s own month).
fn month_end(as_of: NaiveDate, months_back: usize) -> NaiveDate {
    let first_of_as_of_month = NaiveDate::from_ymd_opt(as_of.year(), as_of.month(), 1)
        .expect("as_of's own year/month with day 1 is always valid");
    let first_of_target_month = first_of_as_of_month
        .checked_sub_months(Months::new(months_back as u32))
        .expect("months_back stays well within chrono's representable range");
    let first_of_next_month = first_of_target_month
        .checked_add_months(Months::new(1))
        .expect("adding one month to a valid first-of-month date stays in range");
    first_of_next_month - chrono::Duration::days(1)
}

/// One pass over `account`'s generated ledger (sorted ascending once), sweeping a running
/// balance past each trailing month-end — not `months` separate `balance_as_of` calls, per the
/// handoff's "compute the series in one pass". Oldest month first.
fn monthly_balances_for(account: &Account, months: usize, as_of: NaiveDate) -> Vec<Money> {
    let mut rows = ledger_for(account);
    rows.sort_by_key(|row| row.date);

    let mut result = Vec::with_capacity(months);
    let mut running = account.starting_balance.0.clone();
    let mut rows = rows.into_iter().peekable();

    for months_back in (0..months).rev() {
        let boundary = month_end(as_of, months_back);
        while let Some(row) = rows.peek() {
            if row.date <= boundary {
                running += rows.next().expect("just peeked Some").amount.0;
            } else {
                break;
            }
        }
        result.push(Money(running.clone()));
    }
    result
}

/// An in-memory `AccountStore`, seeded once with a fixed demo account list. Every mutating
/// method mutates this same `Vec` in place — the "in-memory mutable fixture" this map's ticket
/// decided on (discarded on quit, since nothing here is persisted).
pub struct AccountFixture {
    accounts: Vec<Account>,
}

impl Default for AccountFixture {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountFixture {
    /// Seeds the demo account list. Ids are `RowID::from_timestamp` over a fixed,
    /// strictly-increasing sequence of dates (not `RowID::new()`/`RowID::mock()`) so a fresh
    /// fixture is deterministic across runs, matching the repo's "deterministic seeds for
    /// generated test data" convention.
    pub fn new() -> Self {
        use chrono::{DateTime, Utc};

        let mut next = DateTime::parse_from_rfc3339("2021-01-01T00:00:00Z")
            .expect("fixed literal is a valid RFC3339 timestamp")
            .with_timezone(&Utc);
        // `RowID::from_timestamp` delegates to `uuid::Uuid::new_v7`, which fills a real UUIDv7's
        // non-timestamp bits from the OS RNG — two calls with the *same* timestamp still produce
        // different ids, and every id (and therefore every `seed_from_id`-derived ledger) this
        // fixture seeds comes out different on every run, contradicting this very module doc's
        // "deterministic across runs" claim. Build the UUID ourselves instead, via the same
        // timestamp sequence plus a plain incrementing counter standing in for the random bits
        // — deterministic, and still sorts by creation order like a real v7 id would.
        let mut counter: u64 = 0;
        let mut id = move || {
            let millis = next.timestamp_millis() as u64;
            next += chrono::Duration::seconds(1);
            counter += 1;
            let mut counter_bytes = [0u8; 10];
            counter_bytes[2..10].copy_from_slice(&counter.to_be_bytes());
            let uuid =
                uuid::Builder::from_unix_timestamp_millis(millis, &counter_bytes).into_uuid();
            RowID::from_uuid(uuid)
        };

        let aud = || unit("AUD", 2);
        let usd = || unit("USD", 2);

        let wallet = Account {
            id: id(),
            name: "Wallet".to_string(),
            account_type: AccountType::Cash,
            unit: aud(),
            starting_balance: money(320, 40, 2),
            is_active: true,
            created_on: NaiveDate::from_ymd_opt(2024, 1, 10).expect("valid date"),
            updated_on: NaiveDate::from_ymd_opt(2024, 1, 10).expect("valid date"),
            transactions_sum: money(0, 0, 2),
            transaction_count: 0,
            open_count: 0,
            first_posted: None,
            last_posted: None,
            balance_checks: Vec::new(),
        };

        // Deliberately inactive — see the module doc on why `CASH` shouldn't show this pair
        // by default.
        let travel_cash = Account {
            id: id(),
            name: "Travel cash".to_string(),
            account_type: AccountType::Cash,
            unit: usd(),
            starting_balance: money(1_500, 0, 2),
            is_active: false,
            created_on: NaiveDate::from_ymd_opt(2024, 3, 2).expect("valid date"),
            updated_on: NaiveDate::from_ymd_opt(2024, 3, 2).expect("valid date"),
            transactions_sum: money(0, 0, 2),
            transaction_count: 0,
            open_count: 0,
            first_posted: None,
            last_posted: None,
            balance_checks: Vec::new(),
        };

        let mut everyday_spending = Account {
            id: id(),
            name: "Everyday Spending".to_string(),
            account_type: AccountType::Bank,
            unit: aud(),
            starting_balance: money(1_000, 0, 2),
            is_active: true,
            created_on: NaiveDate::from_ymd_opt(2024, 10, 14).expect("valid date"),
            updated_on: FIXTURE_NOW,
            transactions_sum: money(3_210, 65, 2),
            transaction_count: 1_284,
            open_count: 12,
            first_posted: Some(NaiveDate::from_ymd_opt(2024, 8, 31).expect("valid date")),
            last_posted: Some(NaiveDate::from_ymd_opt(2026, 8, 31).expect("valid date")),
            balance_checks: Vec::new(),
        };
        // Eight month-end Balance Checks, each asserting exactly what the ledger computes for
        // its own date — "checked and matched" every time, per §7d's own "8 balance checks"
        // figure for this account.
        for months_back in (0..8).rev() {
            let date = month_end(
                everyday_spending.last_posted.expect("just set this above"),
                months_back,
            );
            let asserted = balance_as_of_for(&everyday_spending, date);
            everyday_spending
                .balance_checks
                .push(AccountBalanceCheck { date, asserted });
        }

        let mortgage_offset = Account {
            id: id(),
            name: "Mortgage Offset".to_string(),
            account_type: AccountType::Bank,
            unit: aud(),
            starting_balance: money(20_000, 0, 2),
            is_active: true,
            created_on: NaiveDate::from_ymd_opt(2021, 9, 8).expect("valid date"),
            updated_on: NaiveDate::from_ymd_opt(2026, 8, 20).expect("valid date"),
            transactions_sum: money(4_429, 50, 2),
            transaction_count: 60,
            open_count: 0,
            first_posted: Some(NaiveDate::from_ymd_opt(2021, 9, 15).expect("valid date")),
            last_posted: Some(NaiveDate::from_ymd_opt(2026, 8, 28).expect("valid date")),
            balance_checks: Vec::new(),
        };

        let amex_platinum = Account {
            id: id(),
            name: "Amex Platinum".to_string(),
            account_type: AccountType::CreditCard,
            unit: aud(),
            starting_balance: money(0, 0, 2),
            is_active: true,
            created_on: NaiveDate::from_ymd_opt(2023, 6, 1).expect("valid date"),
            updated_on: NaiveDate::from_ymd_opt(2026, 9, 2).expect("valid date"),
            transactions_sum: money(-1_284, 30, 2),
            transaction_count: 47,
            open_count: 5,
            first_posted: Some(NaiveDate::from_ymd_opt(2025, 10, 1).expect("valid date")),
            last_posted: Some(NaiveDate::from_ymd_opt(2026, 9, 5).expect("valid date")),
            balance_checks: Vec::new(),
        };

        let vanguard_vdhg = Account {
            id: id(),
            name: "Vanguard VDHG".to_string(),
            account_type: AccountType::Investment,
            unit: unit("VDHG", 4),
            starting_balance: money(0, 0, 4),
            is_active: true,
            created_on: NaiveDate::from_ymd_opt(2022, 11, 1).expect("valid date"),
            updated_on: NaiveDate::from_ymd_opt(2026, 8, 15).expect("valid date"),
            transactions_sum: money(412, 4800, 4),
            transaction_count: 18,
            open_count: 0,
            first_posted: Some(NaiveDate::from_ymd_opt(2022, 12, 1).expect("valid date")),
            last_posted: Some(NaiveDate::from_ymd_opt(2026, 8, 1).expect("valid date")),
            balance_checks: Vec::new(),
        };

        let cold_wallet = Account {
            id: id(),
            name: "Cold wallet".to_string(),
            account_type: AccountType::Investment,
            unit: unit("BTC", 8),
            starting_balance: money(0, 0, 8),
            is_active: true,
            created_on: NaiveDate::from_ymd_opt(2024, 2, 1).expect("valid date"),
            updated_on: NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid date"),
            transactions_sum: money(0, 18_400_000, 8),
            transaction_count: 4,
            open_count: 0,
            first_posted: Some(NaiveDate::from_ymd_opt(2024, 4, 1).expect("valid date")),
            last_posted: Some(NaiveDate::from_ymd_opt(2026, 5, 1).expect("valid date")),
            balance_checks: Vec::new(),
        };

        let home_loan = Account {
            id: id(),
            name: "Home Loan".to_string(),
            account_type: AccountType::Loan,
            unit: aud(),
            starting_balance: money(-650_000, 0, 2),
            is_active: true,
            created_on: NaiveDate::from_ymd_opt(2023, 9, 8).expect("valid date"),
            updated_on: NaiveDate::from_ymd_opt(2026, 9, 1).expect("valid date"),
            transactions_sum: money(37_600, 0, 2),
            transaction_count: 36,
            open_count: 0,
            first_posted: Some(NaiveDate::from_ymd_opt(2023, 10, 8).expect("valid date")),
            last_posted: Some(NaiveDate::from_ymd_opt(2026, 9, 8).expect("valid date")),
            balance_checks: Vec::new(),
        };

        // Deliberately inactive — one of the two `za`-only accounts behind "9 accounts · 7
        // active" (the other is `travel_cash`, above).
        let old_savings = Account {
            id: id(),
            name: "Old Savings".to_string(),
            account_type: AccountType::Bank,
            unit: aud(),
            starting_balance: money(5_000, 0, 2),
            is_active: false,
            created_on: NaiveDate::from_ymd_opt(2020, 1, 1).expect("valid date"),
            updated_on: NaiveDate::from_ymd_opt(2023, 1, 1).expect("valid date"),
            transactions_sum: money(0, 0, 2),
            transaction_count: 0,
            open_count: 0,
            first_posted: None,
            last_posted: None,
            balance_checks: Vec::new(),
        };

        Self {
            accounts: vec![
                wallet,
                travel_cash,
                everyday_spending,
                mortgage_offset,
                amex_platinum,
                vanguard_vdhg,
                cold_wallet,
                home_loan,
                old_savings,
            ],
        }
    }
}

impl AccountStore for AccountFixture {
    fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    fn find(&self, id: RowID) -> Option<&Account> {
        self.accounts.iter().find(|account| account.id == id)
    }

    fn grouped(&self, show_inactive: bool) -> Vec<(AccountType, Vec<&Account>)> {
        AccountType::all()
            .iter()
            .filter_map(|account_type| {
                let mut group: Vec<&Account> = self
                    .accounts
                    .iter()
                    .filter(|account| {
                        account.account_type == *account_type
                            && (show_inactive || account.is_active)
                    })
                    .collect();
                if group.is_empty() {
                    return None;
                }
                group.sort_by_key(|account| account.name.to_lowercase());
                Some((account_type.clone(), group))
            })
            .collect()
    }

    fn balance(&self, id: RowID) -> Money {
        self.find(id)
            .map(balance_for)
            .unwrap_or_else(|| money(0, 0, 2))
    }

    fn balance_as_of(&self, id: RowID, date: NaiveDate) -> Money {
        self.find(id)
            .map(|account| balance_as_of_for(account, date))
            .unwrap_or_else(|| money(0, 0, 2))
    }

    fn monthly_balances(&self, id: RowID, months: usize, as_of: NaiveDate) -> Vec<Money> {
        match self.find(id) {
            Some(account) => monthly_balances_for(account, months, as_of),
            None => std::iter::repeat_with(|| money(0, 0, 2))
                .take(months)
                .collect(),
        }
    }

    fn ledger(&self, id: RowID) -> Vec<AccountTransaction> {
        self.find(id).map(ledger_for).unwrap_or_default()
    }

    fn create(
        &mut self,
        name: String,
        account_type: AccountType,
        unit: AccountUnit,
        starting_balance: Money,
        active: bool,
    ) -> RowID {
        let id = RowID::new();
        let decimal_places = unit.decimal_places;
        self.accounts.push(Account {
            id,
            name,
            account_type,
            unit,
            starting_balance,
            is_active: active,
            created_on: FIXTURE_NOW,
            updated_on: FIXTURE_NOW,
            transactions_sum: money(0, 0, decimal_places as u32),
            transaction_count: 0,
            open_count: 0,
            first_posted: None,
            last_posted: None,
            balance_checks: Vec::new(),
        });
        id
    }

    fn update(
        &mut self,
        id: RowID,
        name: String,
        account_type: AccountType,
        active: bool,
    ) -> Result<(), AccountError> {
        let account = self
            .accounts
            .iter_mut()
            .find(|account| account.id == id)
            .ok_or(AccountError::NotFound)?;
        account.name = name;
        account.account_type = account_type;
        account.is_active = active;
        account.updated_on = FIXTURE_NOW;
        Ok(())
    }

    fn set_active(&mut self, id: RowID, active: bool) -> Result<(), AccountError> {
        let account = self
            .accounts
            .iter_mut()
            .find(|account| account.id == id)
            .ok_or(AccountError::NotFound)?;
        account.is_active = active;
        account.updated_on = FIXTURE_NOW;
        Ok(())
    }

    fn transfer_candidates(&self, id: RowID) -> Vec<&Account> {
        let Some(source) = self.find(id) else {
            return Vec::new();
        };
        self.accounts
            .iter()
            .filter(|account| {
                account.id != id && account.is_active && account.unit.code == source.unit.code
            })
            .collect()
    }

    fn delete(&mut self, id: RowID, target: Option<RowID>) -> Result<(), AccountError> {
        let source = self.find(id).ok_or(AccountError::NotFound)?.clone();
        let needs_transfer = source.transaction_count > 0 || !source.balance_checks.is_empty();

        if needs_transfer {
            let target_id = target.ok_or_else(|| AccountError::RequiresTransferTarget {
                name: source.name.clone(),
            })?;
            if target_id == id {
                return Err(AccountError::TransferTargetIsSource);
            }
            let target_account = self.find(target_id).ok_or(AccountError::NotFound)?;
            if !target_account.is_active {
                return Err(AccountError::TransferTargetInactive);
            }
            if target_account.unit.code != source.unit.code {
                return Err(AccountError::TransferTargetUnitMismatch {
                    name: source.name.clone(),
                    unit: source.unit.code.clone(),
                });
            }

            let target_mut = self
                .accounts
                .iter_mut()
                .find(|account| account.id == target_id)
                .expect("existence just checked above");
            target_mut.transactions_sum =
                Money(target_mut.transactions_sum.0.clone() + source.transactions_sum.0.clone());
            target_mut.transaction_count += source.transaction_count;
            target_mut.open_count += source.open_count;
            target_mut.first_posted = earliest(target_mut.first_posted, source.first_posted);
            target_mut.last_posted = latest(target_mut.last_posted, source.last_posted);
            // `source`'s Balance Checks are discarded, not moved — they assert a balance only
            // `id` ever had (README §7d).
        }

        self.accounts.retain(|account| account.id != id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_by_name<'a>(store: &'a AccountFixture, name: &str) -> &'a Account {
        store
            .accounts()
            .iter()
            .find(|account| account.name == name)
            .unwrap_or_else(|| panic!("fixture should seed an account named {name}"))
    }

    #[test]
    fn seeds_nine_accounts_seven_active() {
        let store = AccountFixture::new();
        assert_eq!(store.accounts().len(), 9);
        assert_eq!(store.accounts().iter().filter(|a| a.is_active).count(), 7);
    }

    /// Compares by `Money`/`BigDecimal` value, not `to_string()` — `bigdecimal` equality is
    /// value-based (`320.4` and `320.40` compare equal) but its `Display` prints whatever scale
    /// the value naturally reduced to, which needn't match a hand-typed literal's own digit
    /// count.
    #[test]
    fn balances_match_the_handoffs_displayed_totals() {
        let store = AccountFixture::new();
        for (name, expected) in [
            ("Wallet", money(320, 40, 2)),
            ("Everyday Spending", money(4_210, 65, 2)),
            ("Mortgage Offset", money(24_429, 50, 2)),
            ("Amex Platinum", money(-1_284, 30, 2)),
            ("Vanguard VDHG", money(412, 4_800, 4)),
            ("Cold wallet", money(0, 18_400_000, 8)),
            ("Home Loan", money(-612_400, 0, 2)),
        ] {
            let account = find_by_name(&store, name);
            assert_eq!(store.balance(account.id), expected, "{name}");
        }
    }

    #[test]
    fn grouped_in_default_view_matches_the_handoffs_bank_and_credit_card_subtotals() {
        let store = AccountFixture::new();
        let groups = store.grouped(false);

        let bank = groups
            .iter()
            .find(|(account_type, _)| *account_type == AccountType::Bank)
            .expect("bank group should be present");
        let bank_total: BigDecimal = bank
            .1
            .iter()
            .map(|account| store.balance(account.id).0)
            .sum();
        assert_eq!(Money(bank_total), money(28_640, 15, 2));

        let credit_card = groups
            .iter()
            .find(|(account_type, _)| *account_type == AccountType::CreditCard)
            .expect("credit card group should be present");
        assert_eq!(credit_card.1.len(), 1);
    }

    #[test]
    fn grouped_follows_account_types_own_enum_order() {
        let store = AccountFixture::new();
        let order: Vec<AccountType> = store
            .grouped(false)
            .into_iter()
            .map(|(account_type, _)| account_type)
            .collect();
        assert_eq!(
            order,
            vec![
                AccountType::Cash,
                AccountType::Bank,
                AccountType::CreditCard,
                AccountType::Investment,
                AccountType::Loan,
            ]
        );
    }

    #[test]
    fn shared_unit_is_none_for_a_mixed_unit_group_and_some_for_a_single_unit_group() {
        let store = AccountFixture::new();
        let groups = store.grouped(false);

        let investment = &groups
            .iter()
            .find(|(account_type, _)| *account_type == AccountType::Investment)
            .expect("investment group should be present")
            .1;
        assert_eq!(super::super::shared_unit(investment), None);

        let bank = &groups
            .iter()
            .find(|(account_type, _)| *account_type == AccountType::Bank)
            .expect("bank group should be present")
            .1;
        assert_eq!(
            super::super::shared_unit(bank).map(|unit| unit.code),
            Some("AUD".to_string())
        );
    }

    #[test]
    fn cash_group_excludes_the_inactive_mixed_unit_pair_by_default() {
        let store = AccountFixture::new();
        let groups = store.grouped(false);
        let cash = &groups
            .iter()
            .find(|(account_type, _)| *account_type == AccountType::Cash)
            .expect("cash group should be present")
            .1;
        assert_eq!(cash.len(), 1);
        assert_eq!(cash[0].name, "Wallet");
    }

    #[test]
    fn everyday_spendings_last_balance_check_has_zero_variance() {
        let store = AccountFixture::new();
        let account = find_by_name(&store, "Everyday Spending");
        assert_eq!(account.balance_checks.len(), 8);

        let last_check = account
            .balance_checks
            .last()
            .expect("seeded with 8 balance checks");
        let computed = store.balance_as_of(account.id, last_check.date);
        assert_eq!(last_check.asserted, computed);
    }

    #[test]
    fn ledger_sums_exactly_to_transactions_sum() {
        let store = AccountFixture::new();
        let account = find_by_name(&store, "Everyday Spending");
        let ledger = store.ledger(account.id);
        assert_eq!(ledger.len(), 1_284);

        let sum: BigDecimal = ledger.iter().map(|row| row.amount.0.clone()).sum();
        assert_eq!(Money(sum), account.transactions_sum);
    }

    #[test]
    fn ledger_marks_exactly_open_count_rows_as_open() {
        let store = AccountFixture::new();
        let account = find_by_name(&store, "Everyday Spending");
        let open = store
            .ledger(account.id)
            .iter()
            .filter(|row| row.status == TransactionStatus::Open)
            .count();
        assert_eq!(open as u32, account.open_count);
    }

    #[test]
    fn monthly_balances_ends_on_the_current_balance() {
        let store = AccountFixture::new();
        let account = find_by_name(&store, "Everyday Spending");
        let series = store.monthly_balances(account.id, 24, FIXTURE_NOW);
        assert_eq!(series.len(), 24);
        assert_eq!(
            *series.last().expect("24 months requested"),
            store.balance(account.id)
        );
    }

    #[test]
    fn delete_refuses_a_non_empty_account_without_a_transfer_target() {
        let mut store = AccountFixture::new();
        let everyday_spending = find_by_name(&store, "Everyday Spending").id;
        assert_eq!(
            store.delete(everyday_spending, None),
            Err(AccountError::RequiresTransferTarget {
                name: "Everyday Spending".to_string()
            })
        );
    }

    #[test]
    fn delete_refuses_a_cross_unit_transfer_target() {
        let mut store = AccountFixture::new();
        let everyday_spending = find_by_name(&store, "Everyday Spending").id;
        let vanguard_vdhg = find_by_name(&store, "Vanguard VDHG").id;
        assert_eq!(
            store.delete(everyday_spending, Some(vanguard_vdhg)),
            Err(AccountError::TransferTargetUnitMismatch {
                name: "Everyday Spending".to_string(),
                unit: "AUD".to_string()
            })
        );
    }

    #[test]
    fn delete_moves_transactions_onto_the_target_and_discards_the_sources_balance_checks() {
        let mut store = AccountFixture::new();
        let everyday_spending = find_by_name(&store, "Everyday Spending").id;
        let mortgage_offset = find_by_name(&store, "Mortgage Offset").id;
        let target_balance_before = store.balance(mortgage_offset);

        store
            .delete(everyday_spending, Some(mortgage_offset))
            .expect("same-unit active target should succeed");

        assert!(store.find(everyday_spending).is_none());
        let target = store.find(mortgage_offset).expect("target survives");
        assert_eq!(target.transaction_count, 60 + 1_284);
        assert_eq!(
            store.balance(mortgage_offset).0,
            target_balance_before.0 + money(3_210, 65, 2).0
        );
    }

    #[test]
    fn delete_skips_the_transfer_step_on_an_empty_account() {
        let mut store = AccountFixture::new();
        let wallet = find_by_name(&store, "Wallet").id;
        store
            .delete(wallet, None)
            .expect("an empty account should delete without a target");
        assert!(store.find(wallet).is_none());
    }

    #[test]
    fn transfer_candidates_exclude_inactive_accounts_the_source_itself_and_other_units() {
        let store = AccountFixture::new();
        let everyday_spending = find_by_name(&store, "Everyday Spending").id;
        let candidates = store.transfer_candidates(everyday_spending);

        assert!(candidates.iter().all(|account| account.is_active));
        assert!(
            candidates
                .iter()
                .all(|account| account.id != everyday_spending)
        );
        assert!(candidates.iter().all(|account| account.unit.code == "AUD"));
        assert!(
            candidates
                .iter()
                .any(|account| account.name == "Mortgage Offset")
        );
    }

    #[test]
    fn create_then_update_then_set_active_round_trip() {
        let mut store = AccountFixture::new();
        let id = store.create(
            "New Cash".to_string(),
            AccountType::Cash,
            unit("AUD", 2),
            money(50, 0, 2),
            true,
        );
        assert_eq!(store.balance(id), money(50, 0, 2));

        store
            .update(id, "Renamed Cash".to_string(), AccountType::Bank, true)
            .expect("update should succeed");
        let account = store.find(id).expect("just created");
        assert_eq!(account.name, "Renamed Cash");
        assert_eq!(account.account_type, AccountType::Bank);

        store
            .set_active(id, false)
            .expect("set_active should succeed");
        assert!(!store.find(id).expect("still present").is_active);
    }
}
