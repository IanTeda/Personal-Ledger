//! # URL Slug Domain Type
//!
//! This module defines the `UrlSlug` domain type for representing URL-safe identifiers.
//! URL slugs are used in web applications to create human-readable, SEO-friendly URLs
//! from titles and names by converting them to lowercase, alphanumeric strings with hyphens.
//!
//! ## Features
//!
//! - **URL Safety**: Ensures slugs contain only lowercase letters, numbers, and hyphens
//! - **Automatic Cleaning**: Parses strings into valid slugs by removing special characters
//! - **Validation**: Prevents invalid characters and formats
//! - **Type Safety**: Prevents mixing slugs with regular strings
//! - **SEO Friendly**: Creates readable, search-engine optimized identifiers
//!
//! ## Example Usage
//!
//! ```rust
//! use lib_core::UrlSlug;
//!
//! // Parse a title into a slug
//! let slug = UrlSlug::parse("Hello World! How are you?")?;
//! assert_eq!(slug.as_str(), "hello-world-how-are-you");
//! # Ok::<(), lib_core::UrlSlugError>(())
//! ```

use std::fmt;

/// Represents a URL-safe slug string.
///
/// A slug is a string that contains only lowercase letters, numbers, and hyphens,
/// making it safe for use in URLs. Slugs are typically created from human-readable
/// titles by removing special characters and replacing spaces with hyphens.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct UrlSlug(String);

/// Errors that can occur when working with URL slugs.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum UrlSlugError {
    /// The slug contains invalid characters (only lowercase letters, numbers, and hyphens allowed).
    #[error("Invalid slug characters: {0}")]
    InvalidCharacters(String),

    /// The slug is empty.
    #[error("Slug cannot be empty")]
    EmptySlug,

    /// The slug starts or ends with a hyphen.
    #[error("Slug cannot start or end with hyphen: {0}")]
    StartsOrEndsWithHyphen(String),
    
    /// The slug contains consecutive hyphens.
    #[error("Slug cannot contain consecutive hyphens: {0}")]
    ConsecutiveHyphens(String),
}

impl UrlSlug {
    /// Parse a string into a URL-safe slug.
    ///
    /// This function performs the following transformations:
    /// - Converts to lowercase
    /// - Replaces spaces and special characters with hyphens
    /// - Removes consecutive hyphens
    /// - Trims leading/trailing hyphens
    /// - Validates the final result
    ///
    /// # Errors
    ///
    /// Returns a `UrlSlugError` if the input cannot be converted to a valid slug.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_core::UrlSlug;
    ///
    /// let slug = UrlSlug::parse("Hello World! How are you?")?;
    /// assert_eq!(slug.as_str(), "hello-world-how-are-you");
    ///
    /// let slug = UrlSlug::parse("valid-slug-123")?;
    /// assert_eq!(slug.as_str(), "valid-slug-123");
    /// # Ok::<(), lib_core::UrlSlugError>(())
    /// ```
    ///
    /// This replaces the previous `new` function and is the primary constructor for `UrlSlug`.
    pub fn parse<S: Into<String>>(s: S) -> Result<Self, UrlSlugError> {
        let s = s.into();
        let cleaned = Self::clean_string(&s);

        if cleaned.is_empty() {
            return Err(UrlSlugError::EmptySlug);
        }

        Ok(UrlSlug(cleaned))
    }

    /// Get the slug as a string slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_core::UrlSlug;
    ///
    /// let slug = UrlSlug::parse("test-slug")?;
    /// assert_eq!(slug.as_str(), "test-slug");
    /// # Ok::<(), lib_core::UrlSlugError>(())
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert the slug into its underlying string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_core::UrlSlug;
    ///
    /// let slug = UrlSlug::parse("test-slug")?;
    /// let string = slug.into_string();
    /// assert_eq!(string, "test-slug");
    /// # Ok::<(), lib_core::UrlSlugError>(())
    /// ```
    pub fn into_string(self) -> String {
        self.0
    }

    /// Check if the slug is empty.
    ///
    /// Note: Empty slugs are not allowed, so this should always return false
    /// for valid UrlSlug instances.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the length of the slug in characters.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Clean a string to make it URL-safe.
    ///
    /// This is the internal cleaning logic used by `parse()`.
    fn clean_string(s: &str) -> String {
        s.chars()
            // Convert to lowercase
            .map(|c| c.to_lowercase().collect::<String>())
            .collect::<String>()
            // Replace spaces and underscores with hyphens
            .replace([' ', '_'], "-")
            // Keep only ASCII alphanumeric characters and hyphens
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect::<String>()
            // Remove leading/trailing hyphens
            .trim_matches('-')
            // Collapse consecutive hyphens
            .split('-')
            .filter(|part| !part.is_empty())
            .collect::<Vec<&str>>()
            .join("-")
    }

    /// Validate that a string is a valid slug.
    #[allow(dead_code)]
    fn validate_slug(s: &str) -> Result<(), UrlSlugError> {
        if s.is_empty() {
            return Err(UrlSlugError::EmptySlug);
        }

        if s.starts_with('-') || s.ends_with('-') {
            return Err(UrlSlugError::StartsOrEndsWithHyphen(s.to_string()));
        }

        if s.contains("--") {
            return Err(UrlSlugError::ConsecutiveHyphens(s.to_string()));
        }

        // Check for invalid characters
        for c in s.chars() {
            if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
                return Err(UrlSlugError::InvalidCharacters(s.to_string()));
            }
        }

        Ok(())
    }
}

impl std::str::FromStr for UrlSlug {
    type Err = UrlSlugError;

    /// Parse a string into a UrlSlug using the same logic as `parse()`.
    ///
    /// This allows using `"string".parse::<UrlSlug>()` syntax.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        UrlSlug::parse(s)
    }
}

impl fmt::Display for UrlSlug {
    /// Format the slug as a string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for UrlSlug {
    /// Get the slug as a string slice.
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<UrlSlug> for String {
    /// Convert a UrlSlug into a String.
    fn from(slug: UrlSlug) -> Self {
        slug.0
    }
}

impl From<String> for UrlSlug {
    /// Convert a String into a UrlSlug by parsing it.
    fn from(s: String) -> Self {
        UrlSlug::parse(s).expect("Failed to parse string into UrlSlug")
    }
}

impl From<&str> for UrlSlug {
    /// Convert a &str into a UrlSlug by parsing it.
    fn from(s: &str) -> Self {
        UrlSlug::parse(s).expect("Failed to parse string into UrlSlug")
    }
}

// SQLx trait implementations for SQLite
impl sqlx::Type<sqlx::Sqlite> for UrlSlug {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for UrlSlug {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(UrlSlug::parse(s).map_err(|e| format!("Invalid slug in database: {}", e))?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for UrlSlug {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(self.0.clone(), buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let slug = UrlSlug::parse("Hello World").unwrap();
        assert_eq!(slug.as_str(), "hello-world");
    }

    #[test]
    fn test_parse_special_characters() {
        let slug = UrlSlug::parse("C++ Programming & Web Dev!").unwrap();
        assert_eq!(slug.as_str(), "c-programming-web-dev");
    }

    #[test]
    fn test_parse_underscores() {
        let slug = UrlSlug::parse("snake_case_string").unwrap();
        assert_eq!(slug.as_str(), "snake-case-string");
    }

    #[test]
    fn test_parse_uppercase() {
        let slug = UrlSlug::parse("UPPERCASE TITLE").unwrap();
        assert_eq!(slug.as_str(), "uppercase-title");
    }

    #[test]
    fn test_parse_numbers() {
        let slug = UrlSlug::parse("Test 123 Numbers").unwrap();
        assert_eq!(slug.as_str(), "test-123-numbers");
    }

    #[test]
    fn test_parse_leading_trailing_spaces() {
        let slug = UrlSlug::parse("  spaced title  ").unwrap();
        assert_eq!(slug.as_str(), "spaced-title");
    }

    #[test]
    fn test_parse_consecutive_spaces() {
        let slug = UrlSlug::parse("multiple   spaces").unwrap();
        assert_eq!(slug.as_str(), "multiple-spaces");
    }

    #[test]
    fn test_parse_only_special_chars() {
        let result = UrlSlug::parse("!!!@@@###");
        assert!(matches!(result, Err(UrlSlugError::EmptySlug)));
    }

    #[test]
    fn test_parse_empty_string() {
        let result = UrlSlug::parse("");
        assert!(matches!(result, Err(UrlSlugError::EmptySlug)));
    }

    #[test]
    fn test_parse_only_spaces() {
        let result = UrlSlug::parse("   ");
        assert!(matches!(result, Err(UrlSlugError::EmptySlug)));
    }

    #[test]
    fn test_from_str_trait() {
        let slug: UrlSlug = "Test String".parse().unwrap();
        assert_eq!(slug.as_str(), "test-string");
    }

    #[test]
    fn test_display() {
        let slug = UrlSlug::parse("test-slug").unwrap();
        assert_eq!(format!("{}", slug), "test-slug");
    }

    #[test]
    fn test_as_ref() {
        let slug = UrlSlug::parse("test-slug").unwrap();
        let s: &str = slug.as_ref();
        assert_eq!(s, "test-slug");
    }

    #[test]
    fn test_into_string() {
        let slug = UrlSlug::parse("test-slug").unwrap();
        let s: String = slug.into_string();
        assert_eq!(s, "test-slug");
    }

    #[test]
    fn test_from_into_string() {
        let original = "test-slug".to_string();
        let slug = UrlSlug::parse(&original).unwrap();
        let result: String = slug.into();
        assert_eq!(result, original);
    }

    #[test]
    fn test_equality() {
        let slug1 = UrlSlug::parse("test-slug").unwrap();
        let slug2 = UrlSlug::parse("test-slug").unwrap();
        let slug3 = UrlSlug::parse("different-slug").unwrap();

        assert_eq!(slug1, slug2);
        assert_ne!(slug1, slug3);
    }

    #[test]
    fn test_clone() {
        let slug1 = UrlSlug::parse("test-slug").unwrap();
        let slug2 = slug1.clone();
        assert_eq!(slug1, slug2);
    }

    #[test]
    fn test_serialization() {
        let slug = UrlSlug::parse("test-slug").unwrap();
        let serialized = serde_json::to_string(&slug).unwrap();
        assert_eq!(serialized, "\"test-slug\"");

        let deserialized: UrlSlug = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, slug);
    }

    #[test]
    fn test_complex_parsing() {
        use fake::Fake;
        use fake::faker::lorem::en::Sentence;

        // Test with Fake-generated sentences
        for _ in 0..20 {
            let sentence: String = Sentence(3..10).fake();
            let slug = UrlSlug::parse(&sentence).unwrap();
            
            // Verify the slug is valid
            assert!(!slug.is_empty());
            
            // Verify it contains only valid characters
            for c in slug.as_str().chars() {
                assert!(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            }
        }

        // Test with some hardcoded cases for specific transformations
        let test_cases = vec![
            ("Hello World!", "hello-world"),
            ("C++ & Rust Programming", "c-rust-programming"),
            ("Multiple   Spaces", "multiple-spaces"),
            ("123 Numbers & Symbols!", "123-numbers-symbols"),
            ("_underscores_and-hyphens-", "underscores-and-hyphens"),
            ("Café résumé naïve", "caf-rsum-nave"), // Unicode characters removed
        ];

        for (input, expected) in test_cases {
            let slug = UrlSlug::parse(input).unwrap();
            assert_eq!(slug.as_str(), expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_edge_cases() {
        use fake::Fake;
        use fake::faker::lorem::en::Word;

        // Test with Fake-generated words
        for _ in 0..10 {
            let word: String = Word().fake();
            let slug = UrlSlug::parse(&word).unwrap();
            
            // Single words should become lowercase
            assert_eq!(slug.as_str(), word.to_lowercase());
        }

        // Specific edge cases
        // Single character
        let slug = UrlSlug::parse("a").unwrap();
        assert_eq!(slug.as_str(), "a");

        // Only numbers
        let slug = UrlSlug::parse("123").unwrap();
        assert_eq!(slug.as_str(), "123");

        // Mixed alphanumeric
        let slug = UrlSlug::parse("abc123def").unwrap();
        assert_eq!(slug.as_str(), "abc123def");

        // Already valid slug
        let slug = UrlSlug::parse("already-valid-slug").unwrap();
        assert_eq!(slug.as_str(), "already-valid-slug");
    }

    #[test]
    fn test_is_empty_and_len() {
        let slug = UrlSlug::parse("test-slug").unwrap();
        assert!(!slug.is_empty());
        assert_eq!(slug.len(), 9); // "test-slug" is 9 characters

        let short_slug = UrlSlug::parse("a").unwrap();
        assert!(!short_slug.is_empty());
        assert_eq!(short_slug.len(), 1);

        let long_slug = UrlSlug::parse("very-long-slug-with-many-characters").unwrap();
        assert!(!long_slug.is_empty());
        assert_eq!(long_slug.len(), 35);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashMap;

        let slug1 = UrlSlug::parse("test-slug").unwrap();
        let slug2 = UrlSlug::parse("test-slug").unwrap();
        let slug3 = UrlSlug::parse("different-slug").unwrap();

        // Equal slugs should have equal hashes
        let mut map = HashMap::new();
        map.insert(slug1.clone(), "value1");
        map.insert(slug2.clone(), "value2"); // Should overwrite value1
        
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&slug1), Some(&"value2"));
        
        // Different slugs should have different entries
        map.insert(slug3.clone(), "value3");
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&slug3), Some(&"value3"));
    }

    #[test]
    fn test_error_messages() {
        // Test specific error messages
        let err = UrlSlug::parse("!!!").unwrap_err();
        assert!(matches!(err, UrlSlugError::EmptySlug));
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_parse_with_fake_generated_strings() {
        use fake::Fake;
        use fake::faker::lorem::en::Sentence;

        // Generate various strings and test parsing
        for _ in 0..50 {
            let sentence: String = Sentence(1..5).fake();

            // All sentences should parse successfully (they contain letters/spaces)
            let slug_result = UrlSlug::parse(&sentence);
            assert!(slug_result.is_ok(), "Failed to parse: {}", sentence);

            let slug = slug_result.unwrap();

            // Verify the result is a valid slug
            assert!(!slug.is_empty());
            // Verify it contains only valid characters
            for c in slug.as_str().chars() {
                assert!(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            }
        }
    }

    #[test]
    fn test_sqlx_type_info() {
        // Test that UrlSlug has the correct SQLx type info for SQLite
        let type_info = <UrlSlug as sqlx::Type<sqlx::Sqlite>>::type_info();
        let string_type_info = <String as sqlx::Type<sqlx::Sqlite>>::type_info();
        assert_eq!(type_info, string_type_info);
    }

    #[test]
    fn test_sqlx_encode() {
        use sqlx::Encode;

        let slug = UrlSlug::parse("test-slug").unwrap();

        // Test that we can encode the slug (basic functionality test)
        // This tests that the Encode trait is properly implemented
        let mut buf = Vec::new();
        let result = slug.encode_by_ref(&mut buf);
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_string_trait() {
        let s = "test-string".to_string();
        let slug: UrlSlug = s.clone().into();
        assert_eq!(slug.as_str(), "test-string");

        // Test with a string that needs cleaning
        let s = "Hello World!".to_string();
        let slug: UrlSlug = s.into();
        assert_eq!(slug.as_str(), "hello-world");
    }

    #[test]
    fn test_from_str_trait_implementation() {
        let s = "test-string";
        let slug: UrlSlug = s.into();
        assert_eq!(slug.as_str(), "test-string");

        // Test with a string that needs cleaning
        let slug: UrlSlug = "Hello World!".into();
        assert_eq!(slug.as_str(), "hello-world");
    }

    #[test]
    fn test_boundary_conditions() {
        // Test very long input
        let long_input = "a".repeat(1000) + " " + &"b".repeat(1000);
        let slug = UrlSlug::parse(long_input).unwrap();
        let expected = "a".repeat(1000) + "-" + &"b".repeat(1000); // No hyphen between since clean_string doesn't add hyphens between alphanumeric sequences
        assert_eq!(slug.as_str(), expected);

        // Test input with many consecutive spaces
        let slug = UrlSlug::parse("word1    word2").unwrap();
        assert_eq!(slug.as_str(), "word1-word2");

        // Test input with many consecutive hyphens
        let slug = UrlSlug::parse("word1----word2").unwrap();
        assert_eq!(slug.as_str(), "word1-word2");

        // Test input with mixed whitespace
        let slug = UrlSlug::parse("word1 \t\n word2").unwrap();
        assert_eq!(slug.as_str(), "word1-word2");

        // Test input with only valid characters
        let slug = UrlSlug::parse("already-valid-slug-123").unwrap();
        assert_eq!(slug.as_str(), "already-valid-slug-123");

        // Test input with unicode characters (should be filtered)
        let slug = UrlSlug::parse("café-résumé").unwrap();
        assert_eq!(slug.as_str(), "caf-rsum");
    }

    #[test]
    fn test_validate_slug_valid_cases() {
        // Test valid slugs that should pass validation
        assert!(UrlSlug::validate_slug("valid-slug").is_ok());
        assert!(UrlSlug::validate_slug("a").is_ok());
        assert!(UrlSlug::validate_slug("slug-with-numbers-123").is_ok());
        assert!(UrlSlug::validate_slug("multiple-hyphens-work").is_ok());
        assert!(UrlSlug::validate_slug("123").is_ok());
        assert!(UrlSlug::validate_slug("a-b-c-d-e").is_ok());
        assert!(UrlSlug::validate_slug("single").is_ok());
    }

    #[test]
    fn test_validate_slug_empty_string() {
        let result = UrlSlug::validate_slug("");
        assert!(matches!(result, Err(UrlSlugError::EmptySlug)));
    }

    #[test]
    fn test_validate_slug_starts_with_hyphen() {
        let test_cases = vec![
            "-starts-with-hyphen",
            "-",
            "-a",
            "-valid-slug",
        ];

        for case in test_cases {
            let result = UrlSlug::validate_slug(case);
            assert!(matches!(result, Err(UrlSlugError::StartsOrEndsWithHyphen(_))), 
                   "Failed for input: {}", case);
            if let Err(UrlSlugError::StartsOrEndsWithHyphen(msg)) = result {
                assert_eq!(msg, case);
            }
        }
    }

    #[test]
    fn test_validate_slug_ends_with_hyphen() {
        let test_cases = vec![
            "ends-with-hyphen-",
            "-",
            "a-",
            "valid-slug-",
        ];

        for case in test_cases {
            let result = UrlSlug::validate_slug(case);
            assert!(matches!(result, Err(UrlSlugError::StartsOrEndsWithHyphen(_))), 
                   "Failed for input: {}", case);
            if let Err(UrlSlugError::StartsOrEndsWithHyphen(msg)) = result {
                assert_eq!(msg, case);
            }
        }
    }

    #[test]
    fn test_validate_slug_consecutive_hyphens() {
        let test_cases = vec![
            "consecutive--hyphens",
            "word--word",
            "a--b",
            "test--case",
            "multiple---hyphens",
            "word----word",
        ];

        for case in test_cases {
            let result = UrlSlug::validate_slug(case);
            assert!(matches!(result, Err(UrlSlugError::ConsecutiveHyphens(_))), 
                   "Failed for input: {}", case);
            if let Err(UrlSlugError::ConsecutiveHyphens(msg)) = result {
                assert_eq!(msg, case);
            }
        }
    }

    #[test]
    fn test_validate_slug_invalid_characters() {
        let test_cases = vec![
            "UPPERCASE",           // uppercase letters
            "Mixed_Case",          // uppercase with underscore
            "slug with spaces",    // spaces
            "special!chars",       // special characters
            "unicode-café",        // unicode characters
            "email@test.com",      // @ symbol
            "path/to/file",        // forward slash
            "question?mark",       // question mark
            "hash#tag",            // hash symbol
            "percent%sign",        // percent sign
            "ampersand&test",      // ampersand
            "asterisk*test",       // asterisk
            "parentheses(test)",   // parentheses
            "brackets[test]",      // brackets
            "braces{test}",        // braces
            "plus+sign",           // plus sign
            "equals=sign",         // equals sign
            "pipe|symbol",         // pipe symbol
            "backslash\\test",     // backslash
            "colon:test",          // colon
            "semicolon;test",      // semicolon
            "quote'test",          // single quote
            "double\"quote",       // double quote
            "lessthan<test",       // less than
            "greaterthan>test",    // greater than
            "caret^test",          // caret
            "tilde~test",          // tilde
            "backtick`test",       // backtick
        ];

        for case in test_cases {
            let result = UrlSlug::validate_slug(case);
            assert!(matches!(result, Err(UrlSlugError::InvalidCharacters(_))), 
                   "Failed for input: {}", case);
            if let Err(UrlSlugError::InvalidCharacters(msg)) = result {
                assert_eq!(msg, case);
            }
        }
    }

    #[test]
    fn test_validate_slug_edge_cases() {
        // Test that single hyphen fails (starts and ends with hyphen)
        let result = UrlSlug::validate_slug("-");
        assert!(matches!(result, Err(UrlSlugError::StartsOrEndsWithHyphen(_))));

        // Test multiple consecutive hyphens at start/end - should fail on starts/ends check first
        let result = UrlSlug::validate_slug("--");
        assert!(matches!(result, Err(UrlSlugError::StartsOrEndsWithHyphen(_))));

        // Test valid single character
        assert!(UrlSlug::validate_slug("a").is_ok());
        assert!(UrlSlug::validate_slug("1").is_ok());
        assert!(UrlSlug::validate_slug("z").is_ok());
        assert!(UrlSlug::validate_slug("9").is_ok());

        // Test valid hyphen usage
        assert!(UrlSlug::validate_slug("a-b").is_ok());
        assert!(UrlSlug::validate_slug("1-2").is_ok());
        assert!(UrlSlug::validate_slug("word-1").is_ok());
        assert!(UrlSlug::validate_slug("1-word").is_ok());
    }

    #[test]
    fn test_validate_slug_comprehensive() {
        // Test a comprehensive set of valid and invalid cases
        let valid_cases = vec![
            "a", "1", "a1", "1a", "a-b", "1-2", "a-1", "1-a",
            "valid-slug", "slug-with-numbers-123", "multiple-parts-1-2-3",
            "abcdefghijklmnopqrstuvwxyz", "0123456789", "a1b2c3d4",
        ];

        let invalid_cases = vec![
            ("", UrlSlugError::EmptySlug),
            ("-", UrlSlugError::StartsOrEndsWithHyphen("-".to_string())),
            ("--", UrlSlugError::StartsOrEndsWithHyphen("--".to_string())), // starts/ends check comes first
            ("-a", UrlSlugError::StartsOrEndsWithHyphen("-a".to_string())),
            ("a-", UrlSlugError::StartsOrEndsWithHyphen("a-".to_string())),
            ("a--b", UrlSlugError::ConsecutiveHyphens("a--b".to_string())),
            ("UPPER", UrlSlugError::InvalidCharacters("UPPER".to_string())),
            ("space test", UrlSlugError::InvalidCharacters("space test".to_string())),
            ("special!@#", UrlSlugError::InvalidCharacters("special!@#".to_string())),
        ];

        for case in valid_cases {
            assert!(UrlSlug::validate_slug(case).is_ok(), "Expected '{}' to be valid", case);
        }

        for (input, expected_error) in invalid_cases {
            let result = UrlSlug::validate_slug(input);
            assert!(result.is_err(), "Expected '{}' to be invalid", input);
            assert_eq!(result.unwrap_err(), expected_error);
        }
    }
}