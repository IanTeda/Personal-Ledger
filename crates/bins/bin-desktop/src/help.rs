//! The content of the `?` help overlay -- `gpui`-free, so what it lists is unit-tested without a
//! window. An About-style card: project facts on the left, the shortcut cheat-sheet on the right
//! (the shared grammar lives in `docs/navigation-design.md`).

use lib_shell::ShellCommand;
use lib_shell::keymap::{self, GLOBAL_BINDINGS};
use lib_viper::keymap::{BACK, Keymap, MOVE_DOWN, MOVE_UP, SELECT};

use crate::navigation::nav::NounLabel;

/// The tagline under the header.
pub fn description() -> String {
    crate::msg::desktop_help_description()
}

/// The build channel shown beside the version in the header.
pub const CHANNEL: &str = "concept";

pub const AUTHOR: &str = "Ian Teda";

/// A labelled, clickable link. `id` is the stable element id; `label` is the Message for the
/// visible label.
#[derive(Debug, Clone, Copy)]
pub struct Link {
    pub id: &'static str,
    pub label: fn() -> String,
    pub text: &'static str,
    pub url: &'static str,
}

/// The facts column's links, top to bottom. Author and licence render around these.
pub const LINKS: [Link; 3] = [
    Link {
        id: "repository",
        label: crate::msg::desktop_help_link_repository,
        text: "github.com/IanTeda/personal-ledger",
        url: "https://github.com/IanTeda/Personal-Ledger",
    },
    Link {
        id: "documentation",
        label: crate::msg::desktop_help_link_documentation,
        text: "ianteda.github.io/personal-ledger",
        url: "https://ianteda.github.io/personal-ledger",
    },
    Link {
        id: "issues",
        label: crate::msg::desktop_help_link_issues,
        text: "github.com/IanTeda/personal-ledger/issues",
        url: "https://github.com/IanTeda/Personal-Ledger/issues",
    },
];

pub const LICENSE_URL: &str = "https://github.com/IanTeda/Personal-Ledger/blob/main/LICENSE";

/// The copyright line.
pub fn copyright() -> String {
    crate::msg::desktop_help_copyright(AUTHOR)
}

/// The `g`-jump cheat-sheet, as `(noun label, "g x")`, in the global keymap's own order (the view
/// lays it out row-major in two columns). Each key is the one `keymap` binds now, so a
/// `[keybindings]` override shows here as it does in the status line.
pub fn jump_shortcuts(keymap: &Keymap<ShellCommand>) -> Vec<(String, String)> {
    GLOBAL_BINDINGS
        .iter()
        .filter_map(|binding| match binding.event {
            ShellCommand::Jump(noun) => Some((noun.label(), bound(keymap, binding.command))),
            _ => None,
        })
        .collect()
}

/// The non-jump shortcuts, as `(action, keys)`, two columns row-major, each key read from
/// `keymap` as [`jump_shortcuts`]'s are.
pub fn global_shortcuts(keymap: &Keymap<ShellCommand>) -> Vec<(String, String)> {
    let key = |command| bound(keymap, command);
    vec![
        (
            crate::msg::desktop_help_shortcut_palette(),
            key(keymap::OPEN_COMMAND_POPUP),
        ),
        (crate::msg::desktop_help_shortcut_help(), key(keymap::HELP)),
        (
            crate::msg::desktop_help_shortcut_focus(),
            format!("{}/{}", key(MOVE_DOWN), key(MOVE_UP)),
        ),
        (crate::msg::desktop_help_shortcut_confirm(), key(SELECT)),
        (crate::msg::desktop_help_shortcut_close(), key(BACK)),
    ]
}

/// The chord `command` is bound to, empty when `[keybindings]` unbinds it.
fn bound(keymap: &Keymap<ShellCommand>, command: &str) -> String {
    keymap.chord(command).unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;

    use super::*;

    #[test]
    fn jump_shortcuts_cover_every_noun_in_the_keymaps_order() {
        crate::locale::init_for_tests();
        let shortcuts = jump_shortcuts(&lib_shell::global_keymap(&KeyBindingConfig::default()));
        assert_eq!(shortcuts.len(), lib_shell::Noun::ALL.len());
        assert_eq!(shortcuts[0], ("Dashboard".to_string(), "g d".to_string()));
        assert!(shortcuts.contains(&("Transactions".to_string(), "g l".to_string())));
        assert!(shortcuts.contains(&("Notifications".to_string(), "g a".to_string())));
        assert!(shortcuts.contains(&("Credit cards".to_string(), "g k".to_string())));
    }

    #[test]
    fn the_cheat_sheet_shows_the_keys_keybindings_rebinds() {
        crate::locale::init_for_tests();
        let mut config = KeyBindingConfig::default();
        for (command, chord) in [
            ("jump_dashboard", "g h"),
            ("help", "f1"),
            ("move_down", "down"),
            ("back", "q"),
        ] {
            config
                .bindings
                .insert(command.to_string(), chord.to_string());
        }
        let keymap = lib_shell::global_keymap(&config);

        assert_eq!(
            jump_shortcuts(&keymap)[0],
            ("Dashboard".to_string(), "g h".to_string())
        );
        let shortcuts = global_shortcuts(&keymap);
        assert!(shortcuts.contains(&("This help".to_string(), "f1".to_string())));
        assert!(shortcuts.contains(&("Close overlay".to_string(), "q".to_string())));
        assert!(shortcuts.iter().any(|(_, keys)| keys == "down/k"));
    }

    #[test]
    fn links_are_https_and_labelled() {
        crate::locale::init_for_tests();
        for link in LINKS {
            assert!(link.url.starts_with("https://"), "{}", link.id);
            assert!(!(link.label)().is_empty() && !link.text.is_empty());
        }
        assert!(LICENSE_URL.starts_with("https://"));
    }

    #[test]
    fn the_help_key_and_close_key_are_listed() {
        crate::locale::init_for_tests();
        let shortcuts = global_shortcuts(&lib_shell::global_keymap(&KeyBindingConfig::default()));
        assert!(shortcuts.contains(&("This help".to_string(), "?".to_string())));
        assert!(shortcuts.contains(&("Close overlay".to_string(), "esc".to_string())));
    }

    #[test]
    fn the_licence_line_links_the_licence_and_follows_the_locales_spelling() {
        crate::locale::init_for_tests();
        let linked = |locale| {
            lib_locale::with_locale(locale, || {
                crate::msg::desktop_help_license()
                    .into_iter()
                    .find(|segment| segment.tag.as_deref() == Some("license"))
                    .map(|segment| segment.text)
            })
        };
        assert_eq!(
            linked(lib_locale::Locale::EnUs).as_deref(),
            Some("MIT License")
        );
        assert_eq!(
            linked(lib_locale::Locale::EnAu).as_deref(),
            Some("MIT Licence")
        );
        assert!(linked(lib_locale::Locale::EnXa).is_some());
    }

    #[test]
    fn the_copyright_line_names_the_author() {
        crate::locale::init_for_tests();
        assert!(copyright().contains(AUTHOR));
    }
}
