//! The content of the `?` help overlay -- `gpui`-free, so what it lists is unit-tested without a
//! window. Mirrors `bin-tui`'s help idea (global keys plus the active screen's own, ADR-0013);
//! the shared grammar it documents lives in `docs/navigation.md`.

use crate::{key_router::JUMPS, nav::Noun};

/// One row: the key(s) as typed, and what they do.
pub type Entry = (String, &'static str);

/// A titled group of rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub title: &'static str,
    pub entries: Vec<Entry>,
}

fn entries(rows: &[(&str, &'static str)]) -> Vec<Entry> {
    rows.iter()
        .map(|(keys, action)| ((*keys).to_string(), *action))
        .collect()
}

/// The screen-specific keys for `noun`, or `None` when the screen binds none of its own yet.
fn screen_section(noun: Noun) -> Option<Section> {
    let (title, rows): (&'static str, &[(&str, &'static str)]) = match noun {
        Noun::Accounts => (
            "Accounts",
            &[
                ("j / k", "select the next / previous account"),
                ("enter", "open the selected account's ledger"),
                ("n", "add an account"),
                ("e", "edit the selected account"),
                ("d", "delete the selected account"),
            ],
        ),
        Noun::Transactions => (
            "Transactions",
            &[
                ("j / k", "select the next / previous transaction"),
                ("/", "search descriptions and payees"),
                ("f", "open the filter popover"),
                ("n", "add a transaction (not yet built)"),
                ("e", "edit the selected transaction (not yet built)"),
            ],
        ),
        Noun::Settings => ("Settings", &[("/", "filter the settings index")]),
        _ => return None,
    };
    Some(Section {
        title,
        entries: entries(rows),
    })
}

/// Every section the overlay shows while `noun` is the active screen: the shared grammar first,
/// then that screen's own keys (omitted when it has none), then what only the desktop client has.
pub fn sections(noun: Noun) -> Vec<Section> {
    let mut sections = vec![
        Section {
            title: "Global",
            entries: entries(&[
                (":", "open the command palette"),
                ("/", "search the current screen"),
                ("?", "show or hide this help"),
                ("esc", "close a popup, leave a mode"),
                (
                    "tab / shift-tab",
                    "cycle focus between the rails and the view",
                ),
            ]),
        },
        Section {
            title: "Movement",
            entries: entries(&[
                ("j / k", "next / previous (also down / up)"),
                ("g g / G", "first / last"),
                ("ctrl-d / ctrl-u", "half a page down / up"),
                ("enter", "activate the selection"),
            ]),
        },
        Section {
            title: "Jump",
            entries: JUMPS
                .iter()
                .map(|(key, _, label)| (format!("g {key}"), *label))
                .collect(),
        },
    ];
    sections.extend(screen_section(noun));
    sections.push(Section {
        title: "Desktop",
        entries: entries(&[
            ("b", "collapse or expand the primary rail"),
            (
                "click",
                "a rail row jumps to it; a collapsed row shows a tooltip on hover",
            ),
        ]),
    });
    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(noun: Noun) -> Vec<&'static str> {
        sections(noun).iter().map(|section| section.title).collect()
    }

    #[test]
    fn every_screen_lists_the_shared_grammar_and_the_desktop_extras() {
        for noun in Noun::ALL {
            let titles = titles(noun);
            assert_eq!(&titles[..3], ["Global", "Movement", "Jump"], "{noun:?}");
            assert_eq!(titles.last(), Some(&"Desktop"), "{noun:?}");
        }
    }

    #[test]
    fn screens_with_their_own_keys_get_a_section_between_jump_and_desktop() {
        for (noun, title) in [
            (Noun::Accounts, "Accounts"),
            (Noun::Transactions, "Transactions"),
            (Noun::Settings, "Settings"),
        ] {
            assert_eq!(
                titles(noun),
                ["Global", "Movement", "Jump", title, "Desktop"],
                "{noun:?}"
            );
        }
    }

    #[test]
    fn screens_without_keys_of_their_own_get_no_screen_section() {
        assert_eq!(titles(Noun::Dashboard).len(), 4);
        assert_eq!(titles(Noun::Budgets).len(), 4);
    }

    #[test]
    fn the_jump_section_lists_every_jump_from_the_router_table() {
        let all = sections(Noun::Dashboard);
        let jump = all
            .iter()
            .find(|section| section.title == "Jump")
            .expect("jump section");
        assert_eq!(jump.entries.len(), JUMPS.len());
        assert!(jump.entries.contains(&("g l".to_string(), "Transactions")));
        assert!(jump.entries.contains(&("g t".to_string(), "Tags")));
    }

    #[test]
    fn no_section_is_empty() {
        for noun in Noun::ALL {
            for section in sections(noun) {
                assert!(!section.entries.is_empty(), "{noun:?} {}", section.title);
            }
        }
    }
}
