//! Parser for a Colour Theme INI file.
//!
//! Shared with `build.rs` through `#[path]`, so it depends on `std` alone. Keeping it a plain
//! function also leaves it ready for a later runtime loader of user-defined Colour Themes.

use std::fmt;

/// The stored Colour Role keys, in the order every parsed Colour Variant holds them.
pub const ROLE_KEYS: [&str; 7] = [
    "foreground",
    "background",
    "accent",
    "cursor",
    "muted",
    "positive",
    "negative",
];

/// The Colour Variant section names a Colour Theme file must contain.
pub const VARIANT_KEYS: [&str; 2] = ["light", "dark"];

/// One Colour Variant's seven roles as RGB, in [`ROLE_KEYS`] order.
pub type VariantRgb = [[u8; 3]; 7];

/// A parsed Colour Theme file: the light and dark Colour Variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeFile {
    pub light: VariantRgb,
    pub dark: VariantRgb,
}

/// Why a Colour Theme file failed to parse. Line numbers are 1-based.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnknownSection {
        line: usize,
        name: String,
    },
    UnknownRole {
        line: usize,
        key: String,
    },
    KeyOutsideSection {
        line: usize,
    },
    Malformed {
        line: usize,
    },
    BadHex {
        line: usize,
        value: String,
    },
    MissingVariant {
        variant: &'static str,
    },
    MissingRole {
        variant: &'static str,
        role: &'static str,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSection { line, name } => {
                write!(f, "line {line}: unknown section [{name}]")
            }
            Self::UnknownRole { line, key } => {
                write!(f, "line {line}: unknown Colour Role `{key}`")
            }
            Self::KeyOutsideSection { line } => {
                write!(f, "line {line}: key before any [light]/[dark] section")
            }
            Self::Malformed { line } => write!(f, "line {line}: expected `role = \"#rrggbb\"`"),
            Self::BadHex { line, value } => write!(f, "line {line}: `{value}` is not a hex colour"),
            Self::MissingVariant { variant } => write!(f, "missing [{variant}] section"),
            Self::MissingRole { variant, role } => write!(f, "[{variant}] is missing `{role}`"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses a hex colour with or without the leading `#`, optionally quoted, as in `[theme]`.
pub fn parse_hex(value: &str) -> Option<[u8; 3]> {
    let value = value.trim().trim_matches('"').trim();
    let digits = value.strip_prefix('#').unwrap_or(value);
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// Parses a Colour Theme file. Every Colour Variant must set every Colour Role exactly as
/// `[theme]` would: a missing one is an error, not a fallback, so a bad file fails the build.
pub fn parse_theme(source: &str) -> Result<ThemeFile, ParseError> {
    let mut variants: [[Option<[u8; 3]>; 7]; 2] = [[None; 7]; 2];
    let mut seen = [false; 2];
    let mut current: Option<usize> = None;

    for (index, raw) in source.lines().enumerate() {
        let line = index + 1;
        let text = raw.trim();
        if text.is_empty() || text.starts_with(';') {
            continue;
        }
        if let Some(name) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
            let name = name.trim().to_ascii_lowercase();
            let Some(position) = VARIANT_KEYS.iter().position(|v| *v == name) else {
                return Err(ParseError::UnknownSection { line, name });
            };
            seen[position] = true;
            current = Some(position);
            continue;
        }
        let Some((key, value)) = text.split_once('=') else {
            return Err(ParseError::Malformed { line });
        };
        let key = key.trim().to_ascii_lowercase();
        let Some(role) = ROLE_KEYS.iter().position(|r| *r == key) else {
            return Err(ParseError::UnknownRole { line, key });
        };
        let Some(variant) = current else {
            return Err(ParseError::KeyOutsideSection { line });
        };
        let Some(rgb) = parse_hex(value) else {
            return Err(ParseError::BadHex {
                line,
                value: value.trim().to_string(),
            });
        };
        variants[variant][role] = Some(rgb);
    }

    let mut out = [[[0u8; 3]; 7]; 2];
    for (v, variant) in VARIANT_KEYS.iter().enumerate() {
        if !seen[v] {
            return Err(ParseError::MissingVariant { variant });
        }
        for (r, role) in ROLE_KEYS.iter().enumerate() {
            out[v][r] = variants[v][r].ok_or(ParseError::MissingRole { variant, role })?;
        }
    }
    Ok(ThemeFile {
        light: out[0],
        dark: out[1],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_variant(name: &str) -> String {
        let mut s = format!("[{name}]\n");
        for role in ROLE_KEYS {
            s.push_str(&format!("{role} = \"#102030\"\n"));
        }
        s
    }

    #[test]
    fn parses_both_variants_with_and_without_hash() {
        let source = format!(
            "; comment\n{}\n{}",
            full_variant("light"),
            full_variant("dark")
        )
        .replace(
            "[dark]\nforeground = \"#102030\"",
            "[dark]\nforeground = ffffff",
        );
        let theme = parse_theme(&source).unwrap();
        assert_eq!(theme.light[0], [0x10, 0x20, 0x30]);
        assert_eq!(theme.dark[0], [0xff, 0xff, 0xff]);
    }

    #[test]
    fn missing_variant_fails() {
        let err = parse_theme(&full_variant("light")).unwrap_err();
        assert_eq!(err, ParseError::MissingVariant { variant: "dark" });
    }

    #[test]
    fn missing_role_fails() {
        let source = format!("{}\n{}", full_variant("light"), full_variant("dark")).replacen(
            "muted = \"#102030\"\n",
            "",
            1,
        );
        let err = parse_theme(&source).unwrap_err();
        assert_eq!(
            err,
            ParseError::MissingRole {
                variant: "light",
                role: "muted"
            }
        );
    }

    #[test]
    fn bad_hex_fails() {
        let source = format!("{}\n{}", full_variant("light"), full_variant("dark"))
            .replacen("#102030", "#12345", 1);
        assert!(matches!(
            parse_theme(&source),
            Err(ParseError::BadHex { line: 2, .. })
        ));
    }

    #[test]
    fn unknown_role_and_section_fail() {
        assert!(matches!(
            parse_theme("[light]\nwarning = \"#000000\""),
            Err(ParseError::UnknownRole { .. })
        ));
        assert!(matches!(
            parse_theme("[theme]"),
            Err(ParseError::UnknownSection { .. })
        ));
        assert!(matches!(
            parse_theme("accent = \"#000000\""),
            Err(ParseError::KeyOutsideSection { .. })
        ));
    }
}
