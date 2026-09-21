//! The content of the `?` help overlay -- `gpui`-free, so what it lists is unit-tested without a
//! window. An About-style card: project facts on the left, the shortcut cheat-sheet on the right
//! (the shared grammar lives in `docs/navigation-design.md`).

use crate::key_router::JUMPS;

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

pub const LICENSE_URL: &str = "https://www.gnu.org/licenses/gpl-3.0.html";

/// The copyright line.
pub fn copyright() -> String {
    crate::msg::desktop_help_copyright(AUTHOR)
}

/// The `g`-jump cheat-sheet, as `(noun label, "g x")`, in the router table's own order (the view
/// lays it out row-major in two columns).
pub fn jump_shortcuts() -> Vec<(String, String)> {
    JUMPS
        .iter()
        .map(|(key, noun)| (noun.label(), format!("g {key}")))
        .collect()
}

/// The non-jump shortcuts, as `(action, keys)`, two columns row-major.
pub fn global_shortcuts() -> [(String, &'static str); 5] {
    [
        (crate::msg::desktop_help_shortcut_palette(), ":"),
        (crate::msg::desktop_help_shortcut_help(), "?"),
        (crate::msg::desktop_help_shortcut_focus(), "j/k"),
        (crate::msg::desktop_help_shortcut_confirm(), "enter"),
        (crate::msg::desktop_help_shortcut_close(), "esc"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_shortcuts_cover_every_router_jump_in_table_order() {
        crate::locale::init_for_tests();
        let shortcuts = jump_shortcuts();
        assert_eq!(shortcuts.len(), JUMPS.len());
        assert_eq!(shortcuts[0], ("Dashboard".to_string(), "g d".to_string()));
        assert!(shortcuts.contains(&("Transactions".to_string(), "g l".to_string())));
        assert!(shortcuts.contains(&("Tags".to_string(), "g t".to_string())));
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
        let shortcuts = global_shortcuts();
        assert!(shortcuts.contains(&("This help".to_string(), "?")));
        assert!(shortcuts.contains(&("Close overlay".to_string(), "esc")));
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
            Some("GPL-3.0 License")
        );
        assert_eq!(
            linked(lib_locale::Locale::EnAu).as_deref(),
            Some("GPL-3.0 Licence")
        );
        assert!(linked(lib_locale::Locale::EnXa).is_some());
    }

    #[test]
    fn the_copyright_line_names_the_author() {
        crate::locale::init_for_tests();
        assert!(copyright().contains(AUTHOR));
    }
}
