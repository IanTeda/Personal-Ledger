//! # Sync Server Configuration
//!
//! Configuration specific to the Sync Server binary (`bin-sync-server`), not read by the
//! TUI or Desktop Clients: the gRPC bind address and the Sync Server's own database URI
//! (its durable Change Set log, per ADR-0009 -- distinct from any Client's local
//! `[Personal-Ledger] file`). TLS is not configured here (see ADR-0014 and the
//! "lib-config / lib-database refactor" Wayfinder map's Out of scope).

/// Default gRPC bind address for the Sync Server.
const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:50051";

/// Default database URI for the Sync Server's own database.
const DEFAULT_DATABASE_URI: &str = "sqlite:./sync-server.sqlite";

/// Configuration specific to the Sync Server.
///
/// TUI and Desktop Clients never read this section; the Sync Server's config search
/// (`LedgerConfig::parse_for_sync_server`) is the only one that populates it from anything
/// other than the defaults below.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct SyncServerConfig {
    /// The socket address the Sync Server's gRPC/HTTP listener binds to.
    pub bind_address: String,

    /// The URI of the Sync Server's own database.
    pub database_uri: String,
}

impl Default for SyncServerConfig {
    fn default() -> Self {
        Self {
            bind_address: DEFAULT_BIND_ADDRESS.to_string(),
            database_uri: DEFAULT_DATABASE_URI.to_string(),
        }
    }
}

impl SyncServerConfig {
    /// Get the configured bind address.
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    /// Get the configured database URI.
    pub fn database_uri(&self) -> &str {
        &self.database_uri
    }

    /// Get the default configuration values as key-value pairs, for seeding a layered
    /// configuration builder's defaults -- mirrors `PersonalLedgerConfig::default_config_values()`.
    pub fn default_config_values() -> Vec<(&'static str, String)> {
        let default_config = Self::default();
        vec![
            (
                "sync_server.bind_address",
                default_config.bind_address().to_string(),
            ),
            (
                "sync_server.database_uri",
                default_config.database_uri().to_string(),
            ),
        ]
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
    fn default_config_has_expected_database_uri() {
        let config = SyncServerConfig::default();
        assert_eq!(config.database_uri(), DEFAULT_DATABASE_URI);
    }

    #[test]
    fn default_config_values_returns_expected_pairs() {
        let defaults = SyncServerConfig::default_config_values();
        assert_eq!(defaults.len(), 2);
        assert_eq!(
            defaults[0],
            ("sync_server.bind_address", DEFAULT_BIND_ADDRESS.to_string())
        );
        assert_eq!(
            defaults[1],
            ("sync_server.database_uri", DEFAULT_DATABASE_URI.to_string())
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
