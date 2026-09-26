//! The "new account" popup — `n` on the Accounts list, or `:acct new <name> <type> <unit>`
//! (`docs/ux/tui/accounts/README.md` "7b — New"). Genuinely interactive and genuinely creates
//! an account in the fixture, the same as `crate::popup::category`'s own new/edit/move popups
//! — unlike `crate::popup::unit::new`, still wireframe-only ("no draft state yet"), whose
//! module/enum-variant/`render()` *structure* this still follows per that ticket's own
//! instruction.
//!
//! **`unit` only ever completes against a Unit a real account already uses**
//! (`crate::account::known_units`) — `view::units` has no store of its own yet, so there is no
//! live Unit registry to complete against beyond "whatever's already in use". A brand-new Unit
//! can't be created through this popup at this map's fidelity.
//!
//! **`type` is focused by default**, not `name` — the handoff's own field table says so
//! explicitly ("five-way inline pick, focused by default"), unlike every other form in this
//! codebase, which defaults to its first text field.

use lib_core::{AccountType, Money};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::account::{AccountStore, AccountUnit, known_units};
use crate::{msg, popup::REFERENCE_TERMINAL_WIDTH};

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 88;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;
const LABEL_WIDTH: u16 = "starting bal".len() as u16 + 1;

/// Content rows inside the border: title, its rule, the five fields (`type`'s row plus a
/// completion row under `unit`), a blank spacer, the two-line note, a blank spacer, the
/// footer's rule, then the footer itself.
const CONTENT_ROWS: u16 = 1 + 1 + 6 + 1 + 2 + 1 + 1 + 1;
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

/// Which editable field currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Type,
    Unit,
    StartingBalance,
    Active,
}

impl Field {
    fn next(self) -> Field {
        match self {
            Field::Name => Field::Type,
            Field::Type => Field::Unit,
            Field::Unit => Field::StartingBalance,
            Field::StartingBalance => Field::Active,
            Field::Active => Field::Name,
        }
    }
}

/// The `:acct new` popup's own draft state.
pub struct NewAccountPopup {
    name: String,
    account_type: AccountType,
    unit_input: String,
    starting_balance_input: String,
    active: bool,
    focus: Field,
}

impl NewAccountPopup {
    /// Opens a fresh, blank popup — `type` defaults to `AccountType::default()` (`Cash`),
    /// `starting balance` to `"0.00"`, `active` to `[×]`, all per the handoff's field table,
    /// and focus starts on `type` (see this module's own doc on why that's not `name`).
    pub fn new() -> Self {
        Self {
            name: String::new(),
            account_type: AccountType::default(),
            unit_input: String::new(),
            starting_balance_input: "0.00".to_string(),
            active: true,
            focus: Field::Type,
        }
    }

    pub fn push_char(&mut self, c: char) {
        match self.focus {
            Field::Name => self.name.push(c),
            Field::Unit => self.unit_input.push(c),
            Field::StartingBalance if c.is_ascii_digit() || c == '.' || c == '-' => {
                self.starting_balance_input.push(c)
            }
            Field::StartingBalance => {}
            // First-letter jump: lands on the first variant (in the enum's own order) whose
            // name starts with this letter — `cash`/`credit_card` share a `c`, so typing `c`
            // always lands on `cash`, the earlier one, rather than cycling between them. In
            // practice `Shell` (`map_account_popup_key`) never forwards a bare `h`/`l` into
            // `push_char` while this field has focus — those always mean "step the pick" (see
            // `type_left`/`type_right`), which shadows `l` ever reaching this branch to jump
            // to `loan` by its first letter; this method's own contract still supports it for
            // any letter, `l` included, for whatever calls it directly.
            Field::Type => {
                if let Some(matched) = AccountType::all()
                    .iter()
                    .find(|t| t.as_str().starts_with(c.to_ascii_lowercase()))
                {
                    self.account_type = matched.clone();
                }
            }
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
            Field::Unit => {
                self.unit_input.pop();
            }
            Field::StartingBalance => {
                self.starting_balance_input.pop();
            }
            Field::Type | Field::Active => {}
        }
    }

    /// Whether the `type` field currently has focus — `Shell` checks this before routing a
    /// bare `h`/`l` to [`type_left`](Self::type_left)/[`type_right`](Self::type_right) rather
    /// than as ordinary text (a Unit code like `CHF` contains `h`; `h`/`l` must only mean
    /// "step the type pick" while that field is actually focused).
    pub fn type_field_focused(&self) -> bool {
        self.focus == Field::Type
    }

    /// `h`: steps the `type` pick back one when it has focus; a no-op on any other field.
    pub fn type_left(&mut self) {
        if self.focus == Field::Type {
            self.cycle_type(-1);
        }
    }

    /// `l`: steps the `type` pick forward one when it has focus; a no-op on any other field.
    pub fn type_right(&mut self) {
        if self.focus == Field::Type {
            self.cycle_type(1);
        }
    }

    fn cycle_type(&mut self, delta: isize) {
        let all = AccountType::all();
        let current = all
            .iter()
            .position(|t| *t == self.account_type)
            .unwrap_or(0) as isize;
        let len = all.len() as isize;
        let next = ((current + delta) % len + len) % len;
        self.account_type = all[next as usize].clone();
    }

    /// `Tab`: completes `unit` against a known Unit code (case-insensitive prefix match) when
    /// it has focus and a candidate exists; otherwise advances focus to the next field.
    pub fn tab(&mut self, store: &dyn AccountStore) {
        if self.focus == Field::Unit {
            let needle = self.unit_input.to_lowercase();
            if let Some(candidate) = known_units(store.accounts())
                .into_iter()
                .find(|unit| unit.code.to_lowercase().starts_with(&needle))
                && candidate.code != self.unit_input
            {
                self.unit_input = candidate.code;
                return;
            }
        }
        self.focus = self.focus.next();
    }

    /// The Unit `unit_input` currently resolves to (a case-insensitive exact match against a
    /// known Unit's code), if any.
    fn resolved_unit(&self, store: &dyn AccountStore) -> Option<AccountUnit> {
        let needle = self.unit_input.to_lowercase();
        known_units(store.accounts())
            .into_iter()
            .find(|unit| unit.code.to_lowercase() == needle)
    }

    /// The `(name, account_type, unit, starting_balance, active)` `^s`/`^a` would create, or
    /// `None` while the draft doesn't validate — an unresolved `unit`, an empty `name`, or an
    /// unparseable `starting balance`. No uniqueness check on `name`: the schema has none
    /// (ids are `RowID`/UUIDv7), per the handoff's own instruction not to add one.
    pub fn create_fields(
        &self,
        store: &dyn AccountStore,
    ) -> Option<(String, AccountType, AccountUnit, Money, bool)> {
        let name = self.name.trim();
        if name.is_empty() {
            return None;
        }
        let unit = self.resolved_unit(store)?;
        let starting_balance: Money = self.starting_balance_input.trim().parse().ok()?;
        Some((
            name.to_string(),
            self.account_type.clone(),
            unit,
            starting_balance,
            self.active,
        ))
    }

    /// `^a`: after a successful create, clears `name` and resets `type`/`starting balance`/
    /// `active` to their own defaults, but keeps `unit_input` as-is — bulk-adding more
    /// accounts in the same Unit is the point, per the handoff's own "create and start
    /// another".
    pub fn reset_for_next_account(&mut self) {
        self.name.clear();
        self.account_type = AccountType::default();
        self.starting_balance_input = "0.00".to_string();
        self.active = true;
        self.focus = Field::Type;
    }

    /// Renders the floating overlay, centred within `area`.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, store: &dyn AccountStore) {
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
                Constraint::Length(1), // type
                Constraint::Length(1), // unit
                Constraint::Length(1), // completion
                Constraint::Length(1), // starting bal
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
            &msg::tui_account_new_field_name(),
            &self.name,
            self.focus == Field::Name,
        );
        render_type_field(
            frame,
            rows[3],
            self.account_type.clone(),
            self.focus == Field::Type,
        );
        render_text_field(
            frame,
            rows[4],
            &msg::tui_account_new_field_unit(),
            &self.unit_input,
            self.focus == Field::Unit,
        );
        render_completion_row(frame, rows[5], store, &self.unit_input);
        render_text_field(
            frame,
            rows[6],
            &msg::tui_account_new_field_starting_balance(),
            &self.starting_balance_input,
            self.focus == Field::StartingBalance,
        );
        render_active_field(frame, rows[7], self.active, self.focus == Field::Active);
        // rows[8] is left blank — breathing space above the note.
        render_note(frame, rows[9], msg::tui_account_new_note_unit_1());
        render_note(frame, rows[10], msg::tui_account_new_note_unit_2());
        // rows[11] is left blank — breathing space above the footer rule.
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[12]);
        render_footer_hints(frame, rows[13]);
    }
}

impl Default for NewAccountPopup {
    fn default() -> Self {
        Self::new()
    }
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// The title row: "new account" flush left, the `:acct new` command dim and right-aligned.
fn render_title(frame: &mut Frame<'_>, area: Rect) {
    let tag = ":acct new";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(msg::tui_account_new_title()), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim())).alignment(Alignment::Right),
        columns[1],
    );
}

/// One `label   value` row.
fn render_field(frame: &mut Frame<'_>, area: Rect, label: &str, value: Line<'static>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled(label, dim())), columns[0]);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

/// One editable text field: the typed value, with a trailing accent cursor only when it has
/// focus.
fn render_text_field(frame: &mut Frame<'_>, area: Rect, label: &str, value: &str, focused: bool) {
    let mut spans = vec![Span::raw(value.to_string())];
    if focused {
        spans.push(Span::styled("\u{258c}", Style::default().fg(ACCENT)));
    }
    render_field(frame, area, label, Line::from(spans));
}

/// The `type` field's value: a segmented five-way control with the selected option rendered
/// as a reversed pill (accented when focused, so `h`/`l` has a visible target), then the dim
/// `· h/l` hint.
fn render_type_field(frame: &mut Frame<'_>, area: Rect, selected: AccountType, focused: bool) {
    let selected_style = if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::REVERSED)
    } else {
        Style::default().add_modifier(Modifier::REVERSED)
    };

    let mut spans = Vec::new();
    for (index, account_type) in AccountType::all().iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        let label = account_type.as_str().replace('_', " ");
        if *account_type == selected {
            spans.push(Span::styled(format!(" {label} "), selected_style));
        } else {
            spans.push(Span::raw(label));
        }
    }
    spans.push(Span::raw(" "));
    spans.push(Span::styled("· h/l", dim()));

    render_field(
        frame,
        area,
        &msg::tui_account_new_field_type(),
        Line::from(spans),
    );
}

/// The `active` checkbox row: the glyph in the accent when focused, the "offered when
/// posting" consequence stated alongside it either way.
fn render_active_field(frame: &mut Frame<'_>, area: Rect, active: bool, focused: bool) {
    let glyph_style = if focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };
    let glyph = if active { "[\u{d7}]" } else { "[ ]" };
    render_field(
        frame,
        area,
        &msg::tui_account_new_field_active(),
        Line::from(vec![
            Span::styled(glyph, glyph_style),
            Span::raw(" "),
            Span::styled("· offered when posting", dim()),
        ]),
    );
}

/// The `completion` row beneath `unit` — mirrors `popup::category::new_popup`'s own row.
fn render_completion_row(frame: &mut Frame<'_>, area: Rect, store: &dyn AccountStore, input: &str) {
    let needle = input.to_lowercase();
    let candidates: Vec<String> = known_units(store.accounts())
        .into_iter()
        .filter(|unit| needle.is_empty() || unit.code.to_lowercase().starts_with(&needle))
        .map(|unit| unit.code)
        .collect();
    let text = if candidates.is_empty() {
        msg::tui_account_new_error_no_matches()
    } else {
        format!("{}  · tab", candidates.join(" · "))
    };
    render_field(
        frame,
        area,
        &msg::tui_account_new_placeholder_completion(),
        Line::from(Span::styled(text, dim())),
    );
}

fn render_note(frame: &mut Frame<'_>, area: Rect, text: String) {
    frame.render_widget(Paragraph::new(Span::styled(text, dim())), area);
}

fn render_footer_hints(frame: &mut Frame<'_>, area: Rect) {
    let hints = [
        ("tab", msg::tui_account_new_help_tab()),
        ("^s", msg::tui_account_new_help_create()),
        ("^a", msg::tui_account_new_help_create_and_add()),
        ("esc", msg::tui_account_new_help_cancel()),
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
        spans.push(Span::styled(label.as_str(), label_style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Computes a centred popup `Rect` sized to [`POPUP_HEIGHT`]'s fixed field list — mirrors
/// `popup::unit::new`'s own `popup_rect` (a fixed field list, no dynamic preview to size
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
    use crate::account::AccountFixture;

    fn render(popup: &NewAccountPopup, store: &dyn AccountStore) -> String {
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
    fn new_defaults_to_cash_zero_balance_active_and_type_focused() {
        let popup = NewAccountPopup::new();
        assert_eq!(popup.account_type, AccountType::Cash);
        assert_eq!(popup.starting_balance_input, "0.00");
        assert!(popup.active);
        assert_eq!(popup.focus, Field::Type);
    }

    #[test]
    fn renders_without_panicking() {
        let store = AccountFixture::new();
        render(&NewAccountPopup::new(), &store);
    }

    #[test]
    fn tab_cycles_focus_through_every_field_and_wraps() {
        let store = AccountFixture::new();
        let mut popup = NewAccountPopup::new();
        assert_eq!(popup.focus, Field::Type);

        popup.tab(&store); // type has no unit completion to apply, so this always advances
        assert_eq!(popup.focus, Field::Unit);
        popup.focus = Field::Active;
        popup.tab(&store);
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn tab_on_the_unit_field_completes_when_a_candidate_exists() {
        let store = AccountFixture::new();
        let mut popup = NewAccountPopup::new();
        popup.focus = Field::Unit;
        popup.unit_input = "au".to_string();

        popup.tab(&store);
        assert_eq!(popup.unit_input, "AUD");
        assert_eq!(popup.focus, Field::Unit, "completing shouldn't move focus");
    }

    #[test]
    fn typing_appends_to_whichever_field_has_focus() {
        let store = AccountFixture::new();
        let mut popup = NewAccountPopup::new();
        popup.focus = Field::Name;
        popup.push_char('H');
        popup.push_char('i');
        assert_eq!(popup.name, "Hi");
        let _ = store; // unused in this test beyond construction
    }

    #[test]
    fn first_letter_on_the_type_field_jumps_to_the_first_matching_variant() {
        let mut popup = NewAccountPopup::new();
        popup.push_char('l');
        assert_eq!(popup.account_type, AccountType::Loan);

        // "c" matches both Cash and CreditCard; the earlier one in enum order wins.
        popup.push_char('c');
        assert_eq!(popup.account_type, AccountType::Cash);
    }

    #[test]
    fn h_and_l_step_the_type_pick_only_while_it_has_focus() {
        let mut popup = NewAccountPopup::new();
        popup.focus = Field::Type;
        popup.type_right();
        assert_eq!(popup.account_type, AccountType::Bank);
        popup.type_left();
        assert_eq!(popup.account_type, AccountType::Cash);

        popup.focus = Field::Name;
        popup.type_right();
        assert_eq!(popup.account_type, AccountType::Cash, "no focus, no effect");
    }

    #[test]
    fn space_toggles_active_only_when_it_has_focus() {
        let mut popup = NewAccountPopup::new();
        assert!(popup.active);

        popup.focus = Field::Name;
        popup.push_char(' '); // just a literal space in the name field
        assert_eq!(popup.name, " ");
        assert!(popup.active);

        popup.focus = Field::Active;
        popup.push_char(' ');
        assert!(!popup.active);
    }

    #[test]
    fn starting_balance_field_rejects_non_numeric_characters() {
        let mut popup = NewAccountPopup::new();
        popup.focus = Field::StartingBalance;
        popup.starting_balance_input.clear();
        popup.push_char('1');
        popup.push_char('x');
        popup.push_char('.');
        popup.push_char('5');
        assert_eq!(popup.starting_balance_input, "1.5");
    }

    #[test]
    fn create_fields_requires_a_non_empty_name_and_a_resolved_unit() {
        let store = AccountFixture::new();
        let popup = NewAccountPopup::new();
        assert_eq!(
            popup.create_fields(&store),
            None,
            "empty name and unresolved unit should not validate"
        );
    }

    #[test]
    fn create_fields_returns_the_trimmed_name_type_unit_balance_and_active() {
        let store = AccountFixture::new();
        let mut popup = NewAccountPopup::new();
        popup.name = "  Emergency Fund  ".to_string();
        popup.unit_input = "AUD".to_string();
        popup.starting_balance_input = "500.00".to_string();

        let (name, account_type, unit, starting_balance, active) =
            popup.create_fields(&store).expect("draft should validate");
        assert_eq!(name, "Emergency Fund");
        assert_eq!(account_type, AccountType::Cash);
        assert_eq!(unit.code, "AUD");
        assert_eq!(starting_balance, "500.00".parse().unwrap());
        assert!(active);
    }

    #[test]
    fn create_fields_rejects_an_unresolved_unit() {
        let store = AccountFixture::new();
        let mut popup = NewAccountPopup::new();
        popup.name = "Something".to_string();
        popup.unit_input = "ZZZ".to_string();
        assert_eq!(popup.create_fields(&store), None);
    }

    #[test]
    fn reset_for_next_account_clears_name_but_keeps_unit() {
        let mut popup = NewAccountPopup::new();
        popup.name = "First".to_string();
        popup.unit_input = "AUD".to_string();
        popup.account_type = AccountType::Loan;
        popup.active = false;

        popup.reset_for_next_account();

        assert_eq!(popup.name, "");
        assert_eq!(popup.unit_input, "AUD");
        assert_eq!(popup.account_type, AccountType::Cash);
        assert_eq!(popup.starting_balance_input, "0.00");
        assert!(popup.active);
        assert_eq!(popup.focus, Field::Type);
    }

    #[test]
    fn shows_the_note_and_footer_hints() {
        let store = AccountFixture::new();
        let text = render(&NewAccountPopup::new(), &store);
        assert!(text.contains("unit cannot change afterwards"));
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
