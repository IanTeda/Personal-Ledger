//! # Category Builder
//!
//! Provides a fluent API for constructing [`Categories`] records. The builder enforces
//! the presence of mandatory fields while providing sensible defaults for optional values.
//! This is particularly useful for tests, fixtures, and data seeding utilities where
//! creating category rows should be ergonomic and explicit.

use super::Categories;
use crate::DatabaseError;

/// Fluent builder for [`Categories`] rows.
///
/// The builder collects optional pieces of data and ensures required values are
/// supplied before constructing a fully-fledged [`Categories`]. Where appropriate,
/// defaults are injected—such as marking the category as active or generating a
/// deterministic code derived from the persisted identifier.
#[derive(Debug, Default, Clone)]
pub struct CategoriesBuilder {
	id: Option<lib_domain::RowID>,
	code: Option<String>,
	name: Option<String>,
	description: Option<String>,
	url_slug: Option<lib_domain::UrlSlug>,
	category_type: Option<lib_domain::CategoryTypes>,
	color: Option<lib_domain::HexColor>,
	icon: Option<String>,
	is_active: Option<bool>,
	created_on: Option<chrono::DateTime<chrono::Utc>>,
	updated_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl CategoriesBuilder {
	/// Start building a new category with no preset values.
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	/// Use an existing [`RowID`] for the category.
	#[must_use]
	pub fn with_id(mut self, id: lib_domain::RowID) -> Self {
		self.id = Some(id);
		self
	}

	/// Provide an optional [`RowID`] for the category.
	#[must_use]
	pub fn with_id_opt(mut self, id: Option<lib_domain::RowID>) -> Self {
		self.id = id;
		self
	}

	/// Set the category code value.
	#[must_use]
	pub fn with_code(mut self, code: impl Into<String>) -> Self {
		self.code = Some(code.into());
		self
	}

	/// Provide an optional category code.
	#[must_use]
	pub fn with_code_opt<T: Into<String>>(mut self, code: Option<T>) -> Self {
		self.code = code.map(Into::into);
		self
	}

	/// Set the human-friendly category name.
	#[must_use]
	pub fn with_name(mut self, name: impl Into<String>) -> Self {
		self.name = Some(name.into());
		self
	}

	/// Provide an optional description.
	#[must_use]
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Set or clear the description.
	#[must_use]
	pub fn with_description_opt<T: Into<String>>(mut self, description: Option<T>) -> Self {
		self.description = description.map(Into::into);
		self
	}

	/// Use a pre-computed URL slug.
	#[must_use]
	pub fn with_url_slug(mut self, url_slug: lib_domain::UrlSlug) -> Self {
		self.url_slug = Some(url_slug);
		self
	}

	/// Provide an optional URL slug.
	#[must_use]
	pub fn with_url_slug_opt(mut self, url_slug: Option<lib_domain::UrlSlug>) -> Self {
		self.url_slug = url_slug;
		self
	}

	/// Assign the accounting category type.
	#[must_use]
	pub fn with_category_type(mut self, category_type: lib_domain::CategoryTypes) -> Self {
		self.category_type = Some(category_type);
		self
	}

	/// Set an optional colour.
	#[must_use]
	pub fn with_color(mut self, color: lib_domain::HexColor) -> Self {
		self.color = Some(color);
		self
	}

	/// Provide an optional colour value.
	#[must_use]
	pub fn with_color_opt(mut self, color: Option<lib_domain::HexColor>) -> Self {
		self.color = color;
		self
	}

	/// Set an icon identifier.
	#[must_use]
	pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
		self.icon = Some(icon.into());
		self
	}

	/// Provide an optional icon value.
	#[must_use]
	pub fn with_icon_opt<T: Into<String>>(mut self, icon: Option<T>) -> Self {
		self.icon = icon.map(Into::into);
		self
	}

	/// Specify whether the category is active.
	#[must_use]
	pub fn with_is_active(mut self, is_active: bool) -> Self {
		self.is_active = Some(is_active);
		self
	}

	/// Provide an optional active flag.
	#[must_use]
	pub fn with_is_active_opt(mut self, is_active: Option<bool>) -> Self {
		self.is_active = is_active;
		self
	}

	/// Set the creation timestamp.
	#[must_use]
	pub fn with_created_on(mut self, created_on: chrono::DateTime<chrono::Utc>) -> Self {
		self.created_on = Some(created_on);
		self
	}

	/// Provide an optional creation timestamp.
	#[must_use]
	pub fn with_created_on_opt(mut self, created_on: Option<chrono::DateTime<chrono::Utc>>) -> Self {
		self.created_on = created_on;
		self
	}

	/// Set the update timestamp.
	#[must_use]
	pub fn with_updated_on(mut self, updated_on: chrono::DateTime<chrono::Utc>) -> Self {
		self.updated_on = Some(updated_on);
		self
	}

	/// Provide an optional update timestamp.
	#[must_use]
	pub fn with_updated_on_opt(mut self, updated_on: Option<chrono::DateTime<chrono::Utc>>) -> Self {
		self.updated_on = updated_on;
		self
	}
	/// Build the [`Categories`], returning an error when required fields are missing.
	pub fn build(self) -> crate::DatabaseResult<Categories> {
		let name = self
			.name
			.ok_or(DatabaseError::CategoryBuilder("category name is required but was not set".to_string()))?;
		let category_type = self
			.category_type
			.ok_or(DatabaseError::CategoryBuilder("category type is required but was not set".to_string()))?;
		let code = self
			.code
			.ok_or(DatabaseError::CategoryBuilder("category code is required but was not set".to_string()))?;

	  let id = self.id.unwrap_or_default();
		let url_slug = self.url_slug;
		let now = chrono::Utc::now();

		Ok(Categories {
			id,
			code,
			name,
			description: self.description,
			url_slug,
			category_type,
			color: self.color,
			icon: self.icon,
			is_active: self.is_active.unwrap_or(true),
			created_on: self.created_on.unwrap_or(now),
			updated_on: self.updated_on.unwrap_or(now),
		})
	}

	/// Build the [`Categories`] with sensible defaults for all fields, useful for tests and fixtures.
	///
	/// This method provides defaults for required fields if not set, optimising for quick construction
	/// in non-production scenarios. It generates a new ID, uses a default code and name, and sets
	/// other fields to reasonable defaults.
	///
	/// # Examples
	/// ```
	/// use lib_database::categories::CategoriesBuilder;
	///
	/// let category = CategoriesBuilder::new().build_with_defaults();
	/// assert!(!category.name.is_empty());
	/// ```
	pub fn build_with_defaults(self) -> Categories {
		let name = self.name.unwrap_or_else(|| "Default Category".to_string());
		let category_type = self.category_type.unwrap_or(lib_domain::CategoryTypes::Expense);
		let code = self.code.unwrap_or_else(|| "DEF.001".to_string());

		let id = self.id.unwrap_or_default();
		let url_slug = self.url_slug;
		let now = chrono::Utc::now();

		Categories {
			id,
			code,
			name,
			description: self.description,
			url_slug,
			category_type,
			color: self.color,
			icon: self.icon,
			is_active: self.is_active.unwrap_or(true),
			created_on: self.created_on.unwrap_or(now),
			updated_on: self.updated_on.unwrap_or(now),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use lib_domain::{CategoryTypes, HexColor, UrlSlug};
	
	#[test]
	fn build_requires_name() {
		let result = CategoriesBuilder::new()
			.with_category_type(CategoryTypes::Expense)
			.build();
		assert_eq!(result.unwrap_err(), DatabaseError::CategoryBuilder("category name is required but was not set".to_string()));
	}

	#[test]
	fn build_requires_category_type() {
		let result = CategoriesBuilder::new().with_name("Travel").build();
		assert_eq!(
			result.unwrap_err(),
			DatabaseError::CategoryBuilder("category type is required but was not set".to_string())
		);
	}

	#[test]
	fn build_requires_code() {
		let result = CategoriesBuilder::new()
			.with_name("Travel")
			.with_category_type(CategoryTypes::Expense)
			.build();
		assert_eq!(result.unwrap_err(), DatabaseError::CategoryBuilder("category code is required but was not set".to_string()));
	}

	#[test]
	fn builder_provides_defaults() {
		let category = CategoriesBuilder::new()
			.with_name("Dining out")
			.with_category_type(CategoryTypes::Expense)
			.with_code("DIN.001")
			.build()
			.expect("build should succeed");

		assert_eq!(category.name, "Dining out");
		assert_eq!(category.code, "DIN.001");
		assert!(category.url_slug.is_none());
		assert!(category.is_active);
		assert!(category.created_on <= chrono::Utc::now());
		assert!(category.updated_on <= chrono::Utc::now());
	}

	#[test]
	fn builder_respects_optional_overrides() {
		let color = HexColor::parse("#123456").unwrap();
		let slug = UrlSlug::parse("custom-slug").unwrap();

		let category = CategoriesBuilder::new()
			.with_id(lib_domain::RowID::new())
			.with_name("Utilities")
			.with_category_type(CategoryTypes::Expense)
			.with_code("UTIL.001")
			.with_description("Household utilities")
			.with_url_slug(slug.clone())
			.with_color(color.clone())
			.with_icon("bolt")
			.with_is_active(false)
			.with_created_on(chrono::Utc::now())
			.with_updated_on(chrono::Utc::now())
			.build()
			.expect("build should succeed");

		assert_eq!(category.code, "UTIL.001");
		assert_eq!(category.description.as_deref(), Some("Household utilities"));
		assert_eq!(category.url_slug.as_ref(), Some(&slug));
		assert_eq!(category.color.as_ref(), Some(&color));
		assert_eq!(category.icon.as_deref(), Some("bolt"));
		assert!(!category.is_active);
	}

	#[test]
	fn optional_setters_clear_values() {
		let category = CategoriesBuilder::new()
			.with_name("Optional")
			.with_category_type(CategoryTypes::Income)
			.with_code("OPT.001")
			.with_description("temp")
			.with_description_opt::<String>(None)
			.with_icon("temp")
			.with_icon_opt::<String>(None)
			.with_color(HexColor::parse("#ABCDEF").unwrap())
			.with_color_opt(None)
			.with_url_slug(UrlSlug::parse("temp-slug").unwrap())
			.with_url_slug_opt(None)
			.with_is_active(false)
			.with_is_active_opt(None)
			.build()
			.expect("build should succeed");

		assert!(category.description.is_none());
		assert!(category.icon.is_none());
		assert!(category.color.is_none());
		assert!(category.url_slug.is_none()); // not generated from name
		assert!(category.is_active); // default restored
	}

	#[test]
	fn build_with_defaults_works() {
		let category = CategoriesBuilder::new().build_with_defaults();
		assert_eq!(category.name, "Default Category");
		assert_eq!(category.code, "DEF.001");
		assert_eq!(category.category_type, CategoryTypes::Expense);
		assert!(category.is_active);
	}

	#[test]
	fn with_id_opt_sets_none() {
		let category = CategoriesBuilder::new()
			.with_id(lib_domain::RowID::new())
			.with_id_opt(None)
			.with_name("Test")
			.with_category_type(CategoryTypes::Expense)
			.with_code("TEST.001")
			.build()
			.expect("build should succeed");

		assert_eq!(category.id, lib_domain::RowID::default());
	}
}
