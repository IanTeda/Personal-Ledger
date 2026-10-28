//! Settings › Categories (TUI-042): a tree per kind, `▾`/`▸` on parents, `├`/`└` connectors and an
//! `n subcategories` note. `→`/`←` expand and collapse, `n` adds a top-level Category and `s` a
//! subcategory under the selected row. Drawn by the list-page pattern ([`super::list`]).
//!
//! A wireframe: the tree is the fixture's until #805's Categories chain moves the TUI onto
//! `lib-categories`, so `n`, `s`, `e` and `d` show the Shell's "not yet built" flash. The counts
//! and the depth are derived from the tree on each draw. Expanding and collapsing is real.

use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, layout::Constraint, layout::Rect};

use super::list::{self, Cell, Column, Page, Row, Table, Tone};
use crate::colours::Colours;
use crate::msg;
use crate::view::Action;

const NOTE_WIDTH: u16 = 20;

// wireframe: TUI-042's twelve Categories, one tree per kind.
struct Node {
    name: &'static str,
    children: &'static [Node],
}

const fn leaf(name: &'static str) -> Node {
    Node {
        name,
        children: &[],
    }
}

const EXPENSE: &[Node] = &[
    Node {
        name: "Housing",
        children: &[leaf("Rent")],
    },
    Node {
        name: "Utilities",
        children: &[leaf("Electricity"), leaf("Water")],
    },
    Node {
        name: "Food",
        children: &[leaf("Groceries"), leaf("Dining")],
    },
    leaf("Transport"),
    leaf("Household"),
];

const INCOME: &[Node] = &[leaf("Salary"), leaf("Interest")];

/// The two trees, in the page's order.
const TREES: [&[Node]; 2] = [EXPENSE, INCOME];

/// A visible row of the flattened trees.
struct Flat {
    tree: usize,
    /// The names from the root, which a collapse is remembered by.
    path: String,
    name: &'static str,
    depth: usize,
    last: bool,
    children: usize,
}

fn count(nodes: &[Node]) -> usize {
    nodes.iter().map(|node| 1 + count(node.children)).sum()
}

fn depth(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + depth(node.children))
        .max()
        .unwrap_or(0)
}

pub(super) struct CategoriesPage {
    selected: usize,
    /// The paths of the parents drawn collapsed.
    collapsed: HashSet<String>,
}

impl CategoriesPage {
    pub(super) fn new() -> Self {
        Self {
            selected: 0,
            collapsed: HashSet::new(),
        }
    }

    /// The rows in view: every node under an expanded parent.
    fn visible(&self) -> Vec<Flat> {
        fn walk(
            nodes: &'static [Node],
            tree: usize,
            depth: usize,
            prefix: &str,
            collapsed: &HashSet<String>,
            out: &mut Vec<Flat>,
        ) {
            for (index, node) in nodes.iter().enumerate() {
                let path = format!("{prefix}/{}", node.name);
                let open = !collapsed.contains(&path);
                out.push(Flat {
                    tree,
                    path: path.clone(),
                    name: node.name,
                    depth,
                    last: index + 1 == nodes.len(),
                    children: node.children.len(),
                });
                if open {
                    walk(node.children, tree, depth + 1, &path, collapsed, out);
                }
            }
        }
        let mut out = Vec::new();
        for (tree, nodes) in TREES.into_iter().enumerate() {
            walk(nodes, tree, 0, &tree.to_string(), &self.collapsed, &mut out);
        }
        out
    }

    fn set_open(&mut self, open: bool) {
        let visible = self.visible();
        let Some(row) = visible.get(self.selected) else {
            return;
        };
        if row.children == 0 {
            return;
        }
        if open {
            self.collapsed.remove(&row.path);
        } else {
            self.collapsed.insert(row.path.clone());
        }
    }

    fn row(&self, flat: &Flat, index: usize) -> Row {
        let mut spans = Vec::new();
        if flat.depth == 0 {
            let mark = if flat.children == 0 {
                "  "
            } else if self.collapsed.contains(&flat.path) {
                "▸ "
            } else {
                "▾ "
            };
            spans.push((mark.to_string(), Tone::Muted));
        } else {
            let connector = if flat.last { "  └ " } else { "  ├ " };
            spans.push((connector.to_string(), Tone::Muted));
        }
        spans.push((flat.name.to_string(), Tone::Plain));
        let note = if flat.children == 0 {
            Cell::muted("")
        } else {
            Cell::muted(msg::tui_settings_categories_subcategories(
                i64::try_from(flat.children).unwrap_or(i64::MAX),
            ))
        };
        Row::Item {
            cells: vec![Cell(spans), note],
            selected: index == self.selected,
            dim: false,
        }
    }
}

impl Page for CategoriesPage {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(step) = list::motion(key) {
            self.selected = lib_list_cursor::step(self.selected, self.visible().len(), step);
            return Some(Action::NoOp);
        }
        if key.modifiers != KeyModifiers::NONE {
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
            KeyCode::Char('n') => Some(Action::FlashNotYetBuilt("category new")),
            KeyCode::Char('s') => Some(Action::FlashNotYetBuilt("category new subcategory")),
            KeyCode::Char('e') => Some(Action::FlashNotYetBuilt("category edit")),
            KeyCode::Char('d') => Some(Action::FlashNotYetBuilt("category delete")),
            KeyCode::Char('o') => Some(Action::OpenCategories),
            _ => None,
        }
    }

    fn scope_note(&self) -> String {
        let total = TREES.iter().map(|nodes| count(nodes)).sum::<usize>();
        let levels = TREES.iter().map(|nodes| depth(nodes)).max().unwrap_or(0);
        msg::tui_settings_scope_categories(
            i64::try_from(total).unwrap_or(i64::MAX),
            i64::try_from(levels).unwrap_or(i64::MAX),
        )
    }

    fn hints(&self) -> Vec<(String, String)> {
        vec![
            ("j/k".into(), msg::tui_settings_hint_row()),
            ("→/←".into(), msg::tui_settings_hint_expand_collapse()),
            ("n".into(), msg::tui_settings_hint_add()),
            ("s".into(), msg::tui_settings_hint_add_sub()),
            ("e".into(), msg::tui_settings_hint_edit()),
            ("d".into(), msg::tui_settings_hint_delete()),
            ("o".into(), msg::tui_settings_hint_open_categories_view()),
        ]
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, focused: bool, c: &Colours) {
        let visible = self.visible();
        let kinds = [
            msg::tui_settings_categories_expense(),
            msg::tui_settings_categories_income(),
        ];
        let tables: Vec<Table> = TREES
            .into_iter()
            .enumerate()
            .map(|(tree, nodes)| Table {
                heading: msg::tui_settings_list_group(
                    &kinds[tree],
                    &crate::format::count(i64::try_from(count(nodes)).unwrap_or(i64::MAX)),
                ),
                add: if tree == 0 {
                    vec![("n".into(), msg::tui_settings_categories_add())]
                } else {
                    Vec::new()
                },
                columns: vec![
                    Column::left(msg::tui_settings_categories_name(), Constraint::Min(0)),
                    Column::left(String::new(), Constraint::Length(NOTE_WIDTH)),
                ],
                rows: visible
                    .iter()
                    .enumerate()
                    .filter(|(_, flat)| flat.tree == tree)
                    .map(|(index, flat)| self.row(flat, index))
                    .collect(),
            })
            .collect();
        let notes = [msg::tui_settings_categories_note()];
        list::render(frame, area, &tables, None, &notes, focused, c);
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

    fn page() -> SettingsView {
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Categories));
        view
    }

    #[test]
    fn the_page_renders_tui_042_with_counts_derived_from_the_tree() {
        crate::locale::init_for_tests();
        let text = render_at(&page(), 96, 30);
        for needle in [
            "Settings › Categories",
            "12 categories · 2 levels deep",
            "EXPENSE · 10",
            "INCOME · 2",
            "n add category",
            "▾ Housing",
            "1 subcategory",
            "2 subcategories",
            "├ Electricity",
            "└ Water",
            "Transport",
            "Salary",
            "Only leaf categories can be picked",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
    }

    #[test]
    fn left_collapses_a_parent_and_right_expands_it_again() {
        crate::locale::init_for_tests();
        let mut view = page();
        view.handle_key(key(KeyCode::Left));
        let collapsed = render_at(&view, 96, 30);
        assert!(collapsed.contains("▸ Housing"), "{collapsed}");
        assert!(!collapsed.contains("└ Rent"), "{collapsed}");
        view.handle_key(key(KeyCode::Right));
        let expanded = render_at(&view, 96, 30);
        assert!(expanded.contains("▾ Housing"), "{expanded}");
        assert!(expanded.contains("└ Rent"), "{expanded}");
    }

    #[test]
    fn s_n_e_and_d_flash_not_yet_built_and_o_opens_the_categories_view() {
        let mut page = CategoriesPage::new();
        for (code, command) in [
            (KeyCode::Char('n'), "category new"),
            (KeyCode::Char('s'), "category new subcategory"),
            (KeyCode::Char('e'), "category edit"),
            (KeyCode::Char('d'), "category delete"),
        ] {
            assert_eq!(
                page.handle_key(key(code)),
                Some(Action::FlashNotYetBuilt(command))
            );
        }
        assert_eq!(
            page.handle_key(key(KeyCode::Char('o'))),
            Some(Action::OpenCategories)
        );
    }
}
