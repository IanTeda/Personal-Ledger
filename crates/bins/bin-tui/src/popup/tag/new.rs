//! The "new tag" popup — `n` on the Tags list, or `:tag new <name>` (issue #128, "Tags: new
//! popup"). Follows `crate::popup::unit::new`'s module/enum-variant/`render()` shape (the
//! simplest existing popup — no tree/parent concept to resolve, closest to what Tag's own two
//! fields need) while being genuinely interactive and genuinely creating a Tag in the fixture,
//! the same way `crate::popup::account::new` is.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::popup::REFERENCE_TERMINAL_WIDTH;
use crate::tag::TagStore;

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 60;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;
const LABEL_WIDTH: u16 = "active".len() as u16 + 1;

/// Content rows inside the border: title, its rule, the two fields, a blank spacer, a
/// one-line note, a blank spacer, the footer's rule, then the footer itself.
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

/// The `:tag new` popup's own draft state.
pub struct NewTagPopup {
    name: String,
    active: bool,
    focus: Field,
}

impl NewTagPopup {
    /// Opens a fresh, blank popup — `active` defaults to `[×]`, focus starts on `name` (the
    /// first and only text field; unlike Account's `type`, Tag has nothing that needs a
    /// default pick instead).
    pub fn new() -> Self {
        Self {
            name: String::new(),
            active: true,
            focus: Field::Name,
        }
    }

    pub fn push_char(&mut self, c: char) {
        match self.focus {
            Field::Name => self.name.push(c),
            // Space toggles the checkbox rather than being typed literally — `active` has no
            // text to hold.
            Field::Active if c == ' ' => self.active = !self.active,
            Field::Active => {}
        }
    }

    pub fn backspace(&mut self) {
        if self.focus == Field::Name {
            self.name.pop();
        }
    }

    /// `Tab`: advances focus to the other field — only two fields, neither with a completion
    /// candidate to apply first, unlike `popup::account::new`'s `unit` or `popup::category::
    /// new_popup`'s `parent`.
    pub fn tab(&mut self) {
        self.focus = self.focus.next();
    }

    /// Whether `name` case-insensitively clashes with an existing Tag (active or not) —
    /// mirrors `TagFixture`'s own private `name_taken`, duplicated here since the popup only
    /// has read access to `&dyn TagStore`.
    fn name_taken(store: &dyn TagStore, name: &str) -> bool {
        store
            .tags()
            .iter()
            .any(|tag| tag.name.eq_ignore_ascii_case(name))
    }

    /// The `(name, active)` `^s`/`^a` would create, or `None` while the draft doesn't
    /// validate — an empty name, or one that case-insensitively clashes with an existing Tag,
    /// per ADR-0015's global (not sibling-scoped) uniqueness rule.
    pub fn create_fields(&self, store: &dyn TagStore) -> Option<(String, bool)> {
        let name = self.name.trim();
        if name.is_empty() || Self::name_taken(store, name) {
            return None;
        }
        Some((name.to_string(), self.active))
    }

    /// `^a`: after a successful create, clears `name` and resets `active` to its own default —
    /// bulk-adding more Tags is the point, per `popup::account::new`'s own "create and start
    /// another".
    pub fn reset_for_next_tag(&mut self) {
        self.name.clear();
        self.active = true;
        self.focus = Field::Name;
    }

    /// Renders the floating overlay, centred within `area`.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn TagStore) {
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
            "name",
            &self.name,
            self.focus == Field::Name,
        );
        render_active_field(frame, rows[3], self.active, self.focus == Field::Active);
        // rows[4] is left blank — breathing space above the note.
        render_clash_note(frame, rows[5], store, &self.name);
        // rows[6] is left blank — breathing space above the footer rule.
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[7]);
        render_footer_hints(frame, rows[8]);
    }
}

impl Default for NewTagPopup {
    fn default() -> Self {
        Self::new()
    }
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// The title row: "new tag" flush left, the `:tag new` command dim and right-aligned.
fn render_title(frame: &mut Frame, area: Rect) {
    let tag = ":tag new";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new("new tag"), columns[0]);
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
/// focus.
fn render_text_field(frame: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let mut spans = vec![Span::raw(value.to_string())];
    if focused {
        spans.push(Span::styled("\u{258c}", Style::default().fg(ACCENT)));
    }
    render_field(frame, area, label, Line::from(spans));
}

/// The `active` checkbox row: the glyph in the accent when focused, the "offered when
/// tagging" consequence stated alongside it either way — mirrors `view::tags`'s own summary
/// box wording.
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
            Span::styled("· offered when tagging", dim()),
        ]),
    );
}

/// States the uniqueness rule plainly, and flags a live clash the moment the typed name
/// matches an existing Tag — there's no dedicated error row elsewhere in this popup, so this
/// line doubles as both the explanation and the feedback.
fn render_clash_note(frame: &mut Frame, area: Rect, store: &dyn TagStore, name: &str) {
    let trimmed = name.trim();
    if !trimmed.is_empty() && NewTagPopup::name_taken(store, trimmed) {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("a tag named \"{trimmed}\" already exists"),
                Style::default().fg(ACCENT),
            )),
            area,
        );
        return;
    }
    frame.render_widget(
        Paragraph::new(Span::styled(
            "name must be globally unique, regardless of case",
            dim(),
        )),
        area,
    );
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

/// Computes a centred popup `Rect` sized to [`POPUP_HEIGHT`]'s fixed field list — mirrors
/// `popup::account::new`'s own `popup_rect` (a fixed field list, no dynamic preview to size
/// around).
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

    fn render(popup: &NewTagPopup, store: &dyn TagStore) -> String {
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
    fn new_defaults_to_active_and_name_focused() {
        let popup = NewTagPopup::new();
        assert!(popup.active);
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn renders_without_panicking() {
        let store = TagFixture::new();
        render(&NewTagPopup::new(), &store);
    }

    #[test]
    fn tab_cycles_focus_between_the_two_fields_and_wraps() {
        let mut popup = NewTagPopup::new();
        assert_eq!(popup.focus, Field::Name);
        popup.tab();
        assert_eq!(popup.focus, Field::Active);
        popup.tab();
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn typing_appends_to_the_name_field_only_when_it_has_focus() {
        let mut popup = NewTagPopup::new();
        popup.push_char('H');
        popup.push_char('i');
        assert_eq!(popup.name, "Hi");

        popup.focus = Field::Active;
        popup.push_char('x');
        assert_eq!(
            popup.name, "Hi",
            "active field isn't text, so this is ignored"
        );
    }

    #[test]
    fn space_toggles_active_only_when_it_has_focus() {
        let mut popup = NewTagPopup::new();
        assert!(popup.active);

        popup.push_char(' '); // focus is Name, so this is just a literal space
        assert_eq!(popup.name, " ");
        assert!(popup.active);

        popup.focus = Field::Active;
        popup.push_char(' ');
        assert!(!popup.active);
    }

    #[test]
    fn backspace_only_touches_the_name_field() {
        let mut popup = NewTagPopup::new();
        popup.name = "Trip".to_string();
        popup.focus = Field::Active;
        popup.backspace();
        assert_eq!(popup.name, "Trip", "active field has nothing to erase");

        popup.focus = Field::Name;
        popup.backspace();
        assert_eq!(popup.name, "Tri");
    }

    #[test]
    fn create_fields_requires_a_non_empty_name() {
        let store = TagFixture::new();
        let popup = NewTagPopup::new();
        assert_eq!(
            popup.create_fields(&store),
            None,
            "empty name should not validate"
        );
    }

    #[test]
    fn create_fields_rejects_a_case_insensitive_clash_against_an_active_tag() {
        let store = TagFixture::new();
        let mut popup = NewTagPopup::new();
        popup.name = "japan trip 2026".to_string(); // fixture seeds "Japan Trip 2026"
        assert_eq!(popup.create_fields(&store), None);
    }

    #[test]
    fn create_fields_rejects_a_case_insensitive_clash_against_an_inactive_tag() {
        let store = TagFixture::new();
        let mut popup = NewTagPopup::new();
        popup.name = "OLD PROJECT".to_string(); // fixture seeds inactive "Old Project"
        assert_eq!(popup.create_fields(&store), None);
    }

    #[test]
    fn create_fields_returns_the_trimmed_name_and_active() {
        let store = TagFixture::new();
        let mut popup = NewTagPopup::new();
        popup.name = "  Wedding  ".to_string();
        popup.active = false;

        assert_eq!(
            popup.create_fields(&store),
            Some(("Wedding".to_string(), false))
        );
    }

    #[test]
    fn reset_for_next_tag_clears_name_and_resets_active_and_focus() {
        let mut popup = NewTagPopup::new();
        popup.name = "First".to_string();
        popup.active = false;
        popup.focus = Field::Active;

        popup.reset_for_next_tag();

        assert_eq!(popup.name, "");
        assert!(popup.active);
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn shows_the_uniqueness_note_by_default() {
        let store = TagFixture::new();
        let text = render(&NewTagPopup::new(), &store);
        assert!(text.contains("name must be globally unique, regardless of case"));
    }

    #[test]
    fn shows_a_clash_warning_once_the_typed_name_matches_an_existing_tag() {
        let store = TagFixture::new();
        let mut popup = NewTagPopup::new();
        popup.name = "Japan Trip 2026".to_string();

        let text = render(&popup, &store);
        assert!(text.contains("a tag named \"Japan Trip 2026\" already exists"));
    }

    #[test]
    fn shows_the_footer_hints() {
        let store = TagFixture::new();
        let text = render(&NewTagPopup::new(), &store);
        for key in ["tab", "^s", "^a", "esc"] {
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
