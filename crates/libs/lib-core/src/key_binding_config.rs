//! The Key Binding Configuration (ADR-0058): the plain `[keybindings]` payload, here so `lib-viper`
//! and the feature crates behind it name it without depending on `lib-config`, which reads and
//! validates it (`lib_config::keybindings::validate`).
//!
//! `quit` is deliberately **not** one of the configurable commands: `docs/navigation-design.md`'s
//! own "Quit" section explains why. `bin-tui` has three separate, non-configurable quit
//! mechanisms (`Ctrl+C` hard-quit, `Q`/`q` graceful quit, the `:quit` command), and none of them
//! read this config, so `DEFAULT_BINDINGS` leaves it out.

use std::collections::BTreeMap;

/// Default modifier held down for global shortcuts. `quit` isn't one of them -- see the
/// module doc's note on why it's excluded from `DEFAULT_BINDINGS` entirely.
const DEFAULT_SUPER_KEY: &str = "ctrl";

/// The command name -> key pairs used when no configuration source overrides them, mirroring
/// the shortcuts already hardcoded across `bin-tui`'s screens. `quit` is deliberately absent
/// -- see the module doc.
const DEFAULT_BINDINGS: &[(&str, &str)] = &[
    ("back", "esc"),
    ("help", "?"),
    ("open_command_popup", ":"),
    ("move_up", "k"),
    ("move_down", "j"),
    ("select", "enter"),
    ("new", "n"),
    ("delete", "d"),
    ("confirm", "y"),
    ("cancel", "x"),
    // Not `esc`: a sticky Error Toast must not vanish on the key that backs out of a dialog.
    ("dismiss_toasts", "ctrl+l"),
];

/// Configuration structure for keyboard shortcuts.
///
/// Each `bindings` entry maps a command name (e.g. `"quit"`, `"new"`) to the key that
/// triggers it (e.g. `"ctrl+c"`, `"n"`). The key strings are opaque to `lib-core` --
/// parsing them into an actual key representation (e.g. `crossterm::event::KeyEvent`) is
/// left to whichever binary consumes the configuration. `super_key` is the modifier the
/// application expects held down for global shortcuts, so a binary can decide which of its
/// own hardcoded shortcuts count as "global" versus screen-local.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct KeyBindingConfig {
    /// The modifier held down for global shortcuts (`"ctrl"`, `"alt"`, `"shift"`,
    /// `"super"`, or `"none"` to require no modifier at all).
    pub super_key: String,

    /// Command name -> key pairs. A `BTreeMap` keeps iteration order deterministic, which
    /// keeps `lib_config::keybindings::validate`'s duplicate-key error messages and any serialised output stable.
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

    /// The defaults with `command` rebound to `chord`, for a test that needs one rebind. Behind
    /// the `test-support` feature so shipped builds never carry it; a crate's tests reach it by
    /// enabling `lib_core/test-support` in `[dev-dependencies]`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn overriding(command: &str, chord: &str) -> Self {
        Self::default().and(command, chord)
    }

    /// Adds one more rebind to this configuration, so a test with several rebinds chains
    /// `overriding(..).and(..)` instead of mutating `bindings` by hand. Test-only, like
    /// [`Self::overriding`].
    #[cfg(any(test, feature = "test-support"))]
    pub fn and(mut self, command: &str, chord: &str) -> Self {
        self.bindings.insert(command.to_string(), chord.to_string());
        self
    }

    /// Get the default configuration values as key-value pairs, for seeding a layered
    /// configuration builder's defaults -- mirrors `SyncServerConfig::default_config_values()`.
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
        assert_eq!(
            config.key_for("quit"),
            None,
            "quit is deliberately not configurable"
        );
        assert_eq!(config.key_for("back"), Some("esc"));
        assert_eq!(config.key_for("help"), Some("?"));
        assert_eq!(config.key_for("open_command_popup"), Some(":"));
        assert_eq!(config.key_for("move_up"), Some("k"));
        assert_eq!(config.key_for("move_down"), Some("j"));
        assert_eq!(config.key_for("select"), Some("enter"));
        assert_eq!(config.key_for("new"), Some("n"));
        assert_eq!(config.key_for("delete"), Some("d"));
        assert_eq!(config.key_for("confirm"), Some("y"));
        assert_eq!(config.key_for("cancel"), Some("x"));
        assert_eq!(config.key_for("dismiss_toasts"), Some("ctrl+l"));
        assert_eq!(
            config.key_for("toast_history"),
            None,
            "toast_history is unbound by default"
        );
    }

    #[test]
    fn an_override_rebinds_one_command_and_keeps_every_other_default() {
        let config = KeyBindingConfig::overriding("back", "ctrl+q");

        assert_eq!(config.key_for("back"), Some("ctrl+q"));
        assert_eq!(config.key_for("help"), Some("?"));
        assert_eq!(config.super_key(), "ctrl");
    }

    #[test]
    fn chained_overrides_rebind_each_command_and_keep_every_other_default() {
        let config = KeyBindingConfig::overriding("back", "ctrl+q").and("select", "ctrl+y");

        assert_eq!(config.key_for("back"), Some("ctrl+q"));
        assert_eq!(config.key_for("select"), Some("ctrl+y"));
        assert_eq!(config.key_for("help"), Some("?"));
        assert_eq!(config.super_key(), "ctrl");
    }

    #[test]
    fn an_override_binds_a_command_with_no_default() {
        let config = KeyBindingConfig::overriding("toast_history", "H");

        assert_eq!(config.key_for("toast_history"), Some("H"));
    }

    #[test]
    fn key_for_unknown_command_returns_none() {
        let config = KeyBindingConfig::default();
        assert_eq!(config.key_for("does_not_exist"), None);
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
