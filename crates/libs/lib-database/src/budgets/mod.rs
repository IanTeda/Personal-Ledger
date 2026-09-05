//! # Budgets Database Module
//!
//! Data access for the `budgets` table — a limit on the total amount of Transactions in one
//! (Expense-type) Category over a recurring period (FR.22-27, CC-TUI-010), line-item only.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`Budgets`](Budgets) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`BudgetsBuilder`](BudgetsBuilder) |
//! | [`insert`](insert) | Insert a new Budget (Expense-Category-only, enforced here) |
//! | [`find`](find) | Look up Budgets by id, list with pagination |
//! | [`update`](update) | Update a Budget's limit/period/active flag |
//! | [`delete`](delete) | Delete a Budget |
//! | [`progress`](progress) | Compute spend-so-far vs. limit for the current period |

mod builder;
mod delete;
mod find;
mod insert;
mod model;
mod progress;
mod update;

/// Database row model representing a persisted Budget.
pub use model::Budgets;

/// Fluent builder for constructing [`Budgets`] instances.
pub use builder::BudgetsBuilder;

/// How a Budget is tracking against its current period.
pub use progress::BudgetProgress;
