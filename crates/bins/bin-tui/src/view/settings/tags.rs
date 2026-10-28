//! Settings › Tags (TUI-043): A–Z with a one-cell colour swatch and the name in bold. A likely
//! duplicate carries `⚑ looks like a duplicate of "…"` in words and an inline `m merge`, and the
//! box below says what the merge does before it is pressed. Drawn by the list-page pattern
//! ([`super::list`]).
//!
//! A wireframe: the rows are the fixture's until #805's Tags chain moves the TUI onto `lib-tags`,
//! so `n`, `e`, `x` and `m` show the Shell's "not yet built" flash. The counts are derived from
//! the rows on each draw.

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Constraint, layout::Rect, style::Color};

use super::keys::{self, Binding, Press};
use super::list::{self, Cell, Column, Detail, Page, Row, Table, Tone};
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;

// wireframe: TUI-043's seven Tags, A–Z with the lower case spelling first.
struct FixtureTag {
    name: &'static str,
    colour: Color,
    /// The Tag this one looks like a duplicate of.
    duplicate_of: Option<&'static str>,
}

const FIXTURE: [FixtureTag; 7] = [
    tag("gift", Color::Rgb(0xd9, 0x6e, 0x8c), None),
    tag("one-off", Color::Rgb(0x8a, 0x9b, 0xa8), None),
    tag("reimbursable", Color::Rgb(0x5f, 0xa8, 0x8a), None),
    tag("shared", Color::Rgb(0x6c, 0x8e, 0xd6), None),
    tag("Shared", Color::Rgb(0x6c, 0x8e, 0xd6), Some("shared")),
    tag("tax-deductible", Color::Rgb(0xd6, 0xa3, 0x4c), None),
    tag("work-trip", Color::Rgb(0xa2, 0x7c, 0xd0), None),
];

const fn tag(name: &'static str, colour: Color, duplicate_of: Option<&'static str>) -> FixtureTag {
    FixtureTag {
        name,
        colour,
        duplicate_of,
    }
}

#[derive(Debug, Clone, Copy)]
enum Command {
    Row,
    New,
    Edit,
    Remove,
    Merge,
    OpenView,
}

/// The page's keys and its hint bar, one table (#898).
const KEYS: &[Binding<Command>] = &[
    Binding {
        keys: keys::ROW_KEYS,
        label: "j/k",
        hint: msg::tui_settings_hint_row,
        command: Command::Row,
    },
    Binding {
        keys: &[Press::char('n')],
        label: "n",
        hint: msg::tui_settings_hint_add,
        command: Command::New,
    },
    Binding {
        keys: &[Press::char('e')],
        label: "e",
        hint: msg::tui_settings_hint_edit,
        command: Command::Edit,
    },
    Binding {
        keys: &[Press::char('x')],
        label: "x",
        hint: msg::tui_settings_hint_remove,
        command: Command::Remove,
    },
    Binding {
        keys: &[Press::char('m')],
        label: "m",
        hint: msg::tui_settings_hint_merge,
        command: Command::Merge,
    },
    Binding {
        keys: &[Press::char('o')],
        label: "o",
        hint: msg::tui_settings_hint_open_tags_view,
        command: Command::OpenView,
    },
];

pub(super) struct TagsPage {
    selected: usize,
}

impl TagsPage {
    pub(super) fn new() -> Self {
        Self { selected: 0 }
    }
}

impl Page for TagsPage {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        Some(match keys::lookup(KEYS, key)? {
            Command::Row => {
                let step = list::motion(key)?;
                self.selected = lib_list_cursor::step(self.selected, FIXTURE.len(), step);
                Action::NoOp
            }
            Command::New => Action::FlashNotYetBuilt("tag new"),
            Command::Edit => Action::FlashNotYetBuilt("tag edit"),
            Command::Remove => Action::FlashNotYetBuilt("tag remove"),
            // Only a likely duplicate merges; on any other Tag `m` does nothing.
            Command::Merge if FIXTURE[self.selected].duplicate_of.is_none() => Action::NoOp,
            Command::Merge => Action::FlashNotYetBuilt("tag merge"),
            Command::OpenView => Action::OpenTags,
        })
    }

    fn scope_note(&self) -> String {
        let duplicates = FIXTURE.iter().filter(|t| t.duplicate_of.is_some()).count();
        msg::tui_settings_scope_tags(
            i64::try_from(FIXTURE.len()).unwrap_or(i64::MAX),
            i64::try_from(duplicates).unwrap_or(i64::MAX),
        )
    }

    fn hints(&self) -> Vec<(String, String)> {
        keys::hints(KEYS)
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let rows = FIXTURE
            .iter()
            .enumerate()
            .map(|(index, tag)| {
                let flag = match tag.duplicate_of {
                    Some(of) => Cell(vec![
                        (msg::tui_settings_tags_duplicate(of), Tone::Accent),
                        (" · ".to_string(), Tone::Muted),
                        ("m".to_string(), Tone::Bold),
                        (format!(" {}", msg::tui_settings_hint_merge()), Tone::Muted),
                    ]),
                    None => Cell::muted(""),
                };
                Row::Item {
                    cells: vec![
                        Cell(vec![
                            ("■".to_string(), Tone::Swatch(tag.colour)),
                            (" ".to_string(), Tone::Plain),
                            (tag.name.to_string(), Tone::Bold),
                        ]),
                        flag,
                    ],
                    selected: index == self.selected,
                    dim: false,
                }
            })
            .collect();
        let table = Table {
            heading: msg::tui_settings_sorted_a_z(),
            add: vec![("n".into(), msg::tui_settings_tags_add())],
            columns: vec![
                Column::left(msg::tui_settings_tags_name(), Constraint::Length(24)),
                Column::left(String::new(), Constraint::Min(0)),
            ],
            rows,
        };
        let selected = &FIXTURE[self.selected];
        let detail = selected.duplicate_of.map(|of| Detail {
            title: msg::tui_settings_tags_merge_title(selected.name, of),
            lines: vec![(
                msg::tui_settings_tags_merge_label(),
                Cell::plain(msg::tui_settings_tags_merge_effect(selected.name, of)),
            )],
        });
        let notes = [msg::tui_settings_tags_note()];
        list::render(frame, area, &[table], detail.as_ref(), &notes, focused, c);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;
    use crate::view::View;
    use crate::view::settings::tests::render_at;
    use crate::view::settings::{SettingsPage, SettingsView};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn page() -> SettingsView {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Tags));
        view
    }

    #[test]
    fn the_page_renders_tui_043_and_counts_its_duplicates() {
        crate::locale::init_for_tests();
        let text = render_at(&page(), 96, 30);
        for needle in [
            "Settings › Tags",
            "7 tags · 1 likely duplicate",
            "n add tag",
            "■ gift",
            "tax-deductible",
            "looks like a duplicate of \"shared\"",
            "Usage counts and last-used dates live in the Tags view",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
    }

    #[test]
    fn selecting_the_duplicate_says_what_the_merge_does_and_m_flashes() {
        crate::locale::init_for_tests();
        let mut view = page();
        for _ in 0..4 {
            view.handle_key(key(KeyCode::Char('j')));
        }
        let text = render_at(&view, 96, 30);
        assert!(text.contains("merge Shared → shared"), "{text}");
        assert!(text.contains("Every transaction tagged Shared"), "{text}");
        assert_eq!(
            view.handle_key(key(KeyCode::Char('m'))),
            Some(Action::FlashNotYetBuilt("tag merge"))
        );
    }

    #[test]
    fn m_does_nothing_on_a_tag_with_no_duplicate() {
        let mut page = TagsPage::new();
        assert_eq!(page.handle_key(key(KeyCode::Char('m'))), Some(Action::NoOp));
        assert_eq!(
            page.handle_key(key(KeyCode::Char('x'))),
            Some(Action::FlashNotYetBuilt("tag remove"))
        );
    }

    #[test]
    fn the_hint_bar_is_the_keys_the_page_takes() {
        keys::tests::assert_the_hints_are_the_keys(KEYS, TagsPage::new().hints(), |key| {
            TagsPage::new().handle_key(key)
        });
    }
}
