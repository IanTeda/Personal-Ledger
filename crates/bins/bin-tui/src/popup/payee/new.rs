//! The "new payee" popup — `n` on the Payees list, or `:payee new <name>`
//! (`docs/ux/tui/payees/README.md` "8b — New"). Genuinely interactive and genuinely creates a
//! Payee in the fixture, following `popup::account::new`'s structural shape.
//!
//! **This form is the rare path, not the main one** — a Payee is auto-created the first time
//! its name is typed on a Transaction (see `crate::payee`'s own module doc), so this form exists
//! only to pre-seed a default category or a match before an import lands. The note rendered
//! beneath the fields says so verbatim, per the handoff's own instruction that a user reaching
//! this form without it would reasonably assume every Payee must be created by hand.
//!
//! **`name` is checked against the live fixture before submit**: a case-insensitive collision
//! refuses `create_fields` outright (rendered as an inline warning) rather than surfacing
//! `PayeeError::DuplicateName` after the fact — the handoff's own "a collision resolves to the
//! existing payee rather than creating a second".
//!
//! **`icon url` has no independent free-text state while `[×] derive from website` is
//! checked** — the value shown (and the value `create_fields` submits) is always computed live
//! from `website` (`{website}/favicon.ico`), never stored separately, so editing `website`
//! can't leave a stale derived value behind. Unchecking reveals `icon_url_input`, a genuinely
//! separate field, and `Tab` skips over it entirely while checked (there is nothing to edit).
//!
//! **`default` completes against `crate::payee::known_category_paths`** — every distinct
//! category path currently in use as some other Payee's own default, mirroring
//! `popup::account::new`'s own `unit` field completing against `crate::account::known_units`
//! (there is no live Category registry either `View` can reach into at this map's fidelity).

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    msg,
    payee::{Payee, PayeeStore, known_category_paths},
    popup::REFERENCE_TERMINAL_WIDTH,
};

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 88;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;
const LABEL_WIDTH: u16 = "icon url".len() as u16 + 1;

/// Content rows inside the border: title, its rule, `name`, its status row, `website`, `icon
/// url`'s checkbox, its value row, `default`, `active`, a blank spacer, the two-line note, a
/// blank spacer, the footer's rule, then the footer itself.
const CONTENT_ROWS: u16 = 1 + 1 + 8 + 1 + 2 + 1 + 1 + 1;
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

/// Which editable field currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Website,
    IconDerive,
    IconUrl,
    Default,
    Active,
}

impl Field {
    /// Advances to the next field, skipping `IconUrl` entirely while `icon_derive` is checked
    /// — there's nothing to edit there (see this module's own doc).
    fn next(self, icon_derive: bool) -> Field {
        match self {
            Field::Name => Field::Website,
            Field::Website => Field::IconDerive,
            Field::IconDerive if icon_derive => Field::Default,
            Field::IconDerive => Field::IconUrl,
            Field::IconUrl => Field::Default,
            Field::Default => Field::Active,
            Field::Active => Field::Name,
        }
    }
}

/// The `:payee new` popup's own draft state.
pub struct NewPayeePopup {
    name: String,
    website: String,
    icon_derive: bool,
    icon_url_input: String,
    default_input: String,
    active: bool,
    focus: Field,
}

impl NewPayeePopup {
    /// Opens a fresh, blank popup — `derive from website` and `active` both default to
    /// checked, per the handoff's own field table; focus starts on `name`, the first text
    /// field (unlike `popup::account::new`'s `type`, nothing here needs a different default).
    pub fn new() -> Self {
        Self {
            name: String::new(),
            website: String::new(),
            icon_derive: true,
            icon_url_input: String::new(),
            default_input: String::new(),
            active: true,
            focus: Field::Name,
        }
    }

    pub fn push_char(&mut self, c: char) {
        match self.focus {
            Field::Name => self.name.push(c),
            Field::Website => self.website.push(c),
            // Space toggles the checkbox rather than being typed literally.
            Field::IconDerive if c == ' ' => self.icon_derive = !self.icon_derive,
            Field::IconDerive => {}
            Field::IconUrl => self.icon_url_input.push(c),
            Field::Default => self.default_input.push(c),
            Field::Active if c == ' ' => self.active = !self.active,
            Field::Active => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.focus {
            Field::Name => {
                self.name.pop();
            }
            Field::Website => {
                self.website.pop();
            }
            Field::IconUrl => {
                self.icon_url_input.pop();
            }
            Field::Default => {
                self.default_input.pop();
            }
            Field::IconDerive | Field::Active => {}
        }
    }

    /// `Tab`: completes `default` against a known category path (case-insensitive prefix
    /// match) when it has focus and a candidate exists; otherwise advances focus, skipping
    /// `icon url` while `[×] derive from website` is checked.
    pub fn tab(&mut self, store: &dyn PayeeStore) {
        if self.focus == Field::Default {
            let needle = self.default_input.to_lowercase();
            if let Some(candidate) = known_category_paths(store.payees())
                .into_iter()
                .find(|path| path.to_lowercase().starts_with(&needle))
                && candidate != self.default_input
            {
                self.default_input = candidate;
                return;
            }
        }
        self.focus = self.focus.next(self.icon_derive);
    }

    /// The icon URL derived from `website` (`{website}/favicon.ico`) — `None` while `website`
    /// is blank, since there's nothing to derive from yet.
    fn derived_icon_url(&self) -> Option<String> {
        let website = self.website.trim();
        if website.is_empty() {
            None
        } else {
            Some(format!("{website}/favicon.ico"))
        }
    }

    /// The existing Payee `name` would collide with (case-insensitively), if any — see this
    /// module's own doc on why that refuses submission rather than surfacing a raw uniqueness
    /// error afterwards.
    pub fn name_conflict<'a>(&self, store: &'a dyn PayeeStore) -> Option<&'a Payee> {
        let name = self.name.trim();
        if name.is_empty() {
            return None;
        }
        store.find_by_name(name)
    }

    /// The `(name, website, icon_url, icon_derived, default_category_path, active)` `^s`/`^a`
    /// would create, or `None` while the draft doesn't validate — an empty `name`, or one that
    /// collides with an existing Payee.
    #[allow(clippy::type_complexity)]
    pub fn create_fields(
        &self,
        store: &dyn PayeeStore,
    ) -> Option<(
        String,
        Option<String>,
        Option<String>,
        bool,
        Option<String>,
        bool,
    )> {
        let name = self.name.trim();
        if name.is_empty() || self.name_conflict(store).is_some() {
            return None;
        }

        let non_empty = |value: &str| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        };
        let website = non_empty(&self.website);
        let icon_url = if self.icon_derive {
            self.derived_icon_url()
        } else {
            non_empty(&self.icon_url_input)
        };
        let default_category_path = non_empty(&self.default_input);

        Some((
            name.to_string(),
            website,
            icon_url,
            self.icon_derive,
            default_category_path,
            self.active,
        ))
    }

    /// `^a`: after a successful create, clears every field back to its own default —
    /// pre-seeding several Payees in a row shares no field worth keeping, unlike
    /// `popup::account::new`'s own `unit_input` (bulk-adding accounts in one Unit is the
    /// point there; nothing here has an equivalent "same again" case).
    pub fn reset_for_next_payee(&mut self) {
        self.name.clear();
        self.website.clear();
        self.icon_derive = true;
        self.icon_url_input.clear();
        self.default_input.clear();
        self.active = true;
        self.focus = Field::Name;
    }

    /// Renders the floating overlay, centred within `area`.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn PayeeStore) {
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
                Constraint::Length(1), // name status
                Constraint::Length(1), // website
                Constraint::Length(1), // icon derive checkbox
                Constraint::Length(1), // icon value (preview or input)
                Constraint::Length(1), // default
                Constraint::Length(1), // active
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // note line 1
                Constraint::Length(1), // note line 2
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
        render_name_status(frame, rows[3], self.name_conflict(store));
        render_text_field(
            frame,
            rows[4],
            "website",
            &self.website,
            self.focus == Field::Website,
        );
        render_icon_derive_field(
            frame,
            rows[5],
            self.icon_derive,
            self.focus == Field::IconDerive,
        );
        render_icon_value(
            frame,
            rows[6],
            self.icon_derive,
            self.derived_icon_url(),
            &self.icon_url_input,
            self.focus == Field::IconUrl,
        );
        render_text_field(
            frame,
            rows[7],
            "default",
            &self.default_input,
            self.focus == Field::Default,
        );
        render_active_field(frame, rows[8], self.active, self.focus == Field::Active);
        // rows[9] is left blank — breathing space above the note.
        render_note(
            frame,
            rows[10],
            "a payee is created automatically the first time its name is",
        );
        render_note(
            frame,
            rows[11],
            "typed on a transaction — this form only pre-seeds one first",
        );
        // rows[12] is left blank — breathing space above the footer rule.
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[13]);
        render_footer_hints(frame, rows[14]);
    }
}

impl Default for NewPayeePopup {
    fn default() -> Self {
        Self::new()
    }
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// The title row: "new payee" flush left, the `:payee new` command dim and right-aligned.
fn render_title(frame: &mut Frame, area: Rect) {
    let tag = ":payee new";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new("new payee"), columns[0]);
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

/// The accent warning beneath `name` when it collides with an existing Payee — blank
/// otherwise, so the row's height never changes.
fn render_name_status(frame: &mut Frame, area: Rect, conflict: Option<&Payee>) {
    if let Some(existing) = conflict {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("a payee named \"{}\" already exists", existing.name),
                Style::default().fg(ACCENT),
            )),
            area,
        );
    }
}

/// The `icon url` checkbox row: `[×]`/`[ ]` in the accent when focused, the consequence stated
/// alongside it either way.
fn render_icon_derive_field(frame: &mut Frame, area: Rect, checked: bool, focused: bool) {
    let glyph_style = if focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };
    let glyph = if checked { "[\u{d7}]" } else { "[ ]" };
    render_field(
        frame,
        area,
        "icon url",
        Line::from(vec![
            Span::styled(glyph, glyph_style),
            Span::raw(" derive from website"),
        ]),
    );
}

/// The row beneath `icon url`'s checkbox: the derived value, dim (or "not set" while
/// `website` is blank), when checked; an editable free-text field, with its own cursor, when
/// unchecked — the handoff's own "unchecking exposes a free-text URL field".
fn render_icon_value(
    frame: &mut Frame,
    area: Rect,
    icon_derive: bool,
    derived: Option<String>,
    input: &str,
    focused: bool,
) {
    if icon_derive {
        let text = derived.unwrap_or_else(|| "not set — needs a website".to_string());
        render_field(frame, area, "", Line::from(Span::styled(text, dim())));
    } else {
        render_text_field(frame, area, "", input, focused);
    }
}

/// The `active` checkbox row: the glyph in the accent when focused, the consequence stated
/// alongside it either way — mirrors `popup::account::new::render_active_field`.
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
            Span::styled("· offered as a suggestion", dim()),
        ]),
    );
}

fn render_note(frame: &mut Frame, area: Rect, text: &'static str) {
    frame.render_widget(Paragraph::new(Span::styled(text, dim())), area);
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
/// `popup::account::new::popup_rect`.
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
    use crate::payee::PayeeFixture;

    fn render(popup: &NewPayeePopup, store: &dyn PayeeStore) -> String {
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
    fn new_defaults_to_derive_active_and_name_focused() {
        let popup = NewPayeePopup::new();
        assert!(popup.icon_derive);
        assert!(popup.active);
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn renders_without_panicking() {
        let store = PayeeFixture::new();
        render(&NewPayeePopup::new(), &store);
    }

    #[test]
    fn tab_skips_icon_url_while_deriving_and_visits_it_once_unchecked() {
        let store = PayeeFixture::new();
        let mut popup = NewPayeePopup::new();
        popup.focus = Field::Website;

        popup.tab(&store); // website -> icon derive
        assert_eq!(popup.focus, Field::IconDerive);
        popup.tab(&store); // still checked -> skips icon url straight to default
        assert_eq!(popup.focus, Field::Default);

        popup.focus = Field::IconDerive;
        popup.icon_derive = false;
        popup.tab(&store); // unchecked -> icon url is now reachable
        assert_eq!(popup.focus, Field::IconUrl);
        popup.tab(&store); // icon url -> default
        assert_eq!(popup.focus, Field::Default);
    }

    #[test]
    fn tab_wraps_from_active_back_to_name() {
        let store = PayeeFixture::new();
        let mut popup = NewPayeePopup::new();
        popup.focus = Field::Active;
        popup.tab(&store);
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn tab_on_the_default_field_completes_when_a_candidate_exists() {
        let store = PayeeFixture::new();
        let mut popup = NewPayeePopup::new();
        popup.focus = Field::Default;
        popup.default_input = "food/groc".to_string();

        popup.tab(&store);
        assert_eq!(popup.default_input, "food/groceries");
        assert_eq!(
            popup.focus,
            Field::Default,
            "completing shouldn't move focus"
        );
    }

    #[test]
    fn typing_appends_to_whichever_field_has_focus() {
        let mut popup = NewPayeePopup::new();
        popup.focus = Field::Name;
        popup.push_char('H');
        popup.push_char('i');
        assert_eq!(popup.name, "Hi");
    }

    #[test]
    fn space_toggles_checkboxes_only_when_they_have_focus() {
        let mut popup = NewPayeePopup::new();
        assert!(popup.icon_derive);
        assert!(popup.active);

        popup.focus = Field::Name;
        popup.push_char(' '); // just a literal space in the name field
        assert_eq!(popup.name, " ");
        assert!(popup.icon_derive);

        popup.focus = Field::IconDerive;
        popup.push_char(' ');
        assert!(!popup.icon_derive);

        popup.focus = Field::Active;
        popup.push_char(' ');
        assert!(!popup.active);
    }

    #[test]
    fn derived_icon_url_is_none_until_a_website_is_typed() {
        let mut popup = NewPayeePopup::new();
        assert_eq!(popup.derived_icon_url(), None);
        popup.website = "bunnings.com.au".to_string();
        assert_eq!(
            popup.derived_icon_url(),
            Some("bunnings.com.au/favicon.ico".to_string())
        );
    }

    #[test]
    fn name_conflict_finds_an_existing_payee_case_insensitively() {
        let store = PayeeFixture::new();
        let mut popup = NewPayeePopup::new();
        popup.name = "woolworths".to_string();
        assert_eq!(
            popup.name_conflict(&store).map(|payee| payee.name.as_str()),
            Some("Woolworths")
        );
    }

    #[test]
    fn create_fields_requires_a_non_empty_non_colliding_name() {
        let store = PayeeFixture::new();
        let popup = NewPayeePopup::new();
        assert_eq!(
            popup.create_fields(&store),
            None,
            "empty name shouldn't validate"
        );

        let mut colliding = NewPayeePopup::new();
        colliding.name = "Woolworths".to_string();
        assert_eq!(
            colliding.create_fields(&store),
            None,
            "a colliding name shouldn't validate"
        );
    }

    #[test]
    fn create_fields_derives_the_icon_url_when_deriving_is_checked() {
        let store = PayeeFixture::new();
        let mut popup = NewPayeePopup::new();
        popup.name = "Bunnings".to_string();
        popup.website = "bunnings.com.au".to_string();
        popup.default_input = "housing/insurance".to_string();

        let (name, website, icon_url, icon_derived, default_category_path, active) =
            popup.create_fields(&store).expect("draft should validate");
        assert_eq!(name, "Bunnings");
        assert_eq!(website, Some("bunnings.com.au".to_string()));
        assert_eq!(icon_url, Some("bunnings.com.au/favicon.ico".to_string()));
        assert!(icon_derived);
        assert_eq!(default_category_path, Some("housing/insurance".to_string()));
        assert!(active);
    }

    #[test]
    fn create_fields_uses_the_typed_icon_url_once_unchecked() {
        let store = PayeeFixture::new();
        let mut popup = NewPayeePopup::new();
        popup.name = "Bunnings".to_string();
        popup.website = "bunnings.com.au".to_string();
        popup.icon_derive = false;
        popup.icon_url_input = "cdn.example.com/logo.png".to_string();

        let (.., icon_url, icon_derived, _, _) = popup.create_fields(&store).expect("valid draft");
        assert_eq!(icon_url, Some("cdn.example.com/logo.png".to_string()));
        assert!(!icon_derived);
    }

    #[test]
    fn reset_for_next_payee_clears_every_field() {
        let mut popup = NewPayeePopup::new();
        popup.name = "First".to_string();
        popup.website = "first.com".to_string();
        popup.icon_derive = false;
        popup.icon_url_input = "x".to_string();
        popup.default_input = "food/groceries".to_string();
        popup.active = false;

        popup.reset_for_next_payee();

        assert_eq!(popup.name, "");
        assert_eq!(popup.website, "");
        assert!(popup.icon_derive);
        assert_eq!(popup.icon_url_input, "");
        assert_eq!(popup.default_input, "");
        assert!(popup.active);
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn shows_the_note_and_footer_hints() {
        let store = PayeeFixture::new();
        let text = render(&NewPayeePopup::new(), &store);
        assert!(text.contains("a payee is created automatically"));
        for key in ["tab", "^s", "^a", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
    }

    #[test]
    fn shows_a_conflict_warning_when_the_typed_name_collides() {
        let store = PayeeFixture::new();
        let mut popup = NewPayeePopup::new();
        popup.name = "Woolworths".to_string();
        let text = render(&popup, &store);
        assert!(text.contains("already exists"));
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }
}
