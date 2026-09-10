//! # Key Binding Configuration
//!
//! This module provides configuration for customisable keyboard shortcuts, shared by the
//! TUI and Desktop Clients (`bin-tui`/`bin-desktop`). Sync Server config parsing
//! (`LedgerConfig::parse_for_sync_server`) still merges its defaults in, but nothing there
//! reads it -- a headless server has no keyboard to bind.
//!
//! ## Configuration Structure
//!
//! `KeyBindingConfig` has one fixed field, `super_key` -- the modifier held down for global
//! shortcuts (mirroring a window manager's `$mainMod`/prefix key, e.g. Hyprland's `$mainMod`)
//! -- plus a flattened map of command name -> key for everything else. Commands are open-ended
//! (new screens add new ones over time) and INI naturally represents an arbitrary
//! `[keybindings]` section as key/value pairs, so a map is a better fit there than a fixed
//! field list.
//!
//! ## Configuration File Example
//!
//! ```ini
//! [keybindings]
//! super_key = "ctrl"
//! quit = "ctrl+c"
//! back = "esc"
//! help = "?"
//! move_up = "k"
//! move_down = "j"
//! select = "enter"
//! new = "n"
//! delete = "d"
//! confirm = "y"
//! cancel = "x"
//! ```
//!
//! ## Environment Variables
//!
//! Individual bindings can be overridden using environment variables with the
//! `PERSONAL_LEDGER_KEYBINDINGS__` prefix:
//!
//! ```bash
//! PERSONAL_LEDGER_KEYBINDINGS__QUIT=ctrl+q
//! PERSONAL_LEDGER_KEYBINDINGS__SUPER_KEY=alt
//! ```

use std::collections::BTreeMap;

use crate::Error;

/// Recognised modifier names for [`KeyBindingConfig::super_key`]. `"none"` disables the
/// modifier requirement entirely, so global shortcuts fire on the bare key.
const VALID_SUPER_KEYS: &[&str] = &["ctrl", "alt", "shift", "super", "none"];

/// Default modifier held down for global shortcuts, matching the `quit` command's
/// already-hardcoded `ctrl+c` in `bin-tui`.
const DEFAULT_SUPER_KEY: &str = "ctrl";

/// The command name -> key pairs used when no configuration source overrides them, mirroring
/// the shortcuts already hardcoded across `bin-tui`'s screens.
const DEFAULT_BINDINGS: &[(&str, &str)] = &[
    ("quit", "ctrl+c"),
    ("back", "esc"),
    ("help", "?"),
    ("move_up", "k"),
    ("move_down", "j"),
    ("select", "enter"),
    ("new", "n"),
    ("delete", "d"),
    ("confirm", "y"),
    ("cancel", "x"),
];

/// Configuration structure for keyboard shortcuts.
///
/// Each `bindings` entry maps a command name (e.g. `"quit"`, `"new"`) to the key that
/// triggers it (e.g. `"ctrl+c"`, `"n"`). The key strings are opaque to `lib-config` --
/// parsing them into an actual key representation (e.g. `crossterm::event::KeyEvent`) is
/// left to whichever binary consumes the configuration. `super_key` is the modifier the
/// application expects held down for global shortcuts, so a binary can decide which of its
/// own hardcoded shortcuts count as "global" versus screen-local.
///
/// # Examples
///
/// ```rust
/// use lib_config::KeyBindingConfig;
///
/// let config = KeyBindingConfig::default();
/// assert_eq!(config.super_key(), "ctrl");
/// assert_eq!(config.key_for("quit"), Some("ctrl+c"));
/// ```
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct KeyBindingConfig {
    /// The modifier held down for global shortcuts (`"ctrl"`, `"alt"`, `"shift"`,
    /// `"super"`, or `"none"` to require no modifier at all).
    pub super_key: String,

    /// Command name -> key pairs. A `BTreeMap` keeps iteration order deterministic, which
    /// keeps `validate()`'s duplicate-key error messages and any serialised output stable.
    /// `#[serde(flatten)]` deserializes this directly from the rest of the `[keybindings]`
    /// section's key/value pairs, rather than expecting a nested `bindings` table.
    #[serde(flatten)]
    pub bindings: BTreeMap<String, String>,
}

impl Default for KeyBindingConfig {
    /// Creates the default set of key bindings, matching the shortcuts already hardcoded
    /// across `bin-tui`'s screens (see `docs/adr/0003-hybrid-tea-component-tui-architecture.md`
    /// for the Navigation/Editing input-mode split these shortcuts assume).
    fn default() -> Self {
        Self {
            super_key: DEFAULT_SUPER_KEY.to_string(),
            bindings: DEFAULT_BINDINGS
                .iter()
                .map(|(command, key)| (command.to_string(), key.to_string()))
                .collect(),
        }
    }
}

impl KeyBindingConfig {
    /// Get the modifier held down for global shortcuts.
    pub fn super_key(&self) -> &str {
        &self.super_key
    }

    /// Get the key bound to a command, if one is configured.
    pub fn key_for(&self, command: &str) -> Option<&str> {
        self.bindings.get(command).map(String::as_str)
    }

    /// Validate the key binding configuration.
    ///
    /// Checks that [`Self::super_key`] is one of the recognised modifier names, and that no
    /// two commands are bound to the same key, since that would leave one of them
    /// unreachable.
    ///
    /// # Returns
    ///
    /// `Ok(())` if `super_key` is recognised and every key is bound to at most one command,
    /// or an [`Error::InvalidKeyBindingConfig`] describing the problem.
    pub fn validate(&self) -> crate::Result<()> {
        if !VALID_SUPER_KEYS.contains(&self.super_key.as_str()) {
            return Err(Error::InvalidKeyBindingConfig(format!(
                "super_key '{}' is not one of {:?}",
                self.super_key, VALID_SUPER_KEYS
            )));
        }

        let mut seen: BTreeMap<&str, &str> = BTreeMap::new();

        for (command, key) in &self.bindings {
            if let Some(existing_command) = seen.insert(key.as_str(), command.as_str()) {
                return Err(Error::InvalidKeyBindingConfig(format!(
                    "key '{}' is bound to both '{}' and '{}'",
                    key, existing_command, command
                )));
            }
        }

        Ok(())
    }

    /// Get the default configuration values as key-value pairs, for seeding a layered
    /// configuration builder's defaults -- mirrors `DatabaseConfig::default_config_values()`.
    pub fn default_config_values() -> Vec<(String, String)> {
        let mut defaults = vec![(
            "keybindings.super_key".to_string(),
            DEFAULT_SUPER_KEY.to_string(),
        )];
        defaults.extend(
            DEFAULT_BINDINGS
                .iter()
                .map(|(command, key)| (format!("keybindings.{command}"), key.to_string())),
        );
        defaults
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_super_key() {
        let config = KeyBindingConfig::default();
        assert_eq!(config.super_key(), "ctrl");
    }

    #[test]
    fn default_config_has_expected_bindings() {
        let config = KeyBindingConfig::default();
        assert_eq!(config.key_for("quit"), Some("ctrl+c"));
        assert_eq!(config.key_for("back"), Some("esc"));
        assert_eq!(config.key_for("help"), Some("?"));
        assert_eq!(config.key_for("move_up"), Some("k"));
        assert_eq!(config.key_for("move_down"), Some("j"));
        assert_eq!(config.key_for("select"), Some("enter"));
        assert_eq!(config.key_for("new"), Some("n"));
        assert_eq!(config.key_for("delete"), Some("d"));
        assert_eq!(config.key_for("confirm"), Some("y"));
        assert_eq!(config.key_for("cancel"), Some("x"));
    }

    #[test]
    fn key_for_unknown_command_returns_none() {
        let config = KeyBindingConfig::default();
        assert_eq!(config.key_for("does_not_exist"), None);
    }

    #[test]
    fn validate_succeeds_with_default_config() {
        let config = KeyBindingConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_fails_with_duplicate_key() {
        let config = KeyBindingConfig {
            super_key: DEFAULT_SUPER_KEY.to_string(),
            bindings: BTreeMap::from([
                ("quit".to_string(), "q".to_string()),
                ("cancel".to_string(), "q".to_string()),
            ]),
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::InvalidKeyBindingConfig(_))));
    }

    #[test]
    fn validate_fails_with_unrecognised_super_key() {
        let config = KeyBindingConfig {
            super_key: "meta".to_string(),
            ..KeyBindingConfig::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::InvalidKeyBindingConfig(_))));
    }

    #[test]
    fn validate_succeeds_for_every_recognised_super_key() {
        for super_key in VALID_SUPER_KEYS {
            let config = KeyBindingConfig {
                super_key: super_key.to_string(),
                ..KeyBindingConfig::default()
            };
            assert!(
                config.validate().is_ok(),
                "expected '{super_key}' to be valid"
            );
        }
    }

    #[test]
    fn config_is_cloneable() {
        let config = KeyBindingConfig::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn config_is_debuggable() {
        let config = KeyBindingConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("KeyBindingConfig"));
    }

    #[test]
    fn config_partial_eq_works() {
        let config1 = KeyBindingConfig::default();
        let config2 = KeyBindingConfig::default();
        assert_eq!(config1, config2);

        let mut bindings = config1.bindings.clone();
        bindings.insert("quit".to_string(), "ctrl+q".to_string());
        let config3 = KeyBindingConfig {
            super_key: config1.super_key.clone(),
            bindings,
        };
        assert_ne!(config1, config3);

        let config4 = KeyBindingConfig {
            super_key: "alt".to_string(),
            ..config1.clone()
        };
        assert_ne!(config1, config4);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = KeyBindingConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: KeyBindingConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn default_config_values_returns_expected_pairs() {
        let defaults = KeyBindingConfig::default_config_values();
        assert_eq!(defaults.len(), DEFAULT_BINDINGS.len() + 1);

        assert!(
            defaults
                .iter()
                .any(|(k, v)| k == "keybindings.super_key" && v == DEFAULT_SUPER_KEY)
        );

        for (command, key) in DEFAULT_BINDINGS {
            let expected_key = format!("keybindings.{command}");
            assert!(
                defaults.iter().any(|(k, v)| k == &expected_key && v == key),
                "Missing or mismatched default for command: {}",
                command
            );
        }
    }
}
