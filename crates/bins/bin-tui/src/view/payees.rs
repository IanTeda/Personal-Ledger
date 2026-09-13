//! The Payees `View`, hosted by `Shell` (ADR-0013). Per `docs/ux/tui/payees/README.md`
//! "8a — Payees screen": the left pane, built here, is the spend-ordered curation-queue list
//! and the selected Payee's six-line record box ("Payees: 8a screen — list pane and record box
//! (left pane)"); the right pane (category mix, transactions) is a bordered placeholder for the
//! next ticket ("Payees: 8a screen — category mix and transactions (right pane)").
//!
//! Real, interactive state — not a wireframe: `store` ([`PayeeFixture`], from "Payees: fixture
//! data seam and mutable View state pattern") genuinely holds the Payee list, and
//! `selected`/`show_inactive`/`filter` are mutated directly in [`PayeesView::handle_key`],
//! mirroring `crate::account`'s/`view::accounts`'s own state-ownership decision. Every key
//! handled this way returns [`Action::NoOp`] rather than `None`, so `Shell`'s event loop still
//! redraws immediately instead of waiting for the next `Tick`.
//!
//! **Keys not wired here**: `n`/`e`/`m`/`c`/`d`/`enter` — the new/edit/rename-matches/delete
//! popups and the filtered-transactions jump are later tickets in this same map (8b–8e, and
//! the right pane); this `View` only builds the list and record box the README's own 8a
//! specifies for this ticket. `a` (toggle active) is likewise deferred — it has no popup to
//! open, but routing it through an `Action` the way `view::accounts`'s own bare `a` does needs
//! an `Action::SetPayeeActive` variant this ticket has no other use for yet.
//!
//! **List, not a grouped tree**: unlike Accounts/Categories, Payees has no classification
//! dimension to group by — the README's own 8a is a flat list, **sorted by `abs(total)`
//! descending**, not by name (today's dead `screen::payees_list` sorted by name; this
//! deliberately replaces that).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crate::payee::{Payee, PayeeFixture, PayeeStore};
use crate::view::{Action, View};
use lib_core::{Money, RowID};

/// Width of the left pane (list + record box), per the handoff's own `Layout::horizontal([
/// Constraint::Length(41), Constraint::Min(0)])`.
const LEFT_PANE_WIDTH: u16 = 41;

/// Width of the list's `M` (alias count) column.
const MATCH_COL_WIDTH: u16 = 2;

/// Width of the list's right-aligned, signed `TOTAL` column, per the handoff's own spec.
const TOTAL_COL_WIDTH: u16 = 11;

/// Width of the record box's label column, e.g. `"default   "`.
const RECORD_LABEL_WIDTH: usize = 10;

/// Number of content rows the record box always shows: the fact line plus five fields
/// (`total`/`website`/`icon`/`default`/`matches`), per the handoff's own "Six lines" (the rule
/// beneath the fact line is drawn separately, mirroring `view::accounts`'s own
/// `SUMMARY_CONTENT_ROWS` convention).
const RECORD_CONTENT_ROWS: u16 = 6;

/// The base Unit every Payee total is stated in — a placeholder, mirroring
/// `view::accounts`'s own `BASE_UNIT_CODE`: `view::settings` has no live `general.base_unit`
/// backend yet.
const BASE_UNIT_CODE: &str = "AUD";

/// The Payees `View`. Owns the fixture Payee list directly — no navigation-stack `Action`
/// carries it, per `crate::payee`'s state-ownership decision — so `handle_key` mutates
/// `store`/`selected`/`show_inactive`/`filter` in place.
pub struct PayeesView {
    store: PayeeFixture,
    selected: RowID,
    /// `za` toggles this — inactive Payees are hidden unless it's `true`.
    show_inactive: bool,
    /// `true` right after a lone `z`, awaiting the `a` that completes the `za` chord.
    pending_z: bool,
    /// `/`'s current query — a case-insensitive substring match against a Payee's name **or**
    /// any of its alias patterns, per the handoff's own "`/` filters by name and by alias
    /// pattern".
    filter: String,
    /// `true` while `/`'s input buffer has focus.
    filtering: bool,
}

impl Default for PayeesView {
    fn default() -> Self {
        Self::new()
    }
}

impl PayeesView {
    pub fn new() -> Self {
        let store = PayeeFixture::new();
        let selected = Self::visible_payees_of(&store, false, "")
            .first()
            .map(|payee| payee.id)
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

    /// Every Payee currently visible, sorted by `abs(total)` descending — the single source of
    /// truth both rendering and selection movement walk. A free function over an explicit
    /// `store`/`show_inactive`/`filter` triple (rather than a method) so `new()` can call it
    /// before `Self` exists, mirroring `view::accounts::AccountsView::visible_accounts_of`.
    fn visible_payees_of<'a>(
        store: &'a PayeeFixture,
        show_inactive: bool,
        filter: &str,
    ) -> Vec<&'a Payee> {
        let needle = filter.to_lowercase();
        let matches_filter = |payee: &&Payee| {
            if needle.is_empty() {
                return true;
            }
            if payee.name.to_lowercase().contains(&needle) {
                return true;
            }
            store
                .aliases(payee.id)
                .iter()
                .any(|alias| alias.pattern.to_lowercase().contains(&needle))
        };

        let mut visible: Vec<&Payee> = store
            .payees()
            .iter()
            .filter(|payee| show_inactive || payee.is_active)
            .filter(matches_filter)
            .collect();
        visible.sort_by_key(|payee| std::cmp::Reverse(store.total(payee.id).0.abs()));
        visible
    }

    fn visible_payees(&self) -> Vec<&Payee> {
        Self::visible_payees_of(&self.store, self.show_inactive, &self.filter)
    }

    fn selected_payee(&self) -> Option<&Payee> {
        self.store.find(self.selected)
    }

    fn move_selection(&mut self, delta: isize) {
        let visible = self.visible_payees();
        let Some(current_index) = visible.iter().position(|payee| payee.id == self.selected) else {
            self.recover_selection();
            return;
        };
        let next_index = (current_index as isize + delta).clamp(0, visible.len() as isize - 1);
        self.selected = visible[next_index as usize].id;
    }

    fn select_first(&mut self) {
        if let Some(first) = self.visible_payees().first() {
            self.selected = first.id;
        }
    }

    fn select_last(&mut self) {
        if let Some(last) = self.visible_payees().last() {
            self.selected = last.id;
        }
    }

    /// Called after `za`/`/` could have hidden the selected Payee — points `selected` at the
    /// first still-visible Payee instead of leaving it dangling, mirroring
    /// `view::accounts::AccountsView::recover_selection`.
    fn recover_selection(&mut self) {
        let visible = self.visible_payees();
        if visible.iter().any(|payee| payee.id == self.selected) {
            return;
        }
        if let Some(first) = visible.first() {
            self.selected = first.id;
        }
    }

    /// The list's curation-queue flag for `payee`: `!` (its own aliases conflict with another
    /// Payee — takes priority, since an unreachable match is the more urgent problem) or `·`
    /// (no default category), or `None` for a Payee needing no attention.
    fn flag_for(&self, payee: &Payee) -> Option<char> {
        if !self.store.conflict_partners(payee.id).is_empty() {
            Some('!')
        } else if payee.default_category_path.is_none() {
            Some('·')
        } else {
            None
        }
    }
}

impl View for PayeesView {
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
            // Bare `g` is Shell's own view-jump leader, so `Home` stands in for the handoff's
            // lowercase `g` ("top"), mirroring every other domain `View`'s own convention.
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
        // record box, matching every other domain `View`'s own leading spacer row.

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(LEFT_PANE_WIDTH), Constraint::Min(0)])
            .spacing(2)
            .split(rows[1]);

        self.render_left_pane(frame, columns[0]);
        frame.render_widget(Block::bordered().title(" Payees "), columns[1]);
    }

    fn title(&self) -> &'static str {
        "Payees"
    }
}

impl PayeesView {
    fn render_left_pane(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),                       // column header
                Constraint::Min(0),                          // list
                Constraint::Length(1),                       // legend
                Constraint::Length(RECORD_CONTENT_ROWS + 3), // content + 1 rule + 2 border
            ])
            .split(area);

        render_list_column_header(frame, rows[0]);
        self.render_list(frame, rows[1]);
        render_legend(frame, rows[2]);
        self.render_record(frame, rows[3]);
    }

    fn render_list(&self, frame: &mut Frame, area: Rect) {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .spacing(1)
            .split(area);
        let content_area = split[0];
        let scrollbar_column = split[1];

        let visible = self.visible_payees();
        let shown = visible.len().min(content_area.height as usize);
        let row_constraints: Vec<Constraint> =
            std::iter::repeat_n(Constraint::Length(1), shown).collect();
        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(content_area);

        for (payee, row_area) in visible.iter().zip(row_areas.iter()) {
            self.render_list_row(frame, *row_area, payee);
        }

        let total = visible.len();
        let mut scrollbar_state = ScrollbarState::new(total)
            .viewport_content_length(shown)
            .position(0);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(scrollbar, scrollbar_column, &mut scrollbar_state);
    }

    fn render_list_row(&self, frame: &mut Frame, area: Rect, payee: &Payee) {
        let is_selected = payee.id == self.selected;
        if is_selected {
            frame.render_widget(
                Block::new().style(Style::default().add_modifier(Modifier::REVERSED)),
                area,
            );
        }

        let (flag_area, name_area, match_area, total_area) = list_row_columns(area);
        let dim = Style::default().add_modifier(Modifier::DIM);
        let accent = Style::default().fg(ratatui::style::Color::Red);

        if let Some(flag) = self.flag_for(payee) {
            let style = if flag == '!' { accent } else { dim };
            frame.render_widget(
                Paragraph::new(Span::styled(flag.to_string(), style)),
                flag_area,
            );
        }

        let name_text = if payee.is_active {
            payee.name.clone()
        } else {
            format!("{} · inactive", payee.name)
        };
        let name_style = if !payee.is_active && !is_selected {
            dim
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Span::styled(name_text, name_style)),
            name_area,
        );

        let match_style = if is_selected { Style::default() } else { dim };
        frame.render_widget(
            Paragraph::new(Span::styled(
                self.store.aliases(payee.id).len().to_string(),
                match_style,
            ))
            .alignment(Alignment::Right),
            match_area,
        );

        let total = self.store.total(payee.id);
        let is_negative = total.0 < 0;
        let total_style = if is_negative {
            accent
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Span::styled(format_money(&total), total_style))
                .alignment(Alignment::Right),
            total_area,
        );
    }

    /// The record box beneath the list, for whichever Payee is selected — see the handoff's
    /// own worked Woolworths example (`docs/ux/tui/payees/README.md` "the selected payee's
    /// record").
    fn render_record(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().padding(Padding::horizontal(1));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(payee) = self.selected_payee() else {
            frame.render_widget(Paragraph::new("no payees match the current filter"), inner);
            return;
        };

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // fact line
                Constraint::Length(1), // rule
                Constraint::Length(1), // total
                Constraint::Length(1), // website
                Constraint::Length(1), // icon
                Constraint::Length(1), // default
                Constraint::Length(1), // matches
            ])
            .split(inner);

        // Compact wording ("N txns", not "seen N times") — the record box's real inner width
        // (41-col pane, minus border and `Padding::horizontal(1)` on both sides) is 37 cells,
        // and the handoff's own literal fact-line wording ("active · seen 184 times · since
        // oct 24", 38 chars) clips its own trailing digit at that width. Cells clip, they
        // don't wrap, per this repo's own convention — shortening the wording keeps every
        // field readable instead.
        let status = if payee.is_active {
            "active"
        } else {
            "inactive"
        };
        let fact_line = format!(
            "{status} · {} txns · since {}",
            payee.transaction_count,
            format_month(payee.created_on)
        );
        frame.render_widget(Paragraph::new(fact_line), rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

        let total = self.store.total(payee.id);
        frame.render_widget(
            record_field_line(
                "total",
                &format!("{} {BASE_UNIT_CODE}", format_money(&total)),
            ),
            rows[2],
        );

        let website_text = payee.website.as_deref().unwrap_or("not set");
        frame.render_widget(record_field_line("website", website_text), rows[3]);

        // A derived icon shows only the path past the website (e.g. "/favicon.ico"), not the
        // full URL — the `website` line right above already names the domain, and repeating it
        // here would overflow the record box's real 37-cell inner width for anything but the
        // shortest domains. `strip_prefix` falls back to the full URL on the rare mismatch
        // (website changed since the icon was derived, say).
        let icon_text = match (&payee.icon_url, &payee.website) {
            (Some(icon_url), Some(website)) if payee.icon_derived => {
                let path = icon_url
                    .strip_prefix(website.as_str())
                    .unwrap_or(icon_url)
                    .trim_start_matches('/');
                format!("{path} · from website")
            }
            (Some(icon_url), _) => icon_url.clone(),
            (None, _) => "not set".to_string(),
        };
        frame.render_widget(record_field_line("icon", &icon_text), rows[4]);

        let default_text = payee.default_category_path.as_deref().unwrap_or("not set");
        frame.render_widget(record_field_line("default", default_text), rows[5]);

        let matches_text = format!("{} · m manage", self.store.aliases(payee.id).len());
        frame.render_widget(record_field_line("matches", &matches_text), rows[6]);
    }
}

/// Splits a list row (or its column header) into flag(1) / name(`Min(0)`) / `M` / `TOTAL`
/// columns.
fn list_row_columns(area: Rect) -> (Rect, Rect, Rect, Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(MATCH_COL_WIDTH),
            Constraint::Length(TOTAL_COL_WIDTH),
        ])
        .spacing(1)
        .split(area);
    (columns[0], columns[1], columns[2], columns[3])
}

fn render_list_column_header(frame: &mut Frame, area: Rect) {
    let (_, name, matches, total) = list_row_columns(area);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled("NAME", dim)), name);
    frame.render_widget(
        Paragraph::new(Span::styled("M", dim)).alignment(Alignment::Right),
        matches,
    );
    frame.render_widget(
        Paragraph::new(Span::styled("TOTAL", dim)).alignment(Alignment::Right),
        total,
    );
}

/// `· no default category   ! conflicting match` — the flag column's legend, per the
/// handoff's own "A legend row sits under the list; without it the glyphs are noise."
fn render_legend(frame: &mut Frame, area: Rect) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let text = "· no default category   ! conflicting match";
    frame.render_widget(Paragraph::new(Span::styled(text, dim)), area);
}

/// One `label   value` record row, the label padded to [`RECORD_LABEL_WIDTH`] and dimmed —
/// mirrors `view::accounts`'s own `summary_field_line`.
fn record_field_line<'a>(label: &'a str, value: &'a str) -> Paragraph<'a> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    Paragraph::new(Line::from(vec![
        Span::styled(format!("{label:<RECORD_LABEL_WIDTH$}"), dim),
        Span::raw(value),
    ]))
}

fn format_month(date: chrono::NaiveDate) -> String {
    date.format("%b %y").to_string().to_lowercase()
}

/// Formats `value` at 2 decimal places (every Payee total is stated in the base unit) with a
/// space thousands-separator — e.g. `-18 402.55`, `165 360.00`.
fn format_money(value: &Money) -> String {
    group_thousands(&value.0.with_scale(2).to_plain_string())
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

    fn render(view: &PayeesView) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering the Payees view should not error");

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

    fn find_id(view: &PayeesView, name: &str) -> RowID {
        view.store
            .payees()
            .iter()
            .find(|payee| payee.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a payee named {name}"))
            .id
    }

    #[test]
    fn title_is_payees() {
        assert_eq!(PayeesView::new().title(), "Payees");
    }

    #[test]
    fn renders_without_panicking() {
        render(&PayeesView::new());
    }

    #[test]
    fn starts_selected_on_the_biggest_abs_total() {
        let view = PayeesView::new();
        let selected = view.selected_payee().expect("a default selection exists");
        assert_eq!(selected.name, "Sunrise Payroll");
    }

    #[test]
    fn list_is_sorted_by_abs_total_descending() {
        let view = PayeesView::new();
        let names: Vec<&str> = view
            .visible_payees()
            .iter()
            .map(|payee| payee.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "Sunrise Payroll",
                "Home Loan Direct",
                "Woolworths",
                "Coles Central",
                "Bunnings Warehouse",
                "Origin Energy",
                "Telstra",
                "WOOLIES",
            ]
        );
    }

    #[test]
    fn default_view_shows_eight_active_payees_and_hides_one_inactive() {
        let view = PayeesView::new();
        let visible = view.visible_payees();
        assert_eq!(visible.len(), 8);
        assert!(!visible.iter().any(|payee| payee.name == "Old Vendor"));
    }

    #[test]
    fn za_reveals_the_inactive_payee() {
        let mut view = PayeesView::new();
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('a')));
        assert_eq!(view.visible_payees().len(), 9);
    }

    #[test]
    fn za_twice_hides_it_again() {
        let mut view = PayeesView::new();
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('a')));
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('a')));
        assert_eq!(view.visible_payees().len(), 8);
    }

    #[test]
    fn z_followed_by_a_non_a_key_aborts_the_chord() {
        let mut view = PayeesView::new();
        view.handle_key(key(KeyCode::Char('z')));
        view.handle_key(key(KeyCode::Char('x')));
        assert!(!view.show_inactive);
    }

    #[test]
    fn j_and_k_move_selection_among_visible_payees_only() {
        let mut view = PayeesView::new();
        let before = view.selected;
        view.handle_key(key(KeyCode::Char('j')));
        assert_ne!(view.selected, before);
        view.handle_key(key(KeyCode::Char('k')));
        assert_eq!(view.selected, before);
    }

    #[test]
    fn home_and_g_jump_to_the_first_and_last_visible_payees() {
        let mut view = PayeesView::new();
        view.handle_key(key(KeyCode::Char('G')));
        let last = view
            .selected_payee()
            .expect("a selection exists")
            .name
            .clone();
        assert_eq!(last, "WOOLIES");

        view.handle_key(key(KeyCode::Home));
        let first = view
            .selected_payee()
            .expect("a selection exists")
            .name
            .clone();
        assert_eq!(first, "Sunrise Payroll");
    }

    #[test]
    fn slash_filters_the_list_by_name() {
        let mut view = PayeesView::new();
        view.handle_key(key(KeyCode::Char('/')));
        for c in "telstra".chars() {
            view.handle_key(key(KeyCode::Char(c)));
        }
        view.handle_key(key(KeyCode::Enter));

        let visible = view.visible_payees();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "Telstra");
    }

    #[test]
    fn slash_filters_the_list_by_alias_pattern() {
        let mut view = PayeesView::new();
        view.handle_key(key(KeyCode::Char('/')));
        for c in "bank direct debit".chars() {
            view.handle_key(key(KeyCode::Char(c)));
        }
        view.handle_key(key(KeyCode::Enter));

        let visible = view.visible_payees();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "Home Loan Direct");
    }

    #[test]
    fn escape_while_filtering_clears_the_filter() {
        let mut view = PayeesView::new();
        view.handle_key(key(KeyCode::Char('/')));
        view.handle_key(key(KeyCode::Char('x')));
        view.handle_key(key(KeyCode::Esc));

        assert!(view.filter.is_empty());
        assert_eq!(view.visible_payees().len(), 8);
    }

    #[test]
    fn filtering_recovers_a_selection_the_filter_hides() {
        let mut view = PayeesView::new();
        view.selected = find_id(&view, "WOOLIES");

        view.handle_key(key(KeyCode::Char('/')));
        for c in "telstra".chars() {
            view.handle_key(key(KeyCode::Char(c)));
        }
        view.handle_key(key(KeyCode::Enter));

        let selected = view.selected_payee().expect("a selection exists");
        assert_eq!(selected.name, "Telstra");
    }

    #[test]
    fn list_flags_a_payee_with_no_default_category() {
        let view = PayeesView::new();
        let coles_central = view
            .store
            .find(find_id(&view, "Coles Central"))
            .expect("seeded");
        assert_eq!(view.flag_for(coles_central), Some('·'));
    }

    #[test]
    fn list_flags_conflicting_payees_with_bang_taking_priority() {
        let view = PayeesView::new();
        let woolworths = view
            .store
            .find(find_id(&view, "Woolworths"))
            .expect("seeded");
        assert_eq!(view.flag_for(woolworths), Some('!'));
    }

    #[test]
    fn negative_totals_render_with_a_minus_sign() {
        let view = PayeesView::new();
        let text = render(&view);
        assert!(text.contains("-42 180.00") || text.contains("-42180.00"));
    }

    #[test]
    fn record_box_shows_the_selections_fact_line_and_five_fields() {
        let mut view = PayeesView::new();
        view.selected = find_id(&view, "Woolworths");
        let text = render(&view);

        assert!(text.contains("active"));
        assert!(text.contains("184 txns"));
        assert!(text.contains("since oct 24"));
        assert!(text.contains("total"));
        assert!(text.contains("website"));
        assert!(text.contains("woolworths.com.au"));
        assert!(text.contains("icon"));
        assert!(text.contains("from website"));
        assert!(text.contains("default"));
        assert!(text.contains("matches"));
        assert!(text.contains("2 · m manage"));
    }

    #[test]
    fn record_box_shows_not_set_for_a_payee_with_no_website_or_default() {
        let mut view = PayeesView::new();
        view.selected = find_id(&view, "Telstra");
        let text = render(&view);
        // Both `website` and `default` should fall back to "not set".
        assert!(text.matches("not set").count() >= 2);
    }

    #[test]
    fn record_box_updates_on_j_and_k() {
        let mut view = PayeesView::new();
        view.selected = find_id(&view, "Sunrise Payroll");
        let before = render(&view);
        view.handle_key(key(KeyCode::Char('j')));
        let after = render(&view);
        assert_ne!(before, after);
    }
}
