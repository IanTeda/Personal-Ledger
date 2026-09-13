//! The Tags `View`, hosted by `Shell` (ADR-0013). Per the "Tags catalog screen, views and
//! popup" map's own grilling session: a flat, alphabetically-sorted list (Tag has no
//! classification dimension to group by) plus a summary box for whichever Tag is selected in
//! the left pane.
//!
//! **The right pane** (issue #133, "Tags right pane" map, issue #131) fills what that first
//! map deliberately left blank, against `docs/ux/tui/tags/README.md`'s own `9a` right-pane
//! spec — three widgets, all reading from the fixture-simulated data "Tags: right-pane fixture
//! data" (issue #132) landed: **Tagged spend** (a monthly sparkline, mirroring `view::
//! accounts::render_balance_chart`), **Where it lands** (the category breakdown as proportional
//! block-glyph bars), and **Transactions** (the fake per-tag rows, newest first). The design
//! doc's own `!`/off-unit apparatus, untagged-remainder row, and Merge/Apply states are all out
//! of scope for this map — see issue #131's own Notes.
//!
//! Real, interactive state — not a wireframe: `store` ([`TagFixture`], from "Tags: fixture
//! data seam and mutable View state pattern") genuinely holds the Tag list, and
//! `selected`/`show_inactive`/`filter`/`pending_delete` are mutated directly in
//! [`TagsView::handle_key`], mirroring `view::accounts`'s own state-ownership decision.
//!
//! **Delete is lightweight, not a popup**: `d` arms `pending_delete`; the list's own heading
//! row (which normally just states the visible/total count) is replaced by the confirm line
//! while armed — the same "a transient state temporarily replaces part of the normal render"
//! trick `view::categories`'s own `merge_hint` already uses for its `X` stub. Any key other
//! than `y` aborts the arm with no mutation; `y` calls `TagStore::delete` unconditionally (it's
//! never refused, per the map's own decision that removing a Tag is harmless and reversible).
//!
//! **Reachable via `g g`** (`Shell`'s own leader chord, doubled — see [`Action::OpenTags`]'s
//! own doc for why) or `:tag` in the command popup. No Dashboard menu row exists; Dashboard has
//! no per-domain menu at all, only headline widgets.
//!
//! **`n`/`e` open the new/edit-tag popups** (`crate::popup::tag::new`/`edit`, "Tags: new
//! popup"/"Tags: edit popup") — `Shell` owns the popups themselves, mirroring `view::
//! accounts`'s own `n`/`e`/`d`; this `View` only ever resolves *which* key to turn into
//! [`Action::OpenTagNewPopup`]/[`Action::OpenTagEditPopup`], and reacts to the eventual
//! [`Action::CreateTag`]/[`Action::UpdateTag`] in [`TagsView::update`].
//!
//! **The `:tag` command grammar** (`popup::command::commands::tags`, "Tags: :tag command
//! grammar") reaches these exact same code paths rather than a parallel one: `tag off`/`tag
//! on` dispatch [`Action::SetTagActive`], handled here identically to [`Action::UpdateTag`]'s
//! own `TagStore::set_active` call; `tag delete` dispatches [`Action::ArmTagDelete`], which
//! just sets `pending_delete = true` — the exact same state bare `d` arms, still gated behind
//! `y` on this view before anything is actually deleted.

use chrono::{Datelike, Months, NaiveDate};
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
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use crate::tag::{Tag, TagFixture, TagStore, TagTransaction};
use crate::view::{Action, View};

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used here for
/// the delete confirm line.
const ACCENT: Color = Color::Red;

/// Width of the list + summary column — matches `view::accounts`'s own `LEFT_PANE_WIDTH`; the
/// remaining terminal width is deliberately left blank, since there's no right-pane content
/// for Tag to show.
const PANE_WIDTH: u16 = 41;

/// Width of the summary box's label column — narrower than `view::accounts`'s own (Tag's
/// labels are all one word, unlike Account's `"balance now · computed"`), which matters here:
/// the pane is only [`PANE_WIDTH`] wide, with no right pane to spill into.
const SUMMARY_LABEL_WIDTH: usize = 10;

/// Number of fact rows the summary box shows beneath its rule: `active`, `tagged`, `created`,
/// `updated` — kept as two separate date rows (rather than one combined line, as `view::
/// accounts`'s own `last check` row does) because combining them doesn't fit `PANE_WIDTH`'s
/// narrower inner width.
const SUMMARY_CONTENT_ROWS: u16 = 4;

/// How many trailing months "Tagged spend" plots — matches `crate::account::fixture`'s own
/// `CHART_MONTHS`/`crate::tag::fixture`'s own `SPEND_MONTHS` convention.
const SPEND_MONTHS: usize = 24;

/// Height of the "Tagged spend" section: heading, rule, the chart itself, then the footer
/// stats line — matches `view::categories::SPEND_CHART_HEIGHT`/`view::accounts::
/// BALANCE_CHART_HEIGHT` exactly, so every domain's own spend/balance chart reads at the same
/// scale (`view::accounts`'s own doc states this convention explicitly).
const TAGGED_SPEND_HEIGHT: u16 = 12;

/// How many category rows "Where it lands" shows before rolling the rest into an "N more" row
/// — per `docs/ux/tui/tags/README.md`'s own "top 4 plus an `N more` roll-up".
const CATEGORY_ROWS_SHOWN: usize = 4;

/// Height of the "Where it lands" section: heading, rule, up to `CATEGORY_ROWS_SHOWN + 1` bar
/// rows (the roll-up counted in that `+1`), then the closing line.
const WHERE_IT_LANDS_HEIGHT: u16 = 1 + 1 + (CATEGORY_ROWS_SHOWN as u16 + 1) + 1;

/// Longest a "Where it lands" bar is ever drawn, in cells — matches the design doc's own
/// "proportional block-glyph bars (█, max 16 cells)".
const BAR_MAX_CELLS: usize = 16;

/// Width of a "Where it lands"/Transactions row's category-path label column.
const CATEGORY_LABEL_WIDTH: u16 = 20;

/// Width of a "Where it lands"/Transactions row's right-aligned amount column.
const RIGHT_AMOUNT_WIDTH: u16 = 10;

/// How many of a Tag's transactions the sub-list ever shows — matches `view::accounts`'s own
/// `LEDGER_VISIBLE_ROWS` convention.
const TAG_TRANSACTIONS_VISIBLE_ROWS: usize = 8;

/// Width of the Transactions sub-list's `DATE` column.
const TXN_DATE_WIDTH: u16 = 6;

/// Width of the Transactions sub-list's `T` (other-tags overlap) column.
const TXN_OTHER_TAGS_WIDTH: u16 = 2;

/// The Tags `View`. Owns the fixture Tag list directly — mutated in place in `handle_key`.
pub struct TagsView {
    store: TagFixture,
    selected: RowID,
    /// `za` toggles this — inactive Tags are hidden unless it's `true`.
    show_inactive: bool,
    /// `true` right after a lone `z`, awaiting the `a` that completes the `za` chord.
    pending_z: bool,
    /// `/`'s current query — a case-insensitive substring match against a Tag's name. Empty
    /// means "no filter".
    filter: String,
    /// `true` while `/`'s input buffer has focus.
    filtering: bool,
    /// `true` right after `d` — the list heading shows the delete confirm line instead of its
    /// usual count until `y` (confirm) or any other key (abort) is pressed.
    pending_delete: bool,
    /// `true` right after `Enter` on the right pane's Transactions sub-list — shows a "not yet
    /// built" message in place of the overlap footer statement until the next key. `view::
    /// transactions` has no real filtered jump yet, so `enter` here can't do the design doc's
    /// own "opens the full transactions view filtered to this tag"; this states that plainly
    /// rather than silently doing nothing indistinguishable from an unhandled key, mirroring
    /// `popup::command::CommandPopup`'s own `not_yet_built` field.
    transactions_not_yet_built: bool,
}

impl Default for TagsView {
    fn default() -> Self {
        Self::new()
    }
}

impl TagsView {
    pub fn new() -> Self {
        let store = TagFixture::new();
        let selected = Self::visible_tags_of(&store, false, "")
            .first()
            .map(|tag| tag.id)
            .unwrap_or_default();

        Self {
            store,
            selected,
            show_inactive: false,
            pending_z: false,
            filter: String::new(),
            filtering: false,
            pending_delete: false,
            transactions_not_yet_built: false,
        }
    }

    fn visible_tags_of<'a>(
        store: &'a TagFixture,
        show_inactive: bool,
        filter: &str,
    ) -> Vec<&'a Tag> {
        let needle = filter.to_lowercase();
        let mut tags: Vec<&Tag> = store
            .tags()
            .iter()
            .filter(|tag| show_inactive || tag.is_active)
            .filter(|tag| needle.is_empty() || tag.name.to_lowercase().contains(&needle))
            .collect();
        tags.sort_by_key(|tag| tag.name.to_lowercase());
        tags
    }

    fn visible_tags(&self) -> Vec<&Tag> {
        Self::visible_tags_of(&self.store, self.show_inactive, &self.filter)
    }

    fn selected_tag(&self) -> Option<&Tag> {
        self.store.find(self.selected)
    }

    fn move_selection(&mut self, delta: isize) {
        let visible = self.visible_tags();
        let Some(current_index) = visible.iter().position(|tag| tag.id == self.selected) else {
            self.recover_selection();
            return;
        };
        let next_index = (current_index as isize + delta).clamp(0, visible.len() as isize - 1);
        self.selected = visible[next_index as usize].id;
    }

    fn select_first(&mut self) {
        if let Some(first) = self.visible_tags().first() {
            self.selected = first.id;
        }
    }

    fn select_last(&mut self) {
        if let Some(last) = self.visible_tags().last() {
            self.selected = last.id;
        }
    }

    fn recover_selection(&mut self) {
        let visible = self.visible_tags();
        if visible.iter().any(|tag| tag.id == self.selected) {
            return;
        }
        if let Some(first) = visible.first() {
            self.selected = first.id;
        }
    }
}

impl View for TagsView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.pending_delete {
            self.pending_delete = false;
            if key.code == KeyCode::Char('y') {
                let _ = self.store.delete(self.selected);
                self.recover_selection();
            }
            return Some(Action::NoOp);
        }

        if self.filtering {
            match key.code {
                KeyCode::Char(c) => self.filter.push(c),
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Enter => self.filtering = false,
                KeyCode::Esc => {
                    self.filtering = false;
                    self.filter.clear();
                }
                _ => return Some(Action::NoOp),
            }
            self.recover_selection();
            return Some(Action::NoOp);
        }

        if self.pending_z {
            self.pending_z = false;
            match key.code {
                KeyCode::Char('a') => self.show_inactive = !self.show_inactive,
                _ => return Some(Action::NoOp),
            }
            self.recover_selection();
            return Some(Action::NoOp);
        }

        // Any key reaching here other than the `Enter` arm below clears a showing "not yet
        // built" message — mirrors `CommandPopup::not_yet_built`'s own "any subsequent
        // mutating key clears it" rule.
        self.transactions_not_yet_built = false;

        match key.code {
            KeyCode::Char('z') => {
                self.pending_z = true;
                None
            }
            KeyCode::Enter => {
                self.transactions_not_yet_built = true;
                Some(Action::NoOp)
            }
            KeyCode::Char('/') => {
                self.filtering = true;
                self.filter.clear();
                Some(Action::NoOp)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection(1);
                Some(Action::NoOp)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection(-1);
                Some(Action::NoOp)
            }
            KeyCode::Home => {
                self.select_first();
                Some(Action::NoOp)
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.select_last();
                Some(Action::NoOp)
            }
            KeyCode::Char('d') => {
                self.pending_delete = true;
                Some(Action::NoOp)
            }
            KeyCode::Char('n') => Some(Action::OpenTagNewPopup),
            KeyCode::Char('e') => Some(Action::OpenTagEditPopup(self.selected)),
            _ => None,
        }
    }

    /// Reacts to the Tag new/edit popups' own confirmed create/save (`Shell` relays these
    /// after resolving them against `tag_store()` — see `crate::popup::tag::new`/`edit`'s own
    /// module docs). Every other `Action` variant is ignored.
    fn update(&mut self, action: &Action) {
        match action {
            Action::CreateTag { name, active, .. } => {
                if let Ok(id) = self.store.create(name.clone(), *active) {
                    self.selected = id;
                }
            }
            Action::UpdateTag { id, name, active } => {
                let _ = self.store.update(*id, name.clone(), *active);
            }
            Action::SetTagActive { id, active } => {
                if self.store.set_active(*id, *active).is_ok() {
                    self.recover_selection();
                }
            }
            Action::ArmTagDelete => self.pending_delete = true,
            _ => {}
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        // The heading/delete-confirm row spans the *full* width, not just `PANE_WIDTH` — a
        // narrow list pane has no room for "delete \"...\" ... y confirms, any other key
        // cancels" even wrapped, so it rides across the whole view region instead (the only
        // row that does; the list and summary box below stay pane-width). Two lines are
        // reserved for it while armed (wrapped via `Wrap`), one otherwise.
        let heading_height = if self.pending_delete { 2 } else { 1 };
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // spacer, matching every other `View`'s own leading row
                Constraint::Length(heading_height),
                Constraint::Min(0),
            ])
            .split(area);

        let visible_count = self.visible_tags().len();
        self.render_heading(frame, rows[1], visible_count);

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(PANE_WIDTH), Constraint::Min(0)])
            .spacing(2)
            .split(rows[2]);

        let pane_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(SUMMARY_CONTENT_ROWS + 4), // name + rule + content + 2 border
            ])
            .split(columns[0]);

        self.render_list(frame, pane_rows[0]);
        self.render_summary(frame, pane_rows[1]);
        self.render_right_pane(frame, columns[1]);
    }

    fn title(&self) -> &'static str {
        "Tags"
    }

    fn tag_store(&self) -> Option<&dyn TagStore> {
        Some(&self.store)
    }

    fn tag_selection(&self) -> Option<RowID> {
        Some(self.selected)
    }
}

impl TagsView {
    fn render_list(&self, frame: &mut Frame, area: Rect) {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .spacing(1)
            .split(area);
        let content_area = split[0];
        let scrollbar_column = split[1];

        let visible = self.visible_tags();
        let row_count = visible.len().min(content_area.height as usize);
        let row_constraints: Vec<Constraint> =
            std::iter::repeat_n(Constraint::Length(1), row_count).collect();
        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(content_area);

        for (tag, row_area) in visible.iter().zip(row_areas.iter()) {
            render_tag_row(frame, *row_area, tag, tag.id == self.selected);
        }

        let total = self.store.tags().len();
        let mut scrollbar_state = ScrollbarState::new(total)
            .viewport_content_length(visible.len())
            .position(0);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, scrollbar_column, &mut scrollbar_state);
    }

    /// The heading row: `tags N of M` normally, replaced by the delete confirm line while
    /// `pending_delete` is armed — see this module's own doc on why that's a render swap, not
    /// a popup, and [`TagsView::view`]'s own doc on why this row alone spans the full width.
    fn render_heading(&self, frame: &mut Frame, area: Rect, visible_count: usize) {
        if self.pending_delete
            && let Some(tag) = self.selected_tag()
        {
            let notice = if tag.tagged_transaction_count > 0 {
                format!(
                    ", {} transactions will lose this tag",
                    tag.tagged_transaction_count
                )
            } else {
                String::new()
            };
            let text = format!(
                "delete \"{}\"{notice} — y confirms, any other key cancels",
                tag.name
            );
            frame.render_widget(
                Paragraph::new(Span::styled(text, Style::default().fg(ACCENT)))
                    .wrap(Wrap { trim: true }),
                area,
            );
            return;
        }

        let total = self.store.tags().len();
        let text = format!("tags {visible_count} of {total}");
        frame.render_widget(
            Paragraph::new(Span::styled(
                text,
                Style::default().add_modifier(Modifier::DIM),
            )),
            area,
        );
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().padding(Padding::horizontal(1));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(tag) = self.selected_tag() else {
            frame.render_widget(Paragraph::new("no tags match the current filter"), inner);
            return;
        };

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // name
                Constraint::Length(1), // rule
                Constraint::Length(1), // active
                Constraint::Length(1), // tagged
                Constraint::Length(1), // created
                Constraint::Length(1), // updated
            ])
            .split(inner);

        frame.render_widget(Paragraph::new(tag.name.clone()), rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

        let active_text = if tag.is_active {
            "[×] offered when tagging"
        } else {
            "[ ] not offered"
        };
        frame.render_widget(summary_field_line("active", active_text), rows[2]);

        let tagged_text = if tag.tagged_transaction_count == 0 {
            "none".to_string()
        } else {
            format!("{} transactions", tag.tagged_transaction_count)
        };
        frame.render_widget(summary_field_line("tagged", &tagged_text), rows[3]);

        // Two separate rows, not one combined "created · upd ..." line (as `view::accounts`'s
        // own `last check` row does) — this pane's narrower inner width (no right pane to
        // spill into) can't fit both dates on one line.
        frame.render_widget(
            summary_field_line("created", &format_date_full_year(tag.created_on)),
            rows[4],
        );
        frame.render_widget(
            summary_field_line("updated", &format_date_full_year(tag.updated_on)),
            rows[5],
        );
    }

    /// The right pane: "Tagged spend", "Where it lands", "Transactions" — nothing renders when
    /// no Tag is selected (an all-filtered-out list), mirroring the left pane's own summary box
    /// falling back to a plain message in that case, just with nothing at all here instead.
    fn render_right_pane(&self, frame: &mut Frame, area: Rect) {
        let Some(tag) = self.selected_tag() else {
            return;
        };

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(TAGGED_SPEND_HEIGHT),
                Constraint::Length(1), // spacer
                Constraint::Length(WHERE_IT_LANDS_HEIGHT),
                Constraint::Length(1), // spacer
                Constraint::Min(0),    // transactions
            ])
            .split(area);

        self.render_tagged_spend(frame, rows[0], tag);
        self.render_where_it_lands(frame, rows[2], tag);
        self.render_tag_transactions(frame, rows[4], tag);
    }

    /// A monthly sparkline over the trailing [`SPEND_MONTHS`]-month window, mirroring
    /// `view::accounts::render_balance_chart`'s own `Chart`/`Dataset`/Braille-marker/
    /// accent-latest-point treatment — the series is a spend total per month, not a running
    /// balance, so there's no "start" value to anchor against, only the series itself.
    fn render_tagged_spend(&self, frame: &mut Frame, area: Rect, tag: &Tag) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // heading
                Constraint::Length(1), // rule
                Constraint::Min(0),    // chart
                Constraint::Length(1), // footer stats
            ])
            .split(area);

        render_section_heading(frame, rows[0], "TAGGED SPEND", &tagged_spend_window_tag());
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

        let series: Vec<f64> = self
            .store
            .monthly_spend(tag.id, SPEND_MONTHS, crate::tag::FIXTURE_NOW)
            .iter()
            .map(money_to_f64)
            .collect();

        render_spend_chart(frame, rows[2], &series);
        render_spend_footer(frame, rows[3], tag, &series);
    }

    /// The category breakdown, biggest first, as proportional block-glyph bars — top
    /// [`CATEGORY_ROWS_SHOWN`] plus an `N more` roll-up, per the design doc's own shape.
    fn render_where_it_lands(&self, frame: &mut Frame, area: Rect, tag: &Tag) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),                              // heading
                Constraint::Length(1),                              // rule
                Constraint::Length(CATEGORY_ROWS_SHOWN as u16 + 1), // bar rows incl. roll-up
                Constraint::Length(1),                              // closing line
            ])
            .split(area);

        let breakdown = self.store.category_breakdown(tag.id);
        let heading_tag = if breakdown.is_empty() {
            String::new()
        } else {
            format!("{} categories", breakdown.len())
        };
        render_section_heading(frame, rows[0], "WHERE IT LANDS", &heading_tag);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

        if breakdown.is_empty() {
            let dim = Style::default().add_modifier(Modifier::DIM);
            frame.render_widget(
                Paragraph::new(Span::styled("no categories yet", dim)),
                rows[2],
            );
            return;
        }

        let total: f64 = breakdown
            .iter()
            .map(|(_, amount)| money_to_f64(amount))
            .sum();
        let shown = breakdown.len().min(CATEGORY_ROWS_SHOWN);
        let bar_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); CATEGORY_ROWS_SHOWN + 1])
            .split(rows[2]);

        for (index, (path, amount)) in breakdown[..shown].iter().enumerate() {
            render_category_bar(frame, bar_rows[index], path, money_to_f64(amount), total);
        }
        if breakdown.len() > shown {
            let rest_total: f64 = breakdown[shown..]
                .iter()
                .map(|(_, amount)| money_to_f64(amount))
                .sum();
            let label = format!("{} more", breakdown.len() - shown);
            render_category_bar(frame, bar_rows[shown], &label, rest_total, total);
        }

        let dim = Style::default().add_modifier(Modifier::DIM);
        frame.render_widget(
            Paragraph::new(Span::styled(
                "a tag crosses the tree — that is what it is for",
                dim,
            )),
            rows[3],
        );
    }

    /// The fake per-tag transaction rows, newest first, capped to whatever fits — mirrors
    /// `view::accounts::render_ledger`'s own column/heading/footer shape. `enter` here can only
    /// show [`TagsView::transactions_not_yet_built`]'s message, not actually jump anywhere —
    /// see this module's own doc.
    fn render_tag_transactions(&self, frame: &mut Frame, area: Rect, tag: &Tag) {
        let all_rows = self.store.transactions(tag.id);
        let total = all_rows.len();
        let visible_count = all_rows.len().min(TAG_TRANSACTIONS_VISIBLE_ROWS);
        let visible = &all_rows[..visible_count];

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // heading
                Constraint::Length(1), // rule
                Constraint::Length(1), // column header
                Constraint::Min(0),    // rows
                Constraint::Length(1), // footer
            ])
            .split(area);

        let heading_tag = format!("{visible_count} of {total} · newest first");
        render_section_heading(frame, sections[0], "TRANSACTIONS", &heading_tag);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
        render_txn_column_header(frame, sections[2]);

        let row_count = visible.len().min(sections[3].height as usize);
        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); row_count])
            .split(sections[3]);
        for (row, row_area) in visible.iter().zip(row_areas.iter()) {
            render_txn_row(frame, *row_area, row);
        }

        self.render_transactions_footer(frame, sections[4], &all_rows);
    }

    /// The overlap statement ("N of M carry another tag — rows overlap"), the not-yet-built
    /// message while [`TagsView::transactions_not_yet_built`] is set, or a plain empty-state
    /// line for a Tag with no transactions — exactly one of the three, per this module's own
    /// doc on why `enter` can't do more than state its own limits yet.
    fn render_transactions_footer(&self, frame: &mut Frame, area: Rect, rows: &[TagTransaction]) {
        if self.transactions_not_yet_built {
            frame.render_widget(
                Paragraph::new("opening filtered Transactions — not yet built"),
                area,
            );
            return;
        }

        let dim = Style::default().add_modifier(Modifier::DIM);
        if rows.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled("no transactions yet", dim)),
                area,
            );
            return;
        }

        let overlap = rows.iter().filter(|row| row.other_tags > 0).count();
        let text = format!(
            "{overlap} of {} carry another tag — rows overlap",
            rows.len()
        );
        frame.render_widget(
            Paragraph::new(Span::styled(text, Style::default().fg(ACCENT))),
            area,
        );
    }
}

fn render_tag_row(frame: &mut Frame, area: Rect, tag: &Tag, selected: bool) {
    if selected {
        frame.render_widget(
            Block::new().style(Style::default().add_modifier(Modifier::REVERSED)),
            area,
        );
    }

    let dim = Style::default().add_modifier(Modifier::DIM);
    let name_text = if tag.is_active {
        tag.name.clone()
    } else {
        format!("{} · inactive", tag.name)
    };
    let style = if !tag.is_active && !selected {
        dim
    } else {
        Style::default()
    };
    frame.render_widget(Paragraph::new(Span::styled(name_text, style)), area);
}

fn summary_field_line<'a>(label: &'a str, value: &'a str) -> Paragraph<'a> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    Paragraph::new(Line::from(vec![
        Span::styled(format!("{label:<SUMMARY_LABEL_WIDTH$}"), dim),
        Span::raw(value),
    ]))
}

fn format_date_full_year(date: chrono::NaiveDate) -> String {
    date.format("%d %b %Y").to_string().to_lowercase()
}

/// A section heading row shared by every right-pane widget: the label flush left (dim), a
/// short dim tag right-aligned — mirrors `view::accounts::render_chart_heading`'s own shape,
/// generalised since every widget here needs a slightly different tag.
fn render_section_heading(frame: &mut Frame, area: Rect, label: &str, tag: &str) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled(label, dim)), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

fn money_to_f64(value: &Money) -> f64 {
    value.0.to_string().parse().unwrap_or(0.0)
}

/// Formats a right-pane figure to two decimal places — deliberately simpler than `view::
/// accounts::format_money_at`'s own thousands-grouped, per-unit-precision formatting: nothing
/// here carries a real Unit (the design doc's own off-unit/"AUD only" apparatus is out of
/// scope for this map), so there's no precision to look up and no unit to denominate by.
fn format_amount(value: f64) -> String {
    format!("{value:.2}")
}

fn format_date(date: chrono::NaiveDate) -> String {
    date.format("%d %b").to_string().to_lowercase()
}

/// "Tagged spend"'s fixed end-of-window month — `crate::tag::FIXTURE_NOW`'s own month, mirroring
/// `view::accounts::chart_end_month`'s convention.
fn spend_chart_end_month() -> NaiveDate {
    let now = crate::tag::FIXTURE_NOW;
    NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .expect("FIXTURE_NOW's own year/month with day 1 is always a valid date")
}

/// The chart's x-axis month for `index` (`0` oldest, `SPEND_MONTHS - 1` newest) — mirrors
/// `view::accounts::chart_month`.
fn spend_chart_month(index: usize) -> NaiveDate {
    spend_chart_end_month()
        .checked_sub_months(Months::new((SPEND_MONTHS - 1 - index) as u32))
        .expect("SPEND_MONTHS stays well within chrono's representable range")
}

fn format_month(date: NaiveDate) -> String {
    date.format("%b %y").to_string().to_lowercase()
}

fn tagged_spend_window_tag() -> String {
    format!(
        "monthly · {} – {}",
        format_month(spend_chart_month(0)),
        format_month(spend_chart_month(SPEND_MONTHS - 1))
    )
}

/// The sparkline itself — mirrors `view::dashboard::render_net_worth_chart`'s own line-graph
/// treatment exactly: one accent-coloured `Dataset` (not a plain line plus a separate
/// last-point marker), and x-axis labels at the first/middle/last month so the chart reads as
/// a graph on its own, without needing the footer line beneath it for orientation.
fn render_spend_chart(frame: &mut Frame, area: Rect, series: &[f64]) {
    let points: Vec<(f64, f64)> = series
        .iter()
        .enumerate()
        .map(|(index, &amount)| (index as f64, amount))
        .collect();

    let min = series.iter().copied().fold(f64::INFINITY, f64::min);
    let max = series.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let margin = (max - min).abs().max(1.0) * 0.1;

    let dataset = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(ACCENT))
        .data(&points);

    let last_index = series.len() - 1;
    let mid_index = last_index / 2;
    let x_labels = [
        format_month(spend_chart_month(0)),
        format_month(spend_chart_month(mid_index)),
        format_month(spend_chart_month(last_index)),
    ];

    let chart = Chart::new(vec![dataset])
        .x_axis(
            Axis::default()
                .bounds([0.0, last_index as f64])
                .labels(x_labels),
        )
        .y_axis(Axis::default().bounds([min - margin, max + margin]));
    frame.render_widget(chart, area);
}

/// `first used <month> · peak <month> <amount> · <latest month> <amount>` — the design doc's
/// own three-figure footer, or a plain empty-state line for a Tag with no transactions.
fn render_spend_footer(frame: &mut Frame, area: Rect, tag: &Tag, series: &[f64]) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    if tag.tagged_transaction_count == 0 {
        frame.render_widget(
            Paragraph::new(Span::styled("no transactions yet", dim)),
            area,
        );
        return;
    }

    let first_used_index = series.iter().position(|&amount| amount != 0.0).unwrap_or(0);
    let (peak_index, peak_amount) = series
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(index, &amount)| (index, amount))
        .unwrap_or((0, 0.0));
    let last_index = series.len() - 1;

    let text = format!(
        "first used {} · peak {} {} · {} {}",
        format_month(spend_chart_month(first_used_index)),
        format_month(spend_chart_month(peak_index)),
        format_amount(peak_amount),
        format_month(spend_chart_month(last_index)),
        format_amount(series[last_index]),
    );
    frame.render_widget(Paragraph::new(Span::styled(text, dim)), area);
}

/// One "Where it lands" row: a category-path label, a proportional block-glyph bar plus its
/// percentage, and the right-aligned amount — `total <= 0.0` (a Tag with no spend) renders an
/// empty bar and `0%` rather than dividing by zero.
fn render_category_bar(frame: &mut Frame, area: Rect, label: &str, amount: f64, total: f64) {
    let fraction = if total > 0.0 { amount / total } else { 0.0 };
    let pct = (fraction * 100.0).round() as i64;
    let bar_len = (fraction * BAR_MAX_CELLS as f64).round() as usize;
    let bar = "█".repeat(bar_len);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(CATEGORY_LABEL_WIDTH),
            Constraint::Min(0),
            Constraint::Length(RIGHT_AMOUNT_WIDTH),
        ])
        .spacing(1)
        .split(area);

    frame.render_widget(Paragraph::new(label.to_string()), columns[0]);
    frame.render_widget(Paragraph::new(format!("{bar} {pct}%")), columns[1]);
    frame.render_widget(
        Paragraph::new(format_amount(amount)).alignment(Alignment::Right),
        columns[2],
    );
}

/// Splits a Transactions row (or its column header) into `DATE`/`PAYEE`(`Min(0)`)/`CATEGORY`/
/// `T`/`AMOUNT` columns.
fn txn_row_columns(area: Rect) -> (Rect, Rect, Rect, Rect, Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(TXN_DATE_WIDTH),
            Constraint::Min(0),
            Constraint::Length(CATEGORY_LABEL_WIDTH),
            Constraint::Length(TXN_OTHER_TAGS_WIDTH),
            Constraint::Length(RIGHT_AMOUNT_WIDTH),
        ])
        .spacing(1)
        .split(area);
    (columns[0], columns[1], columns[2], columns[3], columns[4])
}

fn render_txn_column_header(frame: &mut Frame, area: Rect) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let (date, payee, category, other_tags, amount) = txn_row_columns(area);
    frame.render_widget(Paragraph::new(Span::styled("DATE", dim)), date);
    frame.render_widget(Paragraph::new(Span::styled("PAYEE", dim)), payee);
    frame.render_widget(Paragraph::new(Span::styled("CATEGORY", dim)), category);
    frame.render_widget(
        Paragraph::new(Span::styled("T", dim)).alignment(Alignment::Right),
        other_tags,
    );
    frame.render_widget(
        Paragraph::new(Span::styled("AMOUNT", dim)).alignment(Alignment::Right),
        amount,
    );
}

fn render_txn_row(frame: &mut Frame, area: Rect, row: &TagTransaction) {
    let (date_area, payee_area, category_area, other_tags_area, amount_area) =
        txn_row_columns(area);
    frame.render_widget(Paragraph::new(format_date(row.date)), date_area);
    frame.render_widget(Paragraph::new(row.payee), payee_area);
    frame.render_widget(Paragraph::new(row.category_path.clone()), category_area);
    frame.render_widget(
        Paragraph::new(row.other_tags.to_string()).alignment(Alignment::Right),
        other_tags_area,
    );
    frame.render_widget(
        Paragraph::new(format_amount(money_to_f64(&row.amount))).alignment(Alignment::Right),
        amount_area,
    );
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(view: &TagsView) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering the Tags view should not error");

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

    fn find_id(view: &TagsView, name: &str) -> RowID {
        view.store
            .tags()
            .iter()
            .find(|tag| tag.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a tag named {name}"))
            .id
    }

    #[test]
    fn title_is_tags() {
        assert_eq!(TagsView::new().title(), "Tags");
    }

    #[test]
    fn renders_without_panicking() {
        render(&TagsView::new());
    }

    #[test]
    fn starts_selected_on_the_first_visible_tag_alphabetically() {
        let view = TagsView::new();
        // Active tags sorted alphabetically: Home Renovation, Japan Trip 2026, Tax Deductible.
        assert_eq!(view.selected_tag().unwrap().name, "Home Renovation");
    }

    #[test]
    fn default_view_shows_three_active_tags_and_hides_the_inactive_one() {
        let view = TagsView::new();
        let visible = view.visible_tags();
        assert_eq!(visible.len(), 3);
        assert!(!visible.iter().any(|tag| tag.name == "Old Project"));
    }

    #[test]
    fn za_reveals_the_inactive_tag() {
        let mut view = TagsView::new();
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('a')));
        assert_eq!(view.visible_tags().len(), 4);
    }

    #[test]
    fn j_and_k_move_selection_among_visible_tags_only() {
        let mut view = TagsView::new();
        let before = view.selected;
        view.handle_key(key(KeyCode::Char('j')));
        assert_ne!(view.selected, before);
        view.handle_key(key(KeyCode::Char('k')));
        assert_eq!(view.selected, before);
    }

    #[test]
    fn home_and_g_jump_to_the_first_and_last_visible_tags() {
        let mut view = TagsView::new();
        view.handle_key(key(KeyCode::Char('G')));
        assert_eq!(view.selected_tag().unwrap().name, "Tax Deductible");
        view.handle_key(key(KeyCode::Home));
        assert_eq!(view.selected_tag().unwrap().name, "Home Renovation");
    }

    #[test]
    fn slash_filters_the_list_by_name() {
        let mut view = TagsView::new();
        view.handle_key(key(KeyCode::Char('/')));
        for c in "japan".chars() {
            view.handle_key(key(KeyCode::Char(c)));
        }
        view.handle_key(key(KeyCode::Enter));

        let visible = view.visible_tags();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "Japan Trip 2026");
    }

    #[test]
    fn n_opens_the_new_tag_popup() {
        let mut view = TagsView::new();
        assert_eq!(
            view.handle_key(key(KeyCode::Char('n'))),
            Some(Action::OpenTagNewPopup)
        );
    }

    #[test]
    fn create_tag_selects_the_newly_created_tag() {
        let mut view = TagsView::new();
        let before_count = view.store.tags().len();

        view.update(&Action::CreateTag {
            name: "Wedding".to_string(),
            active: true,
            close_after: true,
        });

        assert_eq!(view.store.tags().len(), before_count + 1);
        let selected = view.selected_tag().expect("a selection exists");
        assert_eq!(selected.name, "Wedding");
    }

    #[test]
    fn create_tag_ignores_a_duplicate_name_without_mutating() {
        let mut view = TagsView::new();
        let before_count = view.store.tags().len();

        view.update(&Action::CreateTag {
            name: "Japan Trip 2026".to_string(),
            active: true,
            close_after: true,
        });

        assert_eq!(view.store.tags().len(), before_count);
    }

    #[test]
    fn e_opens_the_edit_tag_popup_for_the_selection() {
        let mut view = TagsView::new();
        let selected = view.selected;
        assert_eq!(
            view.handle_key(key(KeyCode::Char('e'))),
            Some(Action::OpenTagEditPopup(selected))
        );
    }

    #[test]
    fn tag_selection_returns_the_current_selection() {
        let view = TagsView::new();
        assert_eq!(view.tag_selection(), Some(view.selected));
    }

    #[test]
    fn set_tag_active_deactivates_and_recovers_selection() {
        let mut view = TagsView::new();
        let home_renovation = find_id(&view, "Home Renovation");
        view.selected = home_renovation;

        view.update(&Action::SetTagActive {
            id: home_renovation,
            active: false,
        });

        let tag = view.store.find(home_renovation).expect("still exists");
        assert!(!tag.is_active);
        // Deactivating the selected tag hides it (show_inactive defaults to false), so
        // recover_selection should have moved `selected` onto a still-visible tag.
        assert_ne!(view.selected, home_renovation);
    }

    #[test]
    fn arm_tag_delete_sets_pending_delete_without_mutating() {
        let mut view = TagsView::new();
        let before_count = view.store.tags().len();

        view.update(&Action::ArmTagDelete);

        assert!(view.pending_delete);
        assert_eq!(view.store.tags().len(), before_count);
    }

    #[test]
    fn update_tag_renames_and_deactivates_through_the_store_seam() {
        let mut view = TagsView::new();
        let home_renovation = find_id(&view, "Home Renovation");

        view.update(&Action::UpdateTag {
            id: home_renovation,
            name: "Reno 2026".to_string(),
            active: false,
        });

        let tag = view.store.find(home_renovation).expect("still exists");
        assert_eq!(tag.name, "Reno 2026");
        assert!(!tag.is_active);
    }

    #[test]
    fn d_then_any_other_key_aborts_the_delete_without_mutating() {
        let mut view = TagsView::new();
        let before_count = view.store.tags().len();
        view.handle_key(key(KeyCode::Char('d')));
        view.handle_key(key(KeyCode::Char('n')));

        assert!(!view.pending_delete);
        assert_eq!(view.store.tags().len(), before_count);
    }

    #[test]
    fn d_then_y_deletes_a_zero_reference_tag() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Home Renovation");
        let before_count = view.store.tags().len();

        view.handle_key(key(KeyCode::Char('d')));
        view.handle_key(key(KeyCode::Char('y')));

        assert_eq!(view.store.tags().len(), before_count - 1);
        assert!(
            view.store
                .find(find_id_unchecked(&view, "Home Renovation"))
                .is_none()
        );
    }

    #[test]
    fn d_then_y_deletes_a_non_zero_reference_tag_unconditionally() {
        let mut view = TagsView::new();
        let japan_trip = find_id(&view, "Japan Trip 2026");
        view.selected = japan_trip;

        view.handle_key(key(KeyCode::Char('d')));
        view.handle_key(key(KeyCode::Char('y')));

        assert!(view.store.find(japan_trip).is_none());
    }

    #[test]
    fn delete_confirm_line_states_the_reference_count_when_non_zero() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Japan Trip 2026");
        view.handle_key(key(KeyCode::Char('d')));

        let text = render(&view);
        assert!(text.contains("delete \"Japan Trip 2026\""));
        assert!(text.contains("7 transactions will lose this tag"));
        assert!(text.contains("y confirms, any other key cancels"));
    }

    #[test]
    fn delete_confirm_line_omits_the_notice_when_the_count_is_zero() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Home Renovation");
        view.handle_key(key(KeyCode::Char('d')));

        let text = render(&view);
        assert!(text.contains("delete \"Home Renovation\""));
        assert!(!text.contains("will lose this tag"));
    }

    #[test]
    fn delete_recovers_selection_onto_a_still_visible_tag() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Home Renovation");
        view.handle_key(key(KeyCode::Char('d')));
        view.handle_key(key(KeyCode::Char('y')));

        // Selection should have moved off the deleted tag onto a real, visible one.
        assert!(view.selected_tag().is_some());
    }

    #[test]
    fn summary_box_shows_active_tagged_count_and_dates() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Tax Deductible");
        let text = render(&view);

        assert!(text.contains("Tax Deductible"));
        assert!(text.contains("[×]"));
        assert!(text.contains("23 transactions"));
        assert!(text.contains("01 jul 2024"));
        assert!(text.contains("30 jun 2026"));
    }

    #[test]
    fn summary_box_shows_none_for_a_zero_reference_tag() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Home Renovation");
        let text = render(&view);
        assert!(text.contains("none"));
    }

    // --- Right pane: Tagged spend, Where it lands, Transactions (issue #133) ---

    #[test]
    fn right_pane_shows_empty_states_for_a_zero_reference_tag() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Home Renovation");
        let text = render(&view);

        assert!(text.contains("TAGGED SPEND"));
        assert!(text.contains("no transactions yet"));
        assert!(text.contains("WHERE IT LANDS"));
        assert!(text.contains("no categories yet"));
        assert!(text.contains("TRANSACTIONS"));
        assert!(text.contains("0 of 0 · newest first"));
    }

    #[test]
    fn switching_selection_between_zero_and_real_tags_re_renders_every_widget() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Home Renovation");
        let empty_text = render(&view);
        assert!(empty_text.contains("no transactions yet"));
        assert!(empty_text.contains("no categories yet"));

        view.selected = find_id(&view, "Tax Deductible");
        let real_text = render(&view);
        assert!(!real_text.contains("no transactions yet"));
        assert!(!real_text.contains("no categories yet"));
        assert!(real_text.contains("carry another tag"));
    }

    #[test]
    fn tagged_spend_footer_shows_first_used_peak_and_latest() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Tax Deductible");
        let text = render(&view);

        assert!(text.contains("monthly ·"));
        assert!(text.contains("first used"));
        assert!(text.contains("peak"));
    }

    #[test]
    fn tagged_spend_chart_shows_x_axis_month_labels_like_the_dashboards_own_chart() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Tax Deductible");
        let text = render(&view);

        // Mirrors `view::dashboard::render_net_worth_chart`'s own first/middle/last x-axis
        // labels — the chart should read as a graph on its own, not just a footer statement.
        assert!(text.contains("oct 24"));
        assert!(text.contains("sep 26"));
    }

    #[test]
    fn where_it_lands_rolls_up_past_four_categories() {
        let view = TagsView::new();
        let tag = find_id(&view, "Tax Deductible");
        let breakdown = view.store.category_breakdown(tag);
        assert!(
            breakdown.len() > CATEGORY_ROWS_SHOWN,
            "fixture should seed more than {CATEGORY_ROWS_SHOWN} categories for this tag"
        );

        let mut view = view;
        view.selected = tag;
        let text = render(&view);
        let rolled_up = breakdown.len() - CATEGORY_ROWS_SHOWN;
        assert!(text.contains(&format!("{rolled_up} more")));
    }

    #[test]
    fn category_bar_length_is_proportional_and_capped_at_bar_max_cells() {
        let render_bar = |amount: f64, total: f64| -> usize {
            let backend = TestBackend::new(60, 1);
            let mut terminal = Terminal::new(backend).expect("test backend should initialise");
            terminal
                .draw(|frame| render_category_bar(frame, frame.area(), "test", amount, total))
                .expect("rendering a bar row should not error");
            let buffer = terminal.backend().buffer();
            let mut line = String::new();
            for x in 0..buffer.area.width {
                line.push_str(buffer[(x, 0)].symbol());
            }
            line.chars().filter(|&c| c == '█').count()
        };

        assert_eq!(render_bar(50.0, 100.0), BAR_MAX_CELLS / 2);
        assert_eq!(render_bar(100.0, 100.0), BAR_MAX_CELLS);
        assert_eq!(render_bar(0.0, 100.0), 0);
    }

    #[test]
    fn overlap_footer_count_matches_the_fixtures_own_rows() {
        let mut view = TagsView::new();
        let tag = find_id(&view, "Tax Deductible");
        let rows = view.store.transactions(tag);
        let expected_overlap = rows.iter().filter(|row| row.other_tags > 0).count();

        view.selected = tag;
        let text = render(&view);
        assert!(text.contains(&format!(
            "{expected_overlap} of {} carry another tag",
            rows.len()
        )));
    }

    #[test]
    fn enter_shows_the_not_yet_built_message_for_transactions() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Tax Deductible");
        assert_eq!(view.handle_key(key(KeyCode::Enter)), Some(Action::NoOp));
        assert!(view.transactions_not_yet_built);

        let text = render(&view);
        assert!(text.contains("opening filtered Transactions — not yet built"));
    }

    #[test]
    fn any_other_key_clears_the_not_yet_built_message() {
        let mut view = TagsView::new();
        view.selected = find_id(&view, "Tax Deductible");
        view.handle_key(key(KeyCode::Enter));
        assert!(view.transactions_not_yet_built);

        view.handle_key(key(KeyCode::Char('j')));
        assert!(!view.transactions_not_yet_built);
        let text = render(&view);
        assert!(!text.contains("not yet built"));
    }

    // A tiny helper distinct from `find_id` so the delete tests above read as "find, then
    // assert it's gone" without the lookup itself panicking once the tag no longer exists.
    fn find_id_unchecked(view: &TagsView, name: &str) -> RowID {
        view.store
            .tags()
            .iter()
            .find(|tag| tag.name == name)
            .map(|tag| tag.id)
            .unwrap_or_default()
    }
}
