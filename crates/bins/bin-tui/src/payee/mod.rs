//! The Payees domain model for the "Payees screen, views and popup" Wayfinder map (issue
//! #134): a [`PayeeStore`] seam over an in-memory fixture, mirroring `crate::account`'s/
//! `crate::tag`'s own seams so the screen and popups built on top of it can be swapped onto
//! real `lib_database` persistence later (a future map, once `Shell` has an async-DB mechanism
//! for *any* `View` — see issue #134's Notes) without a UI rewrite — only a second `impl
//! PayeeStore` is needed.
//!
//! **Unlike `crate::category`, the real `lib_database::Payees`/`PayeeAliases` already match
//! most of this design** (case-insensitively unique name, `is_active` soft-delete, write-once
//! rename aliases, `resolve_or_create`'s three-step order) — [`Payee`]/[`PayeeAlias`] carry
//! those fields verbatim, plus the ones the README's design adds that aren't real yet:
//! `website`, `icon_url` (+ whether it was derived from `website` or hand-typed),
//! `default_category_path`, and a `source`/hit-count pair per alias. All fixture-only for now
//! — see issue #134's Notes/Out of scope.
//!
//! **`default_category_path`/category-mix rows are plain display strings, never a `RowID`
//! join** — copied once from a temporary `crate::category::CategoryFixture` at seed time,
//! mirroring `crate::tag::fixture`'s own `category_pool` precedent exactly (`view::categories`
//! has no shared store another `View` can reach into; `Shell` hosts one `View` at a time).
//!
//! **State ownership**: everything that changes (selection, filter/`za` state, form drafts)
//! lives inside whichever `View`/popup struct owns it, mutated directly in its own
//! `handle_key`/`update` — never routed through `crate::view::Action`, mirroring every other
//! domain module's own decision.
//!
//! `docs/ux/tui/payees/README.md` is the authority on the model this mirrors: a Payee is
//! curated, not created by hand; renaming always leaves a `source = Rename` alias, protected
//! from removal; a hand-authored (`source = Manual`) alias can collide with another Payee's
//! own name or pattern, resolving arbitrarily — [`PayeeStore::conflict_partners`] and
//! [`PayeeStore::add_alias`]'s own refusal are this module's answer to that "correctness bug
//! this design surfaces" section.

mod fixture;

pub use fixture::PayeeFixture;

use chrono::NaiveDate;
use lib_core::{Money, RowID};

pub use crate::account::TransactionStatus;

/// Where a Payee Alias came from — `Rename` is protected (only a rename itself, never a user
/// action, can remove it); `Manual` is the hand-authored kind `docs/ux/tui/payees/README.md`'s
/// 8d introduces, and the only kind [`PayeeStore::remove_alias`] will ever remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasSource {
    Rename,
    Manual,
}

/// How [`PayeeStore::add_alias`]'s typed text becomes a stored pattern — the README's 8d "as"
/// control. `ExactText` escapes and anchors exactly as [`PayeeStore::rename`] does
/// (`(?i)^{escaped}$`); `Regex` stores the typed text verbatim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasMode {
    ExactText,
    Regex,
}

/// A former name of a Payee (or a hand-authored pattern, once `source` is `Manual`) — mirrors
/// `lib_database::PayeeAliases`, plus the `source`/`hits` fields the real schema doesn't carry
/// yet (see this module's own doc).
#[derive(Debug, Clone, PartialEq)]
pub struct PayeeAlias {
    pub id: RowID,
    pub payee_id: RowID,
    pub pattern: String,
    pub source: AliasSource,
    /// How many times typed text has resolved through this alias — fixture-authored, not
    /// derived from real Transaction history (there is none yet).
    pub hits: u32,
}

/// One Payee. Mirrors `lib_database::Payees`'s real fields (`id`/`name`/`is_active`/
/// `created_on`/`updated_on`) plus the fixture-only fields the README's design adds:
/// `website`/`icon_url`/`icon_derived`/`default_category_path`.
#[derive(Debug, Clone, PartialEq)]
pub struct Payee {
    pub id: RowID,
    pub name: String,
    pub is_active: bool,
    pub website: Option<String>,
    pub icon_url: Option<String>,
    /// `true` when `icon_url` was derived from `website` (the "`[×] derive from website`"
    /// default); `false` when hand-typed. Meaningless when `icon_url` is `None`.
    pub icon_derived: bool,
    /// A real Category leaf path (e.g. `"food/groceries"`), copied as a plain display string —
    /// see this module's own doc on why nothing here holds a `RowID` back to Category.
    pub default_category_path: Option<String>,
    pub created_on: NaiveDate,
    pub updated_on: NaiveDate,
    /// How many Transactions reference this Payee — the record box's "seen N times".
    pub transaction_count: u32,
    /// How many of `transaction_count`'s generated rows are [`TransactionStatus::Open`] — the
    /// newest `open_count` rows, mirroring `crate::account::Account`'s own field.
    pub open_count: u32,
    /// `None` exactly when `transaction_count` is `0`.
    pub first_posted: Option<NaiveDate>,
    pub last_posted: Option<NaiveDate>,
    /// `Σ` of every generated transaction's signed amount, exact in `BigDecimal` — the
    /// Payee's own total (FR.36), in the base unit.
    pub transactions_sum: Money,
    /// Category path -> relative weight, driving [`fixture::transactions_for`]'s generated
    /// rows — fixture-internal, never part of the public API (mirrors `crate::tag::Tag`'s own
    /// private `category_pool`).
    category_weights: Vec<(String, f64)>,
}

/// One generated transaction row attributed to a Payee — see [`Payee`]'s own doc on why these
/// aren't stored.
#[derive(Debug, Clone, PartialEq)]
pub struct PayeeTransaction {
    pub date: NaiveDate,
    pub category_path: String,
    pub amount: Money,
    pub status: TransactionStatus,
}

/// One row of a Payee's category mix — README §*Right pane*'s proportional bars, biggest share
/// first.
#[derive(Debug, Clone, PartialEq)]
pub struct PayeeCategoryShare {
    pub category_path: String,
    pub amount: Money,
    /// This category's share of the Payee's total *absolute* spend, `0.0..=1.0`.
    pub share: f64,
}

/// What typing text against [`PayeeStore::resolve`] would do — mirrors
/// `lib_database::Payees::resolve_or_create`'s real three-step order exactly, short of actually
/// creating anything (step 3 is reported, not performed, so 8d's "test" can preview it safely).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayeeResolution {
    /// Step 1 — an exact, case-insensitive name match.
    ExactName(RowID),
    /// Step 2 — an alias pattern matched (whichever Payee it belongs to).
    Alias { payee_id: RowID, alias_id: RowID },
    /// Step 3 — nothing matched; a real `resolve_or_create` would create a new Payee here.
    WouldCreate,
}

/// Everything a `PayeeStore` mutation can refuse.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PayeeError {
    #[error("payee not found")]
    NotFound,
    #[error("alias not found")]
    AliasNotFound,
    #[error("a payee named \"{holder}\" already exists")]
    DuplicateName { holder: String },
    #[error("{transaction_count} txns · {alias_count} matches reference this payee")]
    ReferencesExist {
        transaction_count: u32,
        alias_count: usize,
    },
    #[error("a rename match is protected — edit refuses it, only remove is allowed")]
    RenameProtectedAlias,
    #[error(
        "\"{pattern}\" also matches {other} — two payees matching one text resolves arbitrarily"
    )]
    PatternCollision { pattern: String, other: String },
}

/// The seam a future backend ticket implements a second time against real `lib_database`
/// persistence, once `Shell` has an async-DB mechanism for any `View` (out of scope for the
/// current map — see issue #134's Notes). Every method here is synchronous and read methods
/// return plain values because the only implementation today ([`PayeeFixture`]) is in-memory.
pub trait PayeeStore {
    /// Every Payee, in no particular order — callers sort as they need (the list sorts by
    /// `abs(total)` descending).
    fn payees(&self) -> &[Payee];

    fn find(&self, id: RowID) -> Option<&Payee>;

    /// An exact, case-insensitive name match — resolution's own step 1.
    fn find_by_name(&self, name: &str) -> Option<&Payee>;

    /// Every alias belonging to `id`, in no particular order.
    fn aliases(&self, id: RowID) -> Vec<&PayeeAlias>;

    /// `Σ` of every generated transaction's signed amount — the base-unit total the list's
    /// `TOTAL` column and the record box's `total` line both read.
    fn total(&self, id: RowID) -> Money;

    /// This Payee's spend grouped by category, biggest absolute share first — derived from
    /// [`PayeeStore::transactions`], never stored independently (so the two can't disagree).
    fn category_mix(&self, id: RowID) -> Vec<PayeeCategoryShare>;

    /// The Payee's generated transaction rows, newest first.
    fn transactions(&self, id: RowID) -> Vec<PayeeTransaction>;

    /// Every other Payee this one shares an ambiguous resolution with: one of `id`'s aliases
    /// matches the other's name (or vice versa), or the two hold an identical pattern — the
    /// README's "two payees matching one text resolves arbitrarily" bug, and the source of the
    /// list's `!` flag. **A fixture-only heuristic**: it catches an alias colliding with
    /// another Payee's exact name, and two Payees sharing one verbatim pattern string, but not
    /// two textually-different regexes that happen to overlap — full regex-intersection
    /// checking is out of scope here, mirroring the real `resolve_or_create`'s own "first match
    /// wins, in arbitrary order" limitation rather than solving a harder problem it doesn't.
    fn conflict_partners(&self, id: RowID) -> Vec<RowID>;

    /// The name of the other Payee `pattern` would collide with if added to `payee_id` right
    /// now, if any — matches that other Payee's exact name, or duplicates one of their own
    /// alias patterns verbatim. The read-only half of [`PayeeStore::add_alias`]'s own
    /// collision check (see [`PayeeStore::conflict_partners`]'s own doc on what "collides"
    /// means here), exposed so a compose-in-progress popup (8d) can preview the same refusal
    /// before committing.
    fn conflicting_holder(&self, payee_id: RowID, pattern: &str) -> Option<String>;

    /// Creates a new Payee, or `Err(PayeeError::DuplicateName)` on a case-insensitive name
    /// clash (active or not).
    fn create(
        &mut self,
        name: String,
        website: Option<String>,
        icon_url: Option<String>,
        icon_derived: bool,
        default_category_path: Option<String>,
        active: bool,
    ) -> Result<RowID, PayeeError>;

    /// Renames `id`, inserting a `source = Rename` alias for its prior name in the same step —
    /// there is deliberately no plain "set name" (mirrors `lib_database::Payees::rename`'s own
    /// contract). A no-op when `new_name` is unchanged case-insensitively — no alias is written
    /// for that. Fails on a case-insensitive collision with another Payee's current name.
    fn rename(&mut self, id: RowID, new_name: String) -> Result<(), PayeeError>;

    /// Updates `website`/`icon_url`/`icon_derived`/`default_category_path` only — name goes
    /// through [`PayeeStore::rename`], `is_active` through [`PayeeStore::set_active`].
    fn update(
        &mut self,
        id: RowID,
        website: Option<String>,
        icon_url: Option<String>,
        icon_derived: bool,
        default_category_path: Option<String>,
    ) -> Result<(), PayeeError>;

    fn set_active(&mut self, id: RowID, active: bool) -> Result<(), PayeeError>;

    /// Deletes `id` — refused, mirroring the real schema's foreign-key pragma, while it holds
    /// any transaction or any alias (including its own rename history, which references it
    /// forever once it exists).
    fn delete(&mut self, id: RowID) -> Result<(), PayeeError>;

    /// Adds a hand-authored (`source = Manual`) alias to `payee_id`, refusing a pattern that
    /// collides with another Payee (see [`PayeeStore::conflict_partners`]'s own doc on what
    /// "collides" means here).
    fn add_alias(
        &mut self,
        payee_id: RowID,
        typed: &str,
        mode: AliasMode,
    ) -> Result<RowID, PayeeError>;

    /// Removes a `source = Manual` alias; refuses (`Err(PayeeError::RenameProtectedAlias)`) on
    /// a `source = Rename` one.
    fn remove_alias(&mut self, alias_id: RowID) -> Result<(), PayeeError>;

    /// Runs the real three-step resolution order (`lib_database::Payees::resolve_or_create`'s
    /// own algorithm, short of the final create) against typed text.
    fn resolve(&self, text: &str) -> PayeeResolution;
}

/// Every distinct category path currently in use as some Payee's `default_category_path`,
/// sorted — the `default` field's completion source for the New/Edit popups. Mirrors
/// `crate::account::known_units`: `view::categories` has no shared store another `View` can
/// reach into (`Shell` hosts one `View` at a time), so "every path a Payee already defaults to"
/// is the only real candidate list available at this map's fidelity.
pub fn known_category_paths(payees: &[Payee]) -> Vec<String> {
    let mut paths: Vec<String> = payees
        .iter()
        .filter_map(|payee| payee.default_category_path.clone())
        .collect();
    paths.sort();
    paths.dedup();
    paths
}
