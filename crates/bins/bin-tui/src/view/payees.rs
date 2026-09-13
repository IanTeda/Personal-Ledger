//! The Payees `View`, hosted by `Shell` (ADR-0013). Per `docs/ux/tui/payees/README.md`
//! "8a — Payees screen": the left pane is the spend-ordered curation-queue list and the
//! selected Payee's six-line record box ("Payees: 8a screen — list pane and record box (left
//! pane)"); the right pane, built here, is that same selection's category mix and transactions
//! ("Payees: 8a screen — category mix and transactions (right pane)").
//!
//! Real, interactive state — not a wireframe: `store` ([`PayeeFixture`], from "Payees: fixture
//! data seam and mutable View state pattern") genuinely holds the Payee list, and
//! `selected`/`show_inactive`/`filter`/`transactions_not_yet_built` are mutated directly in
//! [`PayeesView::handle_key`], mirroring `crate::account`'s/`view::accounts`'s own
//! state-ownership decision. Every key handled this way returns [`Action::NoOp`] rather than
//! `None`, so `Shell`'s event loop still redraws immediately instead of waiting for the next
//! `Tick`.
//!
//! **`enter` can only show [`PayeesView::transactions_not_yet_built`]'s message, not actually
//! jump anywhere** — the real filtered Transactions view doesn't exist yet (`view::
//! transactions` is still a wireframe), mirroring `view::tags`'s own identical limitation and
//! its "any subsequent key clears a showing message" rule.
//!
//! **`n`/`e`/`m` open the new/edit/rename-matches popups**
//! (`crate::popup::payee::new`/`edit`/`matches`, "Payees: 8b new popup"/"8c edit popup"/"8d
//! rename matches popup") — `Shell` owns the popups themselves, mirroring `view::accounts`'s
//! own `n`/`e`/`d`; this `View` only ever resolves *which* key to turn into
//! [`Action::OpenPayeeNewPopup`]/[`Action::OpenPayeeEditPopup`]/
//! [`Action::OpenPayeeMatchesPopup`], and reacts to the eventual
//! [`Action::CreatePayee`]/[`Action::UpdatePayee`]/[`Action::AddPayeeAlias`]/
//! [`Action::ReplacePayeeAlias`]/[`Action::RemovePayeeAlias`] in [`PayeesView::update`].
//!
//! **Keys not wired here**: `c`/`d` — the delete popup is a later ticket in this same map
//! (8e); this `View` only builds what 8a/8b/8c/8d themselves specify. `a` (toggle active) is
//! likewise deferred — it has no popup to open, but routing it through an
//! `Action` the way `view::accounts`'s own bare `a` does needs an `Action::SetPayeeActive`
//! variant this ticket has no other use for yet.
//!
//! **List, not a grouped tree**: unlike Accounts/Categories, Payees has no classification
//! dimension to group by — the README's own 8a is a flat list, **sorted by `abs(total)`
//! descending**, not by name (today's dead `screen::payees_list` sorted by name; this
//! deliberately replaces that).

use bigdecimal::BigDecimal;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crate::payee::{Payee, PayeeCategoryShare, PayeeFixture, PayeeStore, PayeeTransaction};
use crate::view::{Action, View};
use lib_core::{Money, RowID};

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used for
/// negative totals and the conflict flag.
const ACCENT: Color = Color::Red;

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

/// How many category-mix rows render before rolling the rest into an `N more` row, mirroring
/// `view::tags`'s own `CATEGORY_ROWS_SHOWN` — a Payee's own mix rarely runs past a handful of
/// categories, but the cap (and roll-up) keeps the section a fixed height regardless.
const CATEGORY_ROWS_SHOWN: usize = 4;

/// Widest a category-mix bar is ever drawn, per the handoff's own "max 16 cells".
const BAR_MAX_CELLS: usize = 16;

/// Width of the category-mix / transactions `CATEGORY` label column.
const CATEGORY_LABEL_WIDTH: u16 = 20;

/// Width of the category-mix section's right-aligned amount column.
const MIX_AMOUNT_WIDTH: u16 = 10;

/// Height of the category mix section: heading, rule, [`CATEGORY_ROWS_SHOWN`] bar rows plus an
/// `N more` roll-up row, then the default-agreement footer line.
const CATEGORY_MIX_HEIGHT: u16 = 1 + 1 + (CATEGORY_ROWS_SHOWN as u16 + 1) + 1;

/// How many of a Payee's newest transaction rows the right pane ever shows — no pagination
/// controls exist yet, matching `view::accounts`'s own capped ledger.
const TRANSACTIONS_VISIBLE_ROWS: usize = 10;

/// Width of the transactions list's `DATE` column.
const TXN_DATE_WIDTH: u16 = 6;

/// Width of the transactions list's right-aligned, signed `AMOUNT` column.
const TXN_AMOUNT_WIDTH: u16 = 9;

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
    /// `true` right after `enter` — shows "opening filtered Transactions — not yet built" in
    /// the transactions section instead of jumping anywhere, mirroring `view::tags`'s own
    /// identical flag.
    transactions_not_yet_built: bool,
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
            transactions_not_yet_built: false,
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

        // Any key reaching here other than the `Enter` arm below clears a showing "not yet
        // built" message — mirrors `view::tags`'s own identical rule.
        self.transactions_not_yet_built = false;

        match key.code {
            KeyCode::Char('z') => {
                self.pending_z = true;
                None
            }
            KeyCode::Enter => {
                self.transactions_not_yet_built = true;
                Some(Action::NoOp)
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
            // `n` — opens the new-payee popup (`crate::popup::payee::new`, "Payees: 8b new
            // popup"). `Shell` owns the popup itself, mirroring `view::accounts`'s own `n`;
            // this `View` only ever resolves *which* key to turn into
            // `Action::OpenPayeeNewPopup`.
            KeyCode::Char('n') => Some(Action::OpenPayeeNewPopup),
            // `e` — opens the edit popup (`crate::popup::payee::edit`, "Payees: 8c edit
            // popup"). `Shell` owns the popup itself, mirroring `view::accounts`'s own `e`.
            KeyCode::Char('e') => Some(Action::OpenPayeeEditPopup(self.selected)),
            // `m` — opens the rename-matches popup (`crate::popup::payee::matches`, "Payees:
            // 8d rename matches popup"). `Shell` owns the popup itself, mirroring
            // `view::accounts`'s own `n`/`e`/`d`.
            KeyCode::Char('m') => Some(Action::OpenPayeeMatchesPopup(self.selected)),
            _ => None,
        }
    }

    /// Reacts to the Payee popup's own confirmed create/save/alias mutation (`Shell` relays
    /// these after resolving them against `payee_store()` — see
    /// `crate::popup::payee::new`/`edit`/`matches`'s own module docs). Every other `Action`
    /// variant is ignored.
    fn update(&mut self, action: &Action) {
        match action {
            Action::CreatePayee {
                name,
                website,
                icon_url,
                icon_derived,
                default_category_path,
                active,
                ..
            } => {
                if let Ok(id) = self.store.create(
                    name.clone(),
                    website.clone(),
                    icon_url.clone(),
                    *icon_derived,
                    default_category_path.clone(),
                    *active,
                ) {
                    self.selected = id;
                }
            }
            // `UpdatePayee` bundles what the real domain splits into three separate
            // `PayeeStore` calls (see `Action::UpdatePayee`'s own doc) — a rename first (it
            // alone can fail on a collision the popup already refused to submit, so this
            // never actually errors in practice), then the remaining fields, then active.
            Action::UpdatePayee {
                id,
                name,
                website,
                icon_url,
                icon_derived,
                default_category_path,
                active,
            } => {
                let _ = self.store.rename(*id, name.clone());
                let _ = self.store.update(
                    *id,
                    website.clone(),
                    icon_url.clone(),
                    *icon_derived,
                    default_category_path.clone(),
                );
                let _ = self.store.set_active(*id, *active);
            }
            // `AddPayeeAlias`/`ReplacePayeeAlias`/`RemovePayeeAlias` are only ever dispatched
            // once `Shell`'s key routing (`map_payee_popup_key`) has already checked they'll
            // succeed against the read-only store (a collision refusal, or a `source =
            // Rename` guard, means no action is produced at all) — see
            // `crate::popup::payee::matches`'s own module doc.
            Action::AddPayeeAlias {
                payee_id,
                typed,
                mode,
            } => {
                let _ = self.store.add_alias(*payee_id, typed, *mode);
            }
            Action::ReplacePayeeAlias {
                old_alias_id,
                payee_id,
                typed,
                mode,
            } => {
                let _ = self.store.remove_alias(*old_alias_id);
                let _ = self.store.add_alias(*payee_id, typed, *mode);
            }
            Action::RemovePayeeAlias(alias_id) => {
                let _ = self.store.remove_alias(*alias_id);
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
        // record box, matching every other domain `View`'s own leading spacer row.

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(LEFT_PANE_WIDTH), Constraint::Min(0)])
            .spacing(2)
            .split(rows[1]);

        self.render_left_pane(frame, columns[0]);
        self.render_right_pane(frame, columns[1]);
    }

    fn title(&self) -> &'static str {
        "Payees"
    }

    fn payee_store(&self) -> Option<&dyn PayeeStore> {
        Some(&self.store)
    }

    fn payee_selection(&self) -> Option<RowID> {
        Some(self.selected)
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
        let accent = Style::default().fg(ACCENT);

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

    /// The right pane: category mix, then transactions — nothing renders when no Payee is
    /// selected (an all-filtered-out list), mirroring `view::tags`'s own identical fallback.
    fn render_right_pane(&self, frame: &mut Frame, area: Rect) {
        let Some(payee) = self.selected_payee() else {
            return;
        };

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(CATEGORY_MIX_HEIGHT),
                Constraint::Length(1), // spacer
                Constraint::Min(0),    // transactions
            ])
            .split(area);

        self.render_category_mix(frame, rows[0], payee);
        self.render_transactions(frame, rows[2], payee);
    }

    /// The category mix, biggest first, as proportional block-glyph bars — this is the
    /// justification for the default category, per the handoff's own "it is why `c` sits next
    /// to it".
    fn render_category_mix(&self, frame: &mut Frame, area: Rect, payee: &Payee) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),                              // heading
                Constraint::Length(1),                              // rule
                Constraint::Length(CATEGORY_ROWS_SHOWN as u16 + 1), // bar rows incl. roll-up
                Constraint::Length(1),                              // agreement footer line
            ])
            .split(area);

        let mix = self.store.category_mix(payee.id);
        let heading_tag = if mix.is_empty() {
            String::new()
        } else {
            format!("{} categories", mix.len())
        };
        render_section_heading(frame, rows[0], "CATEGORY MIX", &heading_tag);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

        if mix.is_empty() {
            let dim = Style::default().add_modifier(Modifier::DIM);
            frame.render_widget(
                Paragraph::new(Span::styled("no transactions yet", dim)),
                rows[2],
            );
        } else {
            let shown = mix.len().min(CATEGORY_ROWS_SHOWN);
            let bar_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(1); CATEGORY_ROWS_SHOWN + 1])
                .split(rows[2]);

            for (index, row) in mix[..shown].iter().enumerate() {
                render_mix_bar(
                    frame,
                    bar_rows[index],
                    &row.category_path,
                    row.share,
                    &row.amount,
                );
            }
            if mix.len() > shown {
                let rest_share: f64 = mix[shown..].iter().map(|row| row.share).sum();
                let rest_amount: BigDecimal =
                    mix[shown..].iter().map(|row| row.amount.0.clone()).sum();
                let label = format!("{} more", mix.len() - shown);
                render_mix_bar(
                    frame,
                    bar_rows[shown],
                    &label,
                    rest_share,
                    &Money(rest_amount),
                );
            }
        }

        render_default_agreement_line(frame, rows[3], payee, &mix);
    }

    /// The Payee's transaction rows, newest first, capped to [`TRANSACTIONS_VISIBLE_ROWS`] —
    /// mirrors `view::accounts::render_ledger`'s own column/heading/footer shape.
    fn render_transactions(&self, frame: &mut Frame, area: Rect, payee: &Payee) {
        let all_rows = self.store.transactions(payee.id);
        let total = all_rows.len();
        let visible_count = all_rows.len().min(TRANSACTIONS_VISIBLE_ROWS);
        let visible = &all_rows[..visible_count];

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // heading
                Constraint::Length(1), // rule
                Constraint::Length(1), // column header
                Constraint::Min(0),    // rows
                Constraint::Length(1), // status legend
                Constraint::Length(1), // spacer
                Constraint::Length(1), // span
                Constraint::Length(1), // explanatory line
            ])
            .split(area);

        let heading_tag = format!("{visible_count} of {total} · newest first");
        render_section_heading(frame, sections[0], "TRANSACTIONS", &heading_tag);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
        render_txn_column_header(frame, sections[2]);

        let row_count = visible.len().min(sections[3].height as usize);
        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); row_count])
            .split(sections[3]);
        for (row, row_area) in visible.iter().zip(row_areas.iter()) {
            render_txn_row(frame, *row_area, row);
        }

        let dim = Style::default().add_modifier(Modifier::DIM);
        frame.render_widget(
            Paragraph::new(Span::styled("○ open · ✓ reconciled", dim)),
            sections[4],
        );

        self.render_transactions_footer(frame, sections[6], payee, &all_rows);
        frame.render_widget(
            Paragraph::new(Span::styled(
                "payees are created by typing them on a transaction",
                dim,
            )),
            sections[7],
        );
    }

    /// The not-yet-built message while [`Self::transactions_not_yet_built`] is set, or the
    /// `N txns · first → last` span otherwise — never both, mirroring `view::tags`'s own
    /// "exactly one of the possibilities" shape.
    fn render_transactions_footer(
        &self,
        frame: &mut Frame,
        area: Rect,
        payee: &Payee,
        rows: &[PayeeTransaction],
    ) {
        if self.transactions_not_yet_built {
            frame.render_widget(
                Paragraph::new("opening filtered Transactions — not yet built"),
                area,
            );
            return;
        }

        let dim = Style::default().add_modifier(Modifier::DIM);
        let text = match (payee.first_posted, payee.last_posted) {
            (Some(first), Some(last)) => {
                format!(
                    "{} txns · {} → {}",
                    rows.len(),
                    format_month(first),
                    format_month(last)
                )
            }
            _ => "no transactions yet".to_string(),
        };
        frame.render_widget(Paragraph::new(Span::styled(text, dim)), area);
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

/// A section heading row shared by both right-pane widgets: the label flush left (dim), a
/// short dim tag right-aligned — mirrors `view::tags::render_section_heading`.
fn render_section_heading(frame: &mut Frame, area: Rect, label: &str, tag: &str) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled(label, dim)), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// One category-mix row: a category-path label, a proportional block-glyph bar (`█` full
/// cells, `▌` a half-cell remainder — the handoff's own two glyphs, giving the bar half-cell
/// resolution rather than just rounding to the nearest whole cell) plus its percentage, and the
/// right-aligned signed amount.
fn render_mix_bar(frame: &mut Frame, area: Rect, label: &str, share: f64, amount: &Money) {
    let pct = (share * 100.0).round() as i64;
    let scaled = share * BAR_MAX_CELLS as f64;
    let full_cells = scaled.floor() as usize;
    let has_half_cell = scaled.fract() >= 0.5;
    let bar = format!(
        "{}{}",
        "█".repeat(full_cells),
        if has_half_cell { "▌" } else { "" }
    );

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(CATEGORY_LABEL_WIDTH),
            Constraint::Min(0),
            Constraint::Length(MIX_AMOUNT_WIDTH),
        ])
        .spacing(1)
        .split(area);

    frame.render_widget(Paragraph::new(label.to_string()), columns[0]);
    frame.render_widget(Paragraph::new(format!("{bar} {pct}%")), columns[1]);
    let is_negative = amount.0 < 0;
    let amount_style = if is_negative {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(format_money(amount), amount_style))
            .alignment(Alignment::Right),
        columns[2],
    );
}

/// States whether the Payee's stored `default_category_path` agrees with what its own category
/// mix actually shows — never silently overriding either, per the handoff's own "where the mix
/// and the stored default disagree, say so rather than silently overriding either". A Payee
/// with no default and a mix spread across several categories is the handoff's own "should have
/// no default at all" case, stated plainly rather than flagged as wrong.
fn render_default_agreement_line(
    frame: &mut Frame,
    area: Rect,
    payee: &Payee,
    mix: &[PayeeCategoryShare],
) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let top = mix.first();
    let text = match (&payee.default_category_path, top) {
        (Some(default), Some(top)) if *default == top.category_path => {
            format!(
                "default follows the mix · {}% {}",
                (top.share * 100.0).round() as i64,
                top.category_path
            )
        }
        // Deliberately doesn't restate `default` itself — the record box one pane over
        // already shows it, and a long category path on both sides of the line would overflow
        // the right pane's real width for anything but the shortest names.
        (Some(_), Some(top)) => {
            format!(
                "disagrees with the mix · {}% {}",
                (top.share * 100.0).round() as i64,
                top.category_path
            )
        }
        (Some(_), None) => "default set · no transactions yet to check against".to_string(),
        (None, Some(_)) => format!("no default set · mix spans {} categories", mix.len()),
        (None, None) => "no default set · no transactions yet".to_string(),
    };
    frame.render_widget(Paragraph::new(Span::styled(text, dim)), area);
}

/// Splits a transactions row (or its column header) into status-glyph(1) / `DATE` /
/// `CATEGORY`(`Min(0)`) / `AMOUNT` columns.
fn txn_row_columns(area: Rect) -> (Rect, Rect, Rect, Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(TXN_DATE_WIDTH),
            Constraint::Min(0),
            Constraint::Length(TXN_AMOUNT_WIDTH),
        ])
        .spacing(1)
        .split(area);
    (columns[0], columns[1], columns[2], columns[3])
}

fn render_txn_column_header(frame: &mut Frame, area: Rect) {
    let (_, date, category, amount) = txn_row_columns(area);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled("DATE", dim)), date);
    frame.render_widget(Paragraph::new(Span::styled("CATEGORY", dim)), category);
    frame.render_widget(
        Paragraph::new(Span::styled("AMOUNT", dim)).alignment(Alignment::Right),
        amount,
    );
}

fn render_txn_row(frame: &mut Frame, area: Rect, row: &PayeeTransaction) {
    let (glyph_area, date_area, category_area, amount_area) = txn_row_columns(area);

    frame.render_widget(Paragraph::new(row.status.glyph().to_string()), glyph_area);
    frame.render_widget(Paragraph::new(format_date(row.date)), date_area);
    frame.render_widget(Paragraph::new(row.category_path.clone()), category_area);

    let is_negative = row.amount.0 < 0;
    let amount_style = if is_negative {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(format_money(&row.amount), amount_style))
            .alignment(Alignment::Right),
        amount_area,
    );
}

fn format_date(date: chrono::NaiveDate) -> String {
    date.format("%d %b").to_string().to_lowercase()
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

    /// Renders a single mix bar into an isolated buffer and counts its glyphs — mirrors
    /// `view::tags`'s own `render_bar` test helper.
    fn mix_bar_glyphs(share: f64) -> (usize, usize) {
        // Wide enough that the bar column itself (`Min(0)`, after the label and amount
        // columns) never clips a full 16-cell bar.
        let backend = TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| {
                render_mix_bar(
                    frame,
                    frame.area(),
                    "test",
                    share,
                    &Money(BigDecimal::from(1)),
                )
            })
            .expect("rendering a bar row should not error");
        let buffer = terminal.backend().buffer();
        let mut line = String::new();
        for x in 0..buffer.area.width {
            line.push_str(buffer[(x, 0)].symbol());
        }
        (line.matches('█').count(), line.matches('▌').count())
    }

    #[test]
    fn mix_bar_is_proportional_and_capped_at_bar_max_cells() {
        assert_eq!(mix_bar_glyphs(1.0), (BAR_MAX_CELLS, 0));
        assert_eq!(mix_bar_glyphs(0.0), (0, 0));
        // 0.5 * 16 = 8.0 exactly — no remainder, so no half-cell glyph.
        assert_eq!(mix_bar_glyphs(0.5), (8, 0));
        // 0.84 * 16 = 13.44 — 13 full cells, and the 0.44 remainder is under half a cell.
        assert_eq!(mix_bar_glyphs(0.84), (13, 0));
        // 0.55 * 16 = 8.8 — 8 full cells, and the 0.8 remainder rounds up to a half-cell glyph.
        assert_eq!(mix_bar_glyphs(0.55), (8, 1));
    }

    #[test]
    fn category_mix_heading_shows_the_category_count() {
        let mut view = PayeesView::new();
        view.selected = find_id(&view, "Woolworths");
        let text = render(&view);
        assert!(text.contains("3 categories"));
    }

    #[test]
    fn woolworths_mix_agrees_with_its_stored_default() {
        let mut view = PayeesView::new();
        view.selected = find_id(&view, "Woolworths");
        let mix = view.store.category_mix(view.selected);
        let top = mix.first().expect("woolworths has a mix");
        let expected_pct = format!("{}%", (top.share * 100.0).round() as i64);

        let text = render(&view);
        assert!(text.contains("default follows the mix"));
        assert!(text.contains(&expected_pct));
        assert!(text.contains("food/groceries"));
    }

    #[test]
    fn bunnings_warehouse_mix_disagrees_with_its_stored_default() {
        let mut view = PayeesView::new();
        view.selected = find_id(&view, "Bunnings Warehouse");
        let text = render(&view);
        assert!(text.contains("disagrees with the mix"));
    }

    #[test]
    fn coles_central_has_no_default_and_its_mix_is_stated_as_spread() {
        let mut view = PayeesView::new();
        view.selected = find_id(&view, "Coles Central");
        let text = render(&view);
        assert!(text.contains("no default set"));
        assert!(text.contains("mix spans"));
    }

    #[test]
    fn transactions_show_a_status_glyph_and_a_legend() {
        let mut view = PayeesView::new();
        view.selected = find_id(&view, "Woolworths");
        let rows = view.store.transactions(view.selected);
        assert!(!rows.is_empty());
        for row in &rows {
            assert!(row.status.glyph() == '○' || row.status.glyph() == '✓');
        }

        let text = render(&view);
        assert!(text.contains("○ open · ✓ reconciled"));
    }

    #[test]
    fn transactions_span_shows_count_and_date_range() {
        let mut view = PayeesView::new();
        view.selected = find_id(&view, "Woolworths");
        let text = render(&view);
        assert!(text.contains("184 txns"));
        assert!(text.contains("oct 24"));
        assert!(text.contains("sep 26"));
    }

    #[test]
    fn footer_states_the_screens_own_misreading_warning() {
        let view = PayeesView::new();
        let text = render(&view);
        assert!(text.contains("payees are created by typing them on a transaction"));
    }

    #[test]
    fn enter_shows_a_not_yet_built_message_and_any_other_key_clears_it() {
        let mut view = PayeesView::new();
        view.handle_key(key(KeyCode::Enter));
        assert!(view.transactions_not_yet_built);
        let text = render(&view);
        assert!(text.contains("opening filtered Transactions — not yet built"));

        view.handle_key(key(KeyCode::Char('j')));
        assert!(!view.transactions_not_yet_built);
    }
}
