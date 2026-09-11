//! The Categories domain model shared by every ticket in the "Categories screen, views and
//! popup" Wayfinder map (issue #106): a [`CategoryStore`] seam over an in-memory tree, so the
//! screen and popups built on top of it can be swapped onto real `lib_database` persistence
//! later (a future map, once the tree-shaped schema itself is decided) without a UI rewrite —
//! only a second `impl CategoryStore` is needed.
//!
//! **State ownership**: everything that actually changes (the tree shape, fold state, form
//! drafts) lives inside whichever `View`/popup struct owns it, mutated directly in its own
//! `handle_key`/`update` — never routed through `crate::view::Action`. `Action` only needs to
//! carry a `RowID` (`Copy`) when it signals a transition across the `Shell` boundary (e.g.
//! "open the move popup for this category"); it never needs to carry owned Category data, so
//! its `#[derive(Copy)]` stays as-is.
//!
//! `docs/ux/tui/categories/README.md` is the authority on the model this mirrors: two fixed,
//! non-deletable roots (`income`/`expenses`), kind inherited from the root and never stored
//! per node, unlimited depth, and every node carrying both a direct amount (posted to it) and
//! a rollup (itself plus every descendant, computed — never stored).

mod fixture;

pub use fixture::CategoryFixture;

use lib_core::{Money, RowID};

/// The two roots a Category's `kind` is always inherited from — never stored per node, per
/// the handoff's "Kind is derived, not stored" rule. Deliberately a local, two-variant type
/// rather than reusing `lib_core::CategoryTypes` (five flat accounting classifications for
/// the *old*, unrelated flat-Categories domain — reusing it here would carry three variants
/// that can never apply to this tree, and would tie this map's fixture to a type the closed
/// "Retire old Screen-architecture Categories code" cleanup is removing call sites for).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryKind {
    Income,
    Expense,
}

impl CategoryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CategoryKind::Income => "income",
            CategoryKind::Expense => "expense",
        }
    }
}

/// One node in the Category tree — a root (`parent_id: None`) or any descendant. Mirrors the
/// handoff's `category` table shape (`parent_id`/`name`/`note`/`active`) plus `direct`, the
/// amount posted straight to this node; `rollup` (direct plus every descendant's rollup) is
/// derived by [`CategoryStore::rollup`], never stored, exactly like the handoff's recursive
/// CTE. `transaction_count`, `first_posted` and `last_posted` are direct-only, matching
/// `direct` — see the handoff's summary box `transactions 148 · direct` and `first · last`
/// rows. `first_posted`/`last_posted` are `None` exactly when `transaction_count` is `0`; a
/// full per-transaction fixture (for the 5a right pane's transactions list) is a later
/// ticket's job, this is only the two dates the summary box itself needs.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryNode {
    pub id: RowID,
    pub parent_id: Option<RowID>,
    pub name: String,
    pub note: Option<String>,
    pub active: bool,
    pub direct: Money,
    pub transaction_count: u32,
    pub first_posted: Option<chrono::NaiveDate>,
    pub last_posted: Option<chrono::NaiveDate>,
}

/// Everything [`CategoryStore::move_to`], `insert` and `rename` can refuse, per the handoff's
/// "5b — Move" `refuses` line and its sibling-uniqueness/root-immutability rules.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CategoryError {
    #[error("{name} can't move into its own descendant set")]
    WouldCycle { name: String },
    #[error("{name} can't cross into the other root while it or a descendant has transactions")]
    CrossRootWithTransactions { name: String },
    #[error("a category named \"{name}\" already exists under this parent")]
    DuplicateSibling { name: String },
    #[error("category not found")]
    NotFound,
    #[error("the two roots (income/expenses) can't be renamed, moved or archived")]
    IsRoot,
}

/// The seam a future backend ticket implements a second time against real `lib_database`
/// persistence, once the tree-shaped schema itself is decided (out of scope for the current
/// map — see issue #106's "Out of scope"). Every method here is synchronous and read methods
/// return plain values (no `Result`) because the only implementation today
/// ([`CategoryFixture`]) is in-memory; a real implementation will need to become
/// `async`/fallible on the reads too, which is exactly the kind of change this trait exists to
/// localise to one `impl` rather than every call site in the screen/popups.
pub trait CategoryStore {
    /// Every node, in no particular order — callers derive tree order from `parent_id`.
    fn nodes(&self) -> &[CategoryNode];

    fn find(&self, id: RowID) -> Option<&CategoryNode>;

    /// Direct children of `parent`, in this store's insertion order (callers sort by name, per
    /// the handoff's "siblings sort by name, there is no manual ordering").
    fn children(&self, parent: RowID) -> Vec<&CategoryNode>;

    /// `id`'s own root — walks `parent_id` to the top, per the handoff's `root(id)`.
    fn root(&self, id: RowID) -> Option<&CategoryNode>;

    /// `id`'s inherited kind, derived from its root's name.
    fn kind(&self, id: RowID) -> Option<CategoryKind>;

    /// Every id in `id`'s own subtree, `id` itself included — the set a move's cycle check,
    /// and a future merge/delete's "has descendants" check, both need.
    fn descendants(&self, id: RowID) -> Vec<RowID>;

    /// `direct(id) + Σ rollup(children)`, per the handoff's recursive-CTE definition.
    fn rollup(&self, id: RowID) -> Money;

    fn insert(
        &mut self,
        parent: RowID,
        name: String,
        note: Option<String>,
    ) -> Result<RowID, CategoryError>;

    fn rename(&mut self, id: RowID, name: String) -> Result<(), CategoryError>;

    fn move_to(&mut self, id: RowID, new_parent: RowID) -> Result<(), CategoryError>;

    /// Sets `active` directly — `false` is the handoff's "archive" (soft path: history and
    /// totals intact, just no longer offered when categorising); `true` reverses it.
    fn set_active(&mut self, id: RowID, active: bool) -> Result<(), CategoryError>;
}
