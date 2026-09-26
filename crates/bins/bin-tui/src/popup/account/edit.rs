//! The "edit account" popup — `e` on an Accounts list row, or `:acct edit <acct>`
//! (`docs/ux/tui/accounts/README.md` "7c — Edit"). Genuinely interactive and genuinely
//! mutates the fixture, the same as `crate::popup::category::edit_popup` — unlike
//! `crate::popup::unit::edit`, still wireframe-only ("no draft state yet"), whose
//! module/enum-variant/`render()` *structure* this still follows per that ticket's own
//! instruction.
//!
//! **Editable: `name`/`type`/`active`, and nothing else** — FR.13 fixes `unit` and
//! `starting_balance` at creation. Both are shown read-only under a "fixed at creation ·
//! FR.13" heading rather than hidden, per the handoff's own "a user who came here to fix an
//! opening balance needs to see it and be told why they can't" — the closing note names the
//! actual remedy (an adjusting transaction), which is why it's accented here, not just
//! informational.
//!
//! **`^d` (delete)** hands straight off to `crate::popup::account::delete` — `Shell` swaps
//! this popup for that one rather than this struct knowing anything about deletion itself.

use lib_core::{AccountType, RowID};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{account::AccountStore, msg, popup::REFERENCE_TERMINAL_WIDTH};

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 88;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;
const LABEL_WIDTH: u16 = "starting bal".len() as u16 + 1;

/// Content rows inside the border: title, rule, the three editable fields, a blank spacer,
/// the "fixed at creation" heading plus its two read-only fields, a blank spacer, the
/// "computed" heading plus its three fields, a blank spacer, the two-line label note, the
/// two-line (accent) remedy note, the footer's rule, then the footer itself.
const CONTENT_ROWS: u16 = 1 + 1 + 3 + 1 + 1 + 2 + 1 + 1 + 3 + 1 + 2 + 2 + 1 + 1;
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

/// Which editable field currently has focus. `unit`/`starting balance`/the computed fields
/// are read-only and never gain focus, per the handoff's own field table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Type,
    Active,
}

impl Field {
    fn next(self) -> Field {
        match self {
            Field::Name => Field::Type,
            Field::Type => Field::Active,
            Field::Active => Field::Name,
        }
    }
}

/// The `:acct edit` popup's own draft state, prefilled from the account being edited.
pub struct EditAccountPopup {
    editing_id: RowID,
    name: String,
    account_type: AccountType,
    active: bool,
    focus: Field,
}

impl EditAccountPopup {
    /// Opens a popup editing `editing_id`, prefilling `name`/`type`/`active` from its current
    /// values. `AccountsView::handle_key` only ever opens this against the current selection,
    /// which always names a real account, so the fallbacks here (`Default`/`true`) never
    /// actually arise — they exist so a dangling id can't panic this.
    pub fn new(store: &dyn AccountStore, editing_id: RowID) -> Self {
        let account = store.find(editing_id);
        Self {
            editing_id,
            name: account.map(|a| a.name.clone()).unwrap_or_default(),
            account_type: account.map(|a| a.account_type.clone()).unwrap_or_default(),
            active: account.map(|a| a.is_active).unwrap_or(true),
            focus: Field::Name,
        }
    }

    pub fn editing_id(&self) -> RowID {
        self.editing_id
    }

    pub fn push_char(&mut self, c: char) {
        match self.focus {
            Field::Name => self.name.push(c),
            // First-letter jump — see `popup::account::new`'s own `push_char` for why `l`
            // never reaches here in practice (Shell routes it to `type_right` instead while
            // this field has focus).
            Field::Type => {
                if let Some(matched) = AccountType::all()
                    .iter()
                    .find(|t| t.as_str().starts_with(c.to_ascii_lowercase()))
                {
                    self.account_type = matched.clone();
                }
            }
            Field::Active if c == ' ' => self.active = !self.active,
            Field::Active => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.focus {
            Field::Name => {
                self.name.pop();
            }
            Field::Type | Field::Active => {}
        }
    }

    pub fn tab(&mut self) {
        self.focus = self.focus.next();
    }

    /// Whether the `type` field currently has focus — see `popup::account::new`'s own
    /// `type_field_focused` for why `Shell` checks this before routing `h`/`l`.
    pub fn type_field_focused(&self) -> bool {
        self.focus == Field::Type
    }

    pub fn type_left(&mut self) {
        if self.focus == Field::Type {
            self.cycle_type(-1);
        }
    }

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

    /// The `(id, name, type, active)` `^s` would save, or `None` while `name` doesn't
    /// validate (empty — the schema has no uniqueness constraint to check beyond that, per
    /// the handoff's own instruction not to add one).
    pub fn save_fields(&self) -> Option<(RowID, String, AccountType, bool)> {
        let name = self.name.trim();
        if name.is_empty() {
            return None;
        }
        Some((
            self.editing_id,
            name.to_string(),
            self.account_type.clone(),
            self.active,
        ))
    }

    /// The `(id, name, type, active)` `^a` would save — the draft as typed, but with `active`
    /// forced `false` regardless of the checkbox's own current value, per the handoff's `^a`
    /// "deactivate" being its own quick action.
    pub fn deactivate_fields(&self) -> Option<(RowID, String, AccountType, bool)> {
        self.save_fields()
            .map(|(id, name, account_type, _)| (id, name, account_type, false))
    }

    /// Renders the floating overlay, centred and fixed-height, within `area`.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, store: &dyn AccountStore) {
        let Some(account) = store.find(self.editing_id) else {
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
                Constraint::Length(1), // type
                Constraint::Length(1), // active
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // "fixed at creation · FR.13" heading
                Constraint::Length(1), // unit
                Constraint::Length(1), // starting bal
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // "computed" heading
                Constraint::Length(1), // balance now
                Constraint::Length(1), // transactions
                Constraint::Length(1), // created
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // label note line 1
                Constraint::Length(1), // label note line 2
                Constraint::Length(1), // remedy note line 1 (accent)
                Constraint::Length(1), // remedy note line 2 (accent)
                Constraint::Length(1), // rule
                Constraint::Length(1), // footer hints
            ])
            .split(inner);

        render_title(frame, rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
        render_text_field(
            frame,
            rows[2],
            &msg::tui_account_edit_field_name(),
            &self.name,
            self.focus == Field::Name,
        );
        render_type_field(
            frame,
            rows[3],
            self.account_type.clone(),
            self.focus == Field::Type,
        );
        render_active_field(frame, rows[4], self.active, self.focus == Field::Active);
        // rows[5] is left blank — breathing space above the read-only section.

        render_section_heading(
            frame,
            rows[6],
            &msg::tui_account_edit_note_unit_fixed(),
            Some("FR.13"),
        );
        render_field(
            frame,
            rows[7],
            &msg::tui_account_edit_field_unit(),
            Line::from(account.unit.code.clone()),
        );
        render_field(
            frame,
            rows[8],
            &msg::tui_account_edit_field_opening_balance(),
            Line::from(crate::format::money(
                &account.starting_balance,
                account.unit.decimal_places,
            )),
        );
        // rows[9] is left blank — breathing space above the computed section.

        render_section_heading(
            frame,
            rows[10],
            &msg::tui_account_edit_heading_computed(),
            None,
        );
        let balance = store.balance(account.id);
        render_field(
            frame,
            rows[11],
            &msg::tui_account_edit_field_balance_now(),
            Line::from(Span::styled(
                format!(
                    "{} · not stored",
                    crate::format::money(&balance, account.unit.decimal_places)
                ),
                dim(),
            )),
        );
        let transactions_text = if account.transaction_count == 0 {
            "none".to_string()
        } else {
            format!("{} · enter ledger", account.transaction_count)
        };
        render_field(
            frame,
            rows[12],
            &msg::tui_account_edit_field_transactions(),
            Line::from(Span::styled(transactions_text, dim())),
        );
        render_field(
            frame,
            rows[13],
            &msg::tui_account_edit_field_created(),
            Line::from(Span::styled(
                format!(
                    "{} · upd {}",
                    format_date_full_year(account.created_on),
                    format_date_short_year(account.updated_on)
                ),
                dim(),
            )),
        );
        // rows[14] is left blank — breathing space above the closing note.

        frame.render_widget(
            Paragraph::new("name and type are labels — nothing derives"),
            rows[15],
        );
        frame.render_widget(
            Paragraph::new("from them, so renaming is always safe."),
            rows[16],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(
                "correct a wrong opening balance with an",
                Style::default().fg(ACCENT),
            )),
            rows[17],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(
                "adjusting transaction, not by rewriting it",
                Style::default().fg(ACCENT),
            )),
            rows[18],
        );

        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[19]);
        render_footer_hints(frame, rows[20]);
    }
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

fn render_title(frame: &mut Frame<'_>, area: Rect) {
    let tag = ":acct edit";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(msg::tui_account_edit_title()), columns[0]);
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

/// A bare section heading (`fixed at creation`/`computed`), with an optional right-aligned
/// tag (`FR.13`) — distinct from [`render_field`]: this isn't a label/value row, it introduces
/// the rows beneath it.
fn render_section_heading(frame: &mut Frame<'_>, area: Rect, label: &str, tag: Option<&str>) {
    let Some(tag) = tag else {
        frame.render_widget(Paragraph::new(Span::styled(label, dim())), area);
        return;
    };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled(label, dim())), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim())).alignment(Alignment::Right),
        columns[1],
    );
}

fn render_text_field(frame: &mut Frame<'_>, area: Rect, label: &str, value: &str, focused: bool) {
    let mut spans = vec![Span::raw(value.to_string())];
    if focused {
        spans.push(Span::styled("\u{258c}", Style::default().fg(ACCENT)));
    }
    render_field(frame, area, label, Line::from(spans));
}

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

    render_field(
        frame,
        area,
        &msg::tui_account_edit_field_type(),
        Line::from(spans),
    );
}

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
        &msg::tui_account_edit_field_active(),
        Line::from(vec![
            Span::styled(glyph, glyph_style),
            Span::raw(" "),
            Span::styled("· clear to deactivate", dim()),
        ]),
    );
}

fn render_footer_hints(frame: &mut Frame<'_>, area: Rect) {
    let hints = [
        ("tab", msg::tui_account_edit_help_tab()),
        ("^s", msg::tui_account_edit_help_save()),
        ("^a", msg::tui_account_edit_help_deactivate()),
        ("^d", msg::tui_account_edit_help_delete()),
        ("esc", msg::tui_account_edit_help_cancel()),
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

fn format_date_full_year(date: chrono::NaiveDate) -> String {
    crate::format::date(date)
}

fn format_date_short_year(date: chrono::NaiveDate) -> String {
    crate::format::date(date)
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::account::AccountFixture;

    fn find_id(store: &AccountFixture, name: &str) -> RowID {
        store
            .accounts()
            .iter()
            .find(|account| account.name == name)
            .unwrap_or_else(|| panic!("fixture should seed an account named {name}"))
            .id
    }

    fn render(popup: &EditAccountPopup, store: &dyn AccountStore) -> String {
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
    fn new_prefills_every_field_from_the_current_account() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let popup = EditAccountPopup::new(&store, everyday);

        assert_eq!(popup.name, "Everyday Spending");
        assert_eq!(popup.account_type, AccountType::Bank);
        assert!(popup.active);
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn renders_without_panicking() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        render(&EditAccountPopup::new(&store, everyday), &store);
    }

    #[test]
    fn tab_cycles_focus_through_name_type_active_and_wraps() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let mut popup = EditAccountPopup::new(&store, everyday);
        assert_eq!(popup.focus, Field::Name);

        popup.tab();
        assert_eq!(popup.focus, Field::Type);
        popup.tab();
        assert_eq!(popup.focus, Field::Active);
        popup.tab();
        assert_eq!(popup.focus, Field::Name);
    }

    #[test]
    fn typing_appends_to_the_name_field() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let mut popup = EditAccountPopup::new(&store, everyday);
        popup.push_char('!');
        assert_eq!(popup.name, "Everyday Spending!");
    }

    #[test]
    fn h_and_l_step_the_type_pick_only_while_it_has_focus() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let mut popup = EditAccountPopup::new(&store, everyday);
        assert_eq!(popup.account_type, AccountType::Bank);

        popup.focus = Field::Name;
        popup.type_right();
        assert_eq!(popup.account_type, AccountType::Bank, "no focus, no effect");

        popup.focus = Field::Type;
        popup.type_right();
        assert_eq!(popup.account_type, AccountType::CreditCard);
        popup.type_left();
        assert_eq!(popup.account_type, AccountType::Bank);
    }

    #[test]
    fn space_toggles_active_only_when_it_has_focus() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let mut popup = EditAccountPopup::new(&store, everyday);
        assert!(popup.active);

        popup.focus = Field::Name;
        popup.push_char(' ');
        assert_eq!(popup.name, "Everyday Spending ");
        assert!(popup.active);

        popup.focus = Field::Active;
        popup.push_char(' ');
        assert!(!popup.active);
    }

    #[test]
    fn save_fields_rejects_an_empty_name() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let mut popup = EditAccountPopup::new(&store, everyday);
        popup.name.clear();

        assert_eq!(popup.save_fields(), None);
    }

    #[test]
    fn save_fields_returns_the_trimmed_name_type_and_active() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let mut popup = EditAccountPopup::new(&store, everyday);
        popup.name = "  Renamed  ".to_string();
        popup.account_type = AccountType::Cash;

        assert_eq!(
            popup.save_fields(),
            Some((everyday, "Renamed".to_string(), AccountType::Cash, true))
        );
    }

    #[test]
    fn deactivate_fields_forces_active_false_regardless_of_the_checkbox() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let popup = EditAccountPopup::new(&store, everyday);
        assert!(popup.active, "starts active per the fixture");

        let (_, _, _, active) = popup.deactivate_fields().expect("should validate");
        assert!(!active);
    }

    #[test]
    fn shows_unit_and_starting_balance_read_only_under_the_fr13_heading() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let text = render(&EditAccountPopup::new(&store, everyday), &store);

        assert!(text.contains("fixed at creation"));
        assert!(text.contains("FR.13"));
        assert!(text.contains("AUD"));
        assert!(text.contains("1,000.00"));
    }

    #[test]
    fn shows_the_computed_section_with_real_balance_and_transaction_count() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let text = render(&EditAccountPopup::new(&store, everyday), &store);

        assert!(text.contains("computed"));
        assert!(text.contains("4,210.65 · not stored"));
        assert!(text.contains("1284 · enter ledger"));
        assert!(text.contains("oct 14, 2024"));
    }

    #[test]
    fn shows_none_for_an_account_with_no_transactions() {
        let store = AccountFixture::new();
        let wallet = find_id(&store, "Wallet");
        let text = render(&EditAccountPopup::new(&store, wallet), &store);
        assert!(text.contains("none"));
    }

    #[test]
    fn shows_the_closing_note_and_footer_hints_including_ctrl_d() {
        let store = AccountFixture::new();
        let everyday = find_id(&store, "Everyday Spending");
        let text = render(&EditAccountPopup::new(&store, everyday), &store);

        assert!(text.contains("nothing derives"));
        assert!(text.contains("adjusting transaction"));
        for key in ["tab", "^s", "^a", "^d", "esc"] {
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
