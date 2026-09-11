//! The Categories `View`, hosted by `Shell` (ADR-0013). Per
//! `docs/ux/tui/categories/README.md` "5a — Categories screen": the left pane is the tree
//! (box-drawing guides, session-local fold state, `N`/`12M` columns) and the summary box for
//! whichever node is selected ("Categories: 5a screen — tree pane and summary box (left
//! pane)"); the right pane, built here, is that same selection's direct-spend line chart and
//! its paged transactions list.
//!
//! Real, interactive state — not a wireframe: `store` ([`CategoryFixture`], from "Categories:
//! fixture data seam and mutable View state pattern") genuinely holds the tree, and
//! `selected`/`folded`/`show_archived`/`show_subtree` are mutated directly in
//! [`CategoriesView::handle_key`], per that ticket's state-ownership decision. Every key
//! handled this way returns [`Action::NoOp`] rather than `None`, so `Shell`'s event loop still
//! redraws immediately instead of waiting for the next `Tick`.
//!
//! The chart's monthly series and the transaction rows are generated on demand
//! (`direct_series`/`transactions_for_node`), deterministically seeded from each node's own id
//! — not stored on `CategoryNode` or in `CategoryFixture`, since nothing else needs them and
//! regenerating ~25 categories' worth of rows on every redraw is trivially cheap at this
//! scale. A real per-transaction fixture (or `lib_database` data, once that ticket lands) would
//! replace this generator wholesale, not extend it.
//!
//! **Keys not wired here**: `n`/`N`/`m`/`e`/`r`/`X`/`a` all reach a popup or the command
//! grammar — each a later ticket's own concern (see the "Categories screen, views and popup"
//! map, issue #106). Also not wired: a bare `g` for "jump to top" — `Shell::map_event` already
//! claims a bare, unmodified `g` globally as its own view-jump leader (`g c`, `g u`, ...)
//! before any `View::handle_key` ever sees it, so the handoff's `g`/`G` top/bottom pair uses
//! `Home`/`G` here instead; `/` filtering, `tab` focus-switching between the tree and the
//! transactions list, and `enter` to open a transaction are all left for later too — the
//! transactions list here is a static display, not yet its own navigable focus.

use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{Months, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use lib_core::{Money, RowID};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Dataset, GraphType, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
};

use crate::category::{CategoryFixture, CategoryNode, CategoryStore};
use crate::view::{Action, View};

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used here for
/// the chart's marked last point.
const ACCENT: Color = Color::Red;

/// How many trailing months the direct-spend chart plots, per the handoff's "~24 points".
const CHART_MONTHS: usize = 24;

/// The chart's fixed end-of-window month (first of month) — every category shares this same
/// trailing 24-month x-axis, per the handoff's "oct 24 – sep 26" being a screen-wide window,
/// not a per-category one; a category's own series is just mostly zero outside where it has
/// activity. Matches `CategoryFixture`'s own fixed "now" (`2026-09-08`).
fn chart_end_month() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 1).expect("fixed literal is a valid date")
}

/// Width of the left pane (tree + summary) — matches `view::units`'s own `LEFT_COLUMN_WIDTH`,
/// so the two screens' left columns line up rather than each picking their own width.
const LEFT_PANE_WIDTH: u16 = 46;

/// Width of the tree's `N` (direct child count) column.
const TREE_N_WIDTH: u16 = 2;

/// Width of the tree's `12M` (rollup) column.
const TREE_ROLLUP_WIDTH: u16 = 8;

/// Width of the summary box's label column, e.g. `"direct · rollup 12m   "`.
const SUMMARY_LABEL_WIDTH: usize = 22;

/// Height of the right pane's spend-chart section: heading, rule, the plot itself, then the
/// first/avg/last labels beneath it — matches `view::units`'s own `WEEKLY_CLOSE_HEIGHT`, the
/// closest existing chart section, so the two screens' charts read at the same scale.
const SPEND_CHART_HEIGHT: u16 = 12;

/// Width of the transactions list's `DATE` column.
const TXN_DATE_WIDTH: u16 = 6;

/// Width of the transactions list's `ACCOUNT` column.
const TXN_ACCOUNT_WIDTH: u16 = 10;

/// Width of the transactions list's right-aligned `AMOUNT` column.
const TXN_AMOUNT_WIDTH: u16 = 8;

/// Width of the transactions list's `CATEGORY` column, shown only in subtree mode.
const TXN_CATEGORY_WIDTH: u16 = 12;

/// One visible line in the rendered tree: either a category row, or the blank spacer line the
/// handoff draws between the `INCOME` and `EXPENSES` root sections.
enum TreeLine {
    Blank,
    Node(TreeRow),
}

/// One category's rendered tree row — already resolved against fold/archived state, so
/// rendering never re-touches `CategoryStore`.
struct TreeRow {
    id: RowID,
    /// Everything before the name: for a root, just its fold glyph (`▾`/`▸`); for a
    /// descendant, the ancestor guide plus this row's own connector plus its glyph (`├─▾`) or,
    /// for a leaf, a filler dash in the glyph's place (`├──`) so the line reads unbroken
    /// ("nothing for a leaf" per the handoff, drawn rather than left blank). Rendered dim, per
    /// the shell's "dim: guides" style role.
    prefix: String,
    name: String,
    is_root: bool,
    is_leaf: bool,
    child_count: usize,
    rollup: Money,
    archived: bool,
}

/// The Categories `View`. Owns the fixture Category tree directly — no navigation-stack
/// `Action` carries it, per `crate::category`'s state-ownership decision — so `handle_key`
/// mutates `store`/`selected`/`folded`/`show_archived` in place (fold, archived-visibility,
/// selection; new/edit/move mutate it too, once their own tickets wire the keys that reach
/// them).
pub struct CategoriesView {
    store: CategoryFixture,
    selected: RowID,
    /// Ids currently folded (children hidden). Session-local, never persisted, per the
    /// handoff's "Fold state is per-node, session-local (not a DB column)". Roots start absent
    /// (expanded); every other node starts present (folded), per "Roots default expanded,
    /// everything else folded on first open."
    folded: Vec<RowID>,
    /// `za` toggles this — archived categories are hidden unless it's `true`.
    show_archived: bool,
    /// `true` right after a lone `z`, awaiting the `a`/`R`/`M` that completes the
    /// `za`/`zR`/`zM` chord — mirrors `Shell`'s own `pending_leader` for `g <letter>`, but
    /// local to this `View` since `Shell` only recognises its own single-letter leader.
    pending_z: bool,
    /// `s` toggles this — the transactions list shows the selected category's own direct
    /// transactions when `false`, or every transaction in its subtree (each row gaining a
    /// category column) when `true`, per the handoff's "5a — Categories screen" right pane.
    show_subtree: bool,
    /// `Some` right after `X` on a tree row — merge has no real logic yet ("Not yet designed"
    /// in the handoff), so this replaces the tree header's stats with the "not yet built"
    /// fallback until any other key is pressed (`handle_key` clears it unconditionally before
    /// matching, then sets it again only if the new key is another `X`).
    merge_hint: Option<&'static str>,
}

impl Default for CategoriesView {
    fn default() -> Self {
        Self::new()
    }
}

impl CategoriesView {
    pub fn new() -> Self {
        let store = CategoryFixture::new();
        // "Roots default expanded, everything else folded on first open" — only non-root
        // parents go in the initial folded set; a leaf has no fold state to speak of (checked
        // via `has_children` everywhere this set is consulted, but there's no reason to carry
        // meaningless entries for it).
        let folded = store
            .nodes()
            .iter()
            .filter(|node| node.parent_id.is_some() && !store.children(node.id).is_empty())
            .map(|node| node.id)
            .collect();
        let selected = store
            .nodes()
            .iter()
            .find(|node| node.parent_id.is_none())
            .map(|node| node.id)
            .expect("CategoryFixture always seeds at least one root");

        Self {
            store,
            selected,
            folded,
            show_archived: false,
            pending_z: false,
            show_subtree: false,
            merge_hint: None,
        }
    }

    fn is_folded(&self, id: RowID) -> bool {
        self.folded.contains(&id)
    }

    fn has_children(&self, id: RowID) -> bool {
        !self.store.children(id).is_empty()
    }

    fn max_depth(&self) -> u32 {
        self.store
            .nodes()
            .iter()
            .map(|node| self.store.depth(node.id))
            .max()
            .unwrap_or(1)
    }

    /// `id`'s full path, lowercase and `/`-joined root-to-self, e.g. `"expenses / food /
    /// groceries"`.
    fn path_of(&self, id: RowID) -> String {
        let mut segments = Vec::new();
        let mut current = self.store.find(id);
        while let Some(node) = current {
            segments.push(node.name.to_lowercase());
            current = node
                .parent_id
                .and_then(|parent_id| self.store.find(parent_id));
        }
        segments.reverse();
        segments.join(" / ")
    }

    /// Every currently-visible line, roots to leaves, respecting fold and archived-visibility
    /// state — the single source of truth both rendering and selection movement walk.
    fn visible_lines(&self) -> Vec<TreeLine> {
        let mut lines = Vec::new();
        let root_ids: Vec<RowID> = self
            .store
            .nodes()
            .iter()
            .filter(|node| node.parent_id.is_none())
            .map(|node| node.id)
            .collect();

        for (index, root_id) in root_ids.iter().enumerate() {
            if index > 0 {
                lines.push(TreeLine::Blank);
            }
            let root = self
                .store
                .find(*root_id)
                .expect("root id came from this store's own nodes()");
            let glyph = self.fold_glyph_or_dash(root.id);
            lines.push(TreeLine::Node(TreeRow {
                id: root.id,
                prefix: glyph.to_string(),
                name: root.name.to_uppercase(),
                is_root: true,
                is_leaf: !self.has_children(root.id),
                child_count: self.store.children(root.id).len(),
                rollup: self.store.rollup(root.id),
                archived: false, // roots can never be archived (CategoryStore::set_active refuses IsRoot)
            }));

            if self.has_children(root.id) && !self.is_folded(root.id) {
                self.push_children(root.id, "", &mut lines);
            }
        }

        lines
    }

    fn fold_glyph_or_dash(&self, id: RowID) -> char {
        if !self.has_children(id) {
            '─'
        } else if self.is_folded(id) {
            '▸'
        } else {
            '▾'
        }
    }

    /// Appends `parent`'s visible children (and, recursively, their own visible children) to
    /// `lines`. `ancestor_prefix` carries only the continuation columns above this level
    /// (`"│ "`/`"  "` per ancestor) — this level's own connector (`"├─"`/`"└─"`) is added here.
    fn push_children(&self, parent: RowID, ancestor_prefix: &str, lines: &mut Vec<TreeLine>) {
        let mut children: Vec<&CategoryNode> = self.store.children(parent);
        if !self.show_archived {
            children.retain(|child| child.active);
        }
        children.sort_by_key(|child| child.name.to_lowercase());

        let count = children.len();
        for (index, child) in children.into_iter().enumerate() {
            let is_last = index + 1 == count;
            let connector = if is_last { "└─" } else { "├─" };
            let glyph = self.fold_glyph_or_dash(child.id);
            let prefix = format!("{ancestor_prefix}{connector}{glyph}");

            lines.push(TreeLine::Node(TreeRow {
                id: child.id,
                prefix,
                name: child.name.clone(),
                is_root: false,
                is_leaf: !self.has_children(child.id),
                child_count: self.store.children(child.id).len(),
                rollup: self.store.rollup(child.id),
                archived: !child.active,
            }));

            if self.has_children(child.id) && !self.is_folded(child.id) {
                let next_prefix = format!("{ancestor_prefix}{}", if is_last { "  " } else { "│ " });
                self.push_children(child.id, &next_prefix, lines);
            }
        }
    }

    fn visible_node_ids(&self) -> Vec<RowID> {
        self.visible_lines()
            .into_iter()
            .filter_map(|line| match line {
                TreeLine::Node(row) => Some(row.id),
                TreeLine::Blank => None,
            })
            .collect()
    }

    fn move_selection(&mut self, delta: isize) {
        let visible = self.visible_node_ids();
        let Some(current_index) = visible.iter().position(|id| *id == self.selected) else {
            return;
        };
        let next_index = (current_index as isize + delta).clamp(0, visible.len() as isize - 1);
        self.selected = visible[next_index as usize];
    }

    fn select_first(&mut self) {
        if let Some(first) = self.visible_node_ids().first() {
            self.selected = *first;
        }
    }

    fn select_last(&mut self) {
        if let Some(last) = self.visible_node_ids().last() {
            self.selected = *last;
        }
    }

    /// `h`: folds the selected node if it's an expanded parent; otherwise (a leaf, or an
    /// already-folded parent — "nothing left to fold here") moves the selection to its parent,
    /// per the handoff's "h on a leaf jumps to its parent".
    fn fold_selected_or_jump_to_parent(&mut self) {
        if self.has_children(self.selected) && !self.is_folded(self.selected) {
            self.folded.push(self.selected);
            return;
        }
        if let Some(parent_id) = self.store.find(self.selected).and_then(|n| n.parent_id) {
            self.selected = parent_id;
        }
    }

    /// `l`: unfolds the selected node if it's a folded parent; otherwise a no-op.
    fn unfold_selected(&mut self) {
        if self.has_children(self.selected) && self.is_folded(self.selected) {
            self.folded.retain(|id| *id != self.selected);
        }
    }

    /// `zR`: unfolds everything.
    fn unfold_all(&mut self) {
        self.folded.clear();
    }

    /// `zM`: folds every node that has children, roots included.
    fn fold_all(&mut self) {
        self.folded = self
            .store
            .nodes()
            .iter()
            .filter(|node| self.has_children(node.id))
            .map(|node| node.id)
            .collect();
    }

    /// Called after any operation that could hide the selected node (folding it away, or
    /// toggling archived-visibility while an archived node is selected) — walks up to the
    /// nearest visible ancestor rather than leaving `selected` pointing at a hidden row.
    fn recover_selection(&mut self) {
        let visible = self.visible_node_ids();
        if visible.contains(&self.selected) {
            return;
        }
        let mut current = self.store.find(self.selected).and_then(|n| n.parent_id);
        while let Some(id) = current {
            if visible.contains(&id) {
                self.selected = id;
                return;
            }
            current = self.store.find(id).and_then(|n| n.parent_id);
        }
        // Every ancestor is somehow hidden too (shouldn't happen — roots are always visible
        // and never archived) — fall back to the first visible row rather than leaving a
        // dangling selection.
        if let Some(first) = visible.first() {
            self.selected = *first;
        }
    }
}

impl View for CategoriesView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        // Any key dismisses a showing merge "not yet built" hint; the `X` arm below sets it
        // straight back if that's the key that was just pressed.
        self.merge_hint = None;

        if self.pending_z {
            self.pending_z = false;
            match key.code {
                KeyCode::Char('a') => self.show_archived = !self.show_archived,
                KeyCode::Char('R') => self.unfold_all(),
                KeyCode::Char('M') => self.fold_all(),
                _ => return Some(Action::NoOp), // aborted chord, nothing changed
            }
            self.recover_selection();
            return Some(Action::NoOp);
        }

        match key.code {
            KeyCode::Char('z') => {
                self.pending_z = true;
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection(1);
                Some(Action::NoOp)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection(-1);
                Some(Action::NoOp)
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.fold_selected_or_jump_to_parent();
                Some(Action::NoOp)
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.unfold_selected();
                Some(Action::NoOp)
            }
            // Bare `g` is Shell's own view-jump leader (see the module doc) and never reaches
            // here, so `Home` stands in for the handoff's lowercase `g` ("top").
            KeyCode::Home => {
                self.select_first();
                Some(Action::NoOp)
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.select_last();
                Some(Action::NoOp)
            }
            KeyCode::Char('s') => {
                self.show_subtree = !self.show_subtree;
                Some(Action::NoOp)
            }
            KeyCode::Char('m') => Some(Action::OpenCategoryMovePopup(self.selected)),
            // `n`: new child of the selection. `N`: new sibling of it — its parent, or a
            // no-op when the selection is a root (roots have no parent to attach a sibling
            // under; there are only ever the two fixed ones).
            KeyCode::Char('n') => Some(Action::OpenCategoryNewPopup(self.selected)),
            KeyCode::Char('N') => self
                .store
                .find(self.selected)
                .and_then(|node| node.parent_id)
                .map(Action::OpenCategoryNewPopup),
            // `e`: opens the edit popup — never for a root (it has no editable name/note/
            // active), same guard as `N`.
            KeyCode::Char('e') => self.store.find(self.selected).and_then(|node| {
                if node.parent_id.is_none() {
                    None
                } else {
                    Some(Action::OpenCategoryEditPopup(self.selected))
                }
            }),
            // `a`: archives the selection directly — no popup, no draft to carry, per the
            // handoff's own quick soft-delete path. Silently no-ops on a root
            // (`CategoryStore::set_active` refuses `IsRoot`).
            KeyCode::Char('a') => {
                let _ = self.store.set_active(self.selected, false);
                Some(Action::NoOp)
            }
            // `X`: merge has no real logic yet ("Not yet designed" in the handoff) — shows the
            // "not yet built" fallback in the tree header instead.
            KeyCode::Char('X') => {
                self.merge_hint = Some("X merge — not yet built");
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    /// Reacts to the Category popup's own confirmed writes (`Shell` relays these after
    /// resolving them against `category_store()` — see `crate::popup::category::move_popup`'s
    /// module doc for the full round trip). Every other `Action` variant is ignored.
    fn update(&mut self, action: &Action) {
        match action {
            Action::MoveCategory { id, new_parent } => {
                let _ = self.store.move_to(*id, *new_parent);
            }
            Action::CreateCategoryChild { parent, name } => {
                let _ = self.store.insert(*parent, name.clone(), None);
            }
            Action::CreateCategory {
                parent,
                name,
                note,
                active,
                ..
            } => {
                if let Ok(id) = self.store.insert(*parent, name.clone(), note.clone())
                    && !active
                {
                    // `insert` always creates active — flip it off if the draft's `active`
                    // checkbox was unticked (rare; `[×]` is the handoff's own default).
                    let _ = self.store.set_active(id, false);
                }
            }
            Action::UpdateCategory {
                id,
                name,
                note,
                active,
            } => {
                let _ = self.store.rename(*id, name.clone());
                let _ = self.store.set_note(*id, note.clone());
                let _ = self.store.set_active(*id, *active);
            }
            _ => {}
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);
        // rows[0] is left blank — breathing space between the shell's title bar and the tree/
        // summary and spend-chart/transactions boxes, matching `view::units`/`view::dashboard`'s
        // own leading spacer row.

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(LEFT_PANE_WIDTH), Constraint::Min(0)])
            .spacing(2)
            .split(rows[1]);

        self.render_left_pane(frame, columns[0]);
        self.render_right_pane(frame, columns[1]);
    }

    fn title(&self) -> &'static str {
        "Categories"
    }

    fn category_store(&self) -> Option<&dyn CategoryStore> {
        Some(&self.store)
    }

    fn category_selection(&self) -> Option<RowID> {
        Some(self.selected)
    }
}

impl CategoriesView {
    fn render_left_pane(&self, frame: &mut Frame, area: Rect) {
        let node = self
            .store
            .find(self.selected)
            .expect("selected always points at a real node in this store");
        let rollup = self.store.rollup(self.selected);
        let merged = node.direct == rollup;

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(summary_section_height(merged)),
            ])
            .split(area);

        self.render_tree(frame, rows[0]);
        self.render_summary(frame, rows[1], node, &rollup, merged);
    }

    /// The tree list: its `tree N of M · depth D · K folded` header, a rule, the `N`/`12M`
    /// column header, then the rows themselves with a scrollbar riding the right edge.
    fn render_tree(&self, frame: &mut Frame, area: Rect) {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .spacing(1)
            .split(area);
        let content_area = split[0];
        let scrollbar_column = split[1];

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Length(1), // rule
                Constraint::Length(1), // column header
                Constraint::Min(0),    // rows
            ])
            .split(content_area);

        let lines = self.visible_lines();
        let visible_count = lines
            .iter()
            .filter(|line| matches!(line, TreeLine::Node(_)))
            .count();
        let folded_with_children = self
            .folded
            .iter()
            .filter(|id| self.has_children(**id))
            .count();

        self.render_tree_header(frame, sections[0], visible_count, folded_with_children);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
        render_tree_column_header(frame, sections[2]);
        render_tree_rows(frame, sections[3], &lines, self.selected);

        let rows_scrollbar_area = Rect {
            y: sections[3].y,
            height: sections[3].height,
            ..scrollbar_column
        };
        let total = self.store.nodes().len();
        let mut scrollbar_state = ScrollbarState::new(total)
            .viewport_content_length(visible_count)
            .position(0);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, rows_scrollbar_area, &mut scrollbar_state);
    }

    fn render_tree_header(
        &self,
        frame: &mut Frame,
        area: Rect,
        visible_count: usize,
        folded_count: usize,
    ) {
        let text = match self.merge_hint {
            Some(hint) => hint.to_string(),
            None => {
                let total = self.store.nodes().len();
                format!(
                    "tree {visible_count} of {total} · depth {} · {folded_count} folded",
                    self.max_depth()
                )
            }
        };
        let style = if self.merge_hint.is_some() {
            Style::default().fg(ACCENT)
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };
        frame.render_widget(Paragraph::new(Span::styled(text, style)), area);
    }

    /// The summary box beneath the tree, for whichever node is selected — see the handoff's
    /// own worked example (`docs/ux/tui/categories/README.md` "Summary box").
    fn render_summary(
        &self,
        frame: &mut Frame,
        area: Rect,
        node: &CategoryNode,
        rollup: &Money,
        merged: bool,
    ) {
        let block = Block::bordered().padding(Padding::horizontal(1));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let money_rows = if merged { 1 } else { 2 };
        let mut constraints = vec![Constraint::Length(1), Constraint::Length(1)]; // path, rule
        constraints.extend(std::iter::repeat_n(Constraint::Length(1), money_rows));
        constraints.push(Constraint::Length(1)); // rule
        constraints.extend(std::iter::repeat_n(Constraint::Length(1), 6)); // kind·depth, children, transactions, first·last, note, active
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let mut row = 0usize;
        frame.render_widget(Paragraph::new(self.path_of(node.id)), rows[row]);
        row += 1;
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[row]);
        row += 1;

        if merged {
            frame.render_widget(
                summary_field_line("direct · rollup 12m", &format_money(rollup)),
                rows[row],
            );
            row += 1;
        } else {
            frame.render_widget(
                summary_field_line("direct 12m", &format_money(&node.direct)),
                rows[row],
            );
            row += 1;
            frame.render_widget(
                summary_field_line("rollup 12m", &format_money(rollup)),
                rows[row],
            );
            row += 1;
        }
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[row]);
        row += 1;

        let kind = self
            .store
            .kind(node.id)
            .map(|kind| kind.as_str())
            .unwrap_or("—");
        frame.render_widget(
            summary_field_line(
                "kind · depth",
                &format!("{kind} · {}", self.store.depth(node.id)),
            ),
            rows[row],
        );
        row += 1;

        let child_count = self.store.children(node.id).len();
        let children_text = if child_count == 0 {
            "none · leaf".to_string()
        } else {
            format!("{child_count} · parent")
        };
        frame.render_widget(summary_field_line("children", &children_text), rows[row]);
        row += 1;

        let transactions_text = if node.transaction_count == 0 {
            "none".to_string()
        } else {
            format!("{} · direct", node.transaction_count)
        };
        frame.render_widget(
            summary_field_line("transactions", &transactions_text),
            rows[row],
        );
        row += 1;

        let first_last = match (node.first_posted, node.last_posted) {
            (Some(first), Some(last)) => {
                format!("{} · {}", format_date(first), format_date(last))
            }
            _ => "none".to_string(),
        };
        frame.render_widget(summary_field_line("first · last", &first_last), rows[row]);
        row += 1;

        frame.render_widget(
            summary_field_line("note", node.note.as_deref().unwrap_or("—")),
            rows[row],
        );
        row += 1;

        let active_text = if node.active {
            "[×] · offered"
        } else {
            "[ ] · not offered"
        };
        frame.render_widget(summary_field_line("active", active_text), rows[row]);
    }

    fn render_right_pane(&self, frame: &mut Frame, area: Rect) {
        let node = self
            .store
            .find(self.selected)
            .expect("selected always points at a real node in this store");

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(SPEND_CHART_HEIGHT),
                Constraint::Length(1), // spacer
                Constraint::Min(0),
            ])
            .split(area);

        self.render_spend_chart(frame, rows[0], node);
        self.render_transactions(frame, rows[2], node);
    }

    /// The direct-spend line chart: a trailing `CHART_MONTHS`-month window, per the handoff's
    /// "5a — Categories screen" right pane. Plots `direct` spend, never rollup — a parent's
    /// rollup line would otherwise silently include its children and contradict the tree.
    fn render_spend_chart(&self, frame: &mut Frame, area: Rect, node: &CategoryNode) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // heading
                Constraint::Length(1), // rule
                Constraint::Min(0),    // chart
                Constraint::Length(1), // labels
            ])
            .split(area);

        render_chart_heading(frame, rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

        let series = direct_series(node);
        let points: Vec<(f64, f64)> = series
            .iter()
            .enumerate()
            .map(|(index, &amount)| (index as f64, amount))
            .collect();
        let max_amount = series.iter().copied().fold(0.0_f64, f64::max).max(1.0);
        let avg = series.iter().sum::<f64>() / series.len() as f64;

        // The average line and the last-point marker are separate `Dataset`s layered over the
        // spend line, per the handoff's "dashed average rule ... last point marked" — ratatui
        // has no literal dash pattern for a `Dataset`, so the average line is dim instead
        // (matches the fidelity note: authoritative on structure, not on exact styling).
        let line = Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default())
            .data(&points);

        let avg_points = [(0.0, avg), ((CHART_MONTHS - 1) as f64, avg)];
        let avg_line = Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().add_modifier(Modifier::DIM))
            .data(&avg_points);

        let last_point = [((CHART_MONTHS - 1) as f64, series[CHART_MONTHS - 1])];
        let last_point_marker = Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(ACCENT))
            .data(&last_point);

        let chart = Chart::new(vec![line, avg_line, last_point_marker])
            .x_axis(Axis::default().bounds([0.0, (CHART_MONTHS - 1) as f64]))
            .y_axis(Axis::default().bounds([0.0, max_amount * 1.1]));
        frame.render_widget(chart, rows[2]);

        render_chart_labels(frame, rows[3], &series, avg);
    }

    /// The transactions list: `DATE`/`PAYEE`/`ACCOUNT`/`AMOUNT` in direct mode, gaining a
    /// `CATEGORY` column (at `PAYEE`'s expense) in subtree mode — "the one time PAYEE gives up
    /// width", per the handoff.
    fn render_transactions(&self, frame: &mut Frame, area: Rect, node: &CategoryNode) {
        let rows = self.transaction_rows(node);
        let direct_count = node.transaction_count;
        let subtree_count = self.subtree_transaction_count(node.id);
        let total = if self.show_subtree {
            direct_count + subtree_count
        } else {
            direct_count
        };

        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .spacing(1)
            .split(area);
        let content_area = split[0];
        let scrollbar_column = split[1];

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // heading
                Constraint::Length(1), // rule
                Constraint::Length(1), // column header
                Constraint::Min(0),    // rows
                Constraint::Length(1), // footer
            ])
            .split(content_area);

        render_transactions_heading(frame, sections[0], rows.len(), total);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
        render_transactions_column_header(frame, sections[2], self.show_subtree);
        render_transaction_rows(frame, sections[3], &rows, self.show_subtree);
        render_transactions_footer(frame, sections[4], direct_count, subtree_count);

        let rows_scrollbar_area = Rect {
            y: sections[3].y,
            height: sections[3].height,
            ..scrollbar_column
        };
        let mut scrollbar_state = ScrollbarState::new(total as usize)
            .viewport_content_length(rows.len())
            .position(0);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, rows_scrollbar_area, &mut scrollbar_state);
    }

    /// Every transaction count in `id`'s subtree, excluding `id` itself — the handoff's "in
    /// subtree" figure (`0` for a leaf, the descendants' total for a parent).
    fn subtree_transaction_count(&self, id: RowID) -> u32 {
        self.store
            .descendants(id)
            .into_iter()
            .filter(|descendant_id| *descendant_id != id)
            .filter_map(|descendant_id| self.store.find(descendant_id))
            .map(|descendant| descendant.transaction_count)
            .sum()
    }

    /// The rows this list currently shows: just `node`'s own (direct mode), or every
    /// descendant's too (subtree mode, each row keeping its own category's name) — capped to
    /// the newest 10, per the handoff's "10 of 148 · newest first" (no further pagination
    /// controls built here).
    fn transaction_rows(&self, node: &CategoryNode) -> Vec<TransactionRow> {
        let mut rows = if self.show_subtree {
            self.store
                .descendants(node.id)
                .into_iter()
                .filter_map(|id| self.store.find(id))
                .flat_map(transactions_for_node)
                .collect::<Vec<_>>()
        } else {
            transactions_for_node(node)
        };
        rows.sort_by_key(|row| std::cmp::Reverse(row.date));
        rows.truncate(10);
        rows
    }
}

/// The `N`/`12M` column header row, dim, per the handoff's "Column heads dim and uppercase".
fn render_tree_column_header(frame: &mut Frame, area: Rect) {
    let columns = tree_row_columns(area);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled("N", dim)).alignment(Alignment::Right),
        columns[1],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("12M", dim)).alignment(Alignment::Right),
        columns[2],
    );
}

fn render_tree_rows(frame: &mut Frame, area: Rect, lines: &[TreeLine], selected: RowID) {
    let visible = lines.len().min(area.height as usize);
    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), visible).collect();
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    for (line, row_area) in lines.iter().zip(row_areas.iter()) {
        if let TreeLine::Node(row) = line {
            render_tree_row(frame, *row_area, row, row.id == selected);
        }
    }
}

fn render_tree_row(frame: &mut Frame, area: Rect, row: &TreeRow, selected: bool) {
    if selected {
        frame.render_widget(
            Block::new().style(Style::default().add_modifier(Modifier::REVERSED)),
            area,
        );
    }

    let dim = Style::default().add_modifier(Modifier::DIM);
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let prefix_style = if selected { Style::default() } else { dim };
    let name_style = if row.is_root {
        bold
    } else if row.archived && !selected {
        dim
    } else {
        Style::default()
    };

    let columns = tree_row_columns(area);
    let name_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(row.prefix.chars().count() as u16 + 1),
            Constraint::Min(0),
        ])
        .split(columns[0]);

    frame.render_widget(
        Paragraph::new(Span::styled(&row.prefix, prefix_style)),
        name_columns[0],
    );
    let name_text = if row.archived {
        format!("{} · archived", row.name)
    } else {
        row.name.clone()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(name_text, name_style)),
        name_columns[1],
    );

    let n_style = if selected { Style::default() } else { dim };
    let n_text = if row.is_leaf {
        "—".to_string()
    } else {
        row.child_count.to_string()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(n_text, n_style)).alignment(Alignment::Right),
        columns[1],
    );

    let rollup_style = if selected { Style::default() } else { n_style };
    frame.render_widget(
        Paragraph::new(Span::styled(format_money_whole(&row.rollup), rollup_style))
            .alignment(Alignment::Right),
        columns[2],
    );
}

/// Splits a tree row (or its column header) into name (`Min(0)`) / `N` / `12M` columns.
fn tree_row_columns(area: Rect) -> [Rect; 3] {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(TREE_N_WIDTH),
            Constraint::Length(TREE_ROLLUP_WIDTH),
        ])
        .spacing(1)
        .split(area);
    [columns[0], columns[1], columns[2]]
}

/// One `label   value` summary row, the label padded to [`SUMMARY_LABEL_WIDTH`] and dimmed —
/// mirrors `view::units`'s own `summary_field_line`.
fn summary_field_line<'a>(label: &'a str, value: &'a str) -> Paragraph<'a> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    Paragraph::new(Line::from(vec![
        Span::styled(format!("{label:<SUMMARY_LABEL_WIDTH$}"), dim),
        Span::raw(value),
    ]))
}

/// Height of the summary box's bordered content: path, a rule, the direct/rollup row(s), a
/// rule, then the six fixed fields (`kind · depth`, `children`, `transactions`, `first ·
/// last`, `note`, `active`).
fn summary_section_height(merged: bool) -> u16 {
    let money_rows = if merged { 1 } else { 2 };
    let content_rows = 2 + money_rows + 1 + 6;
    content_rows + 2 // top/bottom border
}

fn format_date(date: NaiveDate) -> String {
    date.format("%d %b").to_string().to_lowercase()
}

/// Formats a `Money` amount with a space thousands-separator, keeping decimals only when the
/// amount actually has a fractional part (e.g. `142100` -> `"142 100"`, `12480.40` ->
/// `"12 480.40"`) — used by the summary box, where full precision matters.
fn format_money(value: &Money) -> String {
    let normalized = if value.0.is_integer() {
        value.0.with_scale(0)
    } else {
        value.0.with_scale(2)
    };
    group_thousands(&normalized.to_plain_string())
}

/// Formats a `Money` amount rounded to whole dollars, with a space thousands-separator — used
/// by the tree's narrow `12M` column, which the handoff draws without cents (see
/// `CategoryFixture`'s own module doc on `Food`'s rollup).
fn format_money_whole(value: &Money) -> String {
    group_thousands(&value.0.with_scale(0).to_plain_string())
}

fn group_thousands(plain: &str) -> String {
    let (sign, rest) = match plain.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", plain),
    };
    let (int_part, frac_part) = match rest.split_once('.') {
        Some((int_part, frac_part)) => (int_part, Some(frac_part)),
        None => (rest, None),
    };

    let mut grouped: String = int_part
        .chars()
        .rev()
        .enumerate()
        .flat_map(|(index, ch)| {
            let mut chars = Vec::with_capacity(2);
            if index != 0 && index % 3 == 0 {
                chars.push(' ');
            }
            chars.push(ch);
            chars
        })
        .collect();
    grouped = grouped.chars().rev().collect();

    match frac_part {
        Some(frac_part) => format!("{sign}{grouped}.{frac_part}"),
        None => format!("{sign}{grouped}"),
    }
}

/// The "DIRECT SPEND" heading over the chart, with the trailing window as its dim tag — the
/// handoff's own "the header says so" call-out that this plots direct, never rollup.
fn render_chart_heading(frame: &mut Frame, area: Rect) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let tag = format!(
        "{} – {}",
        format_month(chart_month(0)),
        format_month(chart_month(CHART_MONTHS - 1))
    );
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(Span::styled("DIRECT SPEND", dim)),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The row beneath the chart: first month + its value, the average, last month + its value —
/// per the handoff's `oct 24  712 / avg 1 040 / sep 26  904`.
fn render_chart_labels(frame: &mut Frame, area: Rect, series: &[f64; CHART_MONTHS], avg: f64) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Min(0), Constraint::Min(0)])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    let first_text = format!(
        "{}  {}",
        format_month(chart_month(0)),
        format_f64_whole(series[0])
    );
    let avg_text = format!("avg {}", format_f64_whole(avg));
    let last_text = format!(
        "{}  {}",
        format_month(chart_month(CHART_MONTHS - 1)),
        format_f64_whole(series[CHART_MONTHS - 1])
    );

    frame.render_widget(Paragraph::new(Span::styled(first_text, dim)), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(avg_text, dim)).alignment(Alignment::Center),
        columns[1],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(last_text, dim)).alignment(Alignment::Right),
        columns[2],
    );
}

/// A whole-dollar amount with a space thousands-separator, no decimals — for the chart's
/// first/avg/last labels, which the handoff shows without cents.
fn format_f64_whole(amount: f64) -> String {
    group_thousands(&format!("{:.0}", amount.round()))
}

/// The chart's x-axis month for `index` (`0` oldest, `CHART_MONTHS - 1` newest).
fn chart_month(index: usize) -> NaiveDate {
    chart_end_month() - Months::new((CHART_MONTHS - 1 - index) as u32)
}

fn format_month(date: NaiveDate) -> String {
    date.format("%b %y").to_string().to_lowercase()
}

/// The "TRANSACTIONS N of M · newest first" heading.
fn render_transactions_heading(frame: &mut Frame, area: Rect, shown: usize, total: u32) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let tag = format!("{shown} of {total} · newest first");
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(Span::styled("TRANSACTIONS", dim)),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// Splits a transactions row (or its column header) into `DATE`/`PAYEE`/`ACCOUNT`/(`CATEGORY`
/// only in subtree mode)/`AMOUNT` columns — "the one time PAYEE gives up width", per the
/// handoff.
fn transaction_columns(area: Rect, show_category: bool) -> (Rect, Rect, Rect, Option<Rect>, Rect) {
    let mut constraints = vec![
        Constraint::Length(TXN_DATE_WIDTH),
        Constraint::Min(0),
        Constraint::Length(TXN_ACCOUNT_WIDTH),
    ];
    if show_category {
        constraints.push(Constraint::Length(TXN_CATEGORY_WIDTH));
    }
    constraints.push(Constraint::Length(TXN_AMOUNT_WIDTH));

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .spacing(1)
        .split(area);

    if show_category {
        (
            columns[0],
            columns[1],
            columns[2],
            Some(columns[3]),
            columns[4],
        )
    } else {
        (columns[0], columns[1], columns[2], None, columns[3])
    }
}

fn render_transactions_column_header(frame: &mut Frame, area: Rect, show_category: bool) {
    let (date, payee, account, category, amount) = transaction_columns(area, show_category);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled("DATE", dim)), date);
    frame.render_widget(Paragraph::new(Span::styled("PAYEE", dim)), payee);
    frame.render_widget(Paragraph::new(Span::styled("ACCOUNT", dim)), account);
    if let Some(category_area) = category {
        frame.render_widget(Paragraph::new(Span::styled("CATEGORY", dim)), category_area);
    }
    frame.render_widget(
        Paragraph::new(Span::styled("AMOUNT", dim)).alignment(Alignment::Right),
        amount,
    );
}

fn render_transaction_rows(
    frame: &mut Frame,
    area: Rect,
    rows: &[TransactionRow],
    show_category: bool,
) {
    let visible = rows.len().min(area.height as usize);
    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), visible).collect();
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    for (row, row_area) in rows.iter().zip(row_areas.iter()) {
        let (date, payee, account, category, amount) =
            transaction_columns(*row_area, show_category);
        frame.render_widget(Paragraph::new(format_date(row.date)), date);
        frame.render_widget(Paragraph::new(row.payee), payee);
        frame.render_widget(Paragraph::new(row.account), account);
        if let Some(category_area) = category {
            frame.render_widget(Paragraph::new(row.category_name.as_str()), category_area);
        }
        frame.render_widget(
            Paragraph::new(format_money(&row.amount)).alignment(Alignment::Right),
            amount,
        );
    }
}

/// The `N direct · M in subtree  ·  enter open txn` footer row — `enter` isn't wired to
/// anything yet (the transactions list has no navigable focus of its own here, see the module
/// doc), the hint is shown as-is regardless.
fn render_transactions_footer(frame: &mut Frame, area: Rect, direct: u32, subtree: u32) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let text = format!("{direct} direct · {subtree} in subtree  ·  enter open txn");
    frame.render_widget(Paragraph::new(Span::styled(text, dim)), area);
}

/// One generated fake transaction row — see the module doc on why these are generated on
/// demand rather than stored on `CategoryNode`/`CategoryFixture`.
struct TransactionRow {
    date: NaiveDate,
    payee: &'static str,
    account: &'static str,
    amount: Money,
    category_name: String,
}

const FAKE_PAYEES: &[&str] = &["Woolworths", "Coles", "IGA", "Farmers Market", "Aldi"];
const FAKE_ACCOUNTS: &[&str] = &["Everyday", "Credit Card", "Joint Account"];

/// A tiny xorshift PRNG step — deterministic across runs/platforms, the same technique
/// `view::dashboard`'s own fake data already uses (no `rand` dependency needed).
fn xorshift(seed: u64) -> u64 {
    let mut seed = seed;
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}

/// A deterministic seed derived from a `RowID`, so the same category always generates the
/// same fake chart series and transaction rows.
fn seed_from_id(id: RowID) -> u64 {
    let uuid = id.into_uuid();
    let bytes = uuid.as_bytes();
    u64::from_be_bytes(
        bytes[8..16]
            .try_into()
            .expect("a uuid's byte array is always at least 16 bytes long"),
    )
}

fn money_to_f64(value: &Money) -> f64 {
    value.0.to_string().parse().unwrap_or(0.0)
}

fn f64_to_money(amount: f64) -> Money {
    Money(BigDecimal::from_f64(amount).unwrap_or_default())
}

/// `node`'s direct spend distributed pseudo-randomly across the most recent
/// `node.transaction_count.min(CHART_MONTHS)` months of the chart's trailing window, so the
/// series always sums to exactly `node.direct`. A node with no direct postings of its own
/// (`transaction_count == 0`, e.g. every parent in this fixture) gets a flat zero series.
fn direct_series(node: &CategoryNode) -> [f64; CHART_MONTHS] {
    let mut points = [0.0_f64; CHART_MONTHS];
    if node.transaction_count == 0 {
        return points;
    }

    let total = money_to_f64(&node.direct);
    let active = (node.transaction_count as usize).clamp(1, CHART_MONTHS);
    let mut seed = seed_from_id(node.id);
    let mut weights = Vec::with_capacity(active);
    let mut weight_sum = 0.0;
    for _ in 0..active {
        seed = xorshift(seed);
        let weight = 0.5 + (seed % 100) as f64 / 100.0; // 0.5..1.5
        weight_sum += weight;
        weights.push(weight);
    }

    for (offset, weight) in weights.into_iter().enumerate() {
        points[CHART_MONTHS - active + offset] = total * weight / weight_sum;
    }
    points
}

/// `node`'s own fake transaction rows, newest first — empty for a node with no direct
/// postings (parents in this fixture, per the module doc).
fn transactions_for_node(node: &CategoryNode) -> Vec<TransactionRow> {
    let count = node.transaction_count as usize;
    let (Some(first), Some(last)) = (node.first_posted, node.last_posted) else {
        return Vec::new();
    };
    if count == 0 {
        return Vec::new();
    }

    let span_days = (last - first).num_days().max(0) as u64;
    let per_transaction = money_to_f64(&node.direct) / count as f64;
    let mut seed = seed_from_id(node.id);
    let mut rows = Vec::with_capacity(count);

    for _ in 0..count {
        seed = xorshift(seed);
        let offset_days = (seed % (span_days + 1)) as i64;
        let date = first + chrono::Duration::days(offset_days);

        seed = xorshift(seed);
        let payee = FAKE_PAYEES[(seed as usize) % FAKE_PAYEES.len()];
        seed = xorshift(seed);
        let account = FAKE_ACCOUNTS[(seed as usize) % FAKE_ACCOUNTS.len()];
        seed = xorshift(seed);
        let wobble = 0.6 + (seed % 80) as f64 / 100.0; // 0.6..1.4

        rows.push(TransactionRow {
            date,
            payee,
            account,
            amount: f64_to_money(per_transaction * wobble),
            category_name: node.name.clone(),
        });
    }

    rows.sort_by_key(|row| std::cmp::Reverse(row.date));
    rows
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(view: &CategoriesView) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering the Categories view should not error");

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    fn find_by_name(view: &CategoriesView, name: &str) -> RowID {
        view.store
            .nodes()
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a category named {name}"))
            .id
    }

    #[test]
    fn renders_without_panicking() {
        render(&CategoriesView::new());
    }

    #[test]
    fn leaves_a_blank_row_between_the_shells_title_bar_and_the_tree() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        let view = CategoriesView::new();
        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering should not error");

        let buffer = terminal.backend().buffer();
        let row_is_blank = |y: u16| -> bool {
            (0..buffer.area.width).all(|x| buffer[(x, y)].symbol().trim().is_empty())
        };
        assert!(
            row_is_blank(0),
            "row 0 should be a blank spacer, matching view::units/view::dashboard's own \
             leading breathing-space row"
        );
        assert!(
            !row_is_blank(1),
            "row 1 should hold real content (the tree header)"
        );
    }

    #[test]
    fn left_pane_is_46_wide_with_a_2_col_gap_before_the_right_pane() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        let view = CategoriesView::new();
        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering should not error");

        let buffer = terminal.backend().buffer();
        let row_text = |y: u16| -> String {
            let mut row = String::new();
            for x in 0..buffer.area.width {
                row.push_str(buffer[(x, y)].symbol());
            }
            row
        };
        // The tree header and the "DIRECT SPEND" heading are both the first row of their own
        // pane, so they land on the same absolute row.
        let header_row = (0..buffer.area.height)
            .find(|&y| row_text(y).contains("DIRECT SPEND"))
            .expect("DIRECT SPEND heading should be on some row");
        assert!(
            row_text(header_row).contains("tree"),
            "the left pane's tree header should be on the same row as DIRECT SPEND"
        );

        for x in LEFT_PANE_WIDTH..LEFT_PANE_WIDTH + 2 {
            assert_eq!(
                buffer[(x, header_row)].symbol().trim(),
                "",
                "expected a blank 2-column gap between the left and right panes at x={x}"
            );
        }
    }

    #[test]
    fn title_is_categories() {
        assert_eq!(CategoriesView::new().title(), "Categories");
    }

    #[test]
    fn owns_a_seeded_fixture_tree_with_both_roots() {
        let view = CategoriesView::new();
        let roots: Vec<_> = view
            .store
            .nodes()
            .iter()
            .filter(|node| node.parent_id.is_none())
            .collect();
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn starts_with_roots_and_their_immediate_children_visible() {
        // "Roots default expanded, everything else folded on first open": the roots' own
        // children show as rows (2 for Income, 4 for Expenses), but not any grandchildren.
        let view = CategoriesView::new();
        assert_eq!(view.visible_node_ids().len(), 2 + 2 + 4);
    }

    #[test]
    fn starts_selected_on_the_income_root() {
        let view = CategoriesView::new();
        let income = find_by_name(&view, "Income");
        assert_eq!(view.selected, income);
    }

    #[test]
    fn shows_both_root_names_uppercase_and_bold() {
        let text = render(&CategoriesView::new());
        assert!(text.contains("INCOME"), "INCOME root missing");
        assert!(text.contains("EXPENSES"), "EXPENSES root missing");
    }

    #[test]
    fn shows_the_tree_header_and_column_header() {
        let view = CategoriesView::new();
        let total = view.store.nodes().len();
        let text = render(&view);
        assert!(
            text.contains(&format!("tree 8 of {total}")),
            "tree header missing"
        );
        assert!(
            text.contains("depth 4"),
            "depth stat missing (Housing/Mortgage/Interest is 4)"
        );
        assert!(text.contains('N'), "N column header missing");
        assert!(text.contains("12M"), "12M column header missing");
    }

    #[test]
    fn j_and_k_move_the_selection_between_visible_rows() {
        let mut view = CategoriesView::new();
        let income = find_by_name(&view, "Income");
        let second_visible = view.visible_node_ids()[1];
        assert_eq!(view.selected, income);

        view.handle_key(key(KeyCode::Char('j')));
        assert_eq!(view.selected, second_visible);

        view.handle_key(key(KeyCode::Char('k')));
        assert_eq!(view.selected, income);
    }

    #[test]
    fn j_returns_no_op_so_shell_redraws_immediately() {
        let mut view = CategoriesView::new();
        assert_eq!(view.handle_key(key(KeyCode::Char('j'))), Some(Action::NoOp));
    }

    #[test]
    fn move_selection_clamps_rather_than_wraps() {
        let mut view = CategoriesView::new();
        let income = find_by_name(&view, "Income");
        view.handle_key(key(KeyCode::Char('k'))); // already at the top
        assert_eq!(view.selected, income);
    }

    #[test]
    fn l_unfolds_a_folded_node_and_reveals_its_children() {
        let mut view = CategoriesView::new();
        let salary = find_by_name(&view, "Salary"); // starts folded, per "everything else folded"
        view.selected = salary;
        let before = view.visible_node_ids().len();

        view.handle_key(key(KeyCode::Char('l')));
        // Salary unfolds to reveal Primary Job and Bonus.
        assert_eq!(view.visible_node_ids().len(), before + 2);
    }

    #[test]
    fn h_folds_an_expanded_selected_node() {
        let mut view = CategoriesView::new();
        let salary = find_by_name(&view, "Salary");
        view.selected = salary;
        let before = view.visible_node_ids().len();

        view.handle_key(key(KeyCode::Char('l'))); // unfold Salary
        assert_eq!(view.visible_node_ids().len(), before + 2);

        view.handle_key(key(KeyCode::Char('h'))); // fold it back
        assert_eq!(view.visible_node_ids().len(), before);
    }

    #[test]
    fn h_on_a_leaf_jumps_to_its_parent() {
        let mut view = CategoriesView::new();
        let salary = find_by_name(&view, "Salary");
        let primary_job = find_by_name(&view, "Primary Job");
        view.selected = primary_job;

        view.handle_key(key(KeyCode::Char('h')));
        assert_eq!(view.selected, salary);
    }

    #[test]
    fn h_on_an_already_folded_parent_jumps_to_its_parent() {
        let mut view = CategoriesView::new();
        let income = find_by_name(&view, "Income");
        let salary = find_by_name(&view, "Salary");
        view.selected = salary; // Salary starts folded (default on first open)

        view.handle_key(key(KeyCode::Char('h')));
        assert_eq!(view.selected, income);
    }

    #[test]
    fn z_then_capital_r_unfolds_everything() {
        let mut view = CategoriesView::new();
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('R')));

        assert!(view.folded.is_empty());
        // Every node except the two roots is now a visible line.
        assert_eq!(view.visible_node_ids().len(), view.store.nodes().len());
    }

    #[test]
    fn z_then_capital_m_folds_everything_including_roots() {
        let mut view = CategoriesView::new();
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('M')));

        assert_eq!(
            view.visible_node_ids().len(),
            2,
            "only the two roots remain visible"
        );
    }

    #[test]
    fn folding_away_the_selection_recovers_to_the_nearest_visible_ancestor() {
        let mut view = CategoriesView::new();
        let income = find_by_name(&view, "Income");
        let salary = find_by_name(&view, "Salary");
        view.selected = salary;

        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('M'))); // fold everything, including Income

        assert_eq!(view.selected, income);
    }

    #[test]
    fn za_toggles_archived_visibility_and_recovers_a_hidden_selection() {
        let mut view = CategoriesView::new();
        let food = find_by_name(&view, "Food");
        let restaurants = find_by_name(&view, "Restaurants");
        view.store
            .set_active(restaurants, false)
            .expect("archiving a non-root should succeed");
        view.folded.retain(|id| *id != food); // unfold Food so Restaurants can show once shown at all
        view.selected = restaurants; // archived + show_archived: false, so this starts hidden

        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('a'))); // za: show archived
        assert!(view.show_archived);
        assert!(view.visible_node_ids().contains(&restaurants));

        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('a'))); // za: hide archived again
        assert!(!view.show_archived);
        assert_eq!(
            view.selected, food,
            "selection should recover to Food, restaurants' parent"
        );
    }

    #[test]
    fn archived_row_renders_dim_with_an_archived_suffix() {
        // A short-named leaf (`Bonus`, not `Restaurants`) — the left pane's name column is
        // narrow enough at depth 3 that a longer name plus " · archived" would truncate before
        // the suffix, which would defeat the point of this assertion.
        let mut view = CategoriesView::new();
        let salary = find_by_name(&view, "Salary");
        let bonus = find_by_name(&view, "Bonus");
        view.store
            .set_active(bonus, false)
            .expect("archiving a non-root should succeed");
        view.folded.retain(|id| *id != salary); // unfold Salary so Bonus actually renders
        view.show_archived = true;

        let text = render(&view);
        assert!(text.contains("Bonus · archived"), "archived suffix missing");
    }

    #[test]
    fn home_and_g_select_the_first_and_last_visible_rows() {
        let mut view = CategoriesView::new();
        let income = find_by_name(&view, "Income");
        let last_visible = *view.visible_node_ids().last().unwrap();

        view.handle_key(key(KeyCode::Char('G')));
        assert_eq!(view.selected, last_visible);

        view.handle_key(key(KeyCode::Home));
        assert_eq!(view.selected, income);
    }

    #[test]
    fn summary_box_shows_the_selected_category_and_merges_direct_and_rollup_for_a_leaf() {
        let mut view = CategoriesView::new();
        let groceries = find_by_name(&view, "Groceries");
        view.selected = groceries;

        let text = render(&view);
        assert!(text.contains("expenses / food / groceries"), "path missing");
        assert!(
            text.contains("direct · rollup 12m"),
            "merged direct/rollup label missing"
        );
        assert!(text.contains("12 480.40"), "exact rollup value missing");
        assert!(
            text.contains("expense · 3"),
            "kind · depth missing (groceries is depth 3)"
        );
        assert!(text.contains("none · leaf"), "children field missing");
        assert!(text.contains("148 · direct"), "transactions field missing");
        // The summary box only leaves ~20 chars for a value after the label column, so a
        // long note still truncates — check a prefix short enough to survive that, not the
        // full "supermarket, greengrocer".
        assert!(text.contains("supermarket"), "note missing");
        assert!(text.contains("[×] · offered"), "active field missing");
    }

    #[test]
    fn summary_box_splits_direct_and_rollup_when_they_differ() {
        let mut view = CategoriesView::new();
        let food = find_by_name(&view, "Food");
        view.selected = food;

        let text = render(&view);
        assert!(text.contains("direct 12m"), "direct label missing");
        assert!(text.contains("rollup 12m"), "rollup label missing");
        assert!(text.contains("18 240.4"), "rollup value missing");
        assert!(
            text.contains("2 · parent"),
            "children field missing for a parent"
        );
    }

    #[test]
    fn summary_box_shows_none_for_a_parents_own_first_last_and_transactions() {
        let mut view = CategoriesView::new();
        let food = find_by_name(&view, "Food");
        view.selected = food;

        let text = render(&view);
        assert!(
            text.contains("none"),
            "expected a 'none' field for a node with no direct postings"
        );
    }

    #[test]
    fn selected_row_renders_reversed() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        let view = CategoriesView::new();
        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering should not error");

        let buffer = terminal.backend().buffer();
        let row_is_reversed = |y: u16| -> bool {
            (0..LEFT_PANE_WIDTH).any(|x| buffer[(x, y)].modifier.contains(Modifier::REVERSED))
        };
        let row_containing = |needle: &str| -> u16 {
            (0..buffer.area.height)
                .find(|&y| {
                    let mut row = String::new();
                    for x in 0..LEFT_PANE_WIDTH {
                        row.push_str(buffer[(x, y)].symbol());
                    }
                    row.contains(needle)
                })
                .unwrap_or_else(|| panic!("no row contains {needle:?}"))
        };

        assert!(
            row_is_reversed(row_containing("INCOME")),
            "the selected Income root should render reversed"
        );
    }

    #[test]
    fn other_keys_fall_through_to_the_shell() {
        let mut view = CategoriesView::new();
        assert_eq!(view.handle_key(key(KeyCode::Char('x'))), None);
    }

    #[test]
    fn e_opens_the_edit_popup_for_a_non_root_selection() {
        let mut view = CategoriesView::new();
        let groceries = find_by_name(&view, "Groceries");
        view.selected = groceries;

        assert_eq!(
            view.handle_key(key(KeyCode::Char('e'))),
            Some(Action::OpenCategoryEditPopup(groceries))
        );
    }

    #[test]
    fn e_on_a_root_selection_does_nothing() {
        let mut view = CategoriesView::new();
        assert_eq!(view.handle_key(key(KeyCode::Char('e'))), None);
    }

    #[test]
    fn a_archives_the_selection_directly_with_no_popup() {
        let mut view = CategoriesView::new();
        let groceries = find_by_name(&view, "Groceries");
        view.selected = groceries;

        let action = view.handle_key(key(KeyCode::Char('a')));
        assert_eq!(action, Some(Action::NoOp));
        assert!(!view.store.find(groceries).unwrap().active);
    }

    #[test]
    fn a_on_a_root_selection_does_nothing_to_the_store() {
        let mut view = CategoriesView::new();
        let income = find_by_name(&view, "Income");
        view.selected = income;

        view.handle_key(key(KeyCode::Char('a')));
        assert!(view.store.find(income).unwrap().active);
    }

    #[test]
    fn capital_x_shows_a_not_yet_built_hint_until_another_key_clears_it() {
        let mut view = CategoriesView::new();
        assert_eq!(view.handle_key(key(KeyCode::Char('X'))), Some(Action::NoOp));
        assert!(render(&view).contains("not yet built"));

        view.handle_key(key(KeyCode::Char('j')));
        assert!(!render(&view).contains("not yet built"));
    }

    #[test]
    fn format_money_keeps_decimals_only_when_the_amount_has_a_fractional_part() {
        assert_eq!(format_money(&"142100".parse().unwrap()), "142 100");
        assert_eq!(format_money(&"12480.40".parse().unwrap()), "12 480.40");
    }

    #[test]
    fn format_money_whole_always_drops_decimals() {
        assert_eq!(format_money_whole(&"12480.40".parse().unwrap()), "12 480");
    }

    #[test]
    fn shows_the_direct_spend_chart_heading_and_labels() {
        let mut view = CategoriesView::new();
        view.selected = find_by_name(&view, "Groceries");

        let text = render(&view);
        assert!(text.contains("DIRECT SPEND"), "chart heading missing");
        assert!(text.contains("avg"), "average label missing");
        // Braille marker cells the `Chart` line draws with — confirms an actual chart
        // rendered rather than a bare empty area (same glyph set `view::dashboard`'s own
        // chart test checks for).
        assert!(
            text.contains(['⠉', '⠊', '⠔', '⠒', '⣀', '⡠']),
            "spend line chart missing"
        );
    }

    #[test]
    fn direct_series_sums_to_the_nodes_direct_amount() {
        let view = CategoriesView::new();
        let groceries = view
            .store
            .find(find_by_name(&view, "Groceries"))
            .unwrap()
            .clone();

        let series = direct_series(&groceries);
        let total: f64 = series.iter().sum();
        assert!(
            (total - money_to_f64(&groceries.direct)).abs() < 0.01,
            "series should sum to the node's direct amount, got {total}"
        );
    }

    #[test]
    fn direct_series_is_flat_zero_for_a_node_with_no_direct_postings() {
        let view = CategoriesView::new();
        let food = view
            .store
            .find(find_by_name(&view, "Food"))
            .unwrap()
            .clone();

        assert_eq!(direct_series(&food), [0.0; CHART_MONTHS]);
    }

    #[test]
    fn transactions_for_node_generates_the_right_count_within_the_posting_date_range() {
        let view = CategoriesView::new();
        let groceries = view
            .store
            .find(find_by_name(&view, "Groceries"))
            .unwrap()
            .clone();

        let rows = transactions_for_node(&groceries);
        assert_eq!(rows.len(), groceries.transaction_count as usize);
        let (first, last) = (
            groceries.first_posted.unwrap(),
            groceries.last_posted.unwrap(),
        );
        for row in &rows {
            assert!(
                row.date >= first && row.date <= last,
                "{:?} outside {first}..={last}",
                row.date
            );
        }
    }

    #[test]
    fn transaction_rows_caps_at_ten_newest_first() {
        let mut view = CategoriesView::new();
        let groceries = find_by_name(&view, "Groceries");
        view.selected = groceries;

        let node = view.store.find(groceries).unwrap().clone();
        let rows = view.transaction_rows(&node);
        assert_eq!(rows.len(), 10);
        assert!(rows.windows(2).all(|pair| pair[0].date >= pair[1].date));
    }

    #[test]
    fn s_toggles_show_subtree_and_returns_no_op() {
        let mut view = CategoriesView::new();
        assert!(!view.show_subtree);

        assert_eq!(view.handle_key(key(KeyCode::Char('s'))), Some(Action::NoOp));
        assert!(view.show_subtree);

        view.handle_key(key(KeyCode::Char('s')));
        assert!(!view.show_subtree);
    }

    #[test]
    fn subtree_transaction_count_excludes_the_node_itself() {
        let view = CategoriesView::new();
        let groceries = find_by_name(&view, "Groceries");
        assert_eq!(
            view.subtree_transaction_count(groceries),
            0,
            "a leaf has no subtree"
        );

        let food = find_by_name(&view, "Food");
        assert_eq!(view.subtree_transaction_count(food), 148 + 62);
    }

    #[test]
    fn transactions_list_gains_a_category_column_only_in_subtree_mode() {
        let mut view = CategoriesView::new();
        view.selected = find_by_name(&view, "Food");

        let direct_text = render(&view);
        assert!(
            !direct_text.contains("CATEGORY"),
            "no category column in direct mode"
        );

        view.handle_key(key(KeyCode::Char('s')));
        let subtree_text = render(&view);
        assert!(
            subtree_text.contains("CATEGORY"),
            "category column missing in subtree mode"
        );
        assert!(
            subtree_text.contains("Groceries"),
            "a descendant's category name missing"
        );
    }

    #[test]
    fn transactions_footer_shows_direct_and_subtree_counts() {
        let mut view = CategoriesView::new();
        view.selected = find_by_name(&view, "Food");

        let text = render(&view);
        assert!(
            text.contains("0 direct"),
            "direct count missing for a category with no direct postings"
        );
        assert!(
            text.contains("210 in subtree"),
            "subtree count missing (148 groceries + 62 restaurants)"
        );
    }
}
