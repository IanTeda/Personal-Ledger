//! # Key Binding Configuration
//!
//! This module provides configuration for customisable keyboard shortcuts, shared by the
//! TUI and Desktop Clients (`bin-tui`/`bin-desktop`). Sync Server config parsing
//! (`LedgerConfig::parse_for_sync_server`) still merges its defaults in, but nothing there
//! reads it -- a headless server has no keyboard to bind.
//!
//! ## Configuration Structure
//!
//! `KeyBindingConfig` (defined in `lib-core`, so feature crates needn't depend on this one; it is no longer re-exported here;
//! ADR-0058) has one fixed field, `super_key` -- the modifier held down for global
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
//! back = "esc"
//! help = "?"
//! open_command_popup = ":"
//! move_up = "k"
//! move_down = "j"
//! select = "enter"
//! new = "n"
//! delete = "d"
//! confirm = "y"
//! cancel = "x"
//! dismiss_toasts = "ctrl+l"
//! toast_history = "ctrl+t"
//! ```
//!
//! `toast_history` has no default key -- it opens with the `toasts` command -- so it only binds
//! when a configuration source sets it.
//!
//! `quit` is deliberately **not** one of these; `lib_core::configuration::KeyBindingConfig`'s
//! module doc says why.
//!
//! ## Environment Variables
//!
//! Individual bindings can be overridden using environment variables with the
//! `PERSONAL_LEDGER_KEYBINDINGS__` prefix:
//!
//! ```bash
//! PERSONAL_LEDGER_KEYBINDINGS__BACK=ctrl+h
//! PERSONAL_LEDGER_KEYBINDINGS__SUPER_KEY=alt
//! ```

use std::collections::BTreeMap;

use lib_core::configuration::KeyBindingConfig;

use crate::Error;

/// Recognised modifier names for [`KeyBindingConfig::super_key`]. `"none"` disables the
/// modifier requirement entirely, so global shortcuts fire on the bare key.
const VALID_SUPER_KEYS: &[&str] = &["ctrl", "alt", "shift", "super", "none"];

/// Validate the key binding configuration.
///
/// Checks that [`KeyBindingConfig::super_key`] is one of the recognised modifier names, and that no
/// two commands are bound to the same key, since that would leave one of them
/// unreachable.
///
/// # Returns
///
/// `Ok(())` if `super_key` is recognised and every key is bound to at most one command,
/// or an [`Error::InvalidKeyBindingConfig`] describing the problem.
pub fn validate(config: &KeyBindingConfig) -> crate::Result<()> {
    if !VALID_SUPER_KEYS.contains(&config.super_key.as_str()) {
        return Err(Error::InvalidKeyBindingConfig(format!(
            "super_key '{}' is not one of {:?}",
            config.super_key, VALID_SUPER_KEYS
        )));
    }

    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();

    for (command, key) in &config.bindings {
        if let Some(existing_command) = seen.insert(key.as_str(), command.as_str()) {
            return Err(Error::InvalidKeyBindingConfig(format!(
                "key '{}' is bound to both '{}' and '{}'",
                key, existing_command, command
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_succeeds_with_default_config() {
        let config = KeyBindingConfig::default();
        assert!(validate(&config).is_ok());
    }

    #[test]
    fn validate_fails_with_duplicate_key() {
        let config = KeyBindingConfig {
            super_key: "ctrl".to_string(),
            bindings: BTreeMap::from([
                ("quit".to_string(), "q".to_string()),
                ("cancel".to_string(), "q".to_string()),
            ]),
        };
        let result = validate(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::InvalidKeyBindingConfig(_))));
    }

    #[test]
    fn validate_fails_with_unrecognised_super_key() {
        let config = KeyBindingConfig {
            super_key: "meta".to_string(),
            ..KeyBindingConfig::default()
        };
        let result = validate(&config);
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
                validate(&config).is_ok(),
                "expected '{super_key}' to be valid"
            );
        }
    }
}
