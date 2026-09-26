//! The "rename matches" popup — `m` on a Payees list row (`docs/ux/tui/payees/README.md`
//! "8d — Rename matches"). A list-inside-an-overlay with its own state, not crammed into the
//! edit form — the closest thing this codebase has built before is a plain field-by-field
//! form (`popup::account::new`/`edit`), so this popup is its own shape: a navigable list of a
//! Payee's aliases, plus a single "compose" slot shared by three different intents (adding a
//! new alias, editing an existing one, or just testing arbitrary text) that only ever renders
//! one at a time.
//!
//! **`source = Manual` is the only kind `d`/`e` ever touch.** A `source = Rename` alias is
//! protected forever (its id stands in for when the rename happened) — `PayeeMatchesPopup`
//! itself refuses to ever produce [`crate::view::Action::PayeeMatchesPopupBeginEdit`]/
//! [`crate::view::Action::RemovePayeeAlias`] for one (`Shell`'s key routing checks
//! [`PayeeMatchesPopup::selected_alias`]'s own `source` before dispatching either), so the
//! protection is stated once, permanently, in the popup's own closing note rather than as a
//! per-keypress refusal message.
//!
//! **There is no `PayeeStore::update_alias`** — write-once is the whole point of a rename
//! alias, and a manual one was never given its own update method either (`crate::payee`'s own
//! module doc). "Editing" an existing manual alias here is therefore remove-then-add: `e`
//! preloads the compose slot from the alias being edited ([`decompile_pattern`] best-effort
//! reverses `PayeeStore::rename`'s own escape-and-anchor wrapping back into typed text plus
//! `exact text` mode when it round-trips exactly, falling back to raw `regex` mode otherwise),
//! and `Ctrl+S` dispatches [`crate::view::Action::ReplacePayeeAlias`], which
//! `PayeesView::update` applies as `remove_alias` then `add_alias`. `add_alias`'s own
//! collision check only ever looks at *other* Payees, never the one being edited, so removing
//! first is safe — nothing this alias itself held can collide with its own replacement.
//!
//! **Every warning is computed live from the current draft against the read-only store, never
//! stored as popup state** — mirrors `popup::payee::new`/`edit`'s own `name_conflict` pattern.
//! `Shell`'s pre-commit check (via [`PayeeMatchesPopup::commit`], which calls
//! [`crate::payee::PayeeStore::conflicting_holder`]) is the *same* check the render path shows
//! inline, so a draft that looks valid on screen is always exactly the draft `Ctrl+S` would
//! accept.

use lib_core::RowID;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::payee::{AliasMode, AliasSource, PayeeAlias, PayeeResolution, PayeeStore};
use crate::popup::REFERENCE_TERMINAL_WIDTH;

const ACCENT: Color = Color::Red;
const POPUP_WIDTH_PERCENT: u32 = 88;
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;

/// How many alias rows the list ever shows at once — the fixture never seeds more than a
/// couple of aliases per Payee, so no scrollbar/roll-up exists yet (out of scope for this
/// ticket; see the map's own Notes on fixture-only scale).
const MAX_LIST_ROWS: usize = 4;

/// Content rows inside the border: title, subtitle, rule, column header, the list, a blank
/// spacer, the four-line compose block, a blank spacer, the two-line resolve-order note, a
/// blank spacer, the two-line conflict block, a blank spacer, the protected-match note, the
/// footer's rule, then the footer itself.
const CONTENT_ROWS: u16 =
    1 + 1 + 1 + 1 + MAX_LIST_ROWS as u16 + 1 + 4 + 1 + 2 + 1 + 2 + 1 + 1 + 1 + 1;
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

const PATTERN_COL_WIDTH: u16 = 32;
const FROM_COL_WIDTH: u16 = 8;
const HITS_COL_WIDTH: u16 = 5;

/// What the shared compose slot is currently for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComposeKind {
    /// A brand-new manual alias.
    Add,
    /// A replacement for an existing manual alias (its id).
    Edit(RowID),
    /// Arbitrary text, previewed against [`PayeeStore::resolve`] but never saved.
    Test,
}

/// The compose slot's own draft state — at most one of these exists at a time.
struct Compose {
    input: String,
    mode: AliasMode,
    kind: ComposeKind,
}

/// What `Ctrl+S` would do with the current compose draft.
pub enum ComposeCommit {
    Add {
        payee_id: RowID,
        typed: String,
        mode: AliasMode,
    },
    Replace {
        old_alias_id: RowID,
        payee_id: RowID,
        typed: String,
        mode: AliasMode,
    },
}

/// The `:payee match` popup's own draft state.
pub struct PayeeMatchesPopup {
    payee_id: RowID,
    selected: usize,
    compose: Option<Compose>,
}

impl PayeeMatchesPopup {
    pub fn new(payee_id: RowID) -> Self {
        Self {
            payee_id,
            selected: 0,
            compose: None,
        }
    }

    pub fn payee_id(&self) -> RowID {
        self.payee_id
    }

    pub fn is_composing(&self) -> bool {
        self.compose.is_some()
    }

    /// The alias the list's own selection currently names, if the Payee has any.
    pub fn selected_alias<'a>(&self, store: &'a dyn PayeeStore) -> Option<&'a PayeeAlias> {
        store.aliases(self.payee_id).into_iter().nth(self.selected)
    }

    pub fn move_up(&mut self, store: &dyn PayeeStore) {
        let len = store.aliases(self.payee_id).len();
        if len == 0 {
            return;
        }
        self.selected = if self.selected == 0 {
            len - 1
        } else {
            self.selected - 1
        };
    }

    pub fn move_down(&mut self, store: &dyn PayeeStore) {
        let len = store.aliases(self.payee_id).len();
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1) % len;
    }

    pub fn begin_add(&mut self) {
        self.compose = Some(Compose {
            input: String::new(),
            mode: AliasMode::ExactText,
            kind: ComposeKind::Add,
        });
    }

    pub fn begin_test(&mut self) {
        self.compose = Some(Compose {
            input: String::new(),
            mode: AliasMode::ExactText,
            kind: ComposeKind::Test,
        });
    }

    /// Preloads the compose slot from the selected alias — a no-op unless it exists and is
    /// `source = Manual` (`Shell` already guards this before dispatching the action that
    /// reaches here, but the check is repeated so calling this directly is also safe).
    pub fn begin_edit(&mut self, store: &dyn PayeeStore) {
        let Some(alias) = self.selected_alias(store) else {
            return;
        };
        if alias.source != AliasSource::Manual {
            return;
        }
        let (input, mode) = decompile_pattern(&alias.pattern);
        self.compose = Some(Compose {
            input,
            mode,
            kind: ComposeKind::Edit(alias.id),
        });
    }

    pub fn cancel_compose(&mut self) {
        self.compose = None;
    }

    pub fn push_char(&mut self, c: char) {
        if let Some(compose) = &mut self.compose {
            compose.input.push(c);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(compose) = &mut self.compose {
            compose.input.pop();
        }
    }

    pub fn toggle_mode(&mut self) {
        if let Some(compose) = &mut self.compose {
            compose.mode = match compose.mode {
                AliasMode::ExactText => AliasMode::Regex,
                AliasMode::Regex => AliasMode::ExactText,
            };
        }
    }

    /// What `Ctrl+S` would do — `None` while composing nothing, composing a pure
    /// [`ComposeKind::Test`], the typed text is blank, or the compiled pattern collides with
    /// another Payee (checked via [`PayeeStore::conflicting_holder`] — the same check
    /// [`Self::render`] shows inline, so a draft that renders clean always validates here
    /// too).
    pub fn commit(&self, store: &dyn PayeeStore) -> Option<ComposeCommit> {
        let compose = self.compose.as_ref()?;
        if compose.kind == ComposeKind::Test {
            return None;
        }
        let typed = compose.input.trim();
        if typed.is_empty() {
            return None;
        }
        let pattern = compiled_pattern(typed, compose.mode);
        if store.conflicting_holder(self.payee_id, &pattern).is_some() {
            return None;
        }
        match compose.kind {
            ComposeKind::Add => Some(ComposeCommit::Add {
                payee_id: self.payee_id,
                typed: typed.to_string(),
                mode: compose.mode,
            }),
            ComposeKind::Edit(old_alias_id) => Some(ComposeCommit::Replace {
                old_alias_id,
                payee_id: self.payee_id,
                typed: typed.to_string(),
                mode: compose.mode,
            }),
            ComposeKind::Test => unreachable!("checked above"),
        }
    }

    /// Renders the floating overlay, centred and fixed-height, within `area`.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, store: &dyn PayeeStore) {
        let Some(payee) = store.find(self.payee_id) else {
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
                Constraint::Length(1),                    // title
                Constraint::Length(1),                    // subtitle
                Constraint::Length(1),                    // rule
                Constraint::Length(1),                    // column header
                Constraint::Length(MAX_LIST_ROWS as u16), // list
                Constraint::Length(1),                    // blank spacer
                Constraint::Length(1),                    // compose: add/edit/test label
                Constraint::Length(1),                    // compose: as
                Constraint::Length(1),                    // compose: stores
                Constraint::Length(1),                    // compose: test
                Constraint::Length(1),                    // blank spacer
                Constraint::Length(1),                    // resolve order line 1
                Constraint::Length(1),                    // resolve order line 2
                Constraint::Length(1),                    // blank spacer
                Constraint::Length(1),                    // conflict line 1
                Constraint::Length(1),                    // conflict line 2
                Constraint::Length(1),                    // blank spacer
                Constraint::Length(1),                    // protected-match note
                Constraint::Length(1),                    // rule
                Constraint::Length(1),                    // footer hints
            ])
            .split(inner);

        render_title(frame, rows[0], &payee.name);
        frame.render_widget(
            Paragraph::new(Span::styled(
                "typed text that resolves to this payee",
                dim(),
            )),
            rows[1],
        );
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[2]);
        render_column_header(frame, rows[3]);
        self.render_list(frame, rows[4], store);
        // rows[5] is left blank — breathing space above the compose block.
        self.render_compose(frame, [rows[6], rows[7], rows[8], rows[9]], store);
        // rows[10] is left blank — breathing space above the resolve-order note.
        frame.render_widget(
            Paragraph::new("1 an exact payee name   2 these matches"),
            rows[11],
        );
        frame.render_widget(
            Paragraph::new("3 create a new payee from what was typed"),
            rows[12],
        );
        // rows[13] is left blank — breathing space above the conflict block.
        render_conflict_block(frame, [rows[14], rows[15]], self.payee_id, store);
        // rows[16] is left blank — breathing space above the protected-match note.
        frame.render_widget(
            Paragraph::new(Span::styled(
                "a rename match is protected — remove and edit both refuse it",
                dim(),
            )),
            rows[17],
        );
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[18]);
        render_footer_hints(frame, rows[19], self.compose.as_ref().map(|c| c.kind));
    }

    fn render_list(&self, frame: &mut Frame<'_>, area: Rect, store: &dyn PayeeStore) {
        let aliases = store.aliases(self.payee_id);
        let shown = aliases.len().min(MAX_LIST_ROWS);
        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); MAX_LIST_ROWS])
            .split(area);

        if aliases.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled("no matches yet", dim())),
                row_areas[0],
            );
            return;
        }

        for (index, alias) in aliases[..shown].iter().enumerate() {
            let is_selected = index == self.selected;
            let row_area = row_areas[index];
            if is_selected {
                frame.render_widget(
                    Block::new().style(Style::default().add_modifier(Modifier::REVERSED)),
                    row_area,
                );
            }

            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(PATTERN_COL_WIDTH),
                    Constraint::Length(FROM_COL_WIDTH),
                    Constraint::Length(HITS_COL_WIDTH),
                ])
                .spacing(1)
                .split(row_area);

            frame.render_widget(Paragraph::new(alias.pattern.clone()), columns[0]);
            let from_text = match alias.source {
                AliasSource::Rename => "rename",
                AliasSource::Manual => "manual",
            };
            frame.render_widget(Paragraph::new(from_text), columns[1]);
            frame.render_widget(
                Paragraph::new(alias.hits.to_string()).alignment(Alignment::Right),
                columns[2],
            );
        }
    }

    fn render_compose(&self, frame: &mut Frame<'_>, rows: [Rect; 4], store: &dyn PayeeStore) {
        let Some(compose) = &self.compose else {
            render_compose_hint(frame, rows[0]);
            return;
        };

        let label = match compose.kind {
            ComposeKind::Add => "add",
            ComposeKind::Edit(_) => "edit",
            ComposeKind::Test => "test",
        };
        render_compose_field(
            frame,
            rows[0],
            label,
            Line::from(vec![
                Span::raw(compose.input.clone()),
                Span::styled("\u{258c}", Style::default().fg(ACCENT)),
            ]),
        );

        let (exact_marker, regex_marker) = match compose.mode {
            AliasMode::ExactText => ("(\u{2022})", "( )"),
            AliasMode::Regex => ("( )", "(\u{2022})"),
        };
        render_compose_field(
            frame,
            rows[1],
            "as",
            Line::from(format!("{exact_marker} exact text   {regex_marker} regex")),
        );

        let pattern = compiled_pattern(compose.input.trim(), compose.mode);
        let mut stores_line = vec![Span::raw(pattern.clone())];
        if let Some(other) = store.conflicting_holder(self.payee_id, &pattern) {
            stores_line.push(Span::styled(
                format!(" · also matches {other}"),
                Style::default().fg(ACCENT),
            ));
        }
        render_compose_field(frame, rows[2], "stores", Line::from(stores_line));

        let resolution = store.resolve(compose.input.trim());
        let test_text = format!(
            "\"{}\" \u{2192} {}",
            compose.input.trim(),
            describe_resolution(resolution, store)
        );
        render_compose_field(frame, rows[3], "test", Line::from(test_text));
    }
}

/// Shown in the compose block's first row while nothing is being composed — the footer hints
/// already state `a`/`e`/`t`, so this just points at the block itself being where they land.
fn render_compose_hint(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new(Span::styled("(a/e/t opens this box)", dim())),
        area,
    );
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

fn render_title(frame: &mut Frame<'_>, area: Rect, payee_name: &str) {
    let tag = ":payee match";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(format!("matches {payee_name}")), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim())).alignment(Alignment::Right),
        columns[1],
    );
}

fn render_column_header(frame: &mut Frame<'_>, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(PATTERN_COL_WIDTH),
            Constraint::Length(FROM_COL_WIDTH),
            Constraint::Length(HITS_COL_WIDTH),
        ])
        .spacing(1)
        .split(area);
    let style = dim();
    frame.render_widget(Paragraph::new(Span::styled("PATTERN", style)), columns[0]);
    frame.render_widget(Paragraph::new(Span::styled("FROM", style)), columns[1]);
    frame.render_widget(
        Paragraph::new(Span::styled("HITS", style)).alignment(Alignment::Right),
        columns[2],
    );
}

/// One `label   value` compose row — mirrors `popup::account::new::render_field`.
fn render_compose_field(frame: &mut Frame<'_>, area: Rect, label: &str, value: Line<'static>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled(label, dim())), columns[0]);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

/// The two-line conflict block — blank when `payee_id` has no [`PayeeStore::conflict_partners`]
/// right now.
fn render_conflict_block(
    frame: &mut Frame<'_>,
    rows: [Rect; 2],
    payee_id: RowID,
    store: &dyn PayeeStore,
) {
    let partners = store.conflict_partners(payee_id);
    if partners.is_empty() {
        return;
    }
    let names: Vec<String> = partners
        .iter()
        .filter_map(|id| store.find(*id))
        .map(|payee| payee.name.clone())
        .collect();
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("! also matches {}", names.join(", ")),
            Style::default().fg(ACCENT),
        )),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            "two payees matching one text resolves arbitrarily",
            dim(),
        )),
        rows[1],
    );
}

fn render_footer_hints(frame: &mut Frame<'_>, area: Rect, composing: Option<ComposeKind>) {
    let hints: &[(&str, &str)] = match composing {
        None => &[
            ("j/k", "match"),
            ("a", "add"),
            ("e", "edit"),
            ("d", "remove"),
            ("t", "test"),
            ("esc", "close"),
        ],
        Some(ComposeKind::Test) => &[("tab", "as"), ("esc", "cancel")],
        Some(_) => &[("tab", "as"), ("^s", "save"), ("esc", "cancel")],
    };
    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let label_style = dim();

    let mut spans = Vec::with_capacity(hints.len() * 3);
    for (index, (key, label)) in hints.iter().enumerate() {
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

/// Compiles typed text into a stored pattern exactly as `PayeeStore::add_alias`/
/// `PayeeStore::rename` do: `exact text` escapes and anchors it, `regex` passes it through
/// verbatim.
fn compiled_pattern(typed: &str, mode: AliasMode) -> String {
    match mode {
        AliasMode::ExactText => format!("(?i)^{}$", regex::escape(typed)),
        AliasMode::Regex => typed.to_string(),
    }
}

/// Best-effort reverses [`compiled_pattern`]'s own `ExactText` wrapping back into typed text —
/// used to preload [`PayeeMatchesPopup::begin_edit`]'s compose slot from a stored pattern.
/// Only trusts the reversal when re-escaping it reproduces the original pattern *exactly*;
/// anything else (a hand-authored regex that merely happens to start `(?i)^` and end `$`)
/// falls back to `regex` mode with the pattern shown verbatim, which is always safe to save
/// unchanged.
fn decompile_pattern(pattern: &str) -> (String, AliasMode) {
    if let Some(inner) = pattern
        .strip_prefix("(?i)^")
        .and_then(|s| s.strip_suffix('$'))
    {
        let unescaped = unescape_regex_literal(inner);
        if compiled_pattern(&unescaped, AliasMode::ExactText) == pattern {
            return (unescaped, AliasMode::ExactText);
        }
    }
    (pattern.to_string(), AliasMode::Regex)
}

/// Removes the backslash before every escaped character — the inverse of `regex::escape`,
/// which only ever inserts single backslashes before meta characters, never multi-character
/// escapes.
fn unescape_regex_literal(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                result.push(next);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Names the Payee `resolution` lands on, with a trailing `✓` — mirrors the handoff's own
/// `"ww metro" → Woolworths ✓` worked example.
fn describe_resolution(resolution: PayeeResolution, store: &dyn PayeeStore) -> String {
    let id = match resolution {
        PayeeResolution::ExactName(id) => id,
        PayeeResolution::Alias { payee_id, .. } => payee_id,
        PayeeResolution::WouldCreate => return "would create a new payee".to_string(),
    };
    match store.find(id) {
        Some(payee) => format!("{} \u{2713}", payee.name),
        None => "?".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::payee::PayeeFixture;

    fn find_id(store: &PayeeFixture, name: &str) -> RowID {
        store
            .payees()
            .iter()
            .find(|payee| payee.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a payee named {name}"))
            .id
    }

    fn render(popup: &PayeeMatchesPopup, store: &dyn PayeeStore) -> String {
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
    fn renders_without_panicking() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        render(&PayeeMatchesPopup::new(woolworths), &store);
    }

    #[test]
    fn move_up_and_down_wrap_within_the_payees_own_aliases() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = PayeeMatchesPopup::new(woolworths);
        assert_eq!(store.aliases(woolworths).len(), 2);

        popup.move_up(&store);
        assert_eq!(popup.selected, 1);
        popup.move_down(&store);
        assert_eq!(popup.selected, 0);
        popup.move_down(&store);
        popup.move_down(&store);
        assert_eq!(popup.selected, 0);
    }

    #[test]
    fn begin_edit_refuses_on_a_rename_sourced_alias() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = PayeeMatchesPopup::new(woolworths);
        // Index 0 is the seeded rename alias (added first in the fixture).
        popup.selected = 0;
        let rename_alias = popup.selected_alias(&store).unwrap();
        assert_eq!(rename_alias.source, AliasSource::Rename);

        popup.begin_edit(&store);
        assert!(
            !popup.is_composing(),
            "editing a rename alias should refuse"
        );
    }

    #[test]
    fn begin_edit_on_a_manual_alias_decompiles_its_exact_text_pattern() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = PayeeMatchesPopup::new(woolworths);
        popup.selected = 1; // the seeded manual "(?i)^WOOLIES$" alias
        let manual_alias = popup.selected_alias(&store).unwrap();
        assert_eq!(manual_alias.source, AliasSource::Manual);
        assert_eq!(manual_alias.pattern, "(?i)^WOOLIES$");

        popup.begin_edit(&store);
        assert!(popup.is_composing());
        let compose = popup.compose.as_ref().unwrap();
        assert_eq!(compose.input, "WOOLIES");
        assert_eq!(compose.mode, AliasMode::ExactText);
        assert_eq!(compose.kind, ComposeKind::Edit(manual_alias.id));
    }

    #[test]
    fn decompile_pattern_falls_back_to_regex_mode_for_a_hand_authored_regex() {
        let (input, mode) = decompile_pattern("(?i)^WOOL.*$");
        assert_eq!(input, "(?i)^WOOL.*$");
        assert_eq!(mode, AliasMode::Regex);
    }

    #[test]
    fn exact_text_mode_escapes_and_anchors_without_a_backslash_for_a_space() {
        let store = PayeeFixture::new();
        let coles_central = find_id(&store, "Coles Central");
        let mut popup = PayeeMatchesPopup::new(coles_central);
        popup.begin_add();
        for c in "WW Metro".chars() {
            popup.push_char(c);
        }

        let text = render(&popup, &store);
        assert!(text.contains("(?i)^WW Metro$"));
    }

    #[test]
    fn regex_mode_passes_the_typed_text_through_verbatim() {
        let store = PayeeFixture::new();
        let coles_central = find_id(&store, "Coles Central");
        let mut popup = PayeeMatchesPopup::new(coles_central);
        popup.begin_add();
        popup.toggle_mode();
        for c in "^WW.*$".chars() {
            popup.push_char(c);
        }

        let text = render(&popup, &store);
        assert!(text.contains("stores"));
        assert!(text.contains("^WW.*$"));
        // Verbatim, not re-escaped — no `(?i)` wrapper added.
        assert!(!text.contains("(?i)^^WW.*$$"));
    }

    #[test]
    fn test_resolves_through_the_real_resolution_order() {
        let store = PayeeFixture::new();
        let coles_central = find_id(&store, "Coles Central");
        let mut popup = PayeeMatchesPopup::new(coles_central);
        popup.begin_test();
        for c in "Bank Direct Debit".chars() {
            popup.push_char(c);
        }

        let text = render(&popup, &store);
        assert!(text.contains("Home Loan Direct \u{2713}"));
    }

    #[test]
    fn commit_refuses_a_pattern_colliding_with_another_payees_name() {
        let store = PayeeFixture::new();
        let coles_central = find_id(&store, "Coles Central");
        let mut popup = PayeeMatchesPopup::new(coles_central);
        popup.begin_add();
        for c in "Woolworths".chars() {
            popup.push_char(c);
        }

        assert!(popup.commit(&store).is_none());
        let text = render(&popup, &store);
        assert!(text.contains("also matches Woolworths"));
    }

    #[test]
    fn commit_returns_add_fields_for_a_fresh_non_colliding_pattern() {
        let store = PayeeFixture::new();
        let coles_central = find_id(&store, "Coles Central");
        let mut popup = PayeeMatchesPopup::new(coles_central);
        popup.begin_add();
        for c in "Coles Supermarket".chars() {
            popup.push_char(c);
        }

        match popup.commit(&store) {
            Some(ComposeCommit::Add {
                payee_id,
                typed,
                mode,
            }) => {
                assert_eq!(payee_id, coles_central);
                assert_eq!(typed, "Coles Supermarket");
                assert_eq!(mode, AliasMode::ExactText);
            }
            _ => panic!("expected a valid Add commit"),
        }
    }

    #[test]
    fn commit_returns_replace_fields_when_editing_an_existing_manual_alias() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = PayeeMatchesPopup::new(woolworths);
        popup.selected = 1; // the seeded manual alias
        let manual_alias_id = popup.selected_alias(&store).unwrap().id;
        popup.begin_edit(&store);
        popup.push_char('S');

        match popup.commit(&store) {
            Some(ComposeCommit::Replace {
                old_alias_id,
                payee_id,
                typed,
                ..
            }) => {
                assert_eq!(old_alias_id, manual_alias_id);
                assert_eq!(payee_id, woolworths);
                assert_eq!(typed, "WOOLIESS");
            }
            _ => panic!("expected a valid Replace commit"),
        }
    }

    #[test]
    fn commit_is_always_none_while_testing() {
        let store = PayeeFixture::new();
        let coles_central = find_id(&store, "Coles Central");
        let mut popup = PayeeMatchesPopup::new(coles_central);
        popup.begin_test();
        popup.push_char('x');
        assert!(popup.commit(&store).is_none());
    }

    #[test]
    fn commit_is_none_while_the_typed_text_is_blank() {
        let store = PayeeFixture::new();
        let coles_central = find_id(&store, "Coles Central");
        let mut popup = PayeeMatchesPopup::new(coles_central);
        popup.begin_add();
        assert!(popup.commit(&store).is_none());
    }

    #[test]
    fn conflict_block_names_the_woolworths_woolies_pair() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let popup = PayeeMatchesPopup::new(woolworths);
        let text = render(&popup, &store);
        assert!(text.contains("! also matches WOOLIES"));
        assert!(text.contains("resolves arbitrarily"));
    }

    #[test]
    fn no_conflict_block_for_an_unrelated_payee() {
        let store = PayeeFixture::new();
        let sunrise_payroll = find_id(&store, "Sunrise Payroll");
        let popup = PayeeMatchesPopup::new(sunrise_payroll);
        let text = render(&popup, &store);
        assert!(!text.contains("also matches"));
    }

    #[test]
    fn shows_the_resolve_order_and_protected_match_notes() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let popup = PayeeMatchesPopup::new(woolworths);
        let text = render(&popup, &store);

        assert!(text.contains("1 an exact payee name"));
        assert!(text.contains("3 create a new payee from what was typed"));
        assert!(text.contains("a rename match is protected"));
    }

    #[test]
    fn footer_hints_change_between_idle_and_composing() {
        let store = PayeeFixture::new();
        let woolworths = find_id(&store, "Woolworths");
        let mut popup = PayeeMatchesPopup::new(woolworths);

        let idle_text = render(&popup, &store);
        assert!(idle_text.contains("remove"));

        popup.begin_add();
        let composing_text = render(&popup, &store);
        assert!(composing_text.contains("save"));

        popup.cancel_compose();
        popup.begin_test();
        let testing_text = render(&popup, &store);
        assert!(!testing_text.contains("save"));
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }
}
