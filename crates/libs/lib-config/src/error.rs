//! Configuration loading and validation errors.
//!
//! This module provides `ConfigError`, a small error type that wraps errors
//! originating from the `config` crate and adds a couple of domain-specific
//! validation variants used by the application.

use config::ConfigError as ConfigLibError;

/// Result type alias used across configuration module.
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

#[derive(thiserror::Error, Debug)]
/// Errors produced while loading or validating configuration.
///
/// `ConfigError` wraps the `config` crate's errors and provides a couple of
/// convenience variants for validation and address-parsing failures.
pub enum ConfigError {
    /// Error from the underlying config crate during file loading or parsing.
    ///
    /// This wraps errors from the `config` crate such as file not found,
    /// invalid syntax, or parsing failures.
    #[error("Configuration parsing error: {0}")]
    Parsing(#[from] ConfigLibError),

    /// Validation error with a descriptive message.
    ///
    /// This is used for configuration validation failures such as invalid
    /// values, missing required fields, or security misconfigurations.
    #[error("Invalid configuration: {0}")]
    Validation(String),

    /// Error indicating an invalid server address format.
    ///
    /// This is used for configuration validation failures such as invalid
    /// values, missing required fields, or security misconfigurations.
    #[error("Invalid server address: {0}")]
    InvalidServerAddress(#[from] std::net::AddrParseError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::net::SocketAddr;

    #[test]
    fn validation_variant_formats_as_expected() {
        let err = ConfigError::Validation("missing field x".into());
        assert_eq!(err.to_string(), "Invalid configuration: missing field x");
    }

    #[test]
    fn invalid_server_address_variant_formats_as_expected() {
        // produce an AddrParseError from an intentionally invalid socket addr
        let parse_err = "not_an_ip:80".parse::<SocketAddr>().unwrap_err();
        let err = ConfigError::InvalidServerAddress(parse_err);
        let s = err.to_string();
        assert!(s.starts_with("Invalid server address:"));
    }

    #[test]
    fn parsing_variant_wraps_config_error() {
        // Create a temporary file with invalid JSON to provoke a parse error
        let mut path = std::env::temp_dir();
        path.push("plb_invalid_config.json");
        let mut f = File::create(&path).expect("create temp file");
        // invalid JSON
        write!(f, "not a json").expect("write invalid content");

        let builder = config::Config::builder()
            .add_source(config::File::from(path).format(config::FileFormat::Json));

        let cfg_err = builder.build().expect_err("expected config build to fail");
        let err = ConfigError::Parsing(cfg_err);
        let s = err.to_string();
        assert!(s.starts_with("Configuration parsing error:"));
    }
}
