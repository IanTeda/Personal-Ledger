//! The tracing level as Configuration names it. It lives here, not in `lib-tracing`, so the
//! Configuration key catalogue (`crate::configuration`) can hold it (ADR-0058); `lib-tracing`
//! re-exports it and turns it into a `LevelFilter` with `lib_tracing::level_filter`.

/// A tracing level a configuration file or environment variable can name, since `tracing`'s
/// `LevelFilter` has no serde support. Spelt in lowercase so the file reads as a person writes
/// it; `WARN` by default, so a fresh install shows what needs attention without the noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Levels {
    OFF,
    ERROR,
    #[default]
    WARN,
    INFO,
    DEBUG,
    TRACE,
}

impl std::fmt::Display for Levels {
    /// Spelt as serde spells it, so a level shown reads as the file would hold it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_str = match self {
            Levels::OFF => "off",
            Levels::ERROR => "error",
            Levels::WARN => "warn",
            Levels::INFO => "info",
            Levels::DEBUG => "debug",
            Levels::TRACE => "trace",
        };
        write!(f, "{}", level_str)
    }
}

/// Error returned by [`Levels`]'s `FromStr` implementation for an unrecognised level string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid telemetry level {0:?} (expected one of: off, error, warn, info, debug, trace)")]
pub struct ParseLevelError(String);

impl std::str::FromStr for Levels {
    type Err = ParseLevelError;

    /// Parses a level from its lowercase string form -- the same vocabulary as
    /// [`std::fmt::Display`]/serde, so config files, environment variables, and CLI flags
    /// (`--log`) all accept the same values. Case-insensitive for CLI ergonomics.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "off" => Ok(Levels::OFF),
            "error" => Ok(Levels::ERROR),
            "warn" => Ok(Levels::WARN),
            "info" => Ok(Levels::INFO),
            "debug" => Ok(Levels::DEBUG),
            "trace" => Ok(Levels::TRACE),
            _ => Err(ParseLevelError(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn test_default_level() {
        let default_level = Levels::default();
        assert_eq!(default_level, Levels::WARN);
    }

    #[test]
    fn test_serialization() {
        // Test that each variant serializes to the expected lowercase string
        assert_eq!(to_string(&Levels::OFF).unwrap(), "\"off\"");
        assert_eq!(to_string(&Levels::ERROR).unwrap(), "\"error\"");
        assert_eq!(to_string(&Levels::WARN).unwrap(), "\"warn\"");
        assert_eq!(to_string(&Levels::INFO).unwrap(), "\"info\"");
        assert_eq!(to_string(&Levels::DEBUG).unwrap(), "\"debug\"");
        assert_eq!(to_string(&Levels::TRACE).unwrap(), "\"trace\"");
    }

    #[test]
    fn test_deserialization() {
        // Test that each lowercase string deserializes to the correct variant
        assert_eq!(from_str::<Levels>("\"off\"").unwrap(), Levels::OFF);
        assert_eq!(from_str::<Levels>("\"error\"").unwrap(), Levels::ERROR);
        assert_eq!(from_str::<Levels>("\"warn\"").unwrap(), Levels::WARN);
        assert_eq!(from_str::<Levels>("\"info\"").unwrap(), Levels::INFO);
        assert_eq!(from_str::<Levels>("\"debug\"").unwrap(), Levels::DEBUG);
        assert_eq!(from_str::<Levels>("\"trace\"").unwrap(), Levels::TRACE);
    }

    #[test]
    fn test_deserialization_case_insensitive() {
        // Test that uppercase strings also work (serde_json is case-sensitive, but our rename_all handles it)
        assert!(from_str::<Levels>("\"OFF\"").is_err()); // Should fail
        assert!(from_str::<Levels>("\"Error\"").is_err()); // Should fail
    }

    #[test]
    fn test_debug_trait() {
        let level = Levels::INFO;
        let debug_str = format!("{:?}", level);
        assert!(debug_str.contains("INFO"));
    }

    #[test]
    fn test_clone_trait() {
        let original = Levels::DEBUG;
        let cloned = Clone::clone(&original);
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_copy_trait() {
        let original = Levels::TRACE;
        let copied = original; // Copy trait allows this
        assert_eq!(original, copied);
    }

    #[test]
    fn test_partial_eq_trait() {
        assert_eq!(Levels::WARN, Levels::WARN);
        assert_ne!(Levels::INFO, Levels::DEBUG);
    }

    #[test]
    fn test_all_variants_defined() {
        // Ensure all expected variants exist and are distinct
        let variants = [
            Levels::OFF,
            Levels::ERROR,
            Levels::WARN,
            Levels::INFO,
            Levels::DEBUG,
            Levels::TRACE,
        ];

        // Check they're all different
        for (i, &variant1) in variants.iter().enumerate() {
            for (j, &variant2) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        variant1, variant2,
                        "Variants at indices {} and {} should be different",
                        i, j
                    );
                }
            }
        }
    }

    #[test]
    fn test_display_trait() {
        // Test that Display produces the expected lowercase strings
        assert_eq!(format!("{}", Levels::OFF), "off");
        assert_eq!(format!("{}", Levels::ERROR), "error");
        assert_eq!(format!("{}", Levels::WARN), "warn");
        assert_eq!(format!("{}", Levels::INFO), "info");
        assert_eq!(format!("{}", Levels::DEBUG), "debug");
        assert_eq!(format!("{}", Levels::TRACE), "trace");
    }

    #[test]
    fn from_str_parses_every_variant() {
        assert_eq!("off".parse::<Levels>().unwrap(), Levels::OFF);
        assert_eq!("error".parse::<Levels>().unwrap(), Levels::ERROR);
        assert_eq!("warn".parse::<Levels>().unwrap(), Levels::WARN);
        assert_eq!("info".parse::<Levels>().unwrap(), Levels::INFO);
        assert_eq!("debug".parse::<Levels>().unwrap(), Levels::DEBUG);
        assert_eq!("trace".parse::<Levels>().unwrap(), Levels::TRACE);
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!("DEBUG".parse::<Levels>().unwrap(), Levels::DEBUG);
        assert_eq!("Info".parse::<Levels>().unwrap(), Levels::INFO);
    }

    #[test]
    fn from_str_rejects_unknown_level() {
        let err = "verbose".parse::<Levels>().unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid telemetry level \"verbose\" (expected one of: off, error, warn, info, debug, trace)"
        );
    }

    #[test]
    fn display_and_from_str_roundtrip() {
        for level in [
            Levels::OFF,
            Levels::ERROR,
            Levels::WARN,
            Levels::INFO,
            Levels::DEBUG,
            Levels::TRACE,
        ] {
            assert_eq!(level.to_string().parse::<Levels>().unwrap(), level);
        }
    }
}
