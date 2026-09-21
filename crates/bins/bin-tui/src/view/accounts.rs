//! The Accounts `View`, hosted by `Shell` (ADR-0013). Per
//! `docs/ux/tui/accounts/README.md` "7a — Accounts screen": the left pane is the
//! `AccountType`-grouped list and the summary box for whichever account is selected
//! ("Accounts: 7a screen — list pane and summary box (left pane)"); the right pane, built
//! here, is that same selection's month-end balance chart and its inline ledger list
//! ("Accounts: 7a screen — balance line and ledger list (right pane)").
//!
//! Real, interactive state — not a wireframe: `store` ([`AccountFixture`], from "Accounts:
//! fixture data seam and mutable View state pattern") genuinely holds the account list, and
//! `selected`/`show_inactive`/`filter` are mutated directly in
//! [`AccountsView::handle_key`], mirroring `crate::category`'s/`view::categories`'s own
//! state-ownership decision. Every key handled this way returns [`Action::NoOp`] rather than
//! `None`, so `Shell`'s event loop still redraws immediately instead of waiting for the next
//! `Tick`.
//!
//! **Keys not wired here**: `b` (Balance Check/reconcile) is out of this map's destination
//! entirely (README §*Not yet designed*) and has no "not yet built" surface of its own to
//! fall back to outside the command popup (tracked separately as issue #96) — it stays a
//! silent no-op, like every other domain's own unbuilt bare keys. `n`/`e`/`d` open the
//! new/edit/delete popups (`crate::popup::account::new`/`edit`/`delete`) — `Shell` owns
//! them, not this `View`, mirroring `view::categories`'s own `m`/`n`/`e` (this `View` only
//! ever resolves *which* key to turn into `Action::OpenAccountNewPopup`/
//! `OpenAccountEditPopup`/`OpenAccountDeletePopup`; the popups themselves, and the
//! `Action::CreateAccount`/`UpdateAccount`/`DeleteAccount` that eventually land back in
//! [`AccountsView::update`], are `Shell`'s concern). `a` deactivates the selection directly
//! (see its own `handle_key` arm) — routed through `Action::SetAccountActive` rather than
//! mutated here in place, so the `:account off <acct>` command (`popup::command::commands::
//! accounts`) reaches the exact same code path. Also not wired: `tab` (focus the ledger list)
//! and `enter` (move focus into it too, per the handoff's "no separate ledger screen"
//! decision) — the ledger here is a static display, not yet its own navigable focus, the same
//! simplification `view::categories`'s own transactions list makes.
//!
//! **The ledger's running `BALANCE` column has no filter or non-date-sort state to react
//! to**: this map's own scope (issue #115's Notes and Out of scope) ships the inline ledger
//! only — no row filter, no alternate sort — so [`running_balance_is_meaningful`] always
//! returns `true` here. The function exists and is tested on its own terms so the rule itself
//! (from the handoff: blank the column whenever a filter or a non-date sort is active) is
//! real, checkable logic, not a comment nobody enforces — a future ledger-filtering ticket
//! only needs to start passing it real state.

use bigdecimal::BigDecimal;
use chrono::{Datelike, Months, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use lib_core::{AccountType, Money, RowID};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Dataset, GraphType, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
};

use crate::account::{
    Account, AccountFixture, AccountStore, AccountTransaction, AccountUnit, FIXTURE_NOW,
    shared_unit,
};
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

/// How many trailing months the balance-line chart plots, per the handoff's "24 points".
const CHART_MONTHS: usize = 24;

/// Height of the right pane's balance-chart section: heading, rule, the plot itself, then the
/// first/low/last labels beneath — matches `view::categories`'s own `SPEND_CHART_HEIGHT`, the
/// closest existing chart section, so the two screens' charts read at the same scale.
const BALANCE_CHART_HEIGHT: u16 = 12;

/// How many of an account's newest ledger rows the right pane ever shows — no pagination
/// controls exist yet, matching `view::categories`'s own "capped to the newest 10" choice for
/// its transactions list.
const LEDGER_VISIBLE_ROWS: usize = 10;

/// Width of the ledger's `DATE` column.
const LEDGER_DATE_WIDTH: u16 = 6;

/// Width of the ledger's right-aligned, signed `AMOUNT` column.
const LEDGER_AMOUNT_WIDTH: u16 = 9;

/// Width of the ledger's right-aligned, running `BALANCE` column.
const LEDGER_BALANCE_WIDTH: u16 = 10;

/// The Unit the footer's `net` line is stated in. A placeholder: `general.base_unit`
/// (`docs/ux/tui/settings/README.md`) is the real source for this, but `view::settings` has
/// no live backend yet (still wireframe-stage) — hardcoded to match every worked example in
/// `docs/ux/tui/accounts/README.md`, which is itself stated in AUD throughout.
const BASE_UNIT_CODE: &str = "AUD";

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
            KeyCode::Char('n') => Some(Action::OpenAccountNewPopup),
            KeyCode::Char('e') => Some(Action::OpenAccountEditPopup(self.selected)),
            KeyCode::Char('d') => Some(Action::OpenAccountDeletePopup(self.selected)),
            // `a`: deactivates the selection directly, per the handoff's own "a deactivate" —
            // no popup, no draft to carry, mirroring `view::categories`'s own bare `a`. Routed
            // through `Action::SetAccountActive` (not mutated here in place) so the command
            // grammar's `account off <acct>` reaches the exact same code path, per "Accounts:
            // :acct command grammar"'s own done-when.
            KeyCode::Char('a') => Some(Action::SetAccountActive {
                id: self.selected,
                active: false,
            }),
            _ => None,
        }
    }

    /// Reacts to the Account popup's own confirmed create/save (`Shell` relays these after
    /// resolving them against `account_store()` — see `crate::popup::account::new`/`edit`'s
    /// own module docs). Every other `Action` variant is ignored.
    fn update(&mut self, action: &Action) {
        use std::str::FromStr;

        match action {
            Action::CreateAccount {
                name,
                account_type,
                unit_code,
                unit_decimal_places,
                starting_balance,
                active,
                ..
            } => {
                let account_type = AccountType::from_str(account_type).unwrap_or_default();
                let unit = AccountUnit {
                    code: unit_code.clone(),
                    decimal_places: *unit_decimal_places,
                };
                let id = self.store.create(
                    name.clone(),
                    account_type,
                    unit,
                    starting_balance.clone(),
                    *active,
                );
                self.selected = id;
            }
            Action::UpdateAccount {
                id,
                name,
                account_type,
                active,
            } => {
                let account_type = AccountType::from_str(account_type).unwrap_or_default();
                let _ = self.store.update(*id, name.clone(), account_type, *active);
            }
            // `#[allow]`: clippy's own `collapsible_match` fix would move the `delete` call
            // into a match guard, which makes this read as a pure predicate when it's actually
            // the mutation — kept as a plain nested `if` for that reason.
            #[allow(clippy::collapsible_match)]
            Action::DeleteAccount { id, target } => {
                if self.store.delete(*id, *target).is_ok() {
                    self.recover_selection();
                }
            }
            // `#[allow]`: see the `DeleteAccount` arm above for why this stays a plain nested
            // `if` rather than clippy's own guard-with-a-side-effect suggestion.
            #[allow(clippy::collapsible_match)]
            Action::SetAccountActive { id, active } => {
                if self.store.set_active(*id, *active).is_ok() {
                    self.recover_selection();
                }
            }
            _ => {}
        }
    }

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
        self.render_right_pane(frame, columns[1]);
    }

    fn title(&self) -> &'static str {
        "Accounts"
    }

    fn account_store(&self) -> Option<&dyn AccountStore> {
        Some(&self.store)
    }

    fn account_selection(&self) -> Option<RowID> {
        Some(self.selected)
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

    fn render_right_pane(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(BALANCE_CHART_HEIGHT),
                Constraint::Length(1), // spacer
                Constraint::Min(0),
            ])
            .split(area);

        let Some(account) = self.selected_account() else {
            frame.render_widget(
                Paragraph::new("no accounts match the current filter"),
                rows[2],
            );
            return;
        };

        self.render_balance_chart(frame, rows[0], account);
        self.render_ledger(frame, rows[2], account);
    }

    /// The month-end balance line, a trailing [`CHART_MONTHS`]-month window ending at
    /// [`FIXTURE_NOW`], per the handoff's "Balance line" — `oct 24 – sep 26` against this
    /// fixture's own fixed "now".
    fn render_balance_chart(&self, frame: &mut Frame, area: Rect, account: &Account) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // heading
                Constraint::Length(1), // rule
                Constraint::Min(0),    // chart
                Constraint::Length(1), // labels
            ])
            .split(area);

        render_chart_heading(frame, rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

        let series: Vec<f64> = self
            .store
            .monthly_balances(account.id, CHART_MONTHS, FIXTURE_NOW)
            .iter()
            .map(money_to_f64)
            .collect();
        let points: Vec<(f64, f64)> = series
            .iter()
            .enumerate()
            .map(|(index, &amount)| (index as f64, amount))
            .collect();

        let min = series.iter().copied().fold(f64::INFINITY, f64::min);
        let max = series.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let margin = (max - min).abs().max(1.0) * 0.1;

        let line = Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default())
            .data(&points);

        let last_index = CHART_MONTHS - 1;
        let last_point = [(last_index as f64, series[last_index])];
        let last_point_marker = Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(ACCENT))
            .data(&last_point);

        let chart = Chart::new(vec![line, last_point_marker])
            .x_axis(Axis::default().bounds([0.0, last_index as f64]))
            .y_axis(Axis::default().bounds([min - margin, max + margin]));
        frame.render_widget(chart, rows[2]);

        render_chart_labels(frame, rows[3], &series, account.unit.decimal_places);
    }

    /// The ledger list: status glyph / `DATE` / `PAYEE` / `AMOUNT` / `BALANCE`, newest first,
    /// capped to [`LEDGER_VISIBLE_ROWS`] — per the handoff's "Ledger list".
    fn render_ledger(&self, frame: &mut Frame, area: Rect, account: &Account) {
        let rows_with_balance = self.ledger_rows_with_running_balance(account);
        let total = rows_with_balance.len();
        let visible: Vec<(&AccountTransaction, &Money)> = rows_with_balance
            .iter()
            .rev() // newest first
            .take(LEDGER_VISIBLE_ROWS)
            .map(|(row, balance)| (row, balance))
            .collect();
        let blank_balance = !running_balance_is_meaningful(false, true);

        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .spacing(1)
            .split(area);
        let content_area = split[0];
        let scrollbar_column = split[1];

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // heading
                Constraint::Length(1), // rule
                Constraint::Length(1), // column header
                Constraint::Min(0),    // rows
                Constraint::Length(1), // status-glyph legend + open count
                Constraint::Length(1), // net line
            ])
            .split(content_area);

        render_ledger_heading(frame, sections[0], visible.len(), total);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
        render_ledger_column_header(frame, sections[2]);
        render_ledger_rows(
            frame,
            sections[3],
            &visible,
            account.unit.decimal_places,
            blank_balance,
        );
        render_ledger_legend(frame, sections[4], account.open_count);
        render_net_line(frame, sections[5], self.visible_accounts());

        let rows_scrollbar_area = Rect {
            y: sections[3].y,
            height: sections[3].height,
            ..scrollbar_column
        };
        let mut scrollbar_state = ScrollbarState::new(total)
            .viewport_content_length(visible.len())
            .position(0);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, rows_scrollbar_area, &mut scrollbar_state);
    }

    /// `account`'s own ledger, oldest first, each row paired with the running balance
    /// immediately after it (`starting_balance` plus every prior row's amount, in date order
    /// with ties broken by the fixture's own generation order — a stable sort, so two same-day
    /// rows still get distinct, deterministic running balances).
    fn ledger_rows_with_running_balance(
        &self,
        account: &Account,
    ) -> Vec<(AccountTransaction, Money)> {
        let mut ascending = self.store.ledger(account.id);
        ascending.sort_by_key(|row| row.date);

        let mut running = account.starting_balance.0.clone();
        ascending
            .into_iter()
            .map(|row| {
                running += row.amount.0.clone();
                (row, Money(running.clone()))
            })
            .collect()
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
    crate::format::day_month(date)
}

/// Formats `value` at the Unit's own `decimal_places` (`412.4800` for a fund at 4dp, `0.18400000`
/// for BTC at 8dp), grouped by the Locale.
fn format_money_at(value: &Money, decimal_places: i64) -> String {
    crate::format::money(value, decimal_places)
}

fn money_to_f64(value: &Money) -> f64 {
    value.0.to_string().parse().unwrap_or(0.0)
}

/// The handoff's own rule for the ledger's running `BALANCE` column — "it is only meaningful
/// newest-first unfiltered": blank it whenever a row filter is active or the sort isn't plain
/// date-descending. A free function (not a method) so it's exactly as testable as the rule
/// itself, independent of whether `AccountsView` currently has any state to feed it — see this
/// module's own doc comment on why that's always `(false, true)` here today.
fn running_balance_is_meaningful(filtered: bool, sorted_by_date_descending: bool) -> bool {
    !filtered && sorted_by_date_descending
}

/// The `BALANCE` heading over the chart, with the trailing window as its dim tag — mirrors
/// `view::categories`'s own `render_chart_heading`, without the direct/subtree label swap
/// Categories needs (an Account's balance line has only one series).
fn render_chart_heading(frame: &mut Frame, area: Rect) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let tag = format!(
        "{} – {}",
        format_month(chart_month(0)),
        format_month(chart_month(CHART_MONTHS - 1))
    );
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(Span::styled(crate::msg::tui_accounts_column_balance(), dim)),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The row beneath the chart: first month + its value, the series' low point + its own
/// month, last month + its value — per the handoff's "Labels beneath: first point, low with
/// its month, last point" (Categories' sibling chart instead labels an average here; Accounts'
/// own handoff asks for the low point specifically).
fn render_chart_labels(frame: &mut Frame, area: Rect, series: &[f64], decimal_places: i64) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Min(0), Constraint::Min(0)])
        .split(area);

    let (low_index, &low_value) = series
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .expect("the chart always plots at least one month");

    let dim = Style::default().add_modifier(Modifier::DIM);
    let first_text = format!(
        "{}  {}",
        format_month(chart_month(0)),
        format_money_at_f64(series[0], decimal_places)
    );
    let low_text = format!(
        "low {}  {}",
        format_month(chart_month(low_index)),
        format_money_at_f64(low_value, decimal_places)
    );
    let last_index = series.len() - 1;
    let last_text = format!(
        "{}  {}",
        format_month(chart_month(last_index)),
        format_money_at_f64(series[last_index], decimal_places)
    );

    frame.render_widget(Paragraph::new(Span::styled(first_text, dim)), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(low_text, dim)).alignment(Alignment::Center),
        columns[1],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(last_text, dim)).alignment(Alignment::Right),
        columns[2],
    );
}

/// The chart's fixed end-of-window month — [`FIXTURE_NOW`]'s own month, so every account
/// shares the same trailing [`CHART_MONTHS`]-month x-axis, matching
/// `view::categories`'s own `chart_end_month` convention.
fn chart_end_month() -> NaiveDate {
    NaiveDate::from_ymd_opt(FIXTURE_NOW.year(), FIXTURE_NOW.month(), 1)
        .expect("FIXTURE_NOW's own year/month with day 1 is always a valid date")
}

/// The chart's x-axis month for `index` (`0` oldest, `CHART_MONTHS - 1` newest).
fn chart_month(index: usize) -> NaiveDate {
    chart_end_month()
        .checked_sub_months(Months::new((CHART_MONTHS - 1 - index) as u32))
        .expect("CHART_MONTHS stays well within chrono's representable range")
}

fn format_month(date: NaiveDate) -> String {
    crate::format::month_year(date)
}

fn format_money_at_f64(amount: f64, decimal_places: i64) -> String {
    crate::format::money_f64(amount, decimal_places)
}

/// Splits a ledger row (or its column header) into status-glyph(1) / `DATE` / `PAYEE`
/// (`Min(0)`) / `AMOUNT` / `BALANCE` columns.
fn ledger_row_columns(area: Rect) -> (Rect, Rect, Rect, Rect, Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(LEDGER_DATE_WIDTH),
            Constraint::Min(0),
            Constraint::Length(LEDGER_AMOUNT_WIDTH),
            Constraint::Length(LEDGER_BALANCE_WIDTH),
        ])
        .spacing(1)
        .split(area);
    (columns[0], columns[1], columns[2], columns[3], columns[4])
}

/// The `LEDGER  N of M · newest first` heading.
fn render_ledger_heading(frame: &mut Frame, area: Rect, shown: usize, total: usize) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let tag = format!("{shown} of {total} · newest first");
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(Span::styled(crate::msg::tui_accounts_column_ledger(), dim)),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

fn render_ledger_column_header(frame: &mut Frame, area: Rect) {
    let (_, date, payee, amount, balance) = ledger_row_columns(area);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled(crate::msg::tui_accounts_column_date(), dim)), date);
    frame.render_widget(Paragraph::new(Span::styled(crate::msg::tui_accounts_column_payee(), dim)), payee);
    frame.render_widget(
        Paragraph::new(Span::styled(crate::msg::tui_accounts_column_amount(), dim)).alignment(Alignment::Right),
        amount,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(crate::msg::tui_accounts_column_balance(), dim)).alignment(Alignment::Right),
        balance,
    );
}

fn render_ledger_rows(
    frame: &mut Frame,
    area: Rect,
    rows: &[(&AccountTransaction, &Money)],
    decimal_places: i64,
    blank_balance: bool,
) {
    let visible = rows.len().min(area.height as usize);
    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), visible).collect();
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    for ((row, balance), row_area) in rows.iter().zip(row_areas.iter()) {
        let (glyph_area, date_area, payee_area, amount_area, balance_area) =
            ledger_row_columns(*row_area);

        frame.render_widget(Paragraph::new(row.status.glyph().to_string()), glyph_area);
        frame.render_widget(Paragraph::new(format_date(row.date)), date_area);
        frame.render_widget(Paragraph::new(row.payee), payee_area);

        let is_negative = row.amount.0 < 0;
        let amount_style = if is_negative {
            Style::default().fg(ACCENT)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format_money_at(&row.amount, decimal_places),
                amount_style,
            ))
            .alignment(Alignment::Right),
            amount_area,
        );

        let balance_text = if blank_balance {
            String::new()
        } else {
            format_money_at(balance, decimal_places)
        };
        let balance_style = if !blank_balance && balance.0 < 0 {
            Style::default().fg(ACCENT)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Span::styled(balance_text, balance_style)).alignment(Alignment::Right),
            balance_area,
        );
    }
}

/// `○ open · ✓ reconciled · N open` — the handoff's "legend in the footer row alongside the
/// open count".
fn render_ledger_legend(frame: &mut Frame, area: Rect, open_count: u32) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let text = format!("○ open · ✓ reconciled · {open_count} open");
    frame.render_widget(Paragraph::new(Span::styled(text, dim)), area);
}

/// `net AUD 84 210.15 · USD, VDHG, BTC held separately` — sums every *currently visible*
/// account in [`BASE_UNIT_CODE`] and names every other Unit among them, never silently
/// omitting one, per the handoff's "Footer rows".
fn render_net_line(frame: &mut Frame, area: Rect, visible_accounts: Vec<&Account>) {
    let net: BigDecimal = visible_accounts
        .iter()
        .filter(|account| account.unit.code == BASE_UNIT_CODE)
        .map(|account| balance_for_display(account).0)
        .sum();

    let mut excluded: Vec<&str> = visible_accounts
        .iter()
        .map(|account| account.unit.code.as_str())
        .filter(|code| *code != BASE_UNIT_CODE)
        .collect();
    excluded.sort_unstable();
    excluded.dedup();

    let text = if excluded.is_empty() {
        format!("net {BASE_UNIT_CODE} {}", format_money_at(&Money(net), 2))
    } else {
        format!(
            "net {BASE_UNIT_CODE} {} · {} held separately",
            format_money_at(&Money(net), 2),
            excluded.join(", ")
        )
    };
    frame.render_widget(Paragraph::new(text), area);
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
        assert!(text.contains("28,640.15"));
    }

    #[test]
    fn negative_balances_render_with_a_minus_sign() {
        let view = AccountsView::new();
        let text = render(&view);
        assert!(text.contains("\u{2212}1,284.3"));
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

    #[test]
    fn running_balance_is_meaningful_only_when_unfiltered_and_date_descending() {
        assert!(running_balance_is_meaningful(false, true));
        assert!(!running_balance_is_meaningful(true, true));
        assert!(!running_balance_is_meaningful(false, false));
        assert!(!running_balance_is_meaningful(true, false));
    }

    #[test]
    fn right_pane_renders_without_panicking_for_an_account_with_no_transactions() {
        let mut view = AccountsView::new();
        view.selected = find_id(&view, "Wallet");
        let text = render(&view);
        assert!(text.contains("0 of 0 · newest first"));
    }

    #[test]
    fn ledger_heading_shows_capped_count_against_the_real_total() {
        let mut view = AccountsView::new();
        view.selected = find_id(&view, "Everyday Spending");
        let text = render(&view);
        assert!(text.contains("10 of 1284 · newest first"));
    }

    #[test]
    fn ledger_rows_show_a_status_glyph_never_colour_alone() {
        let mut view = AccountsView::new();
        view.selected = find_id(&view, "Everyday Spending");
        let account = view.selected_account().unwrap();
        let rows = view.ledger_rows_with_running_balance(account);
        assert!(!rows.is_empty());
        for (row, _) in &rows {
            assert!(row.status.glyph() == '○' || row.status.glyph() == '✓');
        }

        let text = render(&view);
        assert!(text.contains("○ open · ✓ reconciled · 12 open"));
    }

    #[test]
    fn newest_ledger_row_running_balance_equals_the_accounts_current_balance() {
        let view = AccountsView::new();
        let account = find_by_name(&view, "Everyday Spending");
        let rows = view.ledger_rows_with_running_balance(account);
        let (_, newest_balance) = rows.last().expect("seeded with transactions");
        assert_eq!(*newest_balance, view.store.balance(account.id));
    }

    #[test]
    fn ledger_sums_to_exactly_the_accounts_transactions_sum() {
        let view = AccountsView::new();
        let account = find_by_name(&view, "Everyday Spending");
        let rows = view.ledger_rows_with_running_balance(account);
        let sum: bigdecimal::BigDecimal = rows.iter().map(|(row, _)| row.amount.0.clone()).sum();
        assert_eq!(Money(sum), account.transactions_sum);
    }

    #[test]
    fn balance_chart_heading_names_the_trailing_window() {
        let mut view = AccountsView::new();
        view.selected = find_id(&view, "Everyday Spending");
        let text = render(&view);
        assert!(text.contains("BALANCE"));
        assert!(text.contains("oct 2024 – sep 2026"));
    }

    #[test]
    fn balance_chart_series_ends_on_the_accounts_current_balance() {
        let view = AccountsView::new();
        let account = find_by_name(&view, "Everyday Spending");
        let series = view
            .store
            .monthly_balances(account.id, CHART_MONTHS, FIXTURE_NOW);
        let last = series.last().expect("24 months requested");
        assert_eq!(*last, view.store.balance(account.id));
    }

    #[test]
    fn net_line_sums_base_unit_accounts_and_names_every_excluded_unit() {
        let view = AccountsView::new();
        let text = render(&view);
        // Wallet 320.40 + Everyday Spending 4 210.65 + Mortgage Offset 24 429.50
        // + Amex Platinum -1 284.30 + Home Loan -612 400.00 = -584 723.75.
        assert!(text.contains("net AUD \u{2212}584,723.75"));
        assert!(text.contains("BTC, VDHG held separately"));
    }

    #[test]
    fn net_line_excludes_units_that_are_currently_hidden() {
        let mut view = AccountsView::new();
        // "home" isolates Home Loan alone — unlike "wallet", which would also match "Cold
        // wallet" and defeat the point of this test.
        view.handle_key(key(KeyCode::Char('/')));
        for c in "home".chars() {
            view.handle_key(key(KeyCode::Char(c)));
        }
        view.handle_key(key(KeyCode::Enter));

        let text = render(&view);
        assert!(text.contains("net AUD \u{2212}612,400.00"));
        assert!(!text.contains("held separately"));
    }

    fn find_by_name<'a>(view: &'a AccountsView, name: &str) -> &'a Account {
        view.store
            .accounts()
            .iter()
            .find(|account| account.name == name)
            .unwrap_or_else(|| panic!("fixture should seed an account named {name}"))
    }
}
