//! Configuration loading and validation errors.

use config::ConfigError as ConfigLibError;

/// Errors produced while loading or validating configuration.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A `config`-crate error: file not found, invalid syntax, or a parse failure.
    #[error("Configuration parsing error: {0}")]
    Parsing(#[from] ConfigLibError),

    /// A validation failure with a descriptive message.
    #[error("Invalid configuration: {0}")]
    Validation(String),

    /// An invalid socket address string.
    #[error("Invalid server address: {0}")]
    InvalidServerAddress(#[from] std::net::AddrParseError),

    /// Two commands are bound to the same key, leaving one unreachable.
    #[error("Invalid key binding config: {0}")]
    InvalidKeyBindingConfig(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::net::SocketAddr;

    #[test]
    fn validation_variant_formats_as_expected() {
        let err = Error::Validation("missing field x".into());
        assert_eq!(err.to_string(), "Invalid configuration: missing field x");
    }

    #[test]
    fn invalid_server_address_variant_formats_as_expected() {
        // produce an AddrParseError from an intentionally invalid socket addr
        let parse_err = "not_an_ip:80".parse::<SocketAddr>().unwrap_err();
        let err = Error::InvalidServerAddress(parse_err);
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
        let err = Error::Parsing(cfg_err);
        let s = err.to_string();
        assert!(s.starts_with("Configuration parsing error:"));
    }
}
