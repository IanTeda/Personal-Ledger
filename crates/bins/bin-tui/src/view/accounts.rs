//! The Accounts `View`, hosted by `Shell` (ADR-0013). Per
//! `docs/ux/tui/accounts/README.md` "7a — Accounts screen": the left pane is the
//! `AccountType`-grouped list and the summary box for whichever account is selected (built
//! here, "Accounts: 7a screen — list pane and summary box (left pane)"); the right pane (the
//! balance-line chart and the inline ledger list, "Accounts: 7a screen — balance line and
//! ledger list (right pane)") is a later ticket's own concern and stays a bordered
//! placeholder for now.
//!
//! Real, interactive state — not a wireframe: `store` ([`AccountFixture`], from "Accounts:
//! fixture data seam and mutable View state pattern") genuinely holds the account list, and
//! `selected`/`show_inactive`/`filter` are mutated directly in
//! [`AccountsView::handle_key`], mirroring `crate::category`'s/`view::categories`'s own
//! state-ownership decision. Every key handled this way returns [`Action::NoOp`] rather than
//! `None`, so `Shell`'s event loop still redraws immediately instead of waiting for the next
//! `Tick`.
//!
//! **Keys not wired here**: `n`/`e`/`d`/`a`/`b` all reach a popup or the command grammar —
//! each a later ticket's own concern (see the "Accounts screen, views and popup" map, issue
//! #115). Also not wired: `tab` (focus the right pane's ledger list, once it exists) and
//! `enter` (move focus into it too, per the handoff's "no separate ledger screen" decision).

use crossterm::event::{KeyCode, KeyEvent};
use lib_core::{AccountType, Money, RowID};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crate::account::{Account, AccountFixture, AccountStore, shared_unit};
use crate::view::{Action, View};

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used here for
/// negative balances.
const ACCENT: Color = Color::Red;

/// Width of the left pane (list + summary), per the handoff's own `Layout::horizontal([
/// Constraint::Length(41), Constraint::Min(0)])`.
const LEFT_PANE_WIDTH: u16 = 41;

/// Width of the list's `UNIT` column.
const UNIT_COL_WIDTH: u16 = 4;

/// Width of the list's right-aligned, tabular `BALANCE` column. The handoff's own spec
/// names `10`, but a right-aligned `Paragraph` wider than its area clips on the right rather
/// than the left — silently truncating a real balance's trailing digit — and `-612 400.00`
/// (a perfectly ordinary Loan balance) is already 11 characters; `12` leaves enough headroom
/// for that without clipping.
const BALANCE_COL_WIDTH: u16 = 12;

/// Width of the summary box's label column, e.g. `"balance now · computed   "`.
const SUMMARY_LABEL_WIDTH: usize = 24;

/// Number of content rows the summary box ever shows — the fact line plus the four computed
/// rows below it, per the handoff's "capped at five lines".
const SUMMARY_CONTENT_ROWS: u16 = 5;

/// One visible line in the rendered list: a type header (with its group's count and
/// subtotal-or-mixed-units), or one account row.
enum ListLine<'a> {
    Header {
        account_type: AccountType,
        count: usize,
        right_text: String,
    },
    Account {
        account: &'a Account,
        is_last_in_group: bool,
    },
}

/// The Accounts `View`. Owns the fixture account list directly — no navigation-stack
/// `Action` carries it, per `crate::account`'s state-ownership decision — so `handle_key`
/// mutates `store`/`selected`/`show_inactive`/`filter` in place.
pub struct AccountsView {
    store: AccountFixture,
    selected: RowID,
    /// `za` toggles this — inactive accounts are hidden unless it's `true`.
    show_inactive: bool,
    /// `true` right after a lone `z`, awaiting the `a` that completes the `za` chord.
    pending_z: bool,
    /// `/`'s current query — a case-insensitive substring match against an account's name,
    /// applied across every `AccountType` group. Empty means "no filter".
    filter: String,
    /// `true` while `/`'s input buffer has focus — every printable key appends to `filter`
    /// instead of being read as a command.
    filtering: bool,
}

impl Default for AccountsView {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountsView {
    pub fn new() -> Self {
        let store = AccountFixture::new();
        let selected = Self::visible_accounts_of(&store, false, "")
            .first()
            .map(|account| account.id)
            .unwrap_or_default();

        Self {
            store,
            selected,
            show_inactive: false,
            pending_z: false,
            filter: String::new(),
            filtering: false,
        }
    }

    /// Every account currently visible, in the same grouped/sorted order the list renders —
    /// the single source of truth both rendering and selection movement walk. A free function
    /// over an explicit `store`/`show_inactive`/`filter` triple (rather than a method) so
    /// `new()` can call it before `Self` exists.
    fn visible_accounts_of<'a>(
        store: &'a AccountFixture,
        show_inactive: bool,
        filter: &str,
    ) -> Vec<&'a Account> {
        let needle = filter.to_lowercase();
        store
            .grouped(show_inactive)
            .into_iter()
            .flat_map(|(_, accounts)| accounts)
            .filter(|account| needle.is_empty() || account.name.to_lowercase().contains(&needle))
            .collect()
    }

    fn visible_accounts(&self) -> Vec<&Account> {
        Self::visible_accounts_of(&self.store, self.show_inactive, &self.filter)
    }

    /// Every visible group (type, its visible accounts), empty types — or types with no
    /// visible accounts after filtering — omitted entirely, per the handoff's "empty types
    /// are omitted".
    fn visible_groups(&self) -> Vec<(AccountType, Vec<&Account>)> {
        let needle = self.filter.to_lowercase();
        self.store
            .grouped(self.show_inactive)
            .into_iter()
            .filter_map(|(account_type, accounts)| {
                let matching: Vec<&Account> = accounts
                    .into_iter()
                    .filter(|account| {
                        needle.is_empty() || account.name.to_lowercase().contains(&needle)
                    })
                    .collect();
                if matching.is_empty() {
                    None
                } else {
                    Some((account_type, matching))
                }
            })
            .collect()
    }

    fn selected_account(&self) -> Option<&Account> {
        self.store.find(self.selected)
    }

    fn move_selection(&mut self, delta: isize) {
        let visible = self.visible_accounts();
        let Some(current_index) = visible
            .iter()
            .position(|account| account.id == self.selected)
        else {
            self.recover_selection();
            return;
        };
        let next_index = (current_index as isize + delta).clamp(0, visible.len() as isize - 1);
        self.selected = visible[next_index as usize].id;
    }

    fn select_first(&mut self) {
        if let Some(first) = self.visible_accounts().first() {
            self.selected = first.id;
        }
    }

    fn select_last(&mut self) {
        if let Some(last) = self.visible_accounts().last() {
            self.selected = last.id;
        }
    }

    /// Called after `za`/`/` could have hidden the selected account — points `selected` at
    /// the first still-visible account instead of leaving it dangling. Leaves `selected`
    /// untouched when nothing is visible at all (an all-matching-nothing filter); rendering
    /// handles that case on its own rather than panicking.
    fn recover_selection(&mut self) {
        let visible = self.visible_accounts();
        if visible.iter().any(|account| account.id == self.selected) {
            return;
        }
        if let Some(first) = visible.first() {
            self.selected = first.id;
        }
    }

    fn list_lines(&self) -> Vec<ListLine<'_>> {
        let mut lines = Vec::new();
        for (account_type, accounts) in self.visible_groups() {
            let count = accounts.len();
            let right_text = match shared_unit(&accounts) {
                Some(unit) => {
                    let subtotal: bigdecimal::BigDecimal = accounts
                        .iter()
                        .map(|account| self.store.balance(account.id).0)
                        .sum();
                    format_money_at(&Money(subtotal), unit.decimal_places)
                }
                None => "mixed units".to_string(),
            };
            lines.push(ListLine::Header {
                account_type,
                count,
                right_text,
            });
            let last_index = accounts.len().saturating_sub(1);
            for (index, account) in accounts.into_iter().enumerate() {
                lines.push(ListLine::Account {
                    account,
                    is_last_in_group: index == last_index,
                });
            }
        }
        lines
    }
}

impl View for AccountsView {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
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
                _ => return Some(Action::NoOp), // aborted chord, nothing changed
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
            // Bare `g` is Shell's own view-jump leader (see `view::categories`'s own module
            // doc for the full rationale), so `Home` stands in for the handoff's lowercase
            // `g` ("top").
            KeyCode::Home => {
                self.select_first();
                Some(Action::NoOp)
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.select_last();
                Some(Action::NoOp)
            }
            _ => None,
        }
    }

    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);
        // rows[0] is left blank — breathing space between the shell's title bar and the list/
        // summary and ledger boxes, matching `view::units`/`view::categories`'s own leading
        // spacer row.

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(LEFT_PANE_WIDTH), Constraint::Min(0)])
            .spacing(2)
            .split(rows[1]);

        self.render_left_pane(frame, columns[0]);
        frame.render_widget(Block::bordered().title(" Ledger "), columns[1]);
    }

    fn title(&self) -> &'static str {
        "Accounts"
    }
}

impl AccountsView {
    fn render_left_pane(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(SUMMARY_CONTENT_ROWS + 3), // content + 1 rule + 2 border
            ])
            .split(area);

        self.render_list(frame, rows[0]);
        self.render_summary(frame, rows[1]);
    }

    fn render_list(&self, frame: &mut Frame, area: Rect) {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .spacing(1)
            .split(area);
        let content_area = split[0];
        let scrollbar_column = split[1];

        let lines = self.list_lines();
        let visible = lines.len().min(content_area.height as usize);
        let row_constraints: Vec<Constraint> =
            std::iter::repeat_n(Constraint::Length(1), visible).collect();
        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(content_area);

        for (line, row_area) in lines.iter().zip(row_areas.iter()) {
            render_list_line(frame, *row_area, line, self.selected);
        }

        let total = lines.len();
        let mut scrollbar_state = ScrollbarState::new(total)
            .viewport_content_length(visible)
            .position(0);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, scrollbar_column, &mut scrollbar_state);
    }

    /// The summary box beneath the list, for whichever account is selected — see the
    /// handoff's own worked example (`docs/ux/tui/accounts/README.md` "Summary box").
    fn render_summary(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().padding(Padding::horizontal(1));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(account) = self.selected_account() else {
            frame.render_widget(
                Paragraph::new("no accounts match the current filter"),
                inner,
            );
            return;
        };

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // fact line
                Constraint::Length(1), // rule
                Constraint::Length(1), // balance now
                Constraint::Length(1), // transactions
                Constraint::Length(1), // last check
                Constraint::Length(1), // active
            ])
            .split(inner);

        let fact_line = format!(
            "{} · {} · start {}",
            account.account_type.as_str().replace('_', " "),
            account.unit.code,
            format_money_at(&account.starting_balance, account.unit.decimal_places)
        );
        frame.render_widget(Paragraph::new(fact_line), rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

        let balance = self.store.balance(account.id);
        frame.render_widget(
            summary_field_line(
                "balance now · computed",
                &format_money_at(&balance, account.unit.decimal_places),
            ),
            rows[2],
        );

        let transactions_text = if account.transaction_count == 0 {
            "none".to_string()
        } else {
            format!(
                "{} · {} open",
                account.transaction_count, account.open_count
            )
        };
        frame.render_widget(
            summary_field_line("transactions", &transactions_text),
            rows[3],
        );

        let last_check_text = match account.balance_checks.last() {
            Some(check) => {
                let computed = self.store.balance_as_of(account.id, check.date);
                let variance = Money(check.asserted.0.clone() - computed.0);
                format!(
                    "{} · var {}",
                    format_date(check.date),
                    format_money_at(&variance, account.unit.decimal_places)
                )
            }
            None => "none".to_string(),
        };
        frame.render_widget(summary_field_line("last check", &last_check_text), rows[4]);

        let active_text = if account.is_active {
            "[×] · offered when posting"
        } else {
            "[ ] · not offered"
        };
        frame.render_widget(summary_field_line("active", active_text), rows[5]);
    }
}

/// Splits a list row (or its column header) into name (`Min(0)`) / `UNIT` / `BALANCE`
/// columns.
fn list_row_columns(area: Rect) -> (Rect, Rect, Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(UNIT_COL_WIDTH),
            Constraint::Length(BALANCE_COL_WIDTH),
        ])
        .spacing(1)
        .split(area);
    (columns[0], columns[1], columns[2])
}

fn render_list_line(frame: &mut Frame, area: Rect, line: &ListLine<'_>, selected: RowID) {
    match line {
        ListLine::Header {
            account_type,
            count,
            right_text,
        } => {
            let (name_area, unit_area, balance_area) = list_row_columns(area);
            let bold = Style::default().add_modifier(Modifier::BOLD);
            let label = account_type.as_str().replace('_', " ").to_uppercase();
            frame.render_widget(Paragraph::new(Span::styled(label, bold)), name_area);

            // The count-and-subtotal text rides across the UNIT+BALANCE columns combined
            // (not just the 10-col BALANCE column alone) — "mixed units" doesn't fit in 10
            // cols, and the handoff's own drawing shows this text overflowing past where
            // numbers align in the rows beneath it.
            let combined = Rect {
                x: unit_area.x,
                width: unit_area.width + 1 + balance_area.width,
                ..unit_area
            };
            let dim = Style::default().add_modifier(Modifier::DIM);
            frame.render_widget(
                Paragraph::new(Span::styled(format!("{count}    {right_text}"), dim))
                    .alignment(Alignment::Right),
                combined,
            );
        }
        ListLine::Account {
            account,
            is_last_in_group,
        } => {
            let is_selected = account.id == selected;
            if is_selected {
                frame.render_widget(
                    Block::new().style(Style::default().add_modifier(Modifier::REVERSED)),
                    area,
                );
            }

            let dim = Style::default().add_modifier(Modifier::DIM);
            let (name_area, unit_area, balance_area) = list_row_columns(area);

            let connector = if *is_last_in_group { "└ " } else { "├ " };
            let name_text = if account.is_active {
                format!("{connector}{}", account.name)
            } else {
                format!("{connector}{} · inactive", account.name)
            };
            let name_style = if !account.is_active && !is_selected {
                dim
            } else {
                Style::default()
            };
            frame.render_widget(
                Paragraph::new(Span::styled(name_text, name_style)),
                name_area,
            );

            let unit_style = if is_selected { Style::default() } else { dim };
            frame.render_widget(
                Paragraph::new(Span::styled(account.unit.code.clone(), unit_style))
                    .alignment(Alignment::Right),
                unit_area,
            );

            let balance = balance_for_display(account);
            let is_negative = balance.0 < 0;
            let balance_style = if is_negative {
                Style::default().fg(ACCENT)
            } else {
                Style::default()
            };
            frame.render_widget(
                Paragraph::new(Span::styled(
                    format_money_at(&balance, account.unit.decimal_places),
                    balance_style,
                ))
                .alignment(Alignment::Right),
                balance_area,
            );
        }
    }
}

/// `starting_balance + transactions_sum`, computed the same way `AccountStore::balance` does
/// — duplicated here (rather than threading a store reference through every row) because
/// rendering a row only ever needs this one account's own fields, already borrowed.
fn balance_for_display(account: &Account) -> Money {
    Money(account.starting_balance.0.clone() + account.transactions_sum.0.clone())
}

/// One `label   value` summary row, the label padded to [`SUMMARY_LABEL_WIDTH`] and dimmed —
/// mirrors `view::categories`'s own `summary_field_line`.
fn summary_field_line<'a>(label: &'a str, value: &'a str) -> Paragraph<'a> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    Paragraph::new(Line::from(vec![
        Span::styled(format!("{label:<SUMMARY_LABEL_WIDTH$}"), dim),
        Span::raw(value),
    ]))
}

fn format_date(date: chrono::NaiveDate) -> String {
    date.format("%d %b").to_string().to_lowercase()
}

/// Formats `value` at `decimal_places`, with a space thousands-separator — e.g. `412.4800`
/// for a fund at 4dp, `0.18400000` for BTC at 8dp, `1 284.30` for currency at 2dp.
fn format_money_at(value: &Money, decimal_places: i64) -> String {
    group_thousands(&value.0.with_scale(decimal_places).to_plain_string())
}

fn group_thousands(plain: &str) -> String {
    let (sign, rest) = match plain.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", plain),
    };
    let (int_part, frac_part) = match rest.split_once('.') {
        Some((int_part, frac_part)) => (int_part, Some(frac_part)),
        None => (rest, None),
    };

    let mut grouped: String = int_part
        .chars()
        .rev()
        .enumerate()
        .flat_map(|(index, ch)| {
            let mut chars = Vec::with_capacity(2);
            if index != 0 && index % 3 == 0 {
                chars.push(' ');
            }
            chars.push(ch);
            chars
        })
        .collect();
    grouped = grouped.chars().rev().collect();

    match frac_part {
        Some(frac_part) => format!("{sign}{grouped}.{frac_part}"),
        None => format!("{sign}{grouped}"),
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(view: &AccountsView) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering the Accounts view should not error");

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

    fn find_id(view: &AccountsView, name: &str) -> RowID {
        view.store
            .accounts()
            .iter()
            .find(|account| account.name == name)
            .unwrap_or_else(|| panic!("fixture should seed an account named {name}"))
            .id
    }

    #[test]
    fn title_is_accounts() {
        assert_eq!(AccountsView::new().title(), "Accounts");
    }

    #[test]
    fn renders_without_panicking() {
        render(&AccountsView::new());
    }

    #[test]
    fn starts_selected_on_the_first_visible_account() {
        let view = AccountsView::new();
        let selected = view.selected_account().expect("a default selection exists");
        assert_eq!(selected.name, "Wallet");
    }

    #[test]
    fn default_view_shows_seven_active_accounts_and_hides_two_inactive() {
        let view = AccountsView::new();
        let visible = view.visible_accounts();
        assert_eq!(visible.len(), 7);
        assert!(!visible.iter().any(|account| account.name == "Travel cash"));
        assert!(!visible.iter().any(|account| account.name == "Old Savings"));
    }

    #[test]
    fn za_reveals_the_two_inactive_accounts() {
        let mut view = AccountsView::new();
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('a')));
        assert_eq!(view.visible_accounts().len(), 9);
    }

    #[test]
    fn za_twice_hides_them_again() {
        let mut view = AccountsView::new();
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('a')));
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('a')));
        assert_eq!(view.visible_accounts().len(), 7);
    }

    #[test]
    fn z_followed_by_a_non_a_key_aborts_the_chord() {
        let mut view = AccountsView::new();
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('x')));
        assert!(!view.show_inactive);
    }

    #[test]
    fn j_and_k_move_selection_among_visible_accounts_only() {
        let mut view = AccountsView::new();
        let before = view.selected;
        view.handle_key(key(KeyCode::Char('j')));
        assert_ne!(view.selected, before);
        view.handle_key(key(KeyCode::Char('k')));
        assert_eq!(view.selected, before);
    }

    #[test]
    fn home_and_g_jump_to_the_first_and_last_visible_accounts() {
        let mut view = AccountsView::new();
        view.handle_key(key(KeyCode::Char('G')));
        let last = view
            .selected_account()
            .expect("a selection exists")
            .name
            .clone();
        assert_eq!(last, "Home Loan");

        view.handle_key(key(KeyCode::Home));
        let first = view
            .selected_account()
            .expect("a selection exists")
            .name
            .clone();
        assert_eq!(first, "Wallet");
    }

    #[test]
    fn slash_filters_the_list_by_name_across_all_types() {
        let mut view = AccountsView::new();
        view.handle_key(key(KeyCode::Char('/')));
        for c in "vdhg".chars() {
            view.handle_key(key(KeyCode::Char(c)));
        }
        view.handle_key(key(KeyCode::Enter));

        let visible = view.visible_accounts();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "Vanguard VDHG");
    }

    #[test]
    fn escape_while_filtering_clears_the_filter() {
        let mut view = AccountsView::new();
        view.handle_key(key(KeyCode::Char('/')));
        view.handle_key(key(KeyCode::Char('x')));
        view.handle_key(key(KeyCode::Esc));

        assert!(view.filter.is_empty());
        assert_eq!(view.visible_accounts().len(), 7);
    }

    #[test]
    fn filtering_recovers_a_selection_the_filter_hides() {
        let mut view = AccountsView::new();
        view.selected = find_id(&view, "Home Loan");

        view.handle_key(key(KeyCode::Char('/')));
        for c in "wallet".chars() {
            view.handle_key(key(KeyCode::Char(c)));
        }
        view.handle_key(key(KeyCode::Enter));

        let selected = view.selected_account().expect("a selection exists");
        assert_eq!(selected.name, "Wallet");
    }

    #[test]
    fn investment_group_renders_mixed_units_not_a_summed_number() {
        let view = AccountsView::new();
        let text = render(&view);
        assert!(text.contains("mixed units"));
    }

    #[test]
    fn bank_group_renders_a_real_subtotal() {
        let view = AccountsView::new();
        let text = render(&view);
        // Everyday Spending 4 210.65 + Mortgage Offset 24 429.50.
        assert!(text.contains("28 640.15"));
    }

    #[test]
    fn negative_balances_render_with_a_minus_sign() {
        let view = AccountsView::new();
        let text = render(&view);
        assert!(text.contains("-1 284.3") || text.contains("-1284.3"));
    }

    #[test]
    fn summary_box_shows_the_selected_accounts_fact_line_and_computed_rows() {
        let mut view = AccountsView::new();
        view.selected = find_id(&view, "Everyday Spending");
        let text = render(&view);

        assert!(text.contains("bank"));
        assert!(text.contains("start"));
        assert!(text.contains("computed"));
        assert!(text.contains("1284"));
        assert!(text.contains("var"));
        // The full "[×] · offered when posting" clips at this pane width — cells clip, they
        // don't wrap, per this repo's own convention — so only the guaranteed-visible prefix
        // is asserted here.
        assert!(text.contains("[×] · offered"));
    }

    #[test]
    fn summary_box_shows_none_for_an_account_with_no_transactions() {
        let mut view = AccountsView::new();
        view.selected = find_id(&view, "Wallet");
        let text = render(&view);
        assert!(text.contains("none"));
    }

    #[test]
    fn format_money_at_renders_each_units_own_precision() {
        assert_eq!(
            format_money_at(
                &Money(bigdecimal::BigDecimal::from_str("412.48").unwrap()),
                4
            ),
            "412.4800"
        );
        assert_eq!(
            format_money_at(
                &Money(bigdecimal::BigDecimal::from_str("0.184").unwrap()),
                8
            ),
            "0.18400000"
        );
    }

    use std::str::FromStr;
}
