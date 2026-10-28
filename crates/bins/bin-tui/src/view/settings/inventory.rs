//! Settings › Inventory (TUI-046): a tree of locations and their areas, NAME only. A location row
//! carries a muted `n areas · n items` note, and an area is indented under it, name only. The
//! address and cover live on the register in the Inventory View, not here. `→`/`←` expand and
//! collapse, `J`/`K` move an area among its location's. Drawn by the list-page pattern
//! ([`super::list`]).
//!
//! A wireframe: the tree is the fixture's until #805's Inventory chain moves the TUI onto
//! `lib-inventory`, so `n`, `a`, `e` and `x` show the Shell's "not yet built" flash; expanding,
//! collapsing and reordering are real but last only while the page does. The counts are derived
//! from the tree on each draw.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, layout::Constraint, layout::Rect};

use super::list::{self, Cell, Column, Page, Row, Table, Tone};
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;

const NOTE_WIDTH: u16 = 24;

// wireframe: TUI-046's two locations, their eleven areas and 232 items.
struct Location {
    name: &'static str,
    areas: Vec<(&'static str, i64)>,
    open: bool,
}

fn fixture() -> Vec<Location> {
    vec![
        Location {
            name: "12 Elm St contents",
            areas: vec![
                ("Living", 38),
                ("Kitchen", 46),
                ("Bedroom 1", 24),
                ("Bedroom 2", 16),
                ("Bedroom 3", 12),
                ("Office", 19),
                ("Garage", 27),
                ("Laundry", 11),
                ("Bathroom", 9),
                ("Outdoor & shed", 12),
            ],
            open: true,
        },
        Location {
            name: "Storage unit",
            areas: vec![("Unit 14", 18)],
            open: false,
        },
    ]
}

/// A row in view: a location, or an area by its location and place in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Line {
    Location(usize),
    Area(usize, usize),
}

pub(super) struct InventoryPage {
    locations: Vec<Location>,
    selected: usize,
}

impl InventoryPage {
    pub(super) fn new() -> Self {
        Self {
            locations: fixture(),
            selected: 0,
        }
    }

    fn lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        for (index, location) in self.locations.iter().enumerate() {
            lines.push(Line::Location(index));
            if location.open {
                lines.extend((0..location.areas.len()).map(|area| Line::Area(index, area)));
            }
        }
        lines
    }

    fn set_open(&mut self, open: bool) {
        if let Some(Line::Location(index)) = self.lines().get(self.selected).copied() {
            self.locations[index].open = open;
        }
    }

    /// Swaps the selected area with its neighbour in its location; a location doesn't move.
    fn shift(&mut self, up: bool) {
        let Some(Line::Area(location, area)) = self.lines().get(self.selected).copied() else {
            return;
        };
        let areas = &mut self.locations[location].areas;
        let other = if up {
            area.checked_sub(1)
        } else {
            Some(area + 1).filter(|next| *next < areas.len())
        };
        if let Some(other) = other {
            areas.swap(area, other);
            self.selected = if up {
                self.selected - 1
            } else {
                self.selected + 1
            };
        }
    }

    fn area_count(&self) -> usize {
        self.locations.iter().map(|l| l.areas.len()).sum()
    }

    fn item_count(&self) -> i64 {
        self.locations
            .iter()
            .flat_map(|l| &l.areas)
            .map(|(_, items)| items)
            .sum()
    }
}

impl Page for InventoryPage {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(step) = list::motion(key) {
            self.selected = lib_list_cursor::step(self.selected, self.lines().len(), step);
            return Some(Action::NoOp);
        }
        if key.modifiers != KeyModifiers::NONE && key.modifiers != KeyModifiers::SHIFT {
            return None;
        }
        match key.code {
            KeyCode::Right | KeyCode::Char('l') => {
                self.set_open(true);
                Some(Action::NoOp)
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.set_open(false);
                Some(Action::NoOp)
            }
            KeyCode::Char('J') => {
                self.shift(false);
                Some(Action::NoOp)
            }
            KeyCode::Char('K') => {
                self.shift(true);
                Some(Action::NoOp)
            }
            KeyCode::Char('n') => Some(Action::FlashNotYetBuilt("location new")),
            KeyCode::Char('a') => Some(Action::FlashNotYetBuilt("location area new")),
            KeyCode::Char('e') => Some(Action::FlashNotYetBuilt("location edit")),
            KeyCode::Char('x') => Some(Action::FlashNotYetBuilt("location remove")),
            _ => None,
        }
    }

    fn scope_note(&self) -> String {
        msg::tui_settings_scope_inventory(
            i64::try_from(self.locations.len()).unwrap_or(i64::MAX),
            i64::try_from(self.area_count()).unwrap_or(i64::MAX),
            &crate::format::count(self.item_count()),
        )
    }

    fn hints(&self) -> Vec<(String, String)> {
        vec![
            ("j/k".into(), msg::tui_settings_hint_row()),
            ("→/←".into(), msg::tui_settings_hint_expand_collapse()),
            ("J/K".into(), msg::tui_settings_hint_move_area()),
            ("n".into(), msg::tui_settings_hint_add_location()),
            ("a".into(), msg::tui_settings_hint_add_area()),
            ("e".into(), msg::tui_settings_hint_edit()),
            ("x".into(), msg::tui_settings_hint_remove()),
        ]
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let rows = self
            .lines()
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                let (name, note) = match line {
                    Line::Location(location) => {
                        let location = &self.locations[location];
                        let mark = if location.open { "▾ " } else { "▸ " };
                        let items: i64 = location.areas.iter().map(|(_, items)| items).sum();
                        (
                            Cell(vec![
                                (mark.to_string(), Tone::Muted),
                                (location.name.to_string(), Tone::Plain),
                            ]),
                            Cell::muted(msg::tui_settings_inventory_counts(
                                i64::try_from(location.areas.len()).unwrap_or(i64::MAX),
                                &crate::format::count(items),
                            )),
                        )
                    }
                    Line::Area(location, area) => {
                        let areas = &self.locations[location].areas;
                        let connector = if area + 1 == areas.len() {
                            "  └ "
                        } else {
                            "  ├ "
                        };
                        (
                            Cell(vec![
                                (connector.to_string(), Tone::Muted),
                                (areas[area].0.to_string(), Tone::Plain),
                            ]),
                            Cell::muted(""),
                        )
                    }
                };
                Row::Item {
                    cells: vec![name, note],
                    selected: index == self.selected,
                    dim: false,
                }
            })
            .collect();
        let table = Table {
            heading: msg::tui_settings_inventory_heading(),
            add: vec![
                ("n".into(), msg::tui_settings_inventory_add_location()),
                ("a".into(), msg::tui_settings_inventory_add_area()),
            ],
            columns: vec![
                Column::left(msg::tui_settings_inventory_name(), Constraint::Min(0)),
                Column::left(String::new(), Constraint::Length(NOTE_WIDTH)),
            ],
            rows,
        };
        let notes = [
            msg::tui_settings_inventory_note_register(),
            msg::tui_settings_inventory_note_remove(),
        ];
        list::render(frame, area, &[table], None, &notes, focused, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::View;
    use crate::view::settings::tests::render_at;
    use crate::view::settings::{SettingsPage, SettingsView};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn shift(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
    }

    fn page() -> SettingsView {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Inventory));
        view
    }

    #[test]
    fn the_page_renders_tui_046_with_counts_derived_from_the_tree() {
        crate::locale::init_for_tests();
        let text = render_at(&page(), 96, 30);
        for needle in [
            "Settings › Inventory",
            "2 locations · 11 areas · 232 items",
            "n add location",
            "a add area",
            "▾ 12 Elm St contents",
            "10 areas · 214 items",
            "├ Living",
            "└ Outdoor & shed",
            "▸ Storage unit",
            "1 area · 18 items",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
        assert!(
            !text.contains("Unit 14"),
            "the collapsed location is open:\n{text}"
        );
    }

    #[test]
    fn right_expands_the_storage_unit_and_left_collapses_the_first_location() {
        crate::locale::init_for_tests();
        let mut view = page();
        view.handle_key(key(KeyCode::Left));
        let text = render_at(&view, 96, 30);
        assert!(
            text.contains("▸ 12 Elm St contents") && !text.contains("Kitchen"),
            "{text}"
        );
        view.handle_key(key(KeyCode::Char('j')));
        view.handle_key(key(KeyCode::Right));
        let text = render_at(&view, 96, 30);
        assert!(
            text.contains("▾ Storage unit") && text.contains("Unit 14"),
            "{text}"
        );
    }

    #[test]
    fn shift_j_and_k_reorder_areas_only() {
        let mut page = InventoryPage::new();
        // On a location, J/K do nothing.
        page.handle_key(shift('J'));
        assert_eq!((page.selected, page.locations[0].areas[0].0), (0, "Living"));
        page.handle_key(key(KeyCode::Char('j')));
        page.handle_key(shift('J'));
        assert_eq!(page.selected, 2);
        assert_eq!(page.locations[0].areas[0].0, "Kitchen");
        assert_eq!(page.locations[0].areas[1].0, "Living");
        page.handle_key(shift('K'));
        assert_eq!((page.selected, page.locations[0].areas[0].0), (1, "Living"));
    }

    #[test]
    fn n_a_e_and_x_flash_not_yet_built() {
        let mut page = InventoryPage::new();
        for (code, command) in [
            (KeyCode::Char('n'), "location new"),
            (KeyCode::Char('a'), "location area new"),
            (KeyCode::Char('e'), "location edit"),
            (KeyCode::Char('x'), "location remove"),
        ] {
            assert_eq!(
                page.handle_key(key(code)),
                Some(Action::FlashNotYetBuilt(command))
            );
        }
    }
}
