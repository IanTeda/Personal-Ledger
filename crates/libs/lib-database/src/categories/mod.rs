//! # Categories Database Module
//!
//! Provides data access helpers, builders, and models for working with
//! category records in the persistence layer. The module exposes the
//! database representation of a category alongside utilities for creating
//! and inserting records during tests or data seeding.
//!
//! ## Overview
//!
//! This module organises category-related database operations into logical submodules:
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`Categories`](Categories) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`CategoriesBuilder`](CategoriesBuilder) for constructing categories |
//! | [`insert`](insert) | Functions for inserting new category records |
//! | [`update`](update) | Functions for updating existing category records |
//! | [`delete`](delete) | Functions for deleting category records |
//! | [`find`](find) | Functions for querying and retrieving category records |
//!
//! ## Usage
//!
//! Use the builder pattern for creating categories in tests:
//!
//! ```rust,no_run
//! use lib_database::categories::{Categories, CategoriesBuilder};
//! use lib_domain::CategoryTypes;
//!
//! let category = CategoriesBuilder::new()
//!     .with_name("Groceries")
//!     .with_category_type(CategoryTypes::Expense)
//!     .with_code("FOOD.001")
//!     .build()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Perform database operations:
//!
//! ```rust,no_run
//! # use lib_database::{Categories, DatabasePool};
//! # async fn example(pool: &DatabasePool) -> Result<(), Box<dyn std::error::Error>> {
//! let inserted = category.insert(pool).await?;
//! let found = Categories::find_by_id(inserted.id, pool).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Security
//!
//! Error messages do not include sensitive information such as passwords or personal data.
//! Ensure that when logging errors, sensitive details are redacted.
//!
//! ## Performance
//!
//! Bulk operations (e.g., [`insert_many`](insert::Categories::insert_many), [`update_many`](update::Categories::update_many)) use transactions for atomicity.
//! Avoid creating categories in hot paths; use builders for test data only.

#![allow(unused)] // For development only

mod builder;
mod delete;
mod find;
mod insert;
mod model;
mod update;

/// Database row model representing a persisted category.
///
/// Represents a financial category with all database fields.
/// Includes validation and builder pattern support.
///
/// See the model module for implementation details.
pub use model::Categories;

/// Fluent builder for constructing `Category` instances in tests and fixtures.
///
/// Provides a type-safe way to build categories with required and optional fields.
/// Useful for tests, fixtures, and data seeding.
///
/// See the builder module for implementation details.
#[allow(unused)]
pub use builder::CategoriesBuilder;