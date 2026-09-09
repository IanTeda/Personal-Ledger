//! # Sync Server Configuration
//!
//! Configuration specific to the Sync Server binary (`bin-sync-server`), not read by the
//! TUI or Desktop Clients. Currently just the gRPC bind address; TLS is not configured here
//! (see ADR-0014 and the "lib-config / lib-database refactor" Wayfinder map's Out of scope).

/// Default gRPC bind address for the Sync Server.
///
/// Matches the value `bin-sync-server` hardcoded directly in `main.rs` before this section
/// existed.
const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:50051";

/// Configuration specific to the Sync Server.
///
/// TUI and Desktop Clients never read this section; the Sync Server's config search
/// (`LedgerConfig::parse_for_sync_server`) is the only one that populates it from anything
/// other than the default below.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct SyncServerConfig {
    /// The socket address the Sync Server's gRPC/HTTP listener binds to.
    pub bind_address: String,
}

impl Default for SyncServerConfig {
    fn default() -> Self {
        Self {
            bind_address: DEFAULT_BIND_ADDRESS.to_string(),
        }
    }
}

impl SyncServerConfig {
    /// Get the configured bind address.
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    /// Get the default configuration values as key-value pairs, for seeding a layered
    /// configuration builder's defaults -- mirrors `DatabaseConfig::default_config_values()`.
    pub fn default_config_values() -> Vec<(&'static str, String)> {
        let default_config = Self::default();
        vec![(
            "sync_server.bind_address",
            default_config.bind_address().to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_bind_address() {
        let config = SyncServerConfig::default();
        assert_eq!(config.bind_address(), DEFAULT_BIND_ADDRESS);
    }

    #[test]
    fn default_config_values_returns_expected_pair() {
        let defaults = SyncServerConfig::default_config_values();
        assert_eq!(defaults.len(), 1);
        assert_eq!(
            defaults[0],
            ("sync_server.bind_address", DEFAULT_BIND_ADDRESS.to_string())
        );
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = SyncServerConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: SyncServerConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
