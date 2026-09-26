//! # Hex Colour Domain Type
//!
//! This module defines [`HexColor`], an immutable, validated representation of a
//! RGB colour encoded as a hexadecimal string (e.g. `#FFAA00`). It provides
//! parsing utilities, strongly typed access to channel values, helpers for random
//! generation in tests, and trait implementations needed across the code base.

/// Represents a web-style hexadecimal RGB colour in canonical `#RRGGBB` form.
///
/// Use [`HexColor::parse`] or [`HexColor::from_rgb`] to create instances. The
/// internal string is guaranteed to be uppercase, always begins with `#`, and
/// contains exactly six hexadecimal digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HexColor {
    hex_string: String,
    red: u8,
    green: u8,
    blue: u8,
}

/// Errors that can occur when parsing or constructing a [`HexColor`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum HexColorError {
    /// The input was empty or whitespace only.
    #[error("Hex colour cannot be empty")]
    Empty,
    /// The input length after removing an optional leading `#` was not six.
    #[error("Hex colour must contain exactly six hexadecimal digits: {0}")]
    InvalidLength(String),
    /// The input contained non-hexadecimal characters.
    #[error("Hex colour contains invalid characters: {0}")]
    InvalidCharacters(String),
}

impl HexColor {
    /// Parses a string into a [`HexColor`] after validating format and
    /// normalising to uppercase `#RRGGBB` form.
    ///
    /// # Errors
    ///
    /// Returns [`HexColorError`] if the value is empty, the number of
    /// hexadecimal digits is not six, or the string contains non-hex digits.
    pub fn parse<S: AsRef<str>>(input: S) -> Result<Self, HexColorError> {
        let input = input.as_ref().trim();
        if input.is_empty() {
            return Err(HexColorError::Empty);
        }

        let digits = input.strip_prefix('#').unwrap_or(input);
        if digits.len() != 6 {
            return Err(HexColorError::InvalidLength(input.to_string()));
        }

        if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(HexColorError::InvalidCharacters(input.to_string()));
        }

        let value = u32::from_str_radix(digits, 16)
            .map_err(|_| HexColorError::InvalidCharacters(input.to_string()))?;
        let canonical = format!("#{:06X}", value);
        let red = ((value >> 16) & 0xFF) as u8;
        let green = ((value >> 8) & 0xFF) as u8;
        let blue = (value & 0xFF) as u8;

        Ok(HexColor {
            hex_string: canonical,
            red,
            green,
            blue,
        })
    }

    /// Creates a [`HexColor`] from individual RGB components.
    #[must_use]
    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        HexColor {
            hex_string: format!("#{:02X}{:02X}{:02X}", red, green, blue),
            red,
            green,
            blue,
        }
    }

    /// Gets the canonical `#RRGGBB` representation as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.hex_string
    }

    /// Returns the individual red, green, and blue components as a tuple.
    #[must_use]
    pub fn components(&self) -> (u8, u8, u8) {
        (self.red, self.green, self.blue)
    }

    /// Returns the red channel as an integer between 0 and 255.
    #[must_use]
    pub fn red(&self) -> u8 {
        self.components().0
    }

    /// Returns the green channel as an integer between 0 and 255.
    #[must_use]
    pub fn green(&self) -> u8 {
        self.components().1
    }

    /// Returns the blue channel as an integer between 0 and 255.
    #[must_use]
    pub fn blue(&self) -> u8 {
        self.components().2
    }

    /// Convenience helper used in validation contexts.
    #[must_use]
    pub fn is_valid<S: AsRef<str>>(input: S) -> bool {
        Self::parse(input).is_ok()
    }

    /// Generates a random colour for testing scenarios.
    pub fn mock() -> Self {
        use fake::Fake;
        use fake::faker::color::en::HexColor as FakeHex;

        let value: String = FakeHex().fake();
        #[expect(
            clippy::expect_used,
            reason = "fake's HexColor faker always yields a valid #RRGGBB string"
        )]
        let color = HexColor::parse(value).expect("fake hex colour should be valid");
        color
    }

    /// Generate a random option colour or None for testing scenarios.
    pub fn mock_with_option() -> Option<Self> {
        use fake::Fake;
        use fake::faker::boolean::en::Boolean;

        let is_some: bool = Boolean(50).fake(); // 50% chance of Some
        if is_some { Some(Self::mock()) } else { None }
    }
}

impl HexColor {
    /// Converts the colour into its owned String representation.
    #[must_use]
    pub fn into_string(self) -> String {
        self.hex_string
    }
}

impl std::fmt::Display for HexColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hex_string)
    }
}

impl std::str::FromStr for HexColor {
    type Err = HexColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        HexColor::parse(s)
    }
}

impl From<HexColor> for String {
    fn from(value: HexColor) -> Self {
        value.hex_string
    }
}

impl From<&HexColor> for String {
    fn from(value: &HexColor) -> Self {
        value.hex_string.clone()
    }
}

impl AsRef<str> for HexColor {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<[u8; 3]> for HexColor {
    fn from(rgb: [u8; 3]) -> Self {
        HexColor::from_rgb(rgb[0], rgb[1], rgb[2])
    }
}

impl TryFrom<&str> for HexColor {
    type Error = HexColorError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        HexColor::parse(value)
    }
}

// Serde implementations for serializing to/from the canonical hex string format.
impl serde::Serialize for HexColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.hex_string.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        HexColor::parse(s).map_err(serde::de::Error::custom)
    }
}

// SQLx trait implementations ensure the colour can be stored as TEXT.
impl sqlx::Type<sqlx::Sqlite> for HexColor {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for HexColor {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let raw = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(HexColor::parse(raw).map_err(|e| format!("Invalid hex colour in database: {}", e))?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for HexColor {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(self.hex_string.clone(), buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_colour_with_hash() {
        let colour = HexColor::parse("#ff0000").unwrap();
        assert_eq!(colour.as_str(), "#FF0000");
        assert_eq!(colour.components(), (255, 0, 0));
    }

    #[test]
    fn parse_valid_colour_without_hash() {
        let colour = HexColor::parse("00ff7f").unwrap();
        assert_eq!(colour.as_str(), "#00FF7F");
        assert_eq!(colour.green(), 255);
        assert_eq!(colour.blue(), 127);
    }

    #[test]
    fn parse_rejects_invalid_length() {
        let err = HexColor::parse("123").unwrap_err();
        assert!(matches!(err, HexColorError::InvalidLength(_)));
    }

    #[test]
    fn parse_rejects_invalid_characters() {
        let err = HexColor::parse("#GGGGGG").unwrap_err();
        assert!(matches!(err, HexColorError::InvalidCharacters(_)));
    }

    #[test]
    fn from_rgb_round_trip() {
        let colour = HexColor::from_rgb(12, 34, 56);
        assert_eq!(colour.as_str(), "#0C2238");
        assert_eq!(colour.components(), (12, 34, 56));
    }

    #[test]
    fn mock_generates_valid_colour() {
        let colour = HexColor::mock();
        assert!(HexColor::is_valid(colour.as_str()));
    }

    #[test]
    fn display_matches_string() {
        let colour = HexColor::parse("#123abc").unwrap();
        assert_eq!(format!("{}", colour), "#123ABC");
    }

    #[test]
    fn serde_round_trip_preserves_value() {
        let colour = HexColor::parse("#abcdef").unwrap();
        let json = serde_json::to_string(&colour).unwrap();
        assert_eq!(json, "\"#ABCDEF\"");
        let decoded: HexColor = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, colour);
    }

    #[test]
    fn try_from_accepts_valid_values() {
        let colour = HexColor::try_from("#0a1b2c").unwrap();
        assert_eq!(colour.components(), (10, 27, 44));
    }

    #[test]
    fn as_ref_exposes_underlying_str() {
        let colour = HexColor::parse("112233").unwrap();
        let value: &str = colour.as_ref();
        assert_eq!(value, "#112233");
    }

    #[test]
    fn into_string_returns_owned_value() {
        let value = HexColor::parse("#ff00aa").unwrap().into_string();
        assert_eq!(value, "#FF00AA");
    }

    #[test]
    fn mock_with_option_returns_some_or_none() {
        let mut some_count = 0;
        let mut none_count = 0;
        for _ in 0..100 {
            match HexColor::mock_with_option() {
                Some(color) => {
                    some_count += 1;
                    assert!(HexColor::is_valid(color.as_str()));
                }
                None => none_count += 1,
            }
        }
        assert!(some_count > 0, "Should have generated Some at least once");
        assert!(none_count > 0, "Should have generated None at least once");
    }
}
