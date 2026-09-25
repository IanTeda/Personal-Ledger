//! The "edit payee" popup — `e` on a Payees list row, or `:payee edit <payee>`
//! (`docs/ux/tui/payees/README.md` "8c — Edit"). Genuinely interactive and genuinely mutates
//! the fixture, following `popup::account::edit`'s structural shape.
//!
//! **`name` is a rename, and the form never lets that be a surprise**: boxed and focused by
//! default, with an `on save` block beneath it spelling out the exact consequence — the
//! rename itself, the alias pattern it will leave behind (rendered in the *exact* format
//! `lib_database::Payees::rename`'s real code produces: `format!("(?i)^{}$",
//! regex::escape(&current_name))` — a space isn't a regex meta character, so `WW Metro` reads
//! `(?i)^WW Metro$` with no backslash), and that existing Transactions show the new name at
//! once (they join by id, nothing is rewritten). A name that collides with another Payee
//! (case-insensitively) is caught here and named, before `^s` ever runs — never a raw
//! uniqueness error. A same-name-different-case edit is stated as "no rename pending"; nothing
//! about it writes an alias (mirrors `PayeeStore::rename`'s own no-op rule).
//!
//! **`icon url` has no separate "derived" checkbox here**, unlike `popup::payee::new` — this
//! is an edit, not a creation, so there's already a real value (or none) to show either way.
//! `icon_derived` is instead *inferred* on save: if the typed `icon url` equals what deriving
//! from the typed `website` would produce (`{website}/favicon.ico`), the save carries
//! `icon_derived = true`; any other typed value (or a blank `website`) carries `false`. A user
//! who clears `website` after an icon was derived from it simply stops being flagged as
//! derived — there's nothing left to derive from.
//!
//! **`m`/`^d` jump to `crate::popup::payee::matches`/`delete`** ("Payees: 8d rename matches
//! popup"/"8e delete popup") — `Shell` swaps this popup for that one, mirroring
//! `popup::account::edit`'s own `^d` hand-off to its delete popup.

use lib_core::RowID;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::popup::REFERENCE_TERMINAL_WIDTH;
use crate::{
    msg,
    payee::{PayeeStore, known_category_paths},
};

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 88;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;
const LABEL_WIDTH: u16 = "icon url".len() as u16 + 1;

/// Content rows inside the border: title, rule, the name box (border/value/caption/border),
/// the three-line rename preview, a blank spacer, `website`/`icon url`/`default`/`active`, a
/// blank spacer, `created`, a blank spacer, the footer's rule, then the footer itself.
const CONTENT_ROWS: u16 = 1 + 1 + 4 + 3 + 1 + 4 + 1 + 1 + 1 + 1 + 1;
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

/// Which editable field currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Website,
    IconUrl,
    Default,
    Active,
}

impl Field {
    fn next(self) -> Field {
        match self {
            Field::Name => Field::Website,
            Field::Website => Field::IconUrl,
            Field::IconUrl => Field::Default,
            Field::Default => Field::Active,
            Field::Active => Field::Name,
        }
    }
}

/// What `^s` would do to `name`, driving both the `on save` preview and whether saving is
/// allowed at all.
enum RenameStatus {
    /// The typed name is blank — never valid, regardless of the other fields.
    Empty,
    /// Unchanged, case-insensitively — a no-op, per `PayeeStore::rename`'s own rule (no alias
    /// written).
    NoChange,
    /// Collides (case-insensitively) with a Payee other than the one being edited.
    Collision(String),
    /// A genuine rename — carries the alias pattern it will leave behind, in its exact stored
    /// form.
    Renaming(String),
}

/// The `:payee edit` popup's own draft state, prefilled from the Payee being edited.
pub struct EditPayeePopup {
    editing_id: RowID,
    original_name: String,
    name: String,
    website: String,
    icon_url_input: String,
    default_input: String,
    active: bool,
    focus: Field,
}

impl EditPayeePopup {
    /// Opens a popup editing `editing_id`, prefilling every field from its current values.
    /// `PayeesView::handle_key` only ever opens this against the current selection, which
    /// always names a real Payee, so the empty-string/`true` fallbacks here never actually
    /// arise — they exist so a dangling id can't panic this, mirroring
    /// `popup::account::edit::EditAccountPopup::new`.
    pub fn new(store: &dyn PayeeStore, editing_id: RowID) -> Self {
        let payee = store.find(editing_id);
        let name = payee.map(|p| p.name.clone()).unwrap_or_default();
        Self {
            editing_id,
            original_name: name.clone(),
            name,
            website: payee.and_then(|p| p.website.clone()).unwrap_or_default(),
            icon_url_input: payee.and_then(|p| p.icon_url.clone()).unwrap_or_default(),
            default_input: payee
                .and_then(|p| p.default_category_path.clone())
                .unwrap_or_default(),
            active: payee.map(|p| p.is_active).unwrap_or(true),
            focus: Field::Name,
        }
    }

    pub fn editing_id(&self) -> RowID {
        self.editing_id
    }

    pub fn push_char(&mut self, c: char) {
        match self.focus {
            Field::Name => self.name.push(c),
            Field::Website => self.website.push(c),
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
            Field::Active => {}
        }
    }

    /// `Tab`: completes `default` against a known category path (case-insensitive prefix
    /// match) when it has focus and a candidate exists; otherwise advances focus.
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
        self.focus = self.focus.next();
    }

    fn rename_status(&self, store: &dyn PayeeStore) -> RenameStatus {
        let typed = self.name.trim();
        if typed.is_empty() {
            return RenameStatus::Empty;
        }
        if typed.eq_ignore_ascii_case(&self.original_name) {
            return RenameStatus::NoChange;
        }
        if let Some(holder) = store.find_by_name(typed)
            && holder.id != self.editing_id
        {
            return RenameStatus::Collision(holder.name.clone());
        }
        RenameStatus::Renaming(format!("(?i)^{}$", regex::escape(&self.original_name)))
    }

    /// Whether `icon_url_input` is exactly what deriving from `website` would produce —
    /// see this module's own doc on why that's how `icon_derived` is inferred here, with no
    /// separate checkbox.
    fn icon_looks_derived(&self) -> bool {
        let website = self.website.trim();
        !website.is_empty() && self.icon_url_input.trim() == format!("{website}/favicon.ico")
    }

    /// The `(id, name, website, icon_url, icon_derived, default_category_path, active)` `^s`
    /// would save, or `None` while the draft doesn't validate — an empty `name`, or one that
    /// collides with another Payee.
    #[allow(clippy::type_complexity)]
    fn fields(
        &self,
        store: &dyn PayeeStore,
        active: bool,
    ) -> Option<(
        RowID,
        String,
        Option<String>,
        Option<String>,
        bool,
        Option<String>,
        bool,
    )> {
        let name = self.name.trim();
        if name.is_empty() {
            return None;
        }
        if let Some(holder) = store.find_by_name(name)
            && holder.id != self.editing_id
        {
            return None;
        }

        let non_empty = |value: &str| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        };

        Some((
            self.editing_id,
            name.to_string(),
            non_empty(&self.website),
            non_empty(&self.icon_url_input),
            self.icon_looks_derived(),
            non_empty(&self.default_input),
            active,
        ))
    }

    /// The fields `^s` would save, as typed.
    #[allow(clippy::type_complexity)]
    pub fn save_fields(
        &self,
        store: &dyn PayeeStore,
    ) -> Option<(
        RowID,
        String,
        Option<String>,
        Option<String>,
        bool,
        Option<String>,
        bool,
    )> {
        self.fields(store, self.active)
    }

    /// The fields `^a` would save — the draft as typed, but with `active` forced `false`
    /// regardless of the checkbox's own current value, mirroring
    /// `popup::account::edit::EditAccountPopup::deactivate_fields`.
    #[allow(clippy::type_complexity)]
    pub fn deactivate_fields(
        &self,
        store: &dyn PayeeStore,
    ) -> Option<(
        RowID,
        String,
        Option<String>,
        Option<String>,
        bool,
        Option<String>,
        bool,
    )> {
        self.fields(store, false)
    }

    /// Renders the floating overlay, centred and fixed-height, within `area`.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn PayeeStore) {
        let Some(payee) = store.find(self.editing_id) else {
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
                Constraint::Length(4), // name box (border/value/caption/border)
                Constraint::Length(1), // on save line 1
                Constraint::Length(1), // on save line 2
                Constraint::Length(1), // on save line 3
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // website
                Constraint::Length(1), // icon url
                Constraint::Length(1), // default
                Constraint::Length(1), // active
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // created
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // rule
                Constraint::Length(1), // footer hints
            ])
            .split(inner);

        render_title(frame, rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
        render_name_box(frame, rows[2], &self.name, self.focus == Field::Name);
        render_rename_preview(
            frame,
            [rows[3], rows[4], rows[5]],
            &self.rename_status(store),
            &self.original_name,
            self.name.trim(),
            payee.transaction_count,
        );
        // rows[6] is left blank — breathing space above the rest of the form.
        render_text_field(
            frame,
            rows[7],
            "website",
            &self.website,
            self.focus == Field::Website,
        );
        render_text_field(
            frame,
            rows[8],
            "icon url",
            &self.icon_url_input,
            self.focus == Field::IconUrl,
        );
        render_text_field(
            frame,
            rows[9],
            "default",
            &self.default_input,
            self.focus == Field::Default,
        );
        render_active_field(frame, rows[10], self.active, self.focus == Field::Active);
        // rows[11] is left blank — breathing space above the read-only line.
        render_field(
            frame,
            rows[12],
            "created",
            Line::from(Span::styled(format_date(payee.created_on), dim())),
        );
        // rows[13] is left blank — breathing space above the footer rule.
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[14]);
        render_footer_hints(frame, rows[15]);
    }
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

fn render_title(frame: &mut Frame, area: Rect) {
    let tag = ":payee edit";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new("edit payee"), columns[0]);
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

/// `name`, boxed and captioned — the handoff's own "the form must never let this be a
/// surprise". A real bordered `Block`, not just an accented label: the visual weight is the
/// point.
fn render_name_box(frame: &mut Frame, area: Rect, name: &str, focused: bool) {
    let border_style = if focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };
    let block = Block::bordered().border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);
    render_text_field(frame, rows[0], "name", name, focused);
    frame.render_widget(
        Paragraph::new(Span::styled("changing this is a rename", dim())),
        rows[1],
    );
}

/// The three-line `on save` block beneath the name box — exactly one of "empty"/"no rename
/// pending"/a named collision/the real rename consequence, per [`RenameStatus`].
fn render_rename_preview(
    frame: &mut Frame,
    rows: [Rect; 3],
    status: &RenameStatus,
    current_name: &str,
    typed_name: &str,
    transaction_count: u32,
) {
    let accent = Style::default().fg(ACCENT);
    match status {
        RenameStatus::Empty => {
            frame.render_widget(
                Paragraph::new(Span::styled("name cannot be empty", accent)),
                rows[0],
            );
        }
        RenameStatus::NoChange => {
            frame.render_widget(
                Paragraph::new(Span::styled("no rename pending", dim())),
                rows[0],
            );
        }
        RenameStatus::Collision(holder) => {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    format!("a payee named \"{holder}\" already exists"),
                    accent,
                )),
                rows[0],
            );
        }
        RenameStatus::Renaming(pattern) => {
            frame.render_widget(
                Paragraph::new(format!("rename {current_name} \u{2192} {typed_name}")),
                rows[0],
            );
            frame.render_widget(
                Paragraph::new(Span::styled(format!("match kept {pattern} · auto"), dim())),
                rows[1],
            );
            frame.render_widget(
                Paragraph::new(Span::styled(
                    format!(
                        "{transaction_count} txns show the new name at once — no rows rewritten, they join by id"
                    ),
                    dim(),
                )),
                rows[2],
            );
        }
    }
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
            Span::styled("· clear to deactivate", dim()),
        ]),
    );
}

fn render_footer_hints(frame: &mut Frame, area: Rect) {
    const HINTS: &[(&str, &str)] = &[
        ("tab", "next field"),
        ("^s", "save"),
        ("^a", "deactivate"),
        ("m", "matches"),
        ("^d", "delete"),
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

fn format_date(date: chrono::NaiveDate) -> String {
    crate::format::date(date)
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::{msg, payee::PayeeFixture};

    fn find_id(store: &PayeeFixture, name: &str) -> RowID {
        store
            .payees()
            .iter()
            .find(|payee| payee.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a payee named {name}"))
            .id
    }

    fn render(popup: &EditPayeePopup, store: &dyn PayeeStore) -> String {
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
    fn new_prefills_every_field_from_the_current_payee() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let popup = EditPayeePopup::new(&store, woolworths);

        assert_eq!(popup.name, "Woolworths");
        assert_eq!(popup.website, "woolworths.com.au");
        assert_eq!(popup.icon_url_input, "woolworths.com.au/favicon.ico");
        assert_eq!(popup.default_input, "food/groceries");
        assert!(popup.active);
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn renders_without_panicking() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        render(&EditPayeePopup::new(&store, woolworths), &store);
    }

    #[test]
    fn tab_cycles_through_every_field_and_wraps() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = EditPayeePopup::new(&store, woolworths);
        assert_eq!(popup.focus, Field::Name);

        popup.tab(&store);
        assert_eq!(popup.focus, Field::Website);
        popup.tab(&store);
        assert_eq!(popup.focus, Field::IconUrl);
        popup.tab(&store);
        assert_eq!(popup.focus, Field::Default);
        popup.tab(&store);
        assert_eq!(popup.focus, Field::Active);
        popup.tab(&store);
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn tab_on_the_default_field_completes_when_a_candidate_exists() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = EditPayeePopup::new(&store, woolworths);
        popup.focus = Field::Default;
        popup.default_input = "housing/ins".to_string();

        popup.tab(&store);
        assert_eq!(popup.default_input, "housing/insurance");
    }

    #[test]
    fn typing_appends_to_whichever_field_has_focus() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = EditPayeePopup::new(&store, woolworths);
        popup.focus = Field::Website;
        popup.push_char('!');
        assert_eq!(popup.website, "woolworths.com.au!");
    }

    #[test]
    fn space_toggles_active_only_when_it_has_focus() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = EditPayeePopup::new(&store, woolworths);
        assert!(popup.active);

        popup.focus = Field::Website;
        popup.push_char(' ');
        assert!(popup.active);

        popup.focus = Field::Active;
        popup.push_char(' ');
        assert!(!popup.active);
    }

    #[test]
    fn rename_preview_renders_the_exact_stored_pattern_format_with_no_backslash_for_a_space() {
        let store = PayeeFixture::new();
        let coles_central = find_id(&store, "Coles Central");
        let mut popup = EditPayeePopup::new(&store, coles_central);
        popup.name = "WW Metro".to_string();

        let text = render(&popup, &store);
        assert!(text.contains("rename Coles Central \u{2192} WW Metro"));
        assert!(text.contains("(?i)^Coles Central$"));
        assert!(text.contains("no rows rewritten"));
    }

    #[test]
    fn same_name_different_case_shows_no_rename_pending_and_saves_without_writing_an_alias() {
        let mut store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let aliases_before = store.aliases(woolworths).len();
        let mut popup = EditPayeePopup::new(&store, woolworths);
        popup.name = "WOOLWORTHS".to_string();

        let text = render(&popup, &store);
        assert!(text.contains("no rename pending"));

        let (id, name, ..) = popup.save_fields(&store).expect("draft should validate");
        store.rename(id, name).expect("no-op rename should succeed");
        assert_eq!(store.aliases(woolworths).len(), aliases_before);
    }

    #[test]
    fn a_colliding_name_is_refused_and_the_holder_is_named() {
        let store = PayeeFixture::new();
        let coles_central = find_id(&store, "Coles Central");
        let mut popup = EditPayeePopup::new(&store, coles_central);
        popup.name = "woolworths".to_string();

        let text = render(&popup, &store);
        assert!(text.contains("a payee named \"Woolworths\" already exists"));
        assert_eq!(popup.save_fields(&store), None);
    }

    #[test]
    fn save_fields_rejects_an_empty_name() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = EditPayeePopup::new(&store, woolworths);
        popup.name.clear();
        assert_eq!(popup.save_fields(&store), None);
    }

    #[test]
    fn save_fields_infers_icon_derived_from_whether_the_typed_url_matches_the_website() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let popup = EditPayeePopup::new(&store, woolworths);

        let (.., icon_url, icon_derived, _, _) =
            popup.save_fields(&store).expect("draft should validate");
        assert_eq!(icon_url, Some("woolworths.com.au/favicon.ico".to_string()));
        assert!(icon_derived, "prefilled from an already-derived icon url");
    }

    #[test]
    fn save_fields_reports_not_derived_once_the_icon_url_is_hand_edited() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = EditPayeePopup::new(&store, woolworths);
        popup.icon_url_input = "cdn.example.com/logo.png".to_string();

        let (.., icon_url, icon_derived, _, _) =
            popup.save_fields(&store).expect("draft should validate");
        assert_eq!(icon_url, Some("cdn.example.com/logo.png".to_string()));
        assert!(!icon_derived);
    }

    #[test]
    fn deactivate_fields_forces_active_false_regardless_of_the_checkbox() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let popup = EditPayeePopup::new(&store, woolworths);
        assert!(popup.active);

        let (.., active) = popup
            .deactivate_fields(&store)
            .expect("draft should validate");
        assert!(!active);
    }

    #[test]
    fn shows_the_created_date_and_footer_hints() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let text = render(&EditPayeePopup::new(&store, woolworths), &store);

        assert!(text.contains("created"));
        assert!(text.contains("2024"));
        for key in ["tab", "^s", "^a", "m", "^d", "esc"] {
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
