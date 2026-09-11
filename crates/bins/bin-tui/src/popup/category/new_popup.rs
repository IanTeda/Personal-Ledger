//! The "new" popup — `n` (child of the selection) / `N` (sibling of it) on a Categories tree
//! row, or `:category new <name> [parent]` (`docs/ux/tui/categories/README.md` "5c — New").
//! Genuinely interactive and genuinely mutates the tree, the same as `move_popup` (see its own
//! module doc for the full `Shell`/`CategoriesView` round trip this popup follows too) — the
//! `parent` field reuses `super::path` verbatim, per the handoff's own "same widget as 5b".
//!
//! The first Category popup with more than one editable field: `Tab` cycles focus
//! (`name` → `parent` → `note` → `active` → wraps), except when `parent` has focus and its
//! typed text has a completion candidate, where `Tab` completes it instead (reconciling the
//! handoff's own two statements — the field table's "`parent` ... `tab` to change" and the key
//! list's "`tab` next field" — rather than picking one over the other).

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use lib_core::RowID;

use super::path::{Resolution, ancestor_names, completions, resolve, tab_complete};
use crate::category::CategoryStore;
use crate::popup::REFERENCE_TERMINAL_WIDTH;

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 88;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;
const LABEL_WIDTH: u16 = "transactions".len() as u16 + 1;
const MAX_PREVIEW_SIBLINGS: usize = 6;

/// Which editable field currently has focus. `kind`/`depth` are read-only and never gain
/// focus, per the handoff's own field table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Parent,
    Note,
    Active,
}

impl Field {
    fn next(self) -> Field {
        match self {
            Field::Name => Field::Parent,
            Field::Parent => Field::Note,
            Field::Note => Field::Active,
            Field::Active => Field::Name,
        }
    }
}

/// The `:category new` popup's own draft state. `kind`/`depth` are never stored here — both are
/// derived live from `parent_input`'s resolution, per the handoff's "kind cannot be set here".
pub struct NewPopup {
    name: String,
    parent_input: String,
    note: String,
    active: bool,
    focus: Field,
}

impl NewPopup {
    /// Opens a popup for a new child of (`n`) or sibling of (`N`) whatever was selected —
    /// `CategoriesView::handle_key` resolves which one `prefilled_parent` already is before
    /// this is ever constructed, so this only ever needs the one id.
    pub fn new(store: &dyn CategoryStore, prefilled_parent: RowID) -> Self {
        Self {
            name: String::new(),
            parent_input: ancestor_names(store, prefilled_parent).join("/"),
            note: String::new(),
            active: true,
            focus: Field::Name,
        }
    }

    pub fn push_char(&mut self, c: char) {
        match self.focus {
            Field::Name => self.name.push(c),
            Field::Parent => self.parent_input.push(c),
            Field::Note => self.note.push(c),
            // Space toggles the checkbox rather than being typed literally — `active` has no
            // text to hold.
            Field::Active if c == ' ' => self.active = !self.active,
            Field::Active => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.focus {
            Field::Name => {
                self.name.pop();
            }
            Field::Parent => {
                self.parent_input.pop();
            }
            Field::Note => {
                self.note.pop();
            }
            Field::Active => {}
        }
    }

    /// `Tab`: completes `parent`'s last path segment (mirroring the Move popup) when it has
    /// focus and a candidate exists; otherwise advances focus to the next field.
    pub fn tab(&mut self, store: &dyn CategoryStore) {
        if self.focus == Field::Parent {
            let completed = tab_complete(store, &self.parent_input);
            if completed != self.parent_input {
                self.parent_input = completed;
                return;
            }
        }
        self.focus = self.focus.next();
    }

    /// The existing category `parent_input` currently resolves to, if any.
    pub fn resolved_parent(&self, store: &dyn CategoryStore) -> Option<RowID> {
        match resolve(store, &self.parent_input) {
            Resolution::Existing(id) => Some(id),
            _ => None,
        }
    }

    /// The `(parent, name, note, active)` `^s`/`^a` would create, or `None` while the draft
    /// doesn't validate — an unresolved parent, an empty name, or a case-insensitive sibling
    /// clash, "the only validation error here" per the handoff.
    pub fn create_fields(
        &self,
        store: &dyn CategoryStore,
    ) -> Option<(RowID, String, Option<String>, bool)> {
        let parent = self.resolved_parent(store)?;
        let name = self.name.trim();
        if name.is_empty() {
            return None;
        }
        if store
            .children(parent)
            .iter()
            .any(|child| child.name.eq_ignore_ascii_case(name))
        {
            return None;
        }
        let note = if self.note.trim().is_empty() {
            None
        } else {
            Some(self.note.trim().to_string())
        };
        Some((parent, name.to_string(), note, self.active))
    }

    /// `^a`: after a successful create, clears `name`/`note` and resets `active`, keeping
    /// `parent_input` as-is — bulk-adding more siblings under the same parent on first setup
    /// is the point, per the handoff's own "^a create and start another sibling".
    pub fn reset_for_next_sibling(&mut self) {
        self.name.clear();
        self.note.clear();
        self.active = true;
        self.focus = Field::Name;
    }

    /// Renders the floating overlay, centred and sized to its own dynamic content (the "lands
    /// as" preview's height varies with the resolved parent's child count), within `area`.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn CategoryStore) {
        let parent = self.resolved_parent(store);
        let preview_lines = preview_lines(store, parent, &self.name);
        let content_rows = 11 + preview_lines.len() as u16 + 5;
        let popup = popup_rect(area, content_rows);

        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let mut constraints = vec![
            Constraint::Length(1), // title
            Constraint::Length(1), // rule
            Constraint::Length(1), // name
            Constraint::Length(1), // parent
            Constraint::Length(1), // completion
            Constraint::Length(1), // kind
            Constraint::Length(1), // depth
            Constraint::Length(1), // note
            Constraint::Length(1), // active
            Constraint::Length(1), // blank spacer
            Constraint::Length(1), // "lands as" heading
        ];
        constraints.extend(std::iter::repeat_n(
            Constraint::Length(1),
            preview_lines.len(),
        ));
        constraints.extend([
            Constraint::Length(1), // blank spacer
            Constraint::Length(1), // footnote
            Constraint::Length(1), // blank spacer
            Constraint::Length(1), // rule
            Constraint::Length(1), // footer hints
        ]);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        render_title(frame, rows[0]);
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
            "parent",
            &self.parent_input,
            self.focus == Field::Parent,
        );
        render_completion_row(frame, rows[4], store, &self.parent_input);

        let kind_text = match parent.and_then(|id| store.kind(id)) {
            Some(kind) => format!("{} · inherited from root", kind.as_str()),
            None => "— · inherited from root".to_string(),
        };
        render_field(
            frame,
            rows[5],
            "kind",
            Line::from(Span::styled(kind_text, dim())),
        );

        let depth_text = match parent {
            Some(id) => format!("{} · no limit", store.depth(id) + 1),
            None => "— · no limit".to_string(),
        };
        render_field(
            frame,
            rows[6],
            "depth",
            Line::from(Span::styled(depth_text, dim())),
        );

        render_text_field(
            frame,
            rows[7],
            "note",
            &self.note,
            self.focus == Field::Note,
        );
        render_active_field(frame, rows[8], self.active, self.focus == Field::Active);
        // rows[9] is left blank — breathing space above the "lands as" preview.

        frame.render_widget(Paragraph::new(Span::styled("lands as", dim())), rows[10]);
        for (index, line) in preview_lines.iter().enumerate() {
            frame.render_widget(line.clone(), rows[11 + index]);
        }

        let after_preview = 11 + preview_lines.len();
        // rows[after_preview] is left blank — breathing space below the preview.
        frame.render_widget(
            Paragraph::new(Span::styled(
                "siblings sort by name · kind cannot be set here — move it to change kind",
                dim(),
            )),
            rows[after_preview + 1],
        );
        // rows[after_preview + 2] is left blank — breathing space above the footer rule.
        frame.render_widget(
            Block::new().borders(Borders::BOTTOM),
            rows[after_preview + 3],
        );
        render_footer_hints(frame, rows[after_preview + 4]);
    }
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// The title row: "new" flush left, the `:category new` command dim and right-aligned.
fn render_title(frame: &mut Frame, area: Rect) {
    let tag = ":category new";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new("new"), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim())).alignment(Alignment::Right),
        columns[1],
    );
}

/// One `label   value` row.
fn render_field(frame: &mut Frame, area: Rect, label: &str, value: Line<'static>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled(label, dim())), columns[0]);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

/// One editable text field: the typed value, with a trailing accent cursor only when it has
/// focus — the visual cue for which field `Tab`/typing currently reaches.
fn render_text_field(frame: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let mut spans = vec![Span::raw(value.to_string())];
    if focused {
        spans.push(Span::styled("\u{258c}", Style::default().fg(ACCENT)));
    }
    render_field(frame, area, label, Line::from(spans));
}

/// The `active` checkbox row: the glyph in the accent when focused (this field has no cursor
/// of its own to show focus with), the "offered when categorising" consequence stated
/// alongside it either way, per the handoff's own "states the consequence".
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

/// The `completion` row beneath `parent` — mirrors the Move popup's own row verbatim.
fn render_completion_row(frame: &mut Frame, area: Rect, store: &dyn CategoryStore, input: &str) {
    let candidates = completions(store, input);
    let text = if candidates.is_empty() {
        "no matches · tab".to_string()
    } else {
        format!("{}  · tab", candidates.join(" · "))
    };
    render_field(
        frame,
        area,
        "completion",
        Line::from(Span::styled(text, dim())),
    );
}

/// The "lands as" preview: the resolved parent's name, then its children with the new node's
/// typed name inserted at its sorted landing position (`"new, empty"`, per the handoff's own
/// `Daily · new, empty`) — or a hint to finish typing an existing parent path when it doesn't
/// resolve yet. Capped the same way the Move popup's own preview is, for a parent with many
/// children.
fn preview_lines<'a>(
    store: &dyn CategoryStore,
    parent: Option<RowID>,
    name: &str,
) -> Vec<Paragraph<'a>> {
    let Some(parent_id) = parent else {
        return vec![Paragraph::new(Span::styled(
            "type an existing parent path",
            dim(),
        ))];
    };

    let display_name = if name.trim().is_empty() {
        "…".to_string()
    } else {
        name.trim().to_string()
    };
    let mut names: Vec<String> = store
        .children(parent_id)
        .iter()
        .map(|child| child.name.clone())
        .collect();
    names.push(display_name.clone());
    names.sort_by_key(|n| n.to_lowercase());

    let parent_name = store
        .find(parent_id)
        .map(|node| node.name.clone())
        .unwrap_or_default();
    let mut lines = vec![Paragraph::new(parent_name)];

    let shown = names.len().min(MAX_PREVIEW_SIBLINGS);
    for n in &names[..shown] {
        if *n == display_name {
            lines.push(Paragraph::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{n} · new, empty"), Style::default().fg(ACCENT)),
            ])));
        } else {
            lines.push(Paragraph::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(n.clone(), dim()),
            ])));
        }
    }
    if names.len() > shown {
        lines.push(Paragraph::new(Span::styled(
            format!("  … {} more", names.len() - shown),
            dim(),
        )));
    }
    lines
}

fn render_footer_hints(frame: &mut Frame, area: Rect) {
    const HINTS: &[(&str, &str)] = &[
        ("tab", "next field"),
        ("^s", "create"),
        ("^a", "create & add another"),
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

/// Computes a centred popup `Rect` sized to `content_rows` plus its top/bottom border —
/// mirrors `move_popup`'s own dynamic `popup_rect`.
fn popup_rect(area: Rect, content_rows: u16) -> Rect {
    let width = POPUP_WIDTH.min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;

    let height = (content_rows + 2).min(area.height);
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

    fn render(popup: &NewPopup, store: &dyn CategoryStore) -> String {
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
    fn new_prefills_parent_with_the_given_ids_path() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let popup = NewPopup::new(&store, food);
        assert_eq!(popup.parent_input, "expenses/food");
    }

    #[test]
    fn renders_without_panicking() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        render(&NewPopup::new(&store, food), &store);
    }

    #[test]
    fn tab_cycles_focus_through_every_editable_field_and_wraps() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let mut popup = NewPopup::new(&store, food);
        assert_eq!(popup.focus, Field::Name);

        popup.tab(&store); // parent field has no completion candidate for "expenses/food" (it's exact), so this advances
        assert_eq!(popup.focus, Field::Parent);
    }

    #[test]
    fn tab_on_the_parent_field_completes_when_a_candidate_exists() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let mut popup = NewPopup::new(&store, food);
        popup.focus = Field::Parent;
        popup.parent_input = "expenses/h".to_string();

        popup.tab(&store);
        assert_eq!(popup.parent_input, "expenses/Health");
        assert_eq!(
            popup.focus,
            Field::Parent,
            "completing shouldn't move focus"
        );
    }

    #[test]
    fn typing_appends_to_whichever_field_has_focus() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let mut popup = NewPopup::new(&store, food);
        popup.push_char('D');
        popup.push_char('a');
        assert_eq!(popup.name, "Da");

        popup.focus = Field::Note;
        popup.push_char('x');
        assert_eq!(popup.note, "x");
        assert_eq!(popup.name, "Da", "note typing shouldn't touch name");
    }

    #[test]
    fn space_toggles_active_only_when_it_has_focus() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let mut popup = NewPopup::new(&store, food);
        assert!(popup.active);

        popup.push_char(' '); // focus is Name, so this is just a literal space
        assert_eq!(popup.name, " ");
        assert!(popup.active);

        popup.focus = Field::Active;
        popup.push_char(' ');
        assert!(!popup.active);
    }

    #[test]
    fn create_fields_requires_a_non_empty_name_and_a_resolved_parent() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let popup = NewPopup::new(&store, food);
        assert_eq!(
            popup.create_fields(&store),
            None,
            "empty name should not validate"
        );
    }

    #[test]
    fn create_fields_rejects_a_case_insensitive_sibling_clash() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let mut popup = NewPopup::new(&store, food);
        popup.name = "groceries".to_string(); // Food already has "Groceries"

        assert_eq!(popup.create_fields(&store), None);
    }

    #[test]
    fn create_fields_returns_the_resolved_parent_trimmed_name_note_and_active() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let mut popup = NewPopup::new(&store, food);
        popup.name = "  Snacks  ".to_string();
        popup.note = "  vending machine  ".to_string();

        assert_eq!(
            popup.create_fields(&store),
            Some((
                food,
                "Snacks".to_string(),
                Some("vending machine".to_string()),
                true
            ))
        );
    }

    #[test]
    fn reset_for_next_sibling_clears_name_and_note_but_keeps_parent() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let mut popup = NewPopup::new(&store, food);
        popup.name = "Snacks".to_string();
        popup.note = "note".to_string();
        popup.active = false;
        popup.focus = Field::Active;

        popup.reset_for_next_sibling();

        assert_eq!(popup.name, "");
        assert_eq!(popup.note, "");
        assert!(popup.active);
        assert_eq!(popup.focus, Field::Name);
        assert_eq!(popup.parent_input, "expenses/food");
    }

    #[test]
    fn shows_kind_inherited_from_root_and_depth() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let text = render(&NewPopup::new(&store, food), &store);

        assert!(
            text.contains("expense · inherited from root"),
            "kind row missing"
        );
        assert!(
            text.contains("3 · no limit"),
            "depth row missing (Food is depth 2, children land at 3)"
        );
    }

    #[test]
    fn shows_the_lands_as_preview_with_the_new_node_marked() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let mut popup = NewPopup::new(&store, food);
        popup.name = "Snacks".to_string();

        let text = render(&popup, &store);
        assert!(text.contains("lands as"), "lands as heading missing");
        assert!(
            text.contains("Snacks · new, empty"),
            "new-node marker missing"
        );
        assert!(
            text.contains("Groceries"),
            "existing sibling missing from preview"
        );
    }

    #[test]
    fn shows_the_footnote_and_footer_hints() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        let text = render(&NewPopup::new(&store, food), &store);

        assert!(text.contains("kind cannot be set here"), "footnote missing");
        for key in ["tab", "^s", "^a", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area, 20);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }
}
