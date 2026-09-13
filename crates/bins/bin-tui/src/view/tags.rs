//! The Tags `View`, hosted by `Shell` (ADR-0013). Per the "Tags catalog screen, views and
//! popup" map's own grilling session (no pre-drawn `docs/ux/tui/tags/README.md` exists, unlike
//! Categories/Accounts): a flat, alphabetically-sorted list (Tag has no classification
//! dimension to group by) plus a summary box for whichever Tag is selected — one pane, no
//! right-hand chart or ledger, since Tag has no computed series worth one.
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
//! **No `g`-jump chord or Dashboard row exist yet** — see [`Action::OpenTags`]'s own doc for
//! why; this `View` is fully built and tested, just not yet reachable from a live key.
//!
//! **`n`/`e` open the new/edit-tag popups** (`crate::popup::tag::new`/`edit`, "Tags: new
//! popup"/"Tags: edit popup") — `Shell` owns the popups themselves, mirroring `view::
//! accounts`'s own `n`/`e`/`d`; this `View` only ever resolves *which* key to turn into
//! [`Action::OpenTagNewPopup`]/[`Action::OpenTagEditPopup`], and reacts to the eventual
//! [`Action::CreateTag`]/[`Action::UpdateTag`] in [`TagsView::update`].

use crossterm::event::{KeyCode, KeyEvent};
use lib_core::RowID;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use crate::tag::{Tag, TagFixture, TagStore};
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

        match key.code {
            KeyCode::Char('z') => {
                self.pending_z = true;
                None
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
            .split(rows[2]);
        // columns[1] is deliberately left blank — there's no right-pane content for Tag.

        let pane_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(SUMMARY_CONTENT_ROWS + 4), // name + rule + content + 2 border
            ])
            .split(columns[0]);

        self.render_list(frame, pane_rows[0]);
        self.render_summary(frame, pane_rows[1]);
    }

    fn title(&self) -> &'static str {
        "Tags"
    }

    fn tag_store(&self) -> Option<&dyn TagStore> {
        Some(&self.store)
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
