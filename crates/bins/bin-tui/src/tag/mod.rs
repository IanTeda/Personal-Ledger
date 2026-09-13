//! The Tags domain model for the "Tags catalog screen, views and popup" Wayfinder map (issue
//! #125): a [`TagStore`] seam over an in-memory fixture, mirroring `crate::category`'s and
//! `crate::account`'s own seams so the screen and popups built on top of it can be swapped
//! onto real `lib_database` persistence later without a UI rewrite — only a second `impl
//! TagStore` is needed.
//!
//! **The domain model was fully settled before this map started** — `CONTEXT.md`'s **Tag**
//! glossary entry and [ADR-0015](docs/adr/0015-tag-as-independent-transaction-label.md): a
//! freeform, globally-unique (case-insensitive) label, `is_active` soft-delete, no note, no
//! colour, no rename-alias history. `Tag` here carries exactly those fields and nothing more.
//!
//! **`tagged_transaction_count` is fixture-only, never a real join** — `view::transactions`
//! has no store of its own yet (this map's own Notes), so there is no real Transaction to
//! count references against. It exists purely so the screen's summary box and delete confirm
//! have something real (if simulated) to show, the same way `crate::account`'s own fixture
//! simulates a fake ledger without `view::transactions` being involved at all.
//!
//! **State ownership**: everything that changes (selection, fold/filter state, form drafts)
//! lives inside whichever `View`/popup struct owns it, mutated directly in its own
//! `handle_key`/`update` — never routed through `crate::view::Action`, mirroring
//! `crate::category`'s/`crate::account`'s own decision.
//!
//! **The right-pane widgets** (`view::tags`'s own "Tagged spend"/"Where it lands"/
//! "Transactions", issue #133) are backed by [`TagTransaction`] rows [`TagStore::transactions`]
//! generates on demand from a Tag's own `category_pool` — a handful of real leaf category
//! paths copied, as plain display strings, from a temporary `crate::category::CategoryFixture`
//! at fixture-construction time (see `fixture::expense_leaf_paths`). This is exactly
//! `tagged_transaction_count`'s own precedent, extended: fixture-only simulation, never a real
//! join back to Category or a real Transaction.

mod fixture;

pub use fixture::{FIXTURE_NOW, TagFixture};

use chrono::NaiveDate;
use lib_core::{Money, RowID};

/// One Tag — a freeform, globally-unique label a user can attach to any number of
/// Transactions, independent of their Category and Payee (`CONTEXT.md`, ADR-0015).
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub id: RowID,
    pub name: String,
    pub is_active: bool,
    pub created_on: NaiveDate,
    pub updated_on: NaiveDate,
    /// How many Transactions this Tag is attached to — fixture-simulated, see this module's
    /// own doc. Shown in the summary box and in the delete confirm line ("N transactions will
    /// lose this tag"); never gates deletion, per the map's own "delete is never refused"
    /// decision. Always equals `TagStore::transactions(id).len()`, by construction.
    pub tagged_transaction_count: u32,
    /// Real leaf category paths (e.g. `"food/restaurants"`) this Tag's simulated transactions
    /// are drawn from — private, fixture-internal input to [`fixture::transactions_for`], never
    /// part of the public API surface (unlike `tagged_transaction_count`, nothing outside
    /// `crate::tag` reads this directly; `TagStore::transactions`/`category_breakdown`/
    /// `monthly_spend` are the real seam). Empty for a Tag with no transactions.
    category_pool: Vec<String>,
}

/// One fixture-simulated transaction attributed to a Tag, generated on demand by
/// [`TagStore::transactions`] — never stored, mirroring `crate::account::AccountTransaction`'s
/// own "generate, don't store" precedent.
#[derive(Debug, Clone, PartialEq)]
pub struct TagTransaction {
    pub date: NaiveDate,
    pub payee: &'static str,
    /// A real leaf category path, copied as plain display text — see this module's own doc.
    pub category_path: String,
    /// How many *other* Tags this (fake) transaction also carries — the overlap
    /// `view::tags`'s own footer statement ("N of M carry another tag") is about.
    pub other_tags: u32,
    pub amount: Money,
}

/// Everything [`TagStore::create`]/[`TagStore::update`] can refuse — just the one rule, per
/// ADR-0015's global (not sibling-scoped, Tag has no hierarchy) case-insensitive uniqueness.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TagError {
    #[error("tag not found")]
    NotFound,
    #[error("a tag named \"{name}\" already exists")]
    DuplicateName { name: String },
}

/// The seam a future backend ticket implements a second time against real `lib_database`
/// persistence (out of scope for this map — see issue #125's Notes/Out of scope). Every
/// method here is synchronous and read methods return plain values (no `Result`) because the
/// only implementation today ([`TagFixture`]) is in-memory.
pub trait TagStore {
    /// Every Tag, in no particular order — callers sort as they need (the screen sorts
    /// alphabetically; Tag has no classification dimension to group by).
    fn tags(&self) -> &[Tag];

    fn find(&self, id: RowID) -> Option<&Tag>;

    /// Creates a new Tag, or `Err(TagError::DuplicateName)` if `name` clashes
    /// case-insensitively with an existing Tag (active or not — a deactivated Tag's name is
    /// still taken, nothing here frees it back up).
    fn create(&mut self, name: String, active: bool) -> Result<RowID, TagError>;

    /// Updates `name`/`active`. The same case-insensitive uniqueness check as `create`
    /// applies, except against the Tag's own current name (renaming a Tag to the name it
    /// already has is never a clash).
    fn update(&mut self, id: RowID, name: String, active: bool) -> Result<(), TagError>;

    fn set_active(&mut self, id: RowID, active: bool) -> Result<(), TagError>;

    /// Deletes a Tag unconditionally — never refused, regardless of
    /// `tagged_transaction_count`. Removing a Tag just means those (simulated) Transactions
    /// lose a label; unlike deleting an Account, nothing is orphaned or destroyed, per the
    /// map's own decision.
    fn delete(&mut self, id: RowID) -> Result<(), TagError>;

    /// The Tag's fixture-simulated transaction rows, newest first — generated on demand,
    /// deterministically seeded from the Tag's own id (mirroring `AccountStore::ledger`), not
    /// stored. Empty for a Tag with `tagged_transaction_count == 0`. Every other right-pane
    /// widget (`category_breakdown`, `monthly_spend`) derives from exactly these rows, so the
    /// three stay internally consistent by construction rather than needing to agree by hand.
    fn transactions(&self, id: RowID) -> Vec<TagTransaction>;

    /// The Tag's spend grouped by `TagTransaction::category_path`, biggest total first —
    /// derived from [`TagStore::transactions`], for "Where it lands". Empty for a Tag with no
    /// transactions.
    fn category_breakdown(&self, id: RowID) -> Vec<(String, Money)>;

    /// Month-end spend totals for the trailing `months` months ending at `as_of`'s own month
    /// (inclusive), one pass over [`TagStore::transactions`] — not `months` separate queries.
    /// Oldest month first, for "Tagged spend"'s sparkline. Every entry is `0` for a Tag with no
    /// transactions in that month (or none at all), rather than a gap — the flat run before a
    /// Tag's first use is the point, per the design doc this widget is drawn from.
    fn monthly_spend(&self, id: RowID, months: usize, as_of: NaiveDate) -> Vec<Money>;
}
