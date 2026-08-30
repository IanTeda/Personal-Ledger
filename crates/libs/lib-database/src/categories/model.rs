//! # Categories Database Model
//!
//! This module defines the `Categories` struct, which represents a database row
//! for financial transaction categories in the Personal Ledger application.
//!
//! Categories are used to classify and organise financial transactions into
//! meaningful groups such as "Groceries", "Rent", "Salary", etc. Each category
//! belongs to one of the fundamental accounting types (Asset, Liability, Income,
//! Expense, or Equity).
//!
//! ## Database Schema
//!
//! The `Categories` struct maps directly to the `categories` table with the following columns:
//! - `id`: Unique identifier using time-ordered UUID v7
//! - `code`: Three-part alphanumeric code (e.g., "FOO.BAR.BAZ")
//! - `name`: Human-readable category name
//! - `description`: Optional detailed description
//! - `url_slug`: URL-safe identifier for web interfaces
//! - `category_type`: Accounting classification (Asset/Liability/Income/Expense/Equity)
//! - `color`: Optional hex color for UI theming
//! - `icon`: Optional icon identifier for UI display
//! - `is_active`: Soft delete flag
//! - `created_on`: Record creation timestamp
//! - `updated_on`: Last modification timestamp
//!
//! ## Usage
//!
//! ```rust,no_run
//! use lib_database::Categories;
//!
//! // Create a category instance (typically from database query)
//! # // This would normally come from sqlx::FromRow
//! # let category = Categories {
//! #     id: lib_domain::RowID::new(),
//! #     code: "FOO.BAR.BAZ".to_string(),
//! #     name: "Example Category".to_string(),
//! #     description: Some("An example category".to_string()),
//! #     url_slug: Some(lib_domain::UrlSlug::parse("example-category").unwrap()),
//! #     category_type: lib_domain::CategoryTypes::Expense,
//! #     color: Some(lib_domain::HexColor::parse("#FF5733").unwrap()),
//! #     icon: Some("shopping-cart".to_string()),
//! #     is_active: true,
//! #     created_on: chrono::Utc::now(),
//! #     updated_on: chrono::Utc::now(),
//! # };
//!
//! // Access category properties
//! println!("Category: {}", category.name);
//! println!("Type: {}", category.category_type.as_str());
//! ```
//!
//! ## Testing
//!
//! The module includes comprehensive test utilities for generating mock data:
//!
//! ```rust
//! # #[cfg(test)]
//! # use lib_database::categories::Categories;
//! # #[cfg(test)]
//! # fn example() {
//! let mock_category = Categories::mock();
//! assert!(!mock_category.name.is_empty());
//! # }
//! ```

/// Database row model representing a persisted category.
///
/// `Categories` maps directly to the `categories` table and contains all
/// fields necessary to represent a financial transaction category. This struct
/// is primarily used for database operations and serialization.
///
/// ## Field Descriptions
///
/// - `id`: Unique time-ordered identifier for the category
/// - `code`: Structured alphanumeric code (format: XXX.XXX.XXX)
/// - `name`: Human-readable display name
/// - `description`: Optional detailed description of the category's purpose
/// - `url_slug`: URL-safe identifier for web interfaces and APIs
/// - `category_type`: Accounting classification (Asset/Liability/Income/Expense/Equity)
/// - `color`: Optional hex color code for UI theming and visualisation
/// - `icon`: Optional icon identifier for UI display
/// - `is_active`: Soft delete flag - false indicates the category is deactivated
/// - `created_on`: UTC timestamp when the category was first created
/// - `updated_on`: UTC timestamp when the category was last modified
#[derive(Debug, sqlx::FromRow, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct Categories {
    /// Unique time-ordered identifier for the category.
    ///
    /// Uses UUID v7 for chronological ordering and global uniqueness.
    /// This field is the primary key in the database.
    pub id: lib_domain::RowID,

    /// Structured alphanumeric code identifying the category.
    ///
    /// Format: XXX.XXX.XXX (three groups of three uppercase alphanumeric characters
    /// separated by dots). Provides a machine-readable identifier that is also
    /// human-readable and sortable.
    pub code: String,

    /// Human-readable display name for the category.
    ///
    /// Used in user interfaces and reports. Should be concise but descriptive
    /// (e.g., "Groceries", "Office Supplies", "Salary").
    pub name: String,

    /// Optional detailed description of the category's purpose.
    ///
    /// Provides additional context about when and how to use this category.
    /// Useful for complex categories that need explanation.
    pub description: Option<String>,

    /// URL-safe identifier for web interfaces and APIs.
    ///
    /// Automatically generated from the category name, replacing spaces and
    /// special characters with hyphens and converting to lowercase.
    /// Used for RESTful URLs and frontend routing.
    pub url_slug: Option<lib_domain::UrlSlug>,

    /// Accounting classification type.
    ///
    /// Determines how transactions in this category affect financial statements.
    /// Must be one of: Asset, Liability, Income, Expense, or Equity.
    pub category_type: lib_domain::CategoryTypes,

    /// Optional hex color code for UI theming and visualisation.
    ///
    /// Stored in canonical `#RRGGBB` format. Used by frontend applications
    /// to provide visual distinction between categories.
    pub color: Option<lib_domain::HexColor>,

    /// Optional icon identifier for UI display.
    ///
    /// References an icon in the application's icon library (e.g., "shopping-cart",
    /// "home", "briefcase"). Used for visual category recognition.
    pub icon: Option<String>,

    /// Soft delete flag indicating whether the category is active.
    ///
    /// When `false`, the category should not be used for new transactions
    /// but existing transactions remain valid. Defaults to `true` for new categories.
    pub is_active: bool,

    /// UTC timestamp when the category was first created.
    ///
    /// Automatically set by the database on INSERT operations.
    pub created_on: chrono::DateTime<chrono::Utc>,

    /// UTC timestamp when the category was last modified.
    ///
    /// Automatically updated by the database on UPDATE operations.
    pub updated_on: chrono::DateTime<chrono::Utc>,
}

/// Implementation of test utilities and helper methods for `Categories`.
///
/// This implementation provides methods for generating mock data during testing
/// and validation. All mock generation methods use the `fake` crate to create
/// realistic test data that follows the same constraints as production data.
impl Categories {
    /// Generates a mock `Categories` instance with randomised test data.
    ///
    /// This function creates realistic test data for categories, using the `fake` crate
    /// to randomise optional fields and text content. Useful for unit and integration tests.
    ///
    /// The generated category will have:
    /// - A random but valid RowID
    /// - A properly formatted code (XXX.XXX.XXX)
    /// - A realistic name and optional description
    /// - A valid category type
    /// - Randomly assigned optional fields (color, icon, etc.)
    /// - Current timestamps for created/updated
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(test)]
    /// # use lib_database::categories::Categories;
    /// # #[cfg(test)]
    /// # fn example() {
    /// let mock_category = Categories::mock();
    /// assert!(!mock_category.name.is_empty());
    /// assert!(mock_category.code.contains('.'));
    /// # }
    /// ```
    #[cfg(test)]
    pub fn mock() -> Self {
        use crate::categories::CategoriesBuilder;

        CategoriesBuilder::new()
            .with_id(lib_domain::RowID::mock())
            .with_code_opt(Some(Self::generate_mock_code()))
            .with_name(Self::generate_mock_name())
            .with_description_opt(Self::generate_mock_description())
            .with_url_slug_opt(Self::generate_mock_url_slug())
            .with_category_type(lib_domain::CategoryTypes::mock())
            .with_color_opt(lib_domain::HexColor::mock_with_option())
            .with_icon_opt(Self::generate_mock_icon())
            .with_is_active_opt(Some(Self::generate_mock_is_active()))
            .with_created_on_opt(Some(chrono::Utc::now()))
            .with_updated_on_opt(Some(chrono::Utc::now()))
            .build()
            .expect("Mock category should always build successfully")
    }

    /// Generates a mock category code in the required XXX.XXX.XXX format.
    ///
    /// Creates a structured alphanumeric code with three groups of three
    /// uppercase characters separated by dots. This matches the database
    /// schema requirements for category codes.
    ///
    /// # Returns
    ///
    /// A string in the format "ABC.DEF.GHI" where each group contains
    /// random uppercase alphanumeric characters.
    #[cfg(test)]
    fn generate_mock_code() -> String {
        use fake::rand::Rng;

        // Generate 9 random alphanumeric chars, uppercase, then split into 3 groups
        let mut rng = fake::rand::rng();
        let s: String = (&mut rng)
            .sample_iter(&fake::rand::distr::Alphanumeric)
            .take(9)
            .map(|b| (b as char).to_ascii_uppercase())
            .collect();

        format!("{}.{}.{}", &s[0..3], &s[3..6], &s[6..9])
    }

    /// Generates a mock category name using lorem ipsum words.
    ///
    /// Creates realistic category names by combining 1-2 random words.
    /// Examples: "Food", "Office Supplies", "Travel Expenses".
    ///
    /// # Returns
    ///
    /// A string containing 1-2 space-separated words suitable for a category name.
    #[cfg(test)]
    fn generate_mock_name() -> String {
        use fake::Fake;
        use fake::faker::lorem::en::Words;

        let words: Vec<String> = Words(1..3).fake();
        words.join(" ")
    }

    /// Generates a mock category description with 50% probability.
    ///
    /// Randomly decides whether to generate a description, and if so,
    /// creates a realistic description using lorem ipsum text.
    ///
    /// # Returns
    ///
    /// - `Some(String)` containing 3-8 words of lorem ipsum text (50% chance)
    /// - `None` (50% chance)
    #[cfg(test)]
    fn generate_mock_description() -> Option<String> {
        use fake::Fake;
        use fake::faker::boolean::en::Boolean;
        use fake::faker::lorem::en::Words;

        let is_some: bool = Boolean(50).fake(); // 50% chance of Some
        if is_some {
            let words: Vec<String> = Words(3..8).fake();
            Some(words.join(" "))
        } else {
            None
        }
    }

    /// Generates a mock URL slug from a mock category name.
    ///
    /// Creates a URL-safe slug by processing the result of `generate_mock_name()`
    /// through the `UrlSlug` constructor, which handles normalisation and
    /// special character replacement.
    ///
    /// # Returns
    ///
    /// `Some(UrlSlug)` containing the slugged version of a generated name.
    /// Will always return `Some` since `UrlSlug::from` should succeed for
    /// generated lorem ipsum text.
    #[cfg(test)]
    fn generate_mock_url_slug() -> Option<lib_domain::UrlSlug> {
        Some(lib_domain::UrlSlug::from(Self::generate_mock_name()))
    }

    /// Generates a mock icon identifier with 50% probability.
    ///
    /// Randomly decides whether to generate an icon name, and if so,
    /// creates a single word suitable for an icon identifier.
    ///
    /// # Returns
    ///
    /// - `Some(String)` containing a single word (50% chance)
    /// - `None` (50% chance)
    #[cfg(test)]
    fn generate_mock_icon() -> Option<String> {
        use fake::Fake;
        use fake::faker::boolean::en::Boolean;
        use fake::faker::lorem::en::Word;

        let is_some: bool = Boolean(50).fake(); // 50% chance of Some
        if is_some {
            Some(Word().fake())
        } else {
            None
        }
    }

    /// Generates a mock active status with 80% probability of being active.
    ///
    /// Creates a boolean value that favours active categories, which is
    /// more realistic for test data since most categories should be active.
    ///
    /// # Returns
    ///
    /// `true` (80% chance) or `false` (20% chance)
    #[cfg(test)]
    fn generate_mock_is_active() -> bool {
        use fake::Fake;
        use fake::faker::boolean::en::Boolean;

        Boolean(80).fake() // 80% chance of active for more realistic data
    }
}

/// Test module for `Categories` model functionality.
///
/// This module contains comprehensive tests for the `Categories` struct,
/// focusing on mock data generation, validation, and serialization behaviors.
/// Tests ensure that mock data is realistic and that the struct behaves
/// correctly with its derives (Debug, Clone, PartialEq, Serialize, Deserialize).
#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that `Categories::mock()` generates valid category instances.
    ///
    /// Verifies that all required fields are populated with appropriate values
    /// and that generated data meets basic validation criteria.
    #[test]
    fn mock_generates_valid_category() {
        let cat = Categories::mock();
        assert!(!cat.name.is_empty());
        assert!(!cat.code.is_empty());
        assert!(cat.code.contains('.'));
        assert!(cat.url_slug.is_some());
        assert!(cat.created_on <= chrono::Utc::now());
        assert!(cat.updated_on <= chrono::Utc::now());
    }

    /// Tests that mock data generation properly randomises optional fields.
    ///
    /// Runs multiple mock generations to ensure that optional fields like
    /// description, icon, and is_active are properly randomised rather than
    /// always having the same values.
    #[test]
    fn mock_randomises_optional_fields() {
        // Run multiple times to check randomisation
        let mut has_some_description = false;
        let mut has_none_description = false;
        let mut has_some_icon = false;
        let mut has_none_icon = false;
        let mut has_inactive = false;

        for _ in 0..50 {
            let cat = Categories::mock();
            if cat.description.is_some() {
                has_some_description = true;
            } else {
                has_none_description = true;
            }
            if cat.icon.is_some() {
                has_some_icon = true;
            } else {
                has_none_icon = true;
            }
            if !cat.is_active {
                has_inactive = true;
            }
        }

        assert!(has_some_description && has_none_description, "Description should randomise");
        assert!(has_some_icon && has_none_icon, "Icon should randomise");
        assert!(has_inactive, "is_active should sometimes be false");
    }

    /// Tests that `generate_mock_code()` produces codes in the correct format.
    ///
    /// Verifies that generated codes follow the XXX.XXX.XXX pattern with
    /// proper length and structure.
    #[test]
    fn generate_mock_code_produces_valid_format() {
        let code = Categories::generate_mock_code();
        assert_eq!(code.len(), 11); // XXX.XXX.XXX
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric() || c == '.'));
        let parts: Vec<&str> = code.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].len(), 3);
        assert_eq!(parts[1].len(), 3);
        assert_eq!(parts[2].len(), 3);
    }

    /// Tests that `generate_mock_name()` produces valid category names.
    ///
    /// Verifies that generated names are non-empty and contain only
    /// alphabetic characters and whitespace.
    #[test]
    fn generate_mock_name_produces_non_empty_string() {
        let name = Categories::generate_mock_name();
        assert!(!name.is_empty());
        assert!(name.chars().all(|c| c.is_alphabetic() || c.is_whitespace()));
    }

    /// Tests that `generate_mock_description()` properly randomises output.
    ///
    /// Verifies that the function sometimes returns `Some` and sometimes `None`,
    /// and that `Some` values are non-empty strings.
    #[test]
    fn generate_mock_description_randomises() {
        let mut has_some = false;
        let mut has_none = false;
        for _ in 0..20 {
            let desc = Categories::generate_mock_description();
            if desc.is_some() {
                has_some = true;
                assert!(!desc.as_ref().unwrap().is_empty());
            } else {
                has_none = true;
            }
        }
        assert!(has_some && has_none);
    }

    /// Tests that `generate_mock_url_slug()` creates valid URL slugs.
    ///
    /// Verifies that the function returns a `UrlSlug` and that the slug
    /// is not empty after processing.
    #[test]
    fn generate_mock_url_slug_uses_name() {
        let slug = Categories::generate_mock_url_slug();
        assert!(slug.is_some());
        // Since UrlSlug::from handles parsing, just check it's not empty
        assert!(!slug.as_ref().unwrap().as_str().is_empty());
    }

    /// Tests that `generate_mock_icon()` properly randomises output.
    ///
    /// Verifies that the function sometimes returns `Some` and sometimes `None`,
    /// and that `Some` values are non-empty alphabetic strings.
    #[test]
    fn generate_mock_icon_randomises() {
        let mut has_some = false;
        let mut has_none = false;
        for _ in 0..20 {
            let icon = Categories::generate_mock_icon();
            if icon.is_some() {
                has_some = true;
                assert!(!icon.as_ref().unwrap().is_empty());
                assert!(icon.as_ref().unwrap().chars().all(|c| c.is_alphabetic()));
            } else {
                has_none = true;
            }
        }
        assert!(has_some && has_none);
    }

    /// Tests that `generate_mock_is_active()` properly randomises output.
    ///
    /// Verifies that the function returns both `true` and `false` values
    /// over multiple calls.
    #[test]
    fn generate_mock_is_active_randomises() {
        let mut has_true = false;
        let mut has_false = false;
        for _ in 0..20 {
            let active = Categories::generate_mock_is_active();
            if active {
                has_true = true;
            } else {
                has_false = true;
            }
        }
        assert!(has_true && has_false);
    }

    /// Tests that the `Categories` struct works correctly with its derives.
    ///
    /// Verifies that Debug, Clone, PartialEq, Serialize, and Deserialize
    /// traits work as expected for category instances.
    #[test]
    fn category_struct_derives_work() {
        let cat1 = Categories::mock();
        let cat2 = cat1.clone();
        assert_eq!(cat1, cat2);

        // Test Debug (implicitly by using in assert)
        let debug_str = format!("{:?}", cat1);
        assert!(debug_str.contains("Categories"));

        // Test Serialize/Deserialize
        let json = serde_json::to_string(&cat1).unwrap();
        let deserialized: Categories = serde_json::from_str(&json).unwrap();
        assert_eq!(cat1, deserialized);
    }
}