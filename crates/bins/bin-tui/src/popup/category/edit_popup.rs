//! The "edit" popup — `e` on a Categories tree row, or `:cat edit <cat>`
//! (`docs/ux/tui/categories/README.md` "5d — Edit"). Genuinely interactive and genuinely
//! mutates the tree, the same as `move_popup`/`new_popup` (see `move_popup`'s own module doc
//! for the full `Shell`/`CategoriesView` round trip every Category popup follows).
//!
//! **Merge and Delete are "Not yet designed"** in the handoff, and — unlike Move/New, which
//! are both fully specified — this popup's own key list (`tab`/`^s`/`^a`/`X`/`esc`) names no
//! `delete` key at all (only descriptive text about the *rule*, "delete is refused while
//! transactions exist"), so there's nothing to wire for it. `X` (merge) *is* a listed key, so
//! it's wired — to the "not yet built" fallback the closed "Command popup finishing" map
//! (#89/#94) established, not to real merge logic. `CategoriesView`'s own tree-screen `X`
//! reuses the same fallback (`CategoriesView::merge_hint`); this popup gets its own
//! (`show_not_yet_built`) since it isn't `CategoriesView`'s to hold.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use lib_core::RowID;

use crate::category::CategoryStore;
use crate::popup::REFERENCE_TERMINAL_WIDTH;

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 88;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;
const LABEL_WIDTH: u16 = "transactions".len() as u16 + 1;

/// Content rows inside the border: title, rule, the three editable fields, the three
/// read-only fields, a blank spacer, the four-line semantics block, the footer's rule, then
/// the footer itself — always this many, unlike Move/New's dynamic preview.
const CONTENT_ROWS: u16 = 1 + 1 + 3 + 3 + 1 + 4 + 1 + 1;

/// Which editable field currently has focus. `parent`/`kind`/`children` are read-only and
/// never gain focus, per the handoff's own field table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Note,
    Active,
}

impl Field {
    fn next(self) -> Field {
        match self {
            Field::Name => Field::Note,
            Field::Note => Field::Active,
            Field::Active => Field::Name,
        }
    }
}

/// The `:cat edit` popup's own draft state, prefilled from the category being edited.
pub struct EditPopup {
    editing_id: RowID,
    name: String,
    note: String,
    active: bool,
    focus: Field,
    /// `true` right after `X` — shows the "not yet built" fallback in place of the title's
    /// command tag until any other key is pressed (`push_char`/`backspace`/`tab` all clear
    /// it first).
    show_not_yet_built: bool,
}

impl EditPopup {
    /// Opens a popup editing `editing_id`, prefilling every editable field from its current
    /// values. `CategoriesView::handle_key` never opens this for a root (roots have no
    /// editable name/note/active), so every read here can fall back to a sane default without
    /// that case actually arising.
    pub fn new(store: &dyn CategoryStore, editing_id: RowID) -> Self {
        let node = store.find(editing_id);
        Self {
            editing_id,
            name: node.map(|n| n.name.clone()).unwrap_or_default(),
            note: node.and_then(|n| n.note.clone()).unwrap_or_default(),
            active: node.map(|n| n.active).unwrap_or(true),
            focus: Field::Name,
            show_not_yet_built: false,
        }
    }

    pub fn editing_id(&self) -> RowID {
        self.editing_id
    }

    pub fn push_char(&mut self, c: char) {
        self.show_not_yet_built = false;
        match self.focus {
            Field::Name => self.name.push(c),
            Field::Note => self.note.push(c),
            Field::Active if c == ' ' => self.active = !self.active,
            Field::Active => {}
        }
    }

    pub fn backspace(&mut self) {
        self.show_not_yet_built = false;
        match self.focus {
            Field::Name => {
                self.name.pop();
            }
            Field::Note => {
                self.note.pop();
            }
            Field::Active => {}
        }
    }

    pub fn tab(&mut self) {
        self.show_not_yet_built = false;
        self.focus = self.focus.next();
    }

    /// `X`: shows the "not yet built" fallback — merge has no real logic yet.
    pub fn trigger_not_yet_built(&mut self) {
        self.show_not_yet_built = true;
    }

    /// The trimmed, sibling-clash-checked `name` `^s`/`^a` would save, or `None` while it
    /// doesn't validate — empty, or a case-insensitive clash with a sibling other than itself
    /// (rename's own rule, "the sibling clash is the only validation error here" mirrors New's
    /// wording for the same reason).
    fn validated_name(&self, store: &dyn CategoryStore) -> Option<String> {
        let name = self.name.trim();
        if name.is_empty() {
            return None;
        }
        let parent_id = store.find(self.editing_id)?.parent_id?;
        if store
            .children(parent_id)
            .iter()
            .any(|sibling| sibling.id != self.editing_id && sibling.name.eq_ignore_ascii_case(name))
        {
            return None;
        }
        Some(name.to_string())
    }

    /// The `(id, name, note, active)` `^s` would save — the draft as typed, or `None` while
    /// `name` doesn't validate.
    pub fn save_fields(
        &self,
        store: &dyn CategoryStore,
    ) -> Option<(RowID, String, Option<String>, bool)> {
        let name = self.validated_name(store)?;
        Some((self.editing_id, name, self.note_field(), self.active))
    }

    /// The `(id, name, note, active)` `^a` would save — the draft as typed, but with `active`
    /// forced `false` regardless of the checkbox's own current value, per the handoff's `^a`
    /// "archive" being its own quick action distinct from toggling `active` and saving.
    pub fn archive_fields(
        &self,
        store: &dyn CategoryStore,
    ) -> Option<(RowID, String, Option<String>, bool)> {
        self.save_fields(store)
            .map(|(id, name, note, _)| (id, name, note, false))
    }

    fn note_field(&self) -> Option<String> {
        if self.note.trim().is_empty() {
            None
        } else {
            Some(self.note.trim().to_string())
        }
    }

    /// Renders the floating overlay, centred and fixed-height (no dynamic preview, unlike
    /// Move/New), within `area`.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn CategoryStore) {
        let Some(node) = store.find(self.editing_id) else {
            return;
        };
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
                Constraint::Length(1), // note
                Constraint::Length(1), // active
                Constraint::Length(1), // parent
                Constraint::Length(1), // kind
                Constraint::Length(1), // children
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // semantics line 1
                Constraint::Length(1), // semantics line 2
                Constraint::Length(1), // semantics line 3 (accent)
                Constraint::Length(1), // semantics line 4
                Constraint::Length(1), // rule
                Constraint::Length(1), // footer hints
            ])
            .split(inner);

        render_title(frame, rows[0], self.show_not_yet_built);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

        render_text_field(
            frame,
            rows[2],
            "name",
            &self.name,
            self.focus == Field::Name,
        );
        render_text_field(
            frame,
            rows[3],
            "note",
            &self.note,
            self.focus == Field::Note,
        );
        render_active_field(frame, rows[4], self.active, self.focus == Field::Active);

        let parent_text = node
            .parent_id
            .and_then(|parent_id| store.find(parent_id))
            .map(|parent| format!("{} · m move", parent.name.to_lowercase()))
            .unwrap_or_else(|| "— · m move".to_string());
        render_field(
            frame,
            rows[5],
            "parent",
            Line::from(Span::styled(parent_text, dim())),
        );

        let kind_text = store
            .kind(self.editing_id)
            .map(|kind| format!("{} · follows the root", kind.as_str()))
            .unwrap_or_else(|| "— · follows the root".to_string());
        render_field(
            frame,
            rows[6],
            "kind",
            Line::from(Span::styled(kind_text, dim())),
        );

        let child_count = store.children(self.editing_id).len();
        let children_text = if child_count == 0 {
            "none · leaf · n new child".to_string()
        } else {
            format!("{child_count} · n new child")
        };
        render_field(
            frame,
            rows[7],
            "children",
            Line::from(Span::styled(children_text, dim())),
        );
        // rows[8] is left blank — breathing space above the semantics block.

        frame.render_widget(
            Paragraph::new(format!(
                "archiving keeps all {} transactions and totals;",
                node.transaction_count
            )),
            rows[9],
        );
        frame.render_widget(
            Paragraph::new("it only stops the category being offered."),
            rows[10],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(
                "delete is refused while transactions exist",
                Style::default().fg(ACCENT),
            )),
            rows[11],
        );
        frame.render_widget(
            Paragraph::new("X merge into another category instead"),
            rows[12],
        );

        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[13]);
        render_footer_hints(frame, rows[14]);
    }
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// The title row: "edit" flush left, the `:cat edit` command tag dim and right-aligned —
/// replaced by the "not yet built" fallback while `X` was just pressed.
fn render_title(frame: &mut Frame, area: Rect, show_not_yet_built: bool) {
    let tag = if show_not_yet_built {
        ":cat merge — not yet built".to_string()
    } else {
        ":cat edit".to_string()
    };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new("edit"), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim())).alignment(Alignment::Right),
        columns[1],
    );
}

fn render_field(frame: &mut Frame, area: Rect, label: &str, value: Line<'static>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled(label, dim())), columns[0]);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

fn render_text_field(frame: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let mut spans = vec![Span::raw(value.to_string())];
    if focused {
        spans.push(Span::styled("\u{258c}", Style::default().fg(ACCENT)));
    }
    render_field(frame, area, label, Line::from(spans));
}

fn render_active_field(frame: &mut Frame, area: Rect, active: bool, focused: bool) {
    let glyph_style = if focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };
    let glyph = if active { "[\u{d7}]" } else { "[ ]" };
    render_field(
        frame,
        area,
        "active",
        Line::from(vec![
            Span::styled(glyph, glyph_style),
            Span::raw(" "),
            Span::styled("· offered when categorising", dim()),
        ]),
    );
}

fn render_footer_hints(frame: &mut Frame, area: Rect) {
    const HINTS: &[(&str, &str)] = &[
        ("tab", "next field"),
        ("^s", "save"),
        ("^a", "archive"),
        ("X", "merge"),
        ("esc", "cancel"),
    ];
    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let label_style = dim();

    let mut spans = Vec::with_capacity(HINTS.len() * 3);
    for (index, (key, label)) in HINTS.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(*key, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*label, label_style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn popup_rect(area: Rect) -> Rect {
    let width = POPUP_WIDTH.min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;

    let height = (CONTENT_ROWS + 2).min(area.height);
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
    use crate::category::CategoryFixture;

    fn find_by_name(store: &CategoryFixture, name: &str) -> RowID {
        store
            .nodes()
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a category named {name}"))
            .id
    }

    fn render(popup: &EditPopup, store: &dyn CategoryStore) -> String {
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
    fn new_prefills_every_field_from_the_current_category() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let popup = EditPopup::new(&store, groceries);

        assert_eq!(popup.name, "Groceries");
        assert_eq!(popup.note, "supermarket, greengrocer");
        assert!(popup.active);
    }

    #[test]
    fn renders_without_panicking() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        render(&EditPopup::new(&store, groceries), &store);
    }

    #[test]
    fn tab_cycles_focus_through_name_note_active_and_wraps() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let mut popup = EditPopup::new(&store, groceries);
        assert_eq!(popup.focus, Field::Name);

        popup.tab();
        assert_eq!(popup.focus, Field::Note);
        popup.tab();
        assert_eq!(popup.focus, Field::Active);
        popup.tab();
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn typing_appends_to_whichever_field_has_focus() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let mut popup = EditPopup::new(&store, groceries);
        popup.push_char('!');
        assert_eq!(popup.name, "Groceries!");

        popup.focus = Field::Note;
        popup.push_char('?');
        assert_eq!(popup.note, "supermarket, greengrocer?");
    }

    #[test]
    fn save_fields_rejects_an_empty_name() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let mut popup = EditPopup::new(&store, groceries);
        popup.name.clear();

        assert_eq!(popup.save_fields(&store), None);
    }

    #[test]
    fn save_fields_rejects_a_case_insensitive_clash_with_a_different_sibling() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let mut popup = EditPopup::new(&store, groceries);
        popup.name = "restaurants".to_string(); // Food already has "Restaurants"

        assert_eq!(popup.save_fields(&store), None);
    }

    #[test]
    fn save_fields_allows_keeping_the_categorys_own_current_name() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let popup = EditPopup::new(&store, groceries);

        assert_eq!(
            popup.save_fields(&store),
            Some((
                groceries,
                "Groceries".to_string(),
                Some("supermarket, greengrocer".to_string()),
                true
            ))
        );
    }

    #[test]
    fn archive_fields_forces_active_false_regardless_of_the_checkbox() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let popup = EditPopup::new(&store, groceries);
        assert!(popup.active, "starts active per the fixture");

        let (_, _, _, active) = popup.archive_fields(&store).expect("should validate");
        assert!(!active);
    }

    #[test]
    fn x_shows_the_not_yet_built_fallback_until_another_key_clears_it() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let mut popup = EditPopup::new(&store, groceries);

        popup.trigger_not_yet_built();
        assert!(render(&popup, &store).contains("not yet built"));

        popup.push_char('x');
        assert!(!render(&popup, &store).contains("not yet built"));
    }

    #[test]
    fn shows_read_only_fields_with_their_owning_key_hints() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let text = render(&EditPopup::new(&store, groceries), &store);

        assert!(text.contains("food · m move"), "parent hint missing");
        assert!(
            text.contains("expense · follows the root"),
            "kind hint missing"
        );
        assert!(
            text.contains("none · leaf · n new child"),
            "children hint missing"
        );
    }

    #[test]
    fn shows_the_semantics_block_with_the_real_transaction_count() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let text = render(&EditPopup::new(&store, groceries), &store);

        assert!(text.contains("archiving keeps all 148 transactions and totals;"));
        assert!(text.contains("it only stops the category being offered."));
        assert!(text.contains("delete is refused while transactions exist"));
        assert!(text.contains("X merge into another category instead"));
    }

    #[test]
    fn shows_the_footer_hints() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        let text = render(&EditPopup::new(&store, groceries), &store);
        for key in ["tab", "^s", "^a", "X", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }
}
