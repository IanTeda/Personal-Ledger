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
pub mod tags;
pub mod transactions;
pub mod units;

use crossterm::event::KeyEvent;
use lib_core::{Money, RowID};
use ratatui::{Frame, layout::Rect};
use tokio::sync::mpsc::UnboundedSender;

use crate::account::AccountStore;
use crate::category::CategoryStore;
use crate::payee::{AliasMode, PayeeStore};
use crate::tag::TagStore;

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
    /// `Ctrl+r` while the command popup is open — walks backward through `Shell`'s own
    /// `command_history`, replacing the input buffer with each entry in turn (see
    /// `crate::popup::command::CommandPopup::recall_history`'s doc for the cursor's own
    /// behaviour). A no-op against an empty history.
    CommandPopupHistoryRecall,
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
    /// `g c`, or `Enter` on the command popup's `cat` command — opens the Categories view
    /// (`view::categories::CategoriesView`, "Categories screen, views and popup", issue #106).
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
    /// (`docs/ux/tui/settings/README.md` §4a — the "at rest" wireframe, plus the §4b/§4c popups
    /// below; the database-backed registry behind either is later work).
    OpenSettings,
    /// `e` on the Settings view — opens the in-place editor popup over the `general.negatives`
    /// worked example (`docs/ux/tui/settings/README.md` §4b, "Editing in place").
    OpenEditSettingPopup,
    /// `enter` on the Settings view — opens the base-unit guard overlay over the `general.
    /// base_unit` worked example (`docs/ux/tui/settings/README.md` §4c, "Base unit guard").
    /// Bound to a key of its own rather than reached by committing an §4b edit, since there is
    /// no real "currently selected setting" state yet to route a generic commit through —
    /// `view::settings::SettingsView`'s own module doc says more.
    OpenBaseUnitGuardPopup,
    /// `Esc` while a settings popup (the §4b editor or the §4c guard) is open — closes it
    /// without committing anything.
    CloseSettingsPopup,
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
    MoveCategory {
        id: RowID,
        new_parent: RowID,
    },
    /// `Ctrl+N` on the Category move popup, when its typed path's last segment doesn't exist
    /// yet under an otherwise-resolved parent — creates it inline, per the handoff's "`^n`
    /// creates a missing parent inline". Carries an owned `String` (the typed name), which is
    /// why `Action` no longer derives `Copy` — see this enum's own doc comment.
    CreateCategoryChild {
        parent: RowID,
        name: String,
    },
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
    /// `n` on the Accounts list — opens the new-account popup (`crate::popup::account::new`,
    /// "Accounts: 7b new popup").
    OpenAccountNewPopup,
    /// `Esc` while the Account popup is open — closes it without creating anything.
    CloseAccountPopup,
    /// A printable character typed while the Account new popup is open — routed to whichever
    /// of its text fields (`name`/`unit`/`starting balance`) currently has focus; a no-op on
    /// the `type`/`active` fields, which aren't text (see `AccountNewPopupTypeLeft`/`Right`
    /// and the space-toggle on `active`, handled inside the popup's own `push_char`).
    AccountNewPopupInput(char),
    AccountNewPopupBackspace,
    /// `Tab` — completes the `unit` field against a known Unit code when it has focus and a
    /// candidate exists, otherwise advances focus to the next field.
    AccountNewPopupTab,
    /// `h` while the Account new popup's `type` field has focus — steps the five-way pick
    /// back one, per the handoff's "five-way inline pick ... h/l".
    AccountNewPopupTypeLeft,
    /// `l` — steps the five-way `type` pick forward one.
    AccountNewPopupTypeRight,
    /// `Ctrl+S`/`Ctrl+A` on the Account new popup, once its draft validates (a resolved
    /// `unit`, a non-empty `name`, a parseable `starting balance`) — `Shell` resolves the
    /// popup's current fields into this concrete, `Eq`-friendly variant before dispatching,
    /// the same way `CreateCategory` does. `account_type` is carried as `AccountType::as_str`'s
    /// own string rather than the enum itself, purely so this variant's fields all stay `Eq`
    /// (`Action` derives it; `lib_core::AccountType` doesn't) — `AccountsView::update` parses
    /// it back via `FromStr`. `close_after` is `false` for `Ctrl+A` ("create and start
    /// another" — the popup stays open, reset for the next account), `true` for `Ctrl+S`.
    CreateAccount {
        name: String,
        account_type: String,
        unit_code: String,
        unit_decimal_places: i64,
        starting_balance: Money,
        active: bool,
        close_after: bool,
    },
    /// `e` on an Accounts list row — opens the edit popup
    /// (`crate::popup::account::edit`, "Accounts: 7c edit popup") for the given account.
    OpenAccountEditPopup(RowID),
    /// A printable character typed while the Account edit popup is open — routed to
    /// whichever of its fields (`name`/`type`/`active`) currently has focus.
    AccountEditPopupInput(char),
    AccountEditPopupBackspace,
    /// `Tab` while the Account edit popup is open — advances focus to the next field (no
    /// completion to apply, unlike the new popup's `unit` field).
    AccountEditPopupTab,
    /// `h` while the Account edit popup's `type` field has focus — steps the five-way pick
    /// back one.
    AccountEditPopupTypeLeft,
    /// `l` — steps the five-way `type` pick forward one.
    AccountEditPopupTypeRight,
    /// `Ctrl+S` (save as typed) or `Ctrl+A` (save, but with `active` forced `false`
    /// regardless of the checkbox — the handoff's own "`^a` deactivate") on the Account edit
    /// popup, once its draft validates (a non-empty `name`) — `Shell` resolves the popup's
    /// current fields into this pair, mirroring `UpdateCategory`. `account_type` is carried
    /// as a plain `String` for the same reason `CreateAccount` does (`Action` derives `Eq`;
    /// `lib_core::AccountType` doesn't).
    UpdateAccount {
        id: RowID,
        name: String,
        account_type: String,
        active: bool,
    },
    /// `d` on an Accounts list row, or `Ctrl+D` from the edit popup — opens the delete popup
    /// (`crate::popup::account::delete`, "Accounts: 7d delete popup") for the given account.
    OpenAccountDeletePopup(RowID),
    /// A printable character typed while the Account delete popup is open — routed to
    /// whichever of its fields (`move to`/`confirm`) currently has focus.
    AccountDeletePopupInput(char),
    AccountDeletePopupBackspace,
    /// `Tab` while the Account delete popup is open — completes `move to` against a transfer
    /// candidate's name when it has focus and one exists, otherwise advances focus (a no-op
    /// on an empty account, which has no `move to` field to cycle to).
    AccountDeletePopupTab,
    /// `Ctrl+S` on the Account delete popup, once its draft validates (the account's name
    /// typed exactly, plus a resolved same-Unit transfer target when the account isn't empty)
    /// — `Shell` resolves the popup's current fields into this pair before dispatching.
    /// `target` is `None` exactly when the account was empty, matching
    /// `AccountStore::delete`'s own signature.
    DeleteAccount {
        id: RowID,
        target: Option<RowID>,
    },
    /// Bare `a` on the Accounts list (README's own "a deactivate"), or `Enter` on the command
    /// popup's `account off <acct>`/`account on <acct>` entries — both reach this one variant
    /// rather than two different ones, so the keybinding and the command truly share a single
    /// code path (`AccountsView::update` is the only place that calls
    /// `AccountStore::set_active`).
    SetAccountActive {
        id: RowID,
        active: bool,
    },
    /// `g g`, or `Enter` on the command popup's `tag` entry — opens the Tags catalog screen
    /// (`view::tags::TagsView`, "Tags: screen — list, summary box, and lightweight delete").
    /// Every other domain claimed its own initial as a `g <letter>` chord; Transactions
    /// already has `t`, so Tags doubles the leader key itself instead, once the letter
    /// shortage became a real discoverability gap (nothing on Dashboard or in the jump table
    /// otherwise pointed at `:tag` at all).
    OpenTags,
    /// `n` on the Tags list — opens the new-tag popup (`crate::popup::tag::new`, "Tags: new
    /// popup").
    OpenTagNewPopup,
    /// `Esc` while a Tag popup is open — closes it without creating anything.
    CloseTagPopup,
    /// A printable character typed while the Tag new popup's `name` field has focus — routed
    /// to it; a no-op on the `active` field, which isn't text (the space-toggle on it is
    /// handled inside the popup's own `push_char`).
    TagNewPopupInput(char),
    TagNewPopupBackspace,
    /// `Tab` while the Tag new popup is open — advances focus to the other field (no
    /// completion to apply, unlike `popup::account::new`'s `unit` field).
    TagNewPopupTab,
    /// `Ctrl+S`/`Ctrl+A` on the Tag new popup, once its draft validates (a non-empty `name`
    /// with no case-insensitive clash) — `Shell` resolves the popup's current fields into this
    /// pair before dispatching, mirroring `CreateAccount`. `close_after` is `false` for
    /// `Ctrl+A` ("create and start another" — the popup stays open, reset for the next Tag),
    /// `true` for `Ctrl+S`.
    CreateTag {
        name: String,
        active: bool,
        close_after: bool,
    },
    /// `e` on a Tags list row — opens the edit popup (`crate::popup::tag::edit`, "Tags: edit
    /// popup") for the given Tag.
    OpenTagEditPopup(RowID),
    /// A printable character typed while the Tag edit popup's `name` field has focus — routed
    /// to it; a no-op on `active`, mirroring `TagNewPopupInput`.
    TagEditPopupInput(char),
    TagEditPopupBackspace,
    /// `Tab` while the Tag edit popup is open — advances focus to the other field.
    TagEditPopupTab,
    /// `Ctrl+S` (save as typed) or `Ctrl+A` (save, but with `active` forced `false` regardless
    /// of the checkbox — the ticket's own "`^a` deactivates without requiring the checkbox to
    /// be toggled first") on the Tag edit popup, once its draft validates (a non-empty `name`
    /// with no case-insensitive clash against another Tag) — `Shell` resolves the popup's
    /// current fields into this pair before dispatching, mirroring `UpdateAccount`.
    UpdateTag {
        id: RowID,
        name: String,
        active: bool,
    },
    /// `Enter` on the command popup's `tag off <tag>`/`tag on <tag>` entries — reaches
    /// `TagStore::set_active` directly, the same way `SetAccountActive` does for Accounts
    /// (`TagsView::update` is the only place that calls it). No bare key reaches this yet —
    /// unlike Accounts, the Tags list has no bare `a` key of its own — so today this is only
    /// ever dispatched from the command grammar.
    SetTagActive {
        id: RowID,
        active: bool,
    },
    /// `Enter` on the command popup's `tag delete <tag>` entry — arms the exact same
    /// lightweight delete-confirm state the Tags list's own bare `d` key does
    /// (`TagsView::handle_key`'s `pending_delete`), against whatever the list currently has
    /// selected. Still needs `y` on the Tags view itself to actually delete — this command
    /// never deletes on its own, per `popup::command::commands::tags`'s own module doc.
    ArmTagDelete,
    /// `n` on the Payees list, or `:payee new <name>` — opens the new-payee popup
    /// (`crate::popup::payee::new`, "Payees: 8b new popup").
    OpenPayeeNewPopup,
    /// `Esc` while the Payee popup is open — closes it without creating anything.
    ClosePayeePopup,
    /// A printable character typed while the Payee new popup is open — routed to whichever of
    /// its text fields (`name`/`website`/`icon url`/`default`) currently has focus; a no-op on
    /// the `icon derive`/`active` checkboxes, whose space-toggle is handled inside the popup's
    /// own `push_char`.
    PayeeNewPopupInput(char),
    PayeeNewPopupBackspace,
    /// `Tab` — completes the `default` field against a known category path when it has focus
    /// and a candidate exists, otherwise advances focus (skipping `icon url` while `[×] derive
    /// from website` is checked).
    PayeeNewPopupTab,
    /// `Ctrl+S`/`Ctrl+A` on the Payee new popup, once its draft validates (a non-empty `name`
    /// with no case-insensitive collision against an existing Payee) — `Shell` resolves the
    /// popup's current fields into this concrete variant before dispatching, mirroring
    /// `CreateAccount`/`CreateTag`. `close_after` is `false` for `Ctrl+A` ("create and start
    /// another" — the popup stays open, every field reset), `true` for `Ctrl+S`.
    CreatePayee {
        name: String,
        website: Option<String>,
        icon_url: Option<String>,
        icon_derived: bool,
        default_category_path: Option<String>,
        active: bool,
        close_after: bool,
    },
    /// `e` on a Payees list row — opens the edit popup (`crate::popup::payee::edit`, "Payees:
    /// 8c edit popup") for the given Payee.
    OpenPayeeEditPopup(RowID),
    /// A printable character typed while the Payee edit popup is open — routed to whichever of
    /// its text fields (`name`/`website`/`icon url`/`default`) currently has focus; a no-op on
    /// the `active` checkbox, whose space-toggle is handled inside the popup's own
    /// `push_char`.
    PayeeEditPopupInput(char),
    PayeeEditPopupBackspace,
    /// `Tab` — completes the `default` field against a known category path when it has focus
    /// and a candidate exists, otherwise advances focus.
    PayeeEditPopupTab,
    /// `Ctrl+S` (save as typed) or `Ctrl+A` (save, but with `active` forced `false` regardless
    /// of the checkbox) on the Payee edit popup, once its draft validates (a non-empty `name`
    /// with no case-insensitive collision against another Payee) — `Shell` resolves the
    /// popup's current fields into this variant before dispatching, mirroring `UpdateAccount`/
    /// `UpdateTag`. Bundles what the real domain splits into three separate `PayeeStore` calls
    /// (`rename`/`update`/`set_active`) into one user-facing intent, the same way
    /// `UpdateAccount` does for `AccountStore::update`.
    UpdatePayee {
        id: RowID,
        name: String,
        website: Option<String>,
        icon_url: Option<String>,
        icon_derived: bool,
        default_category_path: Option<String>,
        active: bool,
    },
    /// `m` on a Payees list row, or from the edit popup — opens the rename-matches popup
    /// (`crate::popup::payee::matches`, "Payees: 8d rename matches popup") for the given
    /// Payee.
    OpenPayeeMatchesPopup(RowID),
    /// `↑`/`↓` (or `k`/`j`) while the Payee matches popup's list has focus — moves the
    /// selection among the Payee's own aliases.
    PayeeMatchesPopupMoveUp,
    PayeeMatchesPopupMoveDown,
    /// `a` — starts composing a brand-new manual alias.
    PayeeMatchesPopupBeginAdd,
    /// `e` on a `source = Manual` row — starts composing a replacement for it, prefilled from
    /// its current pattern. Never dispatched for a `source = Rename` row (`PayeeMatchesPopup`
    /// itself refuses to produce this action there — see its own module doc); the row's own
    /// protection is stated in the popup's permanent footnote instead of a per-keypress
    /// refusal message.
    PayeeMatchesPopupBeginEdit,
    /// `t` — starts composing arbitrary text purely to preview what it would resolve to
    /// (`PayeeStore::resolve`), with no intent to save anything.
    PayeeMatchesPopupBeginTest,
    /// `Esc` while composing (add/edit/test) — discards the draft and returns focus to the
    /// list, without closing the popup itself (`Esc` with no compose in progress closes the
    /// popup instead, via the ordinary `ClosePayeePopup`).
    PayeeMatchesPopupCancelCompose,
    /// A printable character typed while composing — appended to the draft's typed text.
    PayeeMatchesPopupInput(char),
    PayeeMatchesPopupBackspace,
    /// `Tab` while composing — toggles the `as` control between `exact text` and `regex`.
    PayeeMatchesPopupToggleMode,
    /// `d` on a `source = Manual` row — removes it. Never dispatched for a `source = Rename`
    /// row, mirroring `PayeeMatchesPopupBeginEdit`'s own guard.
    RemovePayeeAlias(RowID),
    /// `Ctrl+S` while composing a brand-new alias, once the typed text compiles to a pattern
    /// that doesn't collide with another Payee (`Shell` checks this against
    /// `PayeeStore::conflicting_holder` before ever producing this action, mirroring
    /// `CreatePayee`'s own name-collision guard).
    AddPayeeAlias {
        payee_id: RowID,
        typed: String,
        mode: AliasMode,
    },
    /// `Ctrl+S` while composing a replacement for an existing `source = Manual` alias —
    /// removes `old_alias_id` and adds the newly-typed one in its place, the closest this
    /// fixture-only model comes to an in-place edit (there is no `PayeeStore::update_alias`;
    /// see `crate::popup::payee::matches`'s own module doc for why that's an acceptable
    /// trade-off here). Guarded the same way as `AddPayeeAlias`.
    ReplacePayeeAlias {
        old_alias_id: RowID,
        payee_id: RowID,
        typed: String,
        mode: AliasMode,
    },
    /// `d` on a Payees list row, or `^d` from the edit popup — opens the delete popup
    /// (`crate::popup::payee::delete`, "Payees: 8e delete popup") for the given Payee.
    OpenPayeeDeletePopup(RowID),
    /// A printable character typed while the Payee delete popup's `confirm` field has focus —
    /// routed to it; a no-op on the `action` radio, whose space-toggle is handled inside the
    /// popup's own `push_char`.
    PayeeDeletePopupInput(char),
    PayeeDeletePopupBackspace,
    /// `Tab` while the Payee delete popup is open — advances focus between the `action` radio
    /// and `confirm`.
    PayeeDeletePopupTab,
    /// Bare `a` on a Payees list row, or `Ctrl+S` on the delete popup with `(•) deactivate`
    /// selected — deactivates the Payee directly, mirroring `Action::SetAccountActive`'s own
    /// "no popup, no draft to carry" shape. `PayeesView::update` is the only place that calls
    /// `PayeeStore::set_active`, so the bare key and the popup's own commit reach the exact
    /// same code path.
    SetPayeeActive {
        id: RowID,
        active: bool,
    },
    /// `Ctrl+S` on the Payee delete popup with `( ) delete` selected, once it validates (the
    /// Payee holds no transaction or alias — the database's own foreign-key pragma would
    /// refuse it otherwise — and `confirm` exactly matches its name) — `Shell` checks this via
    /// `crate::popup::payee::delete::DeletePayeePopup::commit` before ever producing this
    /// action.
    DeletePayee(RowID),
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

    /// The currently-selected category, if this view is `CategoriesView` and has one — lets
    /// `Shell` dispatch the `:category` command grammar's `new`/`edit`/`move`/`archive` entries
    /// (`popup::command::commands::categories`) against the tree's own selection, the same
    /// target its `n`/`e`/`m`/`a` keys already act on, since the command popup has no real
    /// typed-argument resolution to supply a `<cat>` from instead.
    fn category_selection(&self) -> Option<RowID> {
        None
    }

    /// Read-only access to this view's Account list, if it has one — `Some` only for
    /// `view::accounts::AccountsView`. Mirrors `category_store`: lets `Shell` resolve the
    /// Account popup's completion/validation against the live fixture without downcasting the
    /// `Box<dyn View>` trait object. Mutation still goes through `View::update`
    /// (`Action::CreateAccount`), never through this.
    fn account_store(&self) -> Option<&dyn AccountStore> {
        None
    }

    /// The currently-selected account, if this view is `AccountsView` and has one — lets
    /// `Shell` dispatch the `:account` command grammar's `edit`/`delete`/`off`/`on` entries
    /// (`popup::command::commands::accounts`) against the list's own selection, the same
    /// target its `e`/`d`/`a` keys already act on, since the command popup has no real
    /// typed-argument resolution to supply an `<acct>` from instead. Mirrors
    /// `category_selection`.
    fn account_selection(&self) -> Option<RowID> {
        None
    }

    /// Read-only access to this view's Tag list, if it has one — `Some` only for
    /// `view::tags::TagsView`. Mirrors `account_store`/`category_store`: lets `Shell` resolve
    /// the Tag popups' uniqueness validation against the live fixture without downcasting the
    /// `Box<dyn View>` trait object.
    fn tag_store(&self) -> Option<&dyn TagStore> {
        None
    }

    /// The currently-selected Tag, if this view is `TagsView` and has one — lets `Shell`
    /// dispatch the `:tag` command grammar's `edit`/`off`/`on`/`delete` entries
    /// (`popup::command::commands::tags`) against the list's own selection, the same target
    /// its `e`/`d` keys already act on, since the command popup has no real typed-argument
    /// resolution to supply a `<tag>` from instead. Mirrors `account_selection`.
    fn tag_selection(&self) -> Option<RowID> {
        None
    }

    /// Read-only access to this view's Payee list, if it has one — `Some` only for
    /// `view::payees::PayeesView`. Mirrors `account_store`/`tag_store`: lets `Shell` resolve
    /// the Payee new popup's name-collision check and `default` field completion against the
    /// live fixture without downcasting the `Box<dyn View>` trait object.
    fn payee_store(&self) -> Option<&dyn PayeeStore> {
        None
    }

    /// The currently-selected Payee, if this view is `PayeesView` and has one — lets `Shell`
    /// dispatch a future `:payee` command grammar entry against the list's own selection, the
    /// same target its own keys act on. Mirrors `account_selection`/`tag_selection`.
    fn payee_selection(&self) -> Option<RowID> {
        None
    }
}
