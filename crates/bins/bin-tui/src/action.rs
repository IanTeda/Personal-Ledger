//! Actions the application can perform.
//!
//! An [`Action`] is the single message type routed through [`App::update`](crate::app::App::update)
//! — the hybrid Elm/Component architecture locked in by ADR-0003
//! (`docs/adr/0003-hybrid-tea-component-tui-architecture.md`). Input events and background
//! tasks both produce `Action`s onto the same channel rather than mutating state directly.

/// Whether the active screen currently wants raw keystrokes treated as global single-letter
/// shortcuts (`Navigation`) or as literal text entry (`Editing`) — the mode split "Decide
/// keybinding and navigation/workflow scheme" locked in, so a free-text field (Payee,
/// description, ...) never fires a shortcut just because it shares a letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// List navigation and single-letter shortcuts (`n`, `d`, `/`, `s`, ...) are live.
    #[default]
    Navigation,
    /// A text field has focus; every keystroke is literal input except `Esc`/`Enter`/`Tab`.
    #[allow(dead_code)] // No screen has a text field yet; the first one (issue #67+) will.
    Editing,
}

/// A message the application reacts to, however it originated (keyboard input, a periodic
/// tick, or a background task reporting a result).
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// A periodic tick, driving redraws even without input (e.g. for future animated charts).
    Tick,
    /// `Ctrl+C` from anywhere — the hard-quit safety net, always available regardless of
    /// which screen is active or what `InputMode` it's in.
    Quit,
    /// `Esc` — pops one level of the navigation stack; only becomes [`Action::Quit`]-like
    /// (see `App::update`) when there's nothing left to pop (the dashboard is the base).
    Back,
    /// Push the Settings drill-in screen.
    OpenSettings,
    /// Push the Help screen, listing global keys plus the active screen's own.
    OpenHelp,
    /// Push the Units list screen.
    OpenUnits,
    /// Push the Units create/edit screen — `None` starts an empty (create) form, `Some`
    /// pre-fills it with an existing Unit's fields (edit).
    OpenUnitDetail(Option<lib_database::Units>),
    /// Push the Categories list screen.
    OpenCategories,
    /// Push the Categories create/edit screen — `None` starts an empty (create) form, `Some`
    /// pre-fills it with an existing Category's fields (edit).
    OpenCategoryDetail(Option<lib_database::Categories>),
    /// Push the Accounts list screen.
    OpenAccounts,
    /// Push the Accounts create/edit screen — `None` starts an empty (create) form, `Some`
    /// pre-fills it with an existing Account's fields (edit).
    OpenAccountDetail(Option<lib_database::Accounts>),
    /// Push the Transactions list screen.
    OpenTransactions,
    /// Push the Transactions create/edit screen — `None` starts an empty (create) form,
    /// `Some` pre-fills it with an existing Transaction's fields (edit).
    OpenTransactionDetail(Option<lib_database::Transactions>),
    /// Push the Payees list screen.
    OpenPayees,
    /// Push the Payees create/rename screen — `None` starts an empty (create) form, `Some`
    /// pre-fills it with an existing Payee's fields (rename).
    OpenPayeeDetail(Option<lib_database::Payees>),
    /// Push the Balance Checks list screen.
    OpenBalanceChecks,
    /// Push the Balance Checks create/edit screen — `None` starts an empty (create) form,
    /// `Some` pre-fills it with an existing Balance Check's fields (edit).
    OpenBalanceCheckDetail(Option<lib_database::BalanceChecks>),
    /// Push the Budgets list screen.
    OpenBudgets,
    /// Push the Budgets create/edit screen — `None` starts an empty (create) form, `Some`
    /// pre-fills it with an existing Budget's fields (edit).
    OpenBudgetDetail(Option<lib_database::Budgets>),
    /// A key was consumed by the active screen (e.g. appended to a text field) with nothing
    /// further for `App` to do — distinct from returning `None`, which would let the key
    /// fall through to the global `Esc`/`?` bindings.
    NoOp,
    /// The Categories list screen finished loading every Category (originally the
    /// feasibility-cycle FC-TUI-005 demo's action; reused as-is for the real screen).
    CategoriesLoaded(Vec<lib_database::Categories>),
    /// The Categories list screen failed to load Categories.
    CategoriesLoadFailed(String),
    /// The Categories detail screen successfully saved (inserted or updated) a Category.
    CategorySaved(lib_database::Categories),
    /// The Categories detail screen failed to save a Category.
    CategorySaveFailed(String),
    /// The Categories list screen successfully deleted a Category.
    CategoryDeleted(lib_core::RowID),
    /// The Categories list screen failed to delete a Category.
    CategoryDeleteFailed(String),
    /// The Units list screen finished loading every Unit.
    UnitsLoaded(Vec<lib_database::Units>),
    /// The Units list screen failed to load Units.
    UnitsLoadFailed(String),
    /// The Units detail screen successfully saved (inserted or updated) a Unit.
    UnitSaved(lib_database::Units),
    /// The Units detail screen failed to save a Unit.
    UnitSaveFailed(String),
    /// The Units list screen successfully deleted a Unit.
    UnitDeleted(lib_core::RowID),
    /// The Units list screen failed to delete a Unit.
    UnitDeleteFailed(String),
    /// The Accounts list screen finished loading every Account.
    AccountsLoaded(Vec<lib_database::Accounts>),
    /// The Accounts list screen failed to load Accounts.
    AccountsLoadFailed(String),
    /// The Accounts detail screen successfully saved (inserted or updated) an Account.
    AccountSaved(lib_database::Accounts),
    /// The Accounts detail screen failed to save an Account.
    AccountSaveFailed(String),
    /// The Accounts list screen successfully deleted an Account.
    AccountDeleted(lib_core::RowID),
    /// The Accounts list screen failed to delete an Account.
    AccountDeleteFailed(String),
    /// The Transactions list screen finished loading every Transaction.
    TransactionsLoaded(Vec<lib_database::Transactions>),
    /// The Transactions list screen failed to load Transactions.
    TransactionsLoadFailed(String),
    /// The Transactions detail screen successfully saved (inserted, or updated one or more
    /// of its fields) a Transaction.
    TransactionSaved(lib_database::Transactions),
    /// The Transactions detail screen failed to save a Transaction.
    TransactionSaveFailed(String),
    /// The Transactions list screen successfully deleted a Transaction.
    TransactionDeleted(lib_core::RowID),
    /// The Transactions list screen failed to delete a Transaction.
    TransactionDeleteFailed(String),
    /// The Payees list screen, or the Transactions detail screen's suggestion picker,
    /// finished loading every active Payee.
    PayeesLoaded(Vec<lib_database::Payees>),
    /// A Payee load failed.
    PayeesLoadFailed(String),
    /// The Payees detail screen successfully saved (inserted, or renamed) a Payee.
    PayeeSaved(lib_database::Payees),
    /// The Payees detail screen failed to save a Payee.
    PayeeSaveFailed(String),
    /// The Payees list screen successfully deleted a Payee.
    PayeeDeleted(lib_core::RowID),
    /// The Payees list screen failed to delete a Payee.
    PayeeDeleteFailed(String),
    /// The Transactions detail screen's suggestion picker finished loading every Payee
    /// Alias, used to resolve a renamed Payee from its old, typed name.
    PayeeAliasesLoaded(Vec<lib_database::PayeeAliases>),
    /// A Payee Alias load failed.
    PayeeAliasesLoadFailed(String),
    /// The Balance Checks list screen finished loading every Balance Check.
    BalanceChecksLoaded(Vec<lib_database::BalanceChecks>),
    /// A Balance Check load failed.
    BalanceChecksLoadFailed(String),
    /// The Balance Checks detail screen successfully saved (inserted or updated) a Balance
    /// Check.
    BalanceCheckSaved(lib_database::BalanceChecks),
    /// The Balance Checks detail screen failed to save a Balance Check.
    BalanceCheckSaveFailed(String),
    /// The Balance Checks list screen successfully deleted a Balance Check.
    BalanceCheckDeleted(lib_core::RowID),
    /// The Balance Checks list screen failed to delete a Balance Check.
    BalanceCheckDeleteFailed(String),
    /// The Budgets list screen finished loading every active Budget.
    BudgetsLoaded(Vec<lib_database::Budgets>),
    /// A Budget load failed.
    BudgetsLoadFailed(String),
    /// The Budgets list screen finished computing spend-so-far-vs-limit progress for one or
    /// more Budgets (the initial load computes it for every Budget; a save recomputes it for
    /// just the affected one).
    BudgetProgressLoaded(Vec<(lib_core::RowID, lib_database::BudgetProgress)>),
    /// A Budget progress computation failed — non-fatal, the list still shows Budgets
    /// without a progress bar.
    BudgetProgressLoadFailed(String),
    /// The Budgets detail screen successfully saved (inserted or updated) a Budget.
    BudgetSaved(lib_database::Budgets),
    /// The Budgets detail screen failed to save a Budget.
    BudgetSaveFailed(String),
    /// The Budgets list screen successfully deleted a Budget.
    BudgetDeleted(lib_core::RowID),
    /// The Budgets list screen failed to delete a Budget.
    BudgetDeleteFailed(String),
}
