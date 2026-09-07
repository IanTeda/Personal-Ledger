//! The command palette — the shell's own floating command window (`docs/ux/tui/README.md`
//! §3a), opened with `Ctrl+;` from anywhere. `view/mod.rs` calls this out as later work over
//! the `View` trait: the palette overlays whatever `View` is active rather than being one
//! itself, so `Shell` owns it directly instead of hosting it through `View`.
//!
//! Wireframe stage: the candidate list, argument preview and match count are static dummy
//! content matching the handoff's own examples, not the action registry §3a's "later ticket"
//! calls for — this only builds the box and its keys (typing, selection, open/close).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Widget},
};

/// One dummy palette row: `:command  <binding>  description` — §2a's help-list row shape,
/// which the palette now shares rather than §3a's own `action → effect → binding`, per the
/// user's own reordering. `command` excludes the leading `:` (added when rendering) so its
/// length can be measured for column alignment.
struct Candidate {
    command: &'static str,
    binding: &'static str,
    description: &'static str,
}

/// Static stand-in for the action registry §3a defers to a later ticket — enough rows to
/// prove the "up to 7" list, its selection highlight and the fuzzy-match framing.
const CANDIDATES: &[Candidate] = &[
    Candidate {
        command: "dashboard",
        binding: "g d",
        description: "financial position",
    },
    Candidate {
        command: "account list",
        binding: "g a",
        description: "accounts and their balances",
    },
    Candidate {
        command: "budget list",
        binding: "g b",
        description: "budgets vs actual for the period",
    },
    Candidate {
        command: "report variance",
        binding: "g r",
        description: "spending report and charts",
    },
    Candidate {
        command: "txn new",
        binding: "a",
        description: "add a transaction from anywhere",
    },
    Candidate {
        command: "reconcile",
        binding: "g k",
        description: "balance checks and clearing",
    },
    Candidate {
        command: "help",
        binding: "?",
        description: "browse every command",
    },
];

/// Blank cells between columns in the candidate rows.
const COLUMN_GAP: usize = 2;

/// Width of the `:command` column: the longest command (plus its `:`), plus a gap.
fn command_column_width() -> usize {
    CANDIDATES
        .iter()
        .map(|c| c.command.len() + 1)
        .max()
        .unwrap_or(0)
        + COLUMN_GAP
}

/// Width of the `<binding>` column: the longest binding, plus a gap.
fn binding_column_width() -> usize {
    CANDIDATES
        .iter()
        .map(|c| c.binding.len())
        .max()
        .unwrap_or(0)
        + COLUMN_GAP
}

/// The dummy total candidate count the match-count row quotes, per §3a's `7 of 62` example.
const TOTAL_ACTIONS: usize = 62;

/// Dim colour for hint text and secondary detail, matching `ACCENT`'s siblings in
/// `view/dashboard.rs`'s style table.
const DIM: Color = Color::DarkGray;

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table.
const ACCENT: Color = Color::Red;

/// A darker grey than `DIM` for the footer hint row's labels (`select`, `complete`, …) — an
/// explicit RGB value rather than a named/indexed colour, since `DIM`'s `Color::DarkGray`
/// (like `Color::Black` before it) renders however the user's terminal theme happens to remap
/// that palette slot, which isn't reliably "dark" on every theme.
const FOOTER_LABEL: Color = Color::Rgb(90, 90, 90);

/// Reference terminal width `docs/ux/tui/README.md` draws its wireframes against — the
/// palette's fixed width below is computed from this rather than from whatever terminal the
/// user happens to be running, so opening the palette looks the same at 96 columns and at
/// 300.
const REFERENCE_TERMINAL_WIDTH: u16 = 96;

/// Fraction of `REFERENCE_TERMINAL_WIDTH` the popup takes — back to §3a's own suggested ~78%
/// after a narrower value read too cramped in practice.
const PALETTE_WIDTH_PERCENT: u32 = 78;

/// The palette's fixed width in terminal cells: computed once against the reference width
/// above, not recomputed from the live terminal on every resize, so a wider terminal no
/// longer grows the popup — only a narrower one shrinks it (`popup_rect`).
const PALETTE_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * PALETTE_WIDTH_PERCENT) / 100) as u16;

/// State for the floating command palette: its input buffer and candidate selection.
/// `Shell` holds this as `Option<CommandPalette>` — `Some` while the window is open.
#[derive(Default)]
pub struct CommandPalette {
    input: String,
    selected: usize,
}

impl CommandPalette {
    /// Opens a fresh palette with an empty input buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a typed character to the input buffer.
    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
    }

    /// Removes the last character of the input buffer, if any.
    pub fn backspace(&mut self) {
        self.input.pop();
    }

    /// Moves the selection up one candidate row, clamped at the top.
    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Moves the selection down one candidate row, clamped at the bottom.
    pub fn move_down(&mut self) {
        if self.selected + 1 < CANDIDATES.len() {
            self.selected += 1;
        }
    }

    /// Renders the floating overlay, centred and content-sized, within `area` (the full
    /// terminal area — the popup floats over the shell's status line and footer too, not
    /// just the view region, per §3a's "centred floating overlay").
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let content_rows = 2 // prompt row + rule
            + CANDIDATES.len() as u16
            + 1 // argument preview
            + 1 // rule above the footer
            + 1; // footer hint row
        let popup = popup_rect(area, content_rows + 2 /* borders */);

        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(CANDIDATES.len() as u16),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let rule = || Line::from("─".repeat(inner.width as usize));
        frame.render_widget(self.prompt_line(inner.width), rows[0]);
        frame.render_widget(rule(), rows[1]);
        self.render_candidates(frame, rows[2]);
        frame.render_widget(argument_preview_line(inner.width), rows[3]);
        frame.render_widget(rule(), rows[4]);
        frame.render_widget(footer_hint_line(), rows[5]);
    }

    /// The prompt row: `:{input}▌` flush left, match count right-aligned — `:` rather than
    /// §3a's own `>`, so the prompt itself signals that what's typed is a `:command`, per the
    /// user's own ask.
    fn prompt_line(&self, width: u16) -> Line<'static> {
        let left = format!(":{}▌", self.input);
        let right = format!("{} of {TOTAL_ACTIONS}", CANDIDATES.len());
        Line::from(pad_between(&left, &right, width))
    }

    /// Up to 7 result rows, each `:command  <binding>  description`, the selected row a
    /// full-width reversed block per §3a.
    fn render_candidates(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); CANDIDATES.len()])
            .split(area);

        let command_col = command_column_width();
        let binding_col = binding_column_width();

        for (idx, candidate) in CANDIDATES.iter().enumerate() {
            let command = format!(":{}", candidate.command);
            let text = format!(
                "{command:<command_col$}{binding:<binding_col$}{description}",
                binding = candidate.binding,
                description = candidate.description,
            );
            let text = pad_line(&text, rows[idx].width);
            let style = if idx == self.selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            frame.render_widget(Line::from(text).style(style), rows[idx]);
        }
    }
}

/// The argument-preview row above the footer — dummy data resolved against a fake record, per
/// §3a's own example: `arg 1 <category> — dining · limit 300.00 · actual 412.00` left, the
/// over-budget figure right-aligned in the accent.
fn argument_preview_line(width: u16) -> Line<'static> {
    let left = "arg 1 <category> — dining · limit 300.00 · actual 412.00";
    let right = "over by 112.00";
    let padded = pad_between(left, right, width);

    // Colour only the trailing "over by ..." figure in the accent — everything before it
    // keeps the default style, matching §3a's "right-aligned in the accent" instruction.
    let split_at = padded.len().saturating_sub(right.len());
    let (prefix, suffix) = padded.split_at(split_at);
    Line::from(vec![
        Span::styled(prefix.to_string(), Style::default().fg(DIM)),
        Span::styled(suffix.to_string(), Style::default().fg(ACCENT)),
    ])
}

/// The window footer hint row. A background fill turned out to depend on how the user's own
/// terminal theme remaps indexed colours — it rendered invisibly there even though the cells
/// carried the right SGR codes — so the rule `render` draws above this row is what actually
/// separates it from the candidate list; each key is bold instead, which doesn't depend on
/// the palette.
fn footer_hint_line() -> Line<'static> {
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

/// Computes a centred popup `Rect` of the given content height, anchored in `area`'s top
/// third. Width is `PALETTE_WIDTH`, fixed regardless of terminal size — it does not grow on a
/// wider terminal — except when `area` itself is narrower, where it shrinks to fit rather
/// than overflow.
fn popup_rect(area: Rect, height: u16) -> Rect {
    let width = PALETTE_WIDTH.min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;

    let y = area.y + (area.height / 3).max(1);
    let height = height.min(area.height.saturating_sub(y - area.y));

    Rect {
        x,
        y,
        width,
        height,
    }
}

/// A widget that dims an already-drawn area of the buffer without touching its colours,
/// so the view behind the palette "stays visible and heavily dimmed", per §3a — `Buffer` has
/// no public per-cell mutation outside the `Widget`/render path, so this goes through one.
pub struct Dim;

impl Widget for Dim {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        buf.set_style(area, Style::default().add_modifier(Modifier::DIM));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_appends_to_the_input_buffer() {
        let mut palette = CommandPalette::new();
        palette.push_char('b');
        palette.push_char('u');
        palette.push_char('d');
        assert_eq!(palette.input, "bud");
    }

    #[test]
    fn backspace_removes_the_last_character() {
        let mut palette = CommandPalette::new();
        palette.push_char('b');
        palette.backspace();
        assert_eq!(palette.input, "");
    }

    #[test]
    fn backspace_on_empty_input_does_not_panic() {
        let mut palette = CommandPalette::new();
        palette.backspace();
        assert_eq!(palette.input, "");
    }

    #[test]
    fn selection_is_clamped_to_the_candidate_list() {
        let mut palette = CommandPalette::new();
        palette.move_up();
        assert_eq!(palette.selected, 0);

        for _ in 0..CANDIDATES.len() + 5 {
            palette.move_down();
        }
        assert_eq!(palette.selected, CANDIDATES.len() - 1);
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area, 11);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }

    #[test]
    fn popup_width_does_not_grow_with_a_wider_terminal() {
        let narrow = popup_rect(Rect::new(0, 0, 96, 30), 11);
        let wide = popup_rect(Rect::new(0, 0, 300, 30), 11);
        assert_eq!(narrow.width, PALETTE_WIDTH);
        assert_eq!(wide.width, PALETTE_WIDTH);
    }

    #[test]
    fn popup_width_shrinks_to_fit_a_terminal_narrower_than_the_fixed_width() {
        let area = Rect::new(0, 0, PALETTE_WIDTH - 5, 30);
        let popup = popup_rect(area, 11);
        assert_eq!(popup.width, area.width);
    }

    #[test]
    fn renders_without_panicking() {
        use ratatui::{Terminal, backend::TestBackend};

        let palette = CommandPalette::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| palette.render(frame, frame.area()))
            .expect("rendering the palette should not error");
    }
}
