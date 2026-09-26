//! The "edit tag" popup — `e` on a Tags list row, or `:tag edit <tag>` (issue #129, "Tags:
//! edit popup"). Follows `crate::popup::account::edit`'s structure (the closest precedent: a
//! prefilled draft, global — not sibling-scoped — uniqueness check on rename, `^a` as a quick
//! "deactivate" distinct from toggling the checkbox and saving) while being genuinely
//! interactive and genuinely mutating the fixture.
//!
//! **Editable: `name` (rename in place, no alias/history trail per ADR-0015) and `active`,
//! nothing else** — unlike `popup::account::edit`, Tag has no fixed-at-creation fields (`unit`/
//! `starting_balance`) to show read-only, and no computed section; `view::tags`'s own summary
//! box already shows `tagged_transaction_count`/created/updated, so this popup stays as small
//! as `popup::tag::new`.
//!
//! **No `^d` (delete)** — unlike `popup::account::edit`'s hand-off to `popup::account::delete`,
//! Tag's delete is the lightweight arm-and-confirm built directly into `view::tags::TagsView`
//! ("Tags: screen — list, summary box, and lightweight delete"), not a popup this one could
//! hand off to.

use lib_core::RowID;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::msg;
use crate::popup::REFERENCE_TERMINAL_WIDTH;
use crate::tag::TagStore;

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 60;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;
const LABEL_WIDTH: u16 = "active".len() as u16 + 1;

/// Content rows inside the border: title, its rule, the two fields, a blank spacer, a
/// one-line note, a blank spacer, the footer's rule, then the footer itself — matches
/// `popup::tag::new`'s own `CONTENT_ROWS` exactly.
const CONTENT_ROWS: u16 = 1 + 1 + 2 + 1 + 1 + 1 + 1 + 1;
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

/// Which editable field currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Active,
}

impl Field {
    fn next(self) -> Field {
        match self {
            Field::Name => Field::Active,
            Field::Active => Field::Name,
        }
    }
}

/// The `:tag edit` popup's own draft state, prefilled from the Tag being edited.
pub struct EditTagPopup {
    editing_id: RowID,
    name: String,
    active: bool,
    focus: Field,
}

impl EditTagPopup {
    /// Opens a popup editing `editing_id`, prefilling `name`/`active` from its current values.
    /// `TagsView::handle_key` only ever opens this against the current selection, which always
    /// names a real Tag, so the fallbacks here (`String::new`/`true`) never actually arise —
    /// they exist so a dangling id can't panic this, mirroring `EditAccountPopup::new`.
    pub fn new(store: &dyn TagStore, editing_id: RowID) -> Self {
        let tag = store.find(editing_id);
        Self {
            editing_id,
            name: tag.map(|t| t.name.clone()).unwrap_or_default(),
            active: tag.map(|t| t.is_active).unwrap_or(true),
            focus: Field::Name,
        }
    }

    pub fn editing_id(&self) -> RowID {
        self.editing_id
    }

    pub fn push_char(&mut self, c: char) {
        match self.focus {
            Field::Name => self.name.push(c),
            Field::Active if c == ' ' => self.active = !self.active,
            Field::Active => {}
        }
    }

    pub fn backspace(&mut self) {
        if self.focus == Field::Name {
            self.name.pop();
        }
    }

    pub fn tab(&mut self) {
        self.focus = self.focus.next();
    }

    /// Whether `name` case-insensitively clashes with a Tag other than the one being edited —
    /// mirrors `popup::tag::new`'s own `name_taken`, excluding `editing_id` (renaming a Tag to
    /// its own current name is never a clash).
    fn name_taken(&self, store: &dyn TagStore, name: &str) -> bool {
        store
            .tags()
            .iter()
            .any(|tag| tag.id != self.editing_id && tag.name.eq_ignore_ascii_case(name))
    }

    /// The `(id, name, active)` `^s` would save, or `None` while the draft doesn't validate —
    /// an empty name, or one that case-insensitively clashes with another Tag.
    pub fn save_fields(&self, store: &dyn TagStore) -> Option<(RowID, String, bool)> {
        let name = self.name.trim();
        if name.is_empty() || self.name_taken(store, name) {
            return None;
        }
        Some((self.editing_id, name.to_string(), self.active))
    }

    /// The `(id, name, active)` `^a` would save — the draft as typed, but with `active` forced
    /// `false` regardless of the checkbox's own current value, per the ticket's own `^a`
    /// "deactivates without requiring the checkbox to be toggled first", mirroring
    /// `EditAccountPopup::deactivate_fields`.
    pub fn deactivate_fields(&self, store: &dyn TagStore) -> Option<(RowID, String, bool)> {
        self.save_fields(store)
            .map(|(id, name, _)| (id, name, false))
    }

    /// Renders the floating overlay, centred within `area`.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, store: &dyn TagStore) {
        let popup = popup_rect(area);

        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title
                Constraint::Length(1), // rule
                Constraint::Length(1), // name
                Constraint::Length(1), // active
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // note
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // rule
                Constraint::Length(1), // footer hints
            ])
            .split(inner);

        render_title(frame, rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
        render_text_field(
            frame,
            rows[2],
            &msg::tui_tag_edit_field_name(),
            &self.name,
            self.focus == Field::Name,
        );
        render_active_field(frame, rows[3], self.active, self.focus == Field::Active);
        // rows[4] is left blank — breathing space above the note.
        render_clash_note(frame, rows[5], store, self);
        // rows[6] is left blank — breathing space above the footer rule.
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[7]);
        render_footer_hints(frame, rows[8]);
    }
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// The title row: "edit tag" flush left, the `:tag edit` command dim and right-aligned.
fn render_title(frame: &mut Frame<'_>, area: Rect) {
    let tag = msg::tui_tag_edit_command();
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(msg::tui_tag_edit_title()), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim())).alignment(Alignment::Right),
        columns[1],
    );
}

fn render_field(frame: &mut Frame<'_>, area: Rect, label: &str, value: Line<'static>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled(label, dim())), columns[0]);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

fn render_text_field(frame: &mut Frame<'_>, area: Rect, label: &str, value: &str, focused: bool) {
    let mut spans = vec![Span::raw(value.to_string())];
    if focused {
        spans.push(Span::styled("\u{258c}", Style::default().fg(ACCENT)));
    }
    render_field(frame, area, label, Line::from(spans));
}

/// The `active` checkbox row: the glyph in the accent when focused, "clear to deactivate" as
/// the consequence — mirrors `popup::account::edit`'s own wording (distinct from `popup::tag::
/// new`'s "offered when tagging", since this is an edit, not a fresh creation).
fn render_active_field(frame: &mut Frame<'_>, area: Rect, active: bool, focused: bool) {
    let glyph_style = if focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };
    let glyph = if active { "[\u{d7}]" } else { "[ ]" };
    let note = format!("· {}", msg::tui_tag_edit_active_note());
    render_field(
        frame,
        area,
        &msg::tui_tag_edit_field_active(),
        Line::from(vec![
            Span::styled(glyph, glyph_style),
            Span::raw(" "),
            Span::styled(note, dim()),
        ]),
    );
}

/// States that renaming is always safe (nothing derives from a Tag's name), and flags a live
/// clash the moment the typed name matches another Tag — mirrors `popup::tag::new`'s own
/// `render_clash_note`, excluding the Tag being edited.
fn render_clash_note(
    frame: &mut Frame<'_>,
    area: Rect,
    store: &dyn TagStore,
    popup: &EditTagPopup,
) {
    let trimmed = popup.name.trim();
    if !trimmed.is_empty() && popup.name_taken(store, trimmed) {
        frame.render_widget(
            Paragraph::new(Span::styled(
                msg::tui_tag_edit_note_clash(trimmed),
                Style::default().fg(ACCENT),
            )),
            area,
        );
        return;
    }
    frame.render_widget(
        Paragraph::new(Span::styled(msg::tui_tag_edit_note_unique(), dim())),
        area,
    );
}

fn render_footer_hints(frame: &mut Frame<'_>, area: Rect) {
    let hints = [
        ("tab", msg::tui_tag_edit_help_tab()),
        ("^s", msg::tui_tag_edit_help_save()),
        ("^a", msg::tui_tag_edit_help_deactivate()),
        ("esc", msg::tui_tag_edit_help_cancel()),
    ];
    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let label_style = dim();

    let mut spans = Vec::with_capacity(hints.len() * 3);
    for (index, (key, label)) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(*key, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(label.clone(), label_style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Computes a centred popup `Rect` sized to [`POPUP_HEIGHT`]'s fixed field list — mirrors
/// `popup::tag::new`'s own `popup_rect`.
fn popup_rect(area: Rect) -> Rect {
    let width = POPUP_WIDTH.min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;

    let height = POPUP_HEIGHT.min(area.height);
    let anchor_y = area.y + (area.height / 6).max(1);
    let y = anchor_y.min(area.y + area.height.saturating_sub(height));

    Rect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::tag::TagFixture;

    fn find_id(store: &TagFixture, name: &str) -> RowID {
        store
            .tags()
            .iter()
            .find(|tag| tag.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a tag named {name}"))
            .id
    }

    fn render(popup: &EditTagPopup, store: &dyn TagStore) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area(), store))
            .expect("rendering the popup should not error");

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

    #[test]
    fn new_prefills_name_and_active_from_the_current_tag() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let popup = EditTagPopup::new(&store, japan_trip);

        assert_eq!(popup.name, "Japan Trip 2026");
        assert!(popup.active);
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn new_prefills_inactive_correctly() {
        let store = TagFixture::new();
        let old_project = find_id(&store, "Old Project");
        let popup = EditTagPopup::new(&store, old_project);
        assert!(!popup.active);
    }

    #[test]
    fn renders_without_panicking() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        render(&EditTagPopup::new(&store, japan_trip), &store);
    }

    #[test]
    fn tab_cycles_focus_between_the_two_fields_and_wraps() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let mut popup = EditTagPopup::new(&store, japan_trip);
        assert_eq!(popup.focus, Field::Name);
        popup.tab();
        assert_eq!(popup.focus, Field::Active);
        popup.tab();
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn typing_appends_to_the_name_field() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let mut popup = EditTagPopup::new(&store, japan_trip);
        popup.push_char('!');
        assert_eq!(popup.name, "Japan Trip 2026!");
    }

    #[test]
    fn space_toggles_active_only_when_it_has_focus() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let mut popup = EditTagPopup::new(&store, japan_trip);
        assert!(popup.active);

        popup.push_char(' '); // focus is Name, so this is just a literal space
        assert_eq!(popup.name, "Japan Trip 2026 ");
        assert!(popup.active);

        popup.focus = Field::Active;
        popup.push_char(' ');
        assert!(!popup.active);
    }

    #[test]
    fn save_fields_rejects_an_empty_name() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let mut popup = EditTagPopup::new(&store, japan_trip);
        popup.name.clear();

        assert_eq!(popup.save_fields(&store), None);
    }

    #[test]
    fn save_fields_allows_renaming_a_tag_to_its_own_current_name() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let popup = EditTagPopup::new(&store, japan_trip);

        assert_eq!(
            popup.save_fields(&store),
            Some((japan_trip, "Japan Trip 2026".to_string(), true))
        );
    }

    #[test]
    fn save_fields_rejects_a_case_insensitive_clash_with_another_tag() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let mut popup = EditTagPopup::new(&store, japan_trip);
        popup.name = "tax deductible".to_string(); // fixture seeds "Tax Deductible"

        assert_eq!(popup.save_fields(&store), None);
    }

    #[test]
    fn save_fields_returns_the_trimmed_name_and_active() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let mut popup = EditTagPopup::new(&store, japan_trip);
        popup.name = "  Japan Trip 2027  ".to_string();

        assert_eq!(
            popup.save_fields(&store),
            Some((japan_trip, "Japan Trip 2027".to_string(), true))
        );
    }

    #[test]
    fn deactivate_fields_forces_active_false_regardless_of_the_checkbox() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let popup = EditTagPopup::new(&store, japan_trip);
        assert!(popup.active, "starts active per the fixture");

        let (_, _, active) = popup
            .deactivate_fields(&store)
            .expect("should validate without toggling the checkbox first");
        assert!(!active);
    }

    #[test]
    fn deactivate_fields_still_rejects_an_invalid_name() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let mut popup = EditTagPopup::new(&store, japan_trip);
        popup.name.clear();

        assert_eq!(popup.deactivate_fields(&store), None);
    }

    #[test]
    fn shows_the_rename_note_by_default() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let text = render(&EditTagPopup::new(&store, japan_trip), &store);
        assert!(text.contains("renaming is always safe"));
    }

    #[test]
    fn shows_a_clash_warning_once_the_typed_name_matches_another_tag() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let mut popup = EditTagPopup::new(&store, japan_trip);
        popup.name = "Tax Deductible".to_string();

        let text = render(&popup, &store);
        assert!(text.contains("a tag named \"Tax Deductible\" already exists"));
    }

    #[test]
    fn shows_the_footer_hints_without_ctrl_d() {
        let store = TagFixture::new();
        let japan_trip = find_id(&store, "Japan Trip 2026");
        let text = render(&EditTagPopup::new(&store, japan_trip), &store);

        for key in ["tab", "^s", "^a", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
        assert!(
            !text.contains("^d"),
            "Tag's edit popup has no delete hand-off, unlike Account's"
        );
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }
}
