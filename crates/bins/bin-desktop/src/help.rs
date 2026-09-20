//! The content of the `?` help overlay -- `gpui`-free, so what it lists is unit-tested without a
//! window. An About-style card: project facts on the left, the shortcut cheat-sheet on the right
//! (the shared grammar lives in `docs/navigation.md`).

use crate::key_router::JUMPS;

/// The tagline under the header.
pub const DESCRIPTION: &str = "Track your personal expenses, investments and assets to help you understand what you have and make informed decisions.";

/// The build channel shown beside the version in the header.
pub const CHANNEL: &str = "concept";

pub const AUTHOR: &str = "Ian Teda";

/// A labelled, clickable link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub label: &'static str,
    pub text: &'static str,
    pub url: &'static str,
}

/// The facts column's links, top to bottom. Author and licence render around these.
pub const LINKS: [Link; 3] = [
    Link {
        label: "Repository",
        text: "github.com/IanTeda/personal-ledger",
        url: "https://github.com/IanTeda/Personal-Ledger",
    },
    Link {
        label: "Documentation",
        text: "ianteda.github.io/personal-ledger",
        url: "https://ianteda.github.io/personal-ledger",
    },
    Link {
        label: "Report an issue",
        text: "github.com/IanTeda/personal-ledger/issues",
        url: "https://github.com/IanTeda/Personal-Ledger/issues",
    },
];

pub const LICENSE_NAME: &str = "GPL-3.0 License";
pub const LICENSE_URL: &str = "https://www.gnu.org/licenses/gpl-3.0.html";
pub const COPYRIGHT: &str = "\u{a9} 2025\u{2013}2026 Ian Teda. All rights reserved.";
pub const FOOTER_NOTE: &str = "Free software \u{2014} no warranty, see the license for details.";

/// The `g`-jump cheat-sheet, as `(noun label, "g x")`, in the router table's own order (the view
/// lays it out row-major in two columns).
pub fn jump_shortcuts() -> Vec<(&'static str, String)> {
    JUMPS
        .iter()
        .map(|(key, _, label)| (*label, format!("g {key}")))
        .collect()
}

/// The non-jump shortcuts, as `(action, keys)`, two columns row-major.
pub const GLOBAL_SHORTCUTS: [(&str, &str); 5] = [
    ("Command palette", ":"),
    ("This help", "?"),
    ("Move focus", "j/k"),
    ("Open / confirm", "enter"),
    ("Close overlay", "esc"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_shortcuts_cover_every_router_jump_in_table_order() {
        let shortcuts = jump_shortcuts();
        assert_eq!(shortcuts.len(), JUMPS.len());
        assert_eq!(shortcuts[0], ("Dashboard", "g d".to_string()));
        assert!(shortcuts.contains(&("Transactions", "g l".to_string())));
        assert!(shortcuts.contains(&("Tags", "g t".to_string())));
    }

    #[test]
    fn links_are_https_and_labelled() {
        for link in LINKS {
            assert!(link.url.starts_with("https://"), "{}", link.label);
            assert!(!link.label.is_empty() && !link.text.is_empty());
        }
        assert!(LICENSE_URL.starts_with("https://"));
    }

    #[test]
    fn the_help_key_and_close_key_are_listed() {
        assert!(GLOBAL_SHORTCUTS.contains(&("This help", "?")));
        assert!(GLOBAL_SHORTCUTS.contains(&("Close overlay", "esc")));
    }
}
