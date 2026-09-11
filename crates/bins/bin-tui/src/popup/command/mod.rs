//! The command popup — the shell's own floating command window (`docs/ux/tui/README.md` §3a,
//! where it's specified as the "command palette"), opened with `Ctrl+;` from anywhere.
//! `view/mod.rs` calls this out as later work over the `View` trait: the popup overlays
//! whatever `View` is active rather than being one itself, so `Shell` owns it directly instead
//! of hosting it through `View`. It was the first tenant of `crate::popup`; `popup::unit::new`
//! (`docs/ux/tui/units/README.md` §4b) now shares its `Dim` overlay treatment and
//! `REFERENCE_TERMINAL_WIDTH` baseline.
//!
//! The command list itself (`commands`) is real, grouped by domain, each with a fixed
//! argument-preview (`Command::args`) rendered here as the popup's one dynamic info row
//! (`info_row`) — `:help`/footer/keymap generation and real key dispatch beyond the 6
//! commands with content behind them (`unit`, `unit new/edit/delete`, `dashboard`,
//! `settings`) are still later work, built out domain by domain as those areas land on the
//! new Shell/View navigation.

mod commands;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::popup::REFERENCE_TERMINAL_WIDTH;

/// Dim colour for hint text and secondary detail, matching `ACCENT`'s siblings in
/// `view/dashboard.rs`'s style table.
const DIM: Color = Color::DarkGray;

/// A darker grey than `DIM` for the footer hint row's labels (`select`, `complete`, …) — an
/// explicit RGB value rather than a named/indexed colour, since `DIM`'s `Color::DarkGray`
/// (like `Color::Black` before it) renders however the user's terminal theme happens to remap
/// that palette slot, which isn't reliably "dark" on every theme.
const FOOTER_LABEL: Color = Color::Rgb(90, 90, 90);

/// Fraction of `REFERENCE_TERMINAL_WIDTH` the popup takes — back to §3a's own suggested ~78%
/// after a narrower value read too cramped in practice.
const POPUP_WIDTH_PERCENT: u32 = 78;

/// The popup's fixed width in terminal cells: computed once against the reference width
/// above, not recomputed from the live terminal on every resize, so a wider terminal no
/// longer grows the popup — only a narrower one shrinks it (`popup_rect`).
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;

/// Blank cells between the `:command`, `<binding>` and description columns.
const COLUMN_GAP: usize = 2;

/// Fixed chrome rows inside the border: prompt, its rule, the footer's rule, the footer
/// itself. The body between them is whatever's left (`render`), scrolling past it. Does not
/// include the info row (`info_row`) — that's an extra row on top, present only while it has
/// something to show.
const FIXED_ROWS: u16 = 4;

/// One row of the resting/filtered body: a domain header (resting state only, per the user's
/// own ask) or a command entry.
enum Row {
    Header(&'static str),
    Entry {
        domain: &'static str,
        command: &'static commands::Command,
    },
}

/// State for the floating command popup: its input buffer and candidate selection.
/// `Shell` holds this as `Option<CommandPopup>` — `Some` while the window is open.
#[derive(Default)]
pub struct CommandPopup {
    input: String,
    /// Index into the *selectable* rows only (headers are skipped), not the row list's own
    /// index — so it stays meaningful whether or not headers are showing.
    selected: usize,
    /// The `:name` of the last command `Enter` tried to run outside the 6 with real content
    /// behind them — `Some` while its `":{name} — not yet built"` message shows in the info
    /// row. Cleared by any subsequent mutating action (typing, backspace, `↑`/`↓`, `Tab`);
    /// takes priority over the argument-preview row while it's set, since it only ever
    /// appears right after an `Enter` attempt on the currently-highlighted command.
    not_yet_built: Option<&'static str>,
}

impl CommandPopup {
    /// Opens a fresh popup with an empty input buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a typed character to the input buffer, resetting the selection to the top —
    /// each keystroke changes the filtered set, so the old index may no longer point at
    /// anything sensible.
    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
        self.selected = 0;
        self.not_yet_built = None;
    }

    /// Removes the last character of the input buffer, if any, resetting the selection.
    pub fn backspace(&mut self) {
        self.input.pop();
        self.selected = 0;
        self.not_yet_built = None;
    }

    /// Moves the selection up one row, clamped at the top.
    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.not_yet_built = None;
    }

    /// Moves the selection down one row, clamped at the bottom of whatever's currently shown.
    pub fn move_down(&mut self) {
        let count = self.selectable_count();
        if count > 0 && self.selected + 1 < count {
            self.selected += 1;
        }
        self.not_yet_built = None;
    }

    /// `Tab` while the popup is open — completion isn't built yet, so this only clears a
    /// showing "not yet built" message, matching every other mutating key.
    pub fn tab(&mut self) {
        self.not_yet_built = None;
    }

    /// Records that `Enter` was pressed on `name`, a command outside the 6 with real content
    /// behind them — `render` shows `":{name} — not yet built"` in the info row until the
    /// next mutating key clears it.
    pub fn set_not_yet_built(&mut self, name: &'static str) {
        self.not_yet_built = Some(name);
    }

    /// The command currently highlighted by `selected`, if any.
    fn selected_command(&self) -> Option<&'static commands::Command> {
        let rows = self.rows();
        let row_idx = Self::selected_row_index(&rows, self.selected);
        match rows.get(row_idx)? {
            Row::Entry { command, .. } => Some(command),
            Row::Header(_) => None,
        }
    }

    /// The `:name` of the command currently highlighted by `selected`, if any — `Shell` checks
    /// this against `Enter` to decide whether the highlighted command has a real view to open
    /// yet (today, only the unit family and `dashboard` do).
    pub fn selected_command_name(&self) -> Option<&'static str> {
        self.selected_command().map(|command| command.name)
    }

    /// The highlighted command's args, joined into one combined preview string (`"{placeholder}
    /// — {preview}"` per arg, `" · "`-separated) — `None` for a command with no args, per
    /// `Command::args`.
    fn arg_preview(&self) -> Option<String> {
        let command = self.selected_command()?;
        if command.args.is_empty() {
            return None;
        }
        Some(
            command
                .args
                .iter()
                .map(|arg| format!("{} — {}", arg.placeholder, arg.preview))
                .collect::<Vec<_>>()
                .join(" · "),
        )
    }

    /// The popup's one dynamic info-row slot: the "not yet built" message if `Enter` was just
    /// pressed on a not-yet-dispatched command, otherwise the highlighted command's argument
    /// preview, otherwise nothing — `bool` is whether it's the "not yet built" message (styled
    /// plainly) rather than an argument preview (styled dim). `None` means the row doesn't
    /// render at all, and the popup is one row shorter.
    pub(crate) fn info_row(&self) -> Option<(String, bool)> {
        if let Some(name) = self.not_yet_built {
            return Some((format!(":{name} — not yet built"), true));
        }
        self.arg_preview().map(|preview| (preview, false))
    }

    /// Every command matching the current input, case-insensitively against its `:name`,
    /// description or owning domain — ranked so a `:name` that exactly matches or starts with
    /// the input leads (e.g. typing `category` should surface the bare `:category` command
    /// before `budget new <category> <limit>`, whose name only contains it mid-string),
    /// falling back to `DOMAINS` order for ties.
    fn filtered(&self) -> Vec<(&'static str, &'static commands::Command)> {
        let needle = self.input.to_lowercase();
        let mut matches: Vec<_> = commands::all()
            .filter(|(domain, command)| {
                command.name.to_lowercase().contains(&needle)
                    || command.description.to_lowercase().contains(&needle)
                    || domain.to_lowercase().contains(&needle)
            })
            .collect();
        matches.sort_by_key(|(_, command)| name_match_rank(&command.name.to_lowercase(), &needle));
        matches
    }

    /// How many rows are actually selectable right now — every command at rest, or only the
    /// matches while filtering.
    fn selectable_count(&self) -> usize {
        if self.input.is_empty() {
            commands::total_commands()
        } else {
            self.filtered().len()
        }
    }

    /// The rows to render: every domain with a `Header` at rest (the user's own ask, so
    /// browsing stays organised), or a flat, header-less list of matches while filtering —
    /// once you're typing you already know what you want, so ranking beats grouping.
    fn rows(&self) -> Vec<Row> {
        if self.input.is_empty() {
            let mut rows = Vec::with_capacity(commands::total_commands() + commands::DOMAINS.len());
            for domain in commands::DOMAINS {
                rows.push(Row::Header(domain.name));
                for command in domain.commands {
                    rows.push(Row::Entry {
                        domain: domain.name,
                        command,
                    });
                }
            }
            rows
        } else {
            self.filtered()
                .into_iter()
                .map(|(domain, command)| Row::Entry { domain, command })
                .collect()
        }
    }

    /// The absolute index into `rows` of the `self.selected`-th selectable entry — the row
    /// list mixes headers and entries, but `selected` only counts entries.
    fn selected_row_index(rows: &[Row], selected: usize) -> usize {
        let mut entries = 0;
        for (row_idx, row) in rows.iter().enumerate() {
            if let Row::Entry { .. } = row {
                if entries == selected {
                    return row_idx;
                }
                entries += 1;
            }
        }
        0
    }

    /// How far the body has scrolled — just enough to keep the selected row in view, computed
    /// fresh from `rows`/`body_height` rather than tracked as its own field, so it can never
    /// drift out of sync with `selected`. Shared by `render_body` and `render_scrollbar` so
    /// the list and its scrollbar thumb always agree.
    fn scroll_offset(&self, rows: &[Row], body_height: u16) -> usize {
        let body_height = body_height as usize;
        if body_height == 0 {
            return 0;
        }
        let selected_row = Self::selected_row_index(rows, self.selected);
        let max_offset = rows.len().saturating_sub(body_height);
        selected_row
            .saturating_sub(body_height.saturating_sub(1))
            .min(max_offset)
    }

    /// Renders the floating overlay, centred and content-sized up to a terminal-height cap
    /// (mirroring the `:help` window's own §2a scrolling convention), within `area` (the full
    /// terminal area — the popup floats over the shell's status line and footer too, not
    /// just the view region, per §3a's "centred floating overlay"). The info row (`info_row`)
    /// adds exactly one row when present — a zero-arg command with no "not yet built" message
    /// showing renders no info row at all, and the popup is correspondingly one row shorter.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let rows = self.rows();
        let info_row = self.info_row();
        let extra_row: u16 = if info_row.is_some() { 1 } else { 0 };
        let popup = popup_rect(
            area,
            rows.len() as u16 + FIXED_ROWS + extra_row + 2, // + 2 borders
        );

        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let body_height = inner.height.saturating_sub(FIXED_ROWS + extra_row);

        let mut constraints = vec![
            Constraint::Length(1), // prompt
            Constraint::Length(1), // rule
            Constraint::Length(body_height),
        ];
        if info_row.is_some() {
            constraints.push(Constraint::Length(1)); // info row
        }
        constraints.push(Constraint::Length(1)); // rule
        constraints.push(Constraint::Length(1)); // footer

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let scroll_offset = self.scroll_offset(&rows, layout[2].height);

        let rule = || Line::from("─".repeat(inner.width as usize));
        frame.render_widget(self.prompt_line(inner.width), layout[0]);
        frame.render_widget(rule(), layout[1]);
        self.render_body(frame, layout[2], &rows, scroll_offset);
        render_scrollbar(frame, popup, layout[2], rows.len(), scroll_offset);

        let mut next = 3;
        if let Some((text, is_message)) = &info_row {
            self.render_info_row(frame, layout[next], text, *is_message);
            next += 1;
        }
        frame.render_widget(rule(), layout[next]);
        next += 1;
        frame.render_widget(footer_hint_line(), layout[next]);
    }

    /// The info row itself: the "not yet built" message renders plainly, the argument preview
    /// dim (matching the footer hint labels' own dim treatment) so it reads as secondary to
    /// the candidate list above it.
    fn render_info_row(&self, frame: &mut Frame, area: Rect, text: &str, is_message: bool) {
        let style = if is_message {
            Style::default()
        } else {
            Style::default().fg(DIM)
        };
        frame.render_widget(Line::from(pad_line(text, area.width)).style(style), area);
    }

    /// The prompt row: `:{input}▌` flush left, match count right-aligned — `:` rather than
    /// §3a's own `>`, so the prompt itself signals that what's typed is a `:command`.
    fn prompt_line(&self, width: u16) -> Line<'static> {
        let left = format!(":{}▌", self.input);
        let right = format!(
            "{} of {}",
            self.selectable_count(),
            commands::total_commands()
        );
        Line::from(pad_between(&left, &right, width))
    }

    /// The scrollable body: domain headers (resting state only) and `:command  <binding>
    /// description` rows, the selected row a full-width reversed block per §3a.
    fn render_body(&self, frame: &mut Frame, area: Rect, rows: &[Row], scroll_offset: usize) {
        let body_height = area.height as usize;
        if body_height == 0 || rows.is_empty() {
            return;
        }

        let selected_row = Self::selected_row_index(rows, self.selected);
        let visible = &rows[scroll_offset..(scroll_offset + body_height).min(rows.len())];
        let line_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); visible.len()])
            .split(area);

        let command_col = command_column_width();
        let binding_col = binding_column_width();

        for (idx, row) in visible.iter().enumerate() {
            match row {
                Row::Header(name) => {
                    let text = pad_line(&name.to_uppercase(), line_rows[idx].width);
                    frame.render_widget(
                        Line::from(text)
                            .style(Style::default().add_modifier(Modifier::BOLD).fg(DIM)),
                        line_rows[idx],
                    );
                }
                Row::Entry { command, .. } => {
                    let row_idx = scroll_offset + idx;
                    let text = format!(
                        ":{command:<command_col$}{binding:<binding_col$}{description}",
                        command = command.name,
                        binding = command.chord.to_string(),
                        description = command.description,
                    );
                    let text = pad_line(&text, line_rows[idx].width);
                    let style = if row_idx == selected_row {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    };
                    frame.render_widget(Line::from(text).style(style), line_rows[idx]);
                }
            }
        }
    }
}

/// Ranks a (lowercased) command name against a (lowercased) needle for `CommandPopup::filtered`:
/// `0` for an exact match, `1` for a name that starts with the needle, `2` for everything else
/// (a match found only mid-name, or only in the description/domain). Lower sorts first.
fn name_match_rank(name: &str, needle: &str) -> u8 {
    if name == needle {
        0
    } else if name.starts_with(needle) {
        1
    } else {
        2
    }
}

/// Width of the `:command` column: the longest command name (plus its leading `:`), plus a
/// gap, across every domain — not just what's currently visible, so the columns don't jiggle
/// as the list scrolls or filters.
fn command_column_width() -> usize {
    commands::all()
        .map(|(_, command)| command.name.len() + 1)
        .max()
        .unwrap_or(0)
        + COLUMN_GAP
}

/// Width of the `<binding>` column: the longest rendered chord, plus a gap.
fn binding_column_width() -> usize {
    commands::all()
        .map(|(_, command)| command.chord.to_string().chars().count())
        .max()
        .unwrap_or(0)
        + COLUMN_GAP
}

/// The window footer hint row. A background fill turned out to depend on how the user's own
/// terminal theme remaps indexed colours — it rendered invisibly there even though the cells
/// carried the right SGR codes — so the rule `render` draws above this row is what actually
/// separates it from the candidate list; each key is bold instead, which doesn't depend on
/// the palette.
fn footer_hint_line() -> Line<'static> {
    use ratatui::text::Span;

    const HINTS: &[(&str, &str)] = &[
        ("↑↓", "select"),
        ("tab", "complete"),
        ("enter", "run"),
        ("^r", "history"),
        ("esc", "close"),
    ];

    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(FOOTER_LABEL);

    let mut spans = Vec::with_capacity(HINTS.len() * 3);
    for (idx, (key, label)) in HINTS.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(*key, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*label, label_style));
    }
    Line::from(spans)
}

/// Pads `left` and `right` onto one line of exactly `width` cells, `right` flush to the far
/// edge. Truncates `left` rather than overflow if the two don't fit.
fn pad_between(left: &str, right: &str, width: u16) -> String {
    let width = width as usize;
    let right_len = right.chars().count();
    let left_budget = width.saturating_sub(right_len);
    let left: String = left.chars().take(left_budget).collect();
    format!("{left:<left_budget$}{right}", left_budget = left_budget)
}

/// Pads or truncates `text` to exactly `width` cells, so a reversed selection style covers the
/// whole row rather than just its glyphs.
fn pad_line(text: &str, width: u16) -> String {
    let width = width as usize;
    let truncated: String = text.chars().take(width).collect();
    format!("{truncated:<width$}")
}

/// Fraction of the space below the anchor the popup may fill at most — down from filling
/// nearly all of it, so the full resting-state list scrolls well short of dominating the
/// screen (`render_body`'s scrollbar is exactly for this).
const MAX_HEIGHT_NUMERATOR: u16 = 2;
const MAX_HEIGHT_DENOMINATOR: u16 = 5;

/// A few extra rows on top of the `MAX_HEIGHT_NUMERATOR`/`MAX_HEIGHT_DENOMINATOR` cap — plain
/// rows, not a fraction, so the cap grows by a fixed, predictable amount regardless of
/// terminal size.
const MAX_HEIGHT_EXTRA_ROWS: u16 = 2;

/// Computes a centred popup `Rect` of the given content height, anchored in `area`'s top
/// third. Width is `POPUP_WIDTH`, fixed regardless of terminal size — it does not grow on a
/// wider terminal — except when `area` itself is narrower, where it shrinks to fit rather
/// than overflow. Height is capped to `MAX_HEIGHT_NUMERATOR`/`MAX_HEIGHT_DENOMINATOR` of the
/// space below the anchor plus `MAX_HEIGHT_EXTRA_ROWS`, so a long resting-state list scrolls
/// rather than overflowing — short content (a tight filter match) still shrinks below that
/// cap rather than padding out to it.
fn popup_rect(area: Rect, height: u16) -> Rect {
    let width = POPUP_WIDTH.min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;

    let y = area.y + (area.height / 3).max(1);
    let available = area.height.saturating_sub(y - area.y);
    let max_height = ((available * MAX_HEIGHT_NUMERATOR / MAX_HEIGHT_DENOMINATOR).max(1)
        + MAX_HEIGHT_EXTRA_ROWS)
        .min(available);
    let height = height.min(max_height);

    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Draws a scroll thumb/track on the popup's right border column, spanning `body`'s rows only
/// (not the prompt/rule/footer rows above and below it) — so people can tell the list is
/// bigger than the floating window. Hidden when everything already fits, so a short filtered
/// list doesn't grow a scrollbar it doesn't need.
fn render_scrollbar(
    frame: &mut Frame,
    popup: Rect,
    body: Rect,
    total_rows: usize,
    scroll_offset: usize,
) {
    let body_height = body.height as usize;
    if body_height == 0 || total_rows <= body_height {
        return;
    }

    let track = Rect {
        x: popup.right().saturating_sub(1),
        y: body.y,
        width: 1,
        height: body.height,
    };

    let mut state = ScrollbarState::new(total_rows)
        .viewport_content_length(body_height)
        .position(scroll_offset);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None);
    frame.render_stateful_widget(scrollbar, track, &mut state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_appends_to_the_input_buffer() {
        let mut popup = CommandPopup::new();
        popup.push_char('b');
        popup.push_char('u');
        popup.push_char('d');
        assert_eq!(popup.input, "bud");
    }

    #[test]
    fn backspace_removes_the_last_character() {
        let mut popup = CommandPopup::new();
        popup.push_char('b');
        popup.backspace();
        assert_eq!(popup.input, "");
    }

    #[test]
    fn backspace_on_empty_input_does_not_panic() {
        let mut popup = CommandPopup::new();
        popup.backspace();
        assert_eq!(popup.input, "");
    }

    #[test]
    fn selection_is_clamped_to_every_command_at_rest() {
        let mut popup = CommandPopup::new();
        popup.move_up();
        assert_eq!(popup.selected, 0);

        for _ in 0..commands::total_commands() + 5 {
            popup.move_down();
        }
        assert_eq!(popup.selected, commands::total_commands() - 1);
    }

    #[test]
    fn typing_filters_to_matching_commands_only() {
        let mut popup = CommandPopup::new();
        for c in "unit".chars() {
            popup.push_char(c);
        }
        let count = popup.selectable_count();
        assert!(count > 0);
        assert!(count < commands::total_commands());
    }

    #[test]
    fn typing_resets_the_selection() {
        let mut popup = CommandPopup::new();
        popup.move_down();
        popup.move_down();
        assert_eq!(popup.selected, 2);
        popup.push_char('u');
        assert_eq!(popup.selected, 0);
    }

    #[test]
    fn resting_rows_include_a_header_per_domain() {
        let popup = CommandPopup::new();
        let rows = popup.rows();
        let header_count = rows
            .iter()
            .filter(|row| matches!(row, Row::Header(_)))
            .count();
        assert_eq!(header_count, commands::DOMAINS.len());
    }

    #[test]
    fn filtered_rows_have_no_headers() {
        let mut popup = CommandPopup::new();
        popup.push_char('u');
        let rows = popup.rows();
        assert!(rows.iter().all(|row| matches!(row, Row::Entry { .. })));
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area, 11);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }

    #[test]
    fn popup_height_is_capped_well_short_of_the_available_space() {
        let area = Rect::new(0, 0, 96, 30);
        // A tall resting-state list should be capped, not fill the space below the anchor.
        let popup = popup_rect(area, 60);
        let y = area.y + (area.height / 3).max(1);
        let available = area.height - y;
        assert_eq!(popup.height, available * 2 / 5 + MAX_HEIGHT_EXTRA_ROWS);
    }

    #[test]
    fn a_short_list_stays_shorter_than_the_height_cap() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area, 6);
        assert_eq!(popup.height, 6);
    }

    #[test]
    fn popup_width_does_not_grow_with_a_wider_terminal() {
        let narrow = popup_rect(Rect::new(0, 0, 96, 30), 11);
        let wide = popup_rect(Rect::new(0, 0, 300, 30), 11);
        assert_eq!(narrow.width, POPUP_WIDTH);
        assert_eq!(wide.width, POPUP_WIDTH);
    }

    #[test]
    fn popup_width_shrinks_to_fit_a_terminal_narrower_than_the_fixed_width() {
        let area = Rect::new(0, 0, POPUP_WIDTH - 5, 30);
        let popup = popup_rect(area, 11);
        assert_eq!(popup.width, area.width);
    }

    #[test]
    fn renders_without_panicking_at_rest() {
        use ratatui::{Terminal, backend::TestBackend};

        let popup = CommandPopup::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area()))
            .expect("rendering the popup should not error");
    }

    #[test]
    fn renders_without_panicking_while_filtering() {
        use ratatui::{Terminal, backend::TestBackend};

        let mut popup = CommandPopup::new();
        popup.push_char('u');
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area()))
            .expect("rendering the filtered popup should not error");
    }

    #[test]
    fn renders_without_panicking_on_a_short_terminal() {
        use ratatui::{Terminal, backend::TestBackend};

        // Shorter than the full resting-state list — exercises the scrolling path.
        let popup = CommandPopup::new();
        let backend = TestBackend::new(96, 15);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area()))
            .expect("rendering a scrolled popup should not error");
    }

    /// Filters `popup` down to exactly one match.
    fn filter_to(popup: &mut CommandPopup, needle: &str) {
        for c in needle.chars() {
            popup.push_char(c);
        }
    }

    #[test]
    fn arg_preview_joins_every_arg_with_placeholder_and_dash() {
        let mut popup = CommandPopup::new();
        filter_to(&mut popup, "unit edit");
        assert_eq!(popup.selected_command_name(), Some("unit edit <code>"));
        assert_eq!(
            popup.arg_preview().as_deref(),
            Some("<code> — VDHG · etf · Vanguard Diversified High Growth")
        );
    }

    #[test]
    fn an_exact_name_match_outranks_a_command_that_only_contains_it_mid_name() {
        let mut popup = CommandPopup::new();
        filter_to(&mut popup, "category");
        // `budget new <category> <limit>` also matches (its name contains "category"), but the
        // bare `:category` command is an exact match and should lead regardless of domain order.
        assert_eq!(popup.selected_command_name(), Some("category"));
    }

    #[test]
    fn a_zero_arg_command_has_no_arg_preview() {
        let mut popup = CommandPopup::new();
        filter_to(&mut popup, "dashboard");
        assert_eq!(popup.selected_command_name(), Some("dashboard"));
        assert_eq!(popup.arg_preview(), None);
        assert_eq!(popup.info_row(), None);
    }

    #[test]
    fn info_row_prefers_the_not_yet_built_message_over_the_argument_preview() {
        let mut popup = CommandPopup::new();
        filter_to(&mut popup, "unit edit");
        assert!(popup.info_row().is_some_and(|(_, is_message)| !is_message));

        popup.set_not_yet_built("unit edit <code>");
        let (text, is_message) = popup.info_row().expect("a message should now show");
        assert!(is_message);
        assert_eq!(text, ":unit edit <code> — not yet built");
    }

    #[test]
    fn move_up_move_down_backspace_and_push_char_all_clear_a_not_yet_built_message() {
        let clears_via = |mutate: fn(&mut CommandPopup)| {
            let mut popup = CommandPopup::new();
            popup.set_not_yet_built("quit");
            mutate(&mut popup);
            assert!(
                popup.info_row().is_none_or(|(_, is_message)| !is_message),
                "message should be cleared"
            );
        };

        clears_via(|popup| popup.move_up());
        clears_via(|popup| popup.move_down());
        clears_via(|popup| popup.backspace());
        clears_via(|popup| popup.push_char('x'));
        clears_via(|popup| popup.tab());
    }

    #[test]
    fn rendering_grows_the_popup_by_one_row_when_an_info_row_shows() {
        use ratatui::{Terminal, backend::TestBackend};

        let render_height = |popup: &CommandPopup| -> u16 {
            let backend = TestBackend::new(96, 30);
            let mut terminal = Terminal::new(backend).expect("test backend should initialise");
            terminal
                .draw(|frame| popup.render(frame, frame.area()))
                .expect("rendering the popup should not error");
            popup_rect(
                Rect::new(0, 0, 96, 30),
                popup.rows().len() as u16
                    + FIXED_ROWS
                    + if popup.info_row().is_some() { 1 } else { 0 }
                    + 2,
            )
            .height
        };

        let mut no_args = CommandPopup::new();
        filter_to(&mut no_args, "dashboard");
        let mut with_args = CommandPopup::new();
        filter_to(&mut with_args, "unit edit");

        assert_eq!(
            render_height(&with_args),
            render_height(&no_args) + 1,
            "a command with an argument preview should render exactly one row taller"
        );
    }

    #[test]
    fn renders_without_panicking_with_an_argument_preview_row() {
        use ratatui::{Terminal, backend::TestBackend};

        let mut popup = CommandPopup::new();
        filter_to(&mut popup, "unit edit");
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area()))
            .expect("rendering the popup with an argument preview should not error");
    }

    #[test]
    fn renders_without_panicking_with_a_not_yet_built_message() {
        use ratatui::{Terminal, backend::TestBackend};

        let mut popup = CommandPopup::new();
        popup.set_not_yet_built("quit");
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area()))
            .expect("rendering the popup with a not-yet-built message should not error");
    }
}
