//! The Accounts domain model shared by every ticket in the "Accounts screen, views and
//! popup" Wayfinder map (issue #115): an [`AccountStore`] seam over an in-memory fixture, so
//! the screen and popups built on top of it can be swapped onto real `lib_database`
//! persistence later (a future map, once `Shell` has an async-DB mechanism for *any* `View` —
//! see issue #115's Notes) without a UI rewrite — only a second `impl AccountStore` is
//! needed.
//!
//! **Unlike `crate::category`, the real `lib_database::Accounts` model already matches this
//! design exactly** (`lib_core::AccountType`'s five variants, `unit_id` fixed, computed
//! Balance, `is_active` soft-delete) — so this module reuses `lib_core::AccountType` directly
//! rather than inventing a local duplicate (contrast `crate::category::CategoryKind`, which
//! *does* diverge from `lib_core::CategoryTypes`, an unrelated domain).
//!
//! **State ownership**: everything that changes (selection, fold/filter state, form drafts)
//! lives inside whichever `View`/popup struct owns it, mutated directly in its own
//! `handle_key`/`update` — never routed through `crate::view::Action`, mirroring
//! `crate::category`'s own decision.
//!
//! `docs/ux/tui/accounts/README.md` is the authority on the model this mirrors: five fixed
//! `AccountType`s, exactly one Unit per account fixed at creation, Balance computed on read
//! (`starting_balance + Σ transactions`, never stored), and `is_active` a soft-delete flag.

mod fixture;

pub use fixture::{AccountFixture, FIXTURE_NOW};

use chrono::NaiveDate;
use lib_core::{AccountType, Money, RowID};

/// A Unit's two facts an Account needs: its code (for the "same unit"/"mixed units" rules and
/// transfer-target filtering) and its display precision (`decimal_places`, e.g. `2` for AUD,
/// `4` for a fund, `8` for BTC). Denormalised directly onto [`Account`] rather than a separate
/// Units store/join — `crate::view::units` has no fixture/store of its own yet (still a
/// wireframe, see issue #115's Notes), so there is nothing real to join against, and no
/// account-side rule here ever needs a Unit's own name or kind, only these two facts.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountUnit {
    pub code: String,
    pub decimal_places: i64,
}

/// One seeded Balance Check against an Account — an assertion of what its Balance should be
/// on a date, checked against [`AccountStore::balance_as_of`]. Mirrors the handoff's "last
/// check ... var 0.00" (a zero variance is worth printing, not hiding) and 7d's "8 balance
/// checks ... they assert a balance this account no longer has" once deleted.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountBalanceCheck {
    pub date: NaiveDate,
    pub asserted: Money,
}

/// One Account — a place value is held or owed, per `docs/ux/tui/accounts/README.md`.
/// `transaction_count`/`open_count`/`first_posted`/`last_posted` summarise a ledger that
/// [`fixture::ledger_for`] generates on demand (date/payee/amount/status rows, deterministically
/// seeded from `id`) rather than storing individually — the same "generate on demand, don't
/// store per-row" choice `crate::view::categories`'s `transactions_for_node` already made, at a
/// scale (the handoff's own `1 284` for Everyday Spending) that would otherwise mean carrying
/// thousands of rows in memory for a wireframe-fidelity map.
#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: RowID,
    pub name: String,
    pub account_type: AccountType,
    pub unit: AccountUnit,
    pub starting_balance: Money,
    pub is_active: bool,
    pub created_on: NaiveDate,
    pub updated_on: NaiveDate,
    /// `Σ` of every seeded Transaction's signed amount, exact in `BigDecimal` — combined with
    /// `starting_balance` to compute the current Balance, per the handoff's compute-on-read
    /// rule. [`fixture::ledger_for`]'s generated rows always sum back to exactly this value
    /// (a residual is folded into the last row specifically to guarantee that).
    pub transactions_sum: Money,
    pub transaction_count: u32,
    /// How many of `transaction_count` are [`TransactionStatus::Open`] (unreconciled) — the
    /// newest `open_count` generated rows, per [`fixture::ledger_for`].
    pub open_count: u32,
    /// `None` exactly when `transaction_count` is `0`.
    pub first_posted: Option<NaiveDate>,
    pub last_posted: Option<NaiveDate>,
    pub balance_checks: Vec<AccountBalanceCheck>,
}

/// One generated ledger row — see [`Account`]'s own doc on why these aren't stored.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountTransaction {
    pub date: NaiveDate,
    pub payee: &'static str,
    pub amount: Money,
    pub status: TransactionStatus,
}

/// A Transaction's reconciliation state — rendered as a glyph, never colour alone, per the
/// handoff's "Status is a glyph, not a colour".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionStatus {
    Open,
    Reconciled,
}

impl TransactionStatus {
    pub fn glyph(self) -> char {
        match self {
            TransactionStatus::Open => '○',
            TransactionStatus::Reconciled => '✓',
        }
    }
}

/// Everything [`AccountStore::delete`] can refuse, per the handoff's "7d — Delete" rules.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AccountError {
    #[error("account not found")]
    NotFound,
    #[error(
        "{name} holds transactions or Balance Checks — choose a same-unit account to move them into"
    )]
    RequiresTransferTarget { name: String },
    #[error("the transfer target must share {name}'s unit ({unit})")]
    TransferTargetUnitMismatch { name: String, unit: String },
    #[error("can't transfer an account's transactions to itself")]
    TransferTargetIsSource,
    #[error("the transfer target must be active")]
    TransferTargetInactive,
}

/// The seam a future backend ticket implements a second time against real `lib_database`
/// persistence, once `Shell` has an async-DB mechanism for any `View` (out of scope for the
/// current map — see issue #115's Notes). Every method here is synchronous and read methods
/// return plain values (no `Result`) because the only implementation today
/// ([`AccountFixture`]) is in-memory — a real implementation will need to become
/// `async`/fallible on the reads too, which is exactly the kind of change this trait exists to
/// localise to one `impl` rather than every call site in the screen/popups.
pub trait AccountStore {
    /// Every account, in no particular order — callers group/sort as they need.
    fn accounts(&self) -> &[Account];

    fn find(&self, id: RowID) -> Option<&Account>;

    /// Accounts grouped by `AccountType`, in the enum's own declared order
    /// (`AccountType::all()`); a type with no accounts (after the `show_inactive` filter) is
    /// omitted entirely, per the handoff's "empty types are omitted".
    fn grouped(&self, show_inactive: bool) -> Vec<(AccountType, Vec<&Account>)>;

    /// `starting_balance + transactions_sum` — the Balance, computed on read, never stored.
    fn balance(&self, id: RowID) -> Money;

    /// The Balance pinned to `date`: `starting_balance` plus every generated ledger row on or
    /// before `date`. Used by the summary box's Balance Check variance and the balance-line
    /// chart's month-end series.
    fn balance_as_of(&self, id: RowID, date: NaiveDate) -> Money;

    /// Month-end balances for the trailing `months` months ending at `as_of`'s own month
    /// (inclusive), in one pass over the account's generated ledger rather than `months`
    /// separate [`AccountStore::balance_as_of`] calls, per the handoff's "compute the series in
    /// one pass". Oldest month first.
    fn monthly_balances(&self, id: RowID, months: usize, as_of: NaiveDate) -> Vec<Money>;

    /// The account's own ledger rows, newest first — generates via [`fixture::ledger_for`], not
    /// stored; exposed on the trait so a real implementation can page a query instead.
    fn ledger(&self, id: RowID) -> Vec<AccountTransaction>;

    fn create(
        &mut self,
        name: String,
        account_type: AccountType,
        unit: AccountUnit,
        starting_balance: Money,
        active: bool,
    ) -> RowID;

    /// Updates `name`/`account_type`/`is_active` only — `unit` and `starting_balance` are fixed
    /// at creation (FR.13) and have no setter here.
    fn update(
        &mut self,
        id: RowID,
        name: String,
        account_type: AccountType,
        active: bool,
    ) -> Result<(), AccountError>;

    fn set_active(&mut self, id: RowID, active: bool) -> Result<(), AccountError>;

    /// Candidates for `delete(id, ..)`'s transfer target: active accounts sharing `id`'s unit,
    /// excluding `id` itself — the handoff's "same-unit only ... excludes inactive and the
    /// account being deleted".
    fn transfer_candidates(&self, id: RowID) -> Vec<&Account>;

    /// Deletes `id`. When it holds transactions or Balance Checks, `target` must name an
    /// active, same-unit, non-`id` account — its seeded ledger and `transactions_sum` move onto
    /// `target` (mirroring the handoff's `UPDATE transactions SET account_id = :target`), `id`'s
    /// Balance Checks are discarded (they assert a balance only `id` ever had), then `id` itself
    /// is removed. On an empty account (`transaction_count == 0` and no Balance Checks) `target`
    /// is ignored and this is just the removal.
    fn delete(&mut self, id: RowID, target: Option<RowID>) -> Result<(), AccountError>;
}

/// Whether every account in `accounts` shares one Unit code — the handoff's "a group subtotals
/// only when every account in it shares the base unit" rule. `None` means mixed (print "mixed
/// units"); `Some` carries the shared unit so callers can also read its `decimal_places`.
pub fn shared_unit(accounts: &[&Account]) -> Option<AccountUnit> {
    let first = accounts.first()?.unit.clone();
    if accounts
        .iter()
        .all(|account| account.unit.code == first.code)
    {
        Some(first)
    } else {
        None
    }
}
