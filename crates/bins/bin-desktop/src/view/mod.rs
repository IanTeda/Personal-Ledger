//! View interiors, one module per noun that has one built. Dashboard (issue #148), Settings
//! (issue #173), Accounts (issue #150), Categories (issue #272), Payees (issue #286), Tags (issue #357), Documents, Bills (issue #371) and Budgets (issue #402) exist so far -- the rest are
//! placeholder views, a separate ticket (#153).

pub mod accounts;
pub mod bills;
pub mod budgets;
pub mod categories;
pub mod dashboard;
pub mod documents;
pub mod event;
pub mod form_fields;
pub(crate) mod format;
pub mod help;
pub mod import;
pub mod institutions;
pub mod inventory;
pub mod payees;
pub mod settings;
pub mod tags;
pub mod transactions;
pub mod units;
