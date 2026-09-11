//! The per-view model each `View` implements, hosted by `Shell` — the new navigation shape
//! ADR-0013 (`docs/adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md`) introduces in
//! place of the breadcrumb `App`/`Screen` stack (`app.rs`/`screen/`, left compiling but
//! disconnected from `main.rs`). `Shell` hosts exactly one active `View` at a time — no
//! navigation stack, no breadcrumb; the shell's own command window (a later ticket) is the
//! only way the design specifies for switching which `View` is active.

pub mod accounts;
pub mod balance_checks;
pub mod budgets;
pub mod categories;
pub mod dashboard;
pub mod help;
pub mod payees;
pub mod reports;
pub mod settings;
pub mod transactions;
pub mod units;

use crossterm::event::KeyEvent;
use lib_core::RowID;
use ratatui::{Frame, layout::Rect};
use tokio::sync::mpsc::UnboundedSender;

use crate::category::CategoryStore;

/// A message `Shell` or the active `View` reacts to. Deliberately minimal for now — the full
/// action registry the command window will dispatch through (ADR-0013) is later work; this
/// only needs enough to drive the event loop and let a `View` react to a tick.
///
/// The `CommandPopup*` variants drive `Shell`'s own command window (`crate::popup::command`)
/// rather than the active `View` — they exist here because `Shell`'s event loop only redraws
/// in response to an `Action`, and every `View::update` implementation ignores variants it
/// doesn't care about, so adding shell-chrome messages alongside `Quit`/`Tick` costs a `View`
/// nothing. The `CategoryMovePopup*`/`MoveCategory`/`CreateCategoryChild` variants are the
/// `Category`-domain popup's own equivalent (`crate::popup::category`), and are the reason
/// this enum no longer derives `Copy` — `CreateCategoryChild` carries an owned `String` (a
/// typed category name), which a `Copy` type can't. Every other variant still copies for
/// free; only code that actually needs one of these two now needs to `.clone()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// A periodic tick, driving redraws even without input.
    Tick,
    /// `Ctrl+C` — the hard-quit safety net, recognised by `Shell` itself before any `View`
    /// sees the key.
    Quit,
    /// `Ctrl+;` — opens the command popup overlay.
    OpenCommandPopup,
    /// `Esc`, or `Ctrl+;` again — closes the command popup overlay.
    CloseCommandPopup,
    /// A printable character typed while the command popup is open, appended to its input
    /// buffer.
    CommandPopupInput(char),
    /// `Backspace` while the command popup is open — removes the last character of its input.
    CommandPopupBackspace,
    /// `↑` while the command popup is open — moves the selection up one candidate row.
    CommandPopupMoveUp,
    /// `↓` while the command popup is open — moves the selection down one candidate row.
    CommandPopupMoveDown,
    /// `Tab` while the command popup is open — clears a showing "not yet built" message;
    /// completion against the filtered candidates isn't built yet, so this has no other
    /// effect.
    CommandPopupTab,
    /// `Enter` on a command outside the 6 with real content behind them (`unit`, `unit
    /// new/edit/delete`, `dashboard`, `settings`) — shows `":{name} — not yet built"` in the
    /// popup's info row, replacing whatever argument preview was showing. The popup stays
    /// open; `Enter` never closes it.
    CommandPopupSetNotYetBuilt(&'static str),
    /// `Ctrl+U`, or `Enter` on the command popup's `unit` command — opens the placeholder
    /// Units view (`docs/ux/tui/units/README.md`), the first domain to land a real (if still
    /// wireframe-stage) destination behind the command popup's `unit` entry.
    OpenUnits,
    /// `n` on the Units view, or `Enter` on the command popup's `unit new <code> <type>`
    /// command — opens the "new unit" popup (`docs/ux/tui/units/README.md` §4b).
    OpenNewUnitPopup,
    /// `e` on the Units view, or `Enter` on the command popup's `unit edit <code>` command —
    /// opens the "edit unit" popup (`docs/ux/tui/units/README.md` §4c).
    OpenEditUnitPopup,
    /// `d` on the Units view, or `Enter` on the command popup's `unit delete <code>` command —
    /// opens the "delete unit" popup (`docs/ux/tui/units/README.md` §4d/§4e).
    OpenDeleteUnitPopup,
    /// `Esc` while a unit popup (new, edit or delete) is open — closes it without saving or
    /// deleting anything.
    CloseUnitPopup,
    /// `g d` (the `docs/ux/tui/README.md` "Jumps" table's chord), or `Enter` on the command
    /// popup's `dashboard` command — returns to the Dashboard view.
    OpenDashboard,
    /// `g a`, or `Enter` on the command popup's `account list` command — opens the
    /// placeholder Accounts view.
    OpenAccounts,
    /// `g k`, or `Enter` on the command popup's `check list` command — opens the placeholder
    /// Balance Checks view.
    OpenBalanceChecks,
    /// `g b`, or `Enter` on the command popup's `budget list [period]` command — opens the
    /// placeholder Budgets view.
    OpenBudgets,
    /// `g c`, or `Enter` on the command popup's `category list` command — opens the
    /// placeholder Categories view.
    OpenCategories,
    /// `?`, or `Enter` on the command popup's `help` command — opens the placeholder Help
    /// view.
    OpenHelp,
    /// `g p`, or `Enter` on the command popup's `payee list` command — opens the placeholder
    /// Payees view.
    OpenPayees,
    /// `g r`, or `Enter` on the command popup's `report list` command — opens the placeholder
    /// Reports view.
    OpenReports,
    /// `g s`, or `Enter` on the command popup's `settings` command — opens the Settings view
    /// (`docs/ux/tui/settings/README.md` §4a — the "at rest" wireframe only; the
    /// database-backed registry, in-place editor and base-unit guard are later work).
    OpenSettings,
    /// `g t`, or `Enter` on the command popup's `txn recent` command — opens the placeholder
    /// Transactions view.
    OpenTransactions,
    /// `Esc`, only reached when no popup is open (a popup's own `Esc` maps to
    /// `CloseCommandPopup`/`CloseUnitPopup` first) — pops one view off `Shell`'s
    /// `view_stack`, or no-ops if the stack is empty (already at Dashboard, the app's root).
    PopView,
    /// `q` (lowercase), live only when no popup is open — quits the app, identically to
    /// `Quit` today, but through its own variant so a future confirm-before-quit check (once
    /// any view holds dirty/unsaved state) can be inserted without re-plumbing the keybinding.
    GracefulQuit,
    /// A key a `View` (or `Shell`, for the Category popup's own text-editing keys) handled
    /// entirely by mutating local state directly, with nothing further to relay — distinct
    /// from returning `None`, which `Shell::run`'s event loop treats as "nothing happened" and
    /// skips the next redraw for.
    NoOp,
    /// `m` on a Categories tree row — opens the move popup (`crate::popup::category::
    /// move_popup`, "Categories: 5b move popup") for the given category.
    OpenCategoryMovePopup(RowID),
    /// `Esc` while the Category popup is open — closes it without moving anything.
    CloseCategoryPopup,
    /// A printable character typed while the Category move popup's `new parent` field has
    /// focus (its only field) — appended to its input buffer.
    CategoryMovePopupInput(char),
    /// `Backspace` while the Category move popup is open — removes the last character of its
    /// input.
    CategoryMovePopupBackspace,
    /// `Tab` while the Category move popup is open — completes the input's last path segment
    /// against the first matching category name, per the handoff's "new parent completes on
    /// full paths".
    CategoryMovePopupTab,
    /// `Ctrl+S` on the Category move popup, once its typed path resolves to an existing
    /// category `validate_move` accepts — `Shell` resolves the input against the active
    /// view's `CategoryStore` (via `View::category_store`) into this concrete, `Copy`-friendly
    /// pair before dispatching, so only this variant (not the raw typed text) ever needs to
    /// reach a `View::update`.
    MoveCategory { id: RowID, new_parent: RowID },
    /// `Ctrl+N` on the Category move popup, when its typed path's last segment doesn't exist
    /// yet under an otherwise-resolved parent — creates it inline, per the handoff's "`^n`
    /// creates a missing parent inline". Carries an owned `String` (the typed name), which is
    /// why `Action` no longer derives `Copy` — see this enum's own doc comment.
    CreateCategoryChild { parent: RowID, name: String },
    /// `n` (child of the tree selection) / `N` (sibling of it) on a Categories tree row —
    /// opens the new popup (`crate::popup::category::new_popup`, "Categories: 5c new popup").
    /// `CategoriesView::handle_key` resolves which parent `n`/`N` prefill before this is ever
    /// dispatched, so this only ever carries the one, already-resolved id.
    OpenCategoryNewPopup(RowID),
    /// A printable character typed while the Category new popup is open — routed to whichever
    /// of its fields (`name`/`parent`/`note`/`active`) currently has focus.
    CategoryNewPopupInput(char),
    /// `Backspace` while the Category new popup is open — removes the last character of
    /// whichever field has focus.
    CategoryNewPopupBackspace,
    /// `Tab` while the Category new popup is open — completes the `parent` field's last path
    /// segment when it has focus and a candidate exists, otherwise advances focus to the next
    /// field (reconciling the handoff's "`parent` ... `tab` to change" with its own "`tab`
    /// next field").
    CategoryNewPopupTab,
    /// `Ctrl+S`/`Ctrl+A` on the Category new popup, once its draft validates (a resolved
    /// `parent`, a non-empty `name`, no sibling clash) — `Shell` resolves the popup's current
    /// fields into this concrete pair before dispatching, the same way `MoveCategory` does.
    /// `close_after` is `false` for `Ctrl+A` ("create and start another sibling" — the popup
    /// stays open, reset for the next name), `true` for `Ctrl+S`.
    CreateCategory {
        parent: RowID,
        name: String,
        note: Option<String>,
        active: bool,
        close_after: bool,
    },
    /// `e` on a Categories tree row — opens the edit popup (`crate::popup::category::
    /// edit_popup`, "Categories: 5d edit popup"). Never dispatched for a root
    /// (`CategoriesView::handle_key` refuses it there — roots have no editable name/note/
    /// active).
    OpenCategoryEditPopup(RowID),
    /// A printable character typed while the Category edit popup is open — routed to
    /// whichever of its fields (`name`/`note`/`active`) currently has focus.
    CategoryEditPopupInput(char),
    /// `Backspace` while the Category edit popup is open.
    CategoryEditPopupBackspace,
    /// `Tab` while the Category edit popup is open — advances focus to the next field (no
    /// path to complete, unlike Move/New's `parent` field).
    CategoryEditPopupTab,
    /// `X` while the Category edit popup is open, or on a Categories tree row directly —
    /// merge has no real logic yet ("Not yet designed" in the handoff), so this only shows
    /// the "not yet built" fallback (in the popup's own title tag, or
    /// `CategoriesView::merge_hint` on the bare tree).
    CategoryMerge,
    /// `Ctrl+S` (save the draft as typed) or `Ctrl+A` (save it, but with `active` forced
    /// `false` regardless of the checkbox — the handoff's own soft-delete path, "archiving
    /// keeps all N transactions and totals; it only stops the category being offered") on the
    /// Category edit popup, once the draft validates (a non-empty `name`, no sibling clash) —
    /// `Shell` resolves the popup's current fields into this pair the same way
    /// `MoveCategory`/`CreateCategory` do. `a` on a Categories tree row reaches the same
    /// underlying `CategoryStore::set_active`, but directly (no popup, no draft to carry), so
    /// it never goes through this variant.
    UpdateCategory {
        id: RowID,
        name: String,
        note: Option<String>,
        active: bool,
    },
}

/// The single view `Shell` hosts at a time.
pub trait View {
    /// Called once when the view becomes active, with a sender any background work (e.g. a
    /// data load) can use to report results back as an [`Action`]. Most views have no
    /// background work and can rely on this default no-op.
    fn init(&mut self, _action_tx: UnboundedSender<Action>) {}

    /// First refusal on a raw key press while this view is active: interpret it as an
    /// [`Action`], or return `None` to fall through to `Shell`'s own global keys (`Ctrl+C`).
    /// The default no-op means a view with nothing view-specific to bind just gets the
    /// global keys.
    fn handle_key(&mut self, _key: KeyEvent) -> Option<Action> {
        None
    }

    /// Reacts to an [`Action`] that wasn't handled at the `Shell` level.
    fn update(&mut self, action: &Action);

    /// Renders the view into the given area of the frame — the full-bleed view region below
    /// the status line and above the command line.
    fn view(&self, frame: &mut Frame, area: Rect);

    /// Short name for the view, shown in the shell's status line.
    fn title(&self) -> &'static str;

    /// Read-only access to this view's Category tree, if it has one — `Some` only for
    /// `view::categories::CategoriesView`. Lets `Shell` render and resolve the Category
    /// popup's live preview/validation (it needs the tree; `Shell` itself doesn't own one)
    /// without downcasting the `Box<dyn View>` trait object. Mutation still goes through
    /// `View::update` (`Action::MoveCategory`/`CreateCategoryChild`), never through this.
    fn category_store(&self) -> Option<&dyn CategoryStore> {
        None
    }
}
