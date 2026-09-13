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

mod fixture;

pub use fixture::TagFixture;

use chrono::NaiveDate;
use lib_core::RowID;

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
    /// decision.
    pub tagged_transaction_count: u32,
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
}
