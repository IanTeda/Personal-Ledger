//! The in-place editor popup — `e` from the Settings view, or `:set <key>` (`docs/ux/tui/
//! settings/README.md` §4b, "Editing in place"). Wireframe stage, same as `popup::unit::edit`:
//! every field renders §4b's own `general.negatives` worked example verbatim — including the
//! candidate already chosen (`minus`) and its resolved preview/on-accept lines — rather than a
//! real, editable draft against the (not yet built) settings registry. `Shell` wires up
//! opening and closing it (`Esc`) so the navigation path exists before field editing and
//! committing are built out.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::popup::REFERENCE_TERMINAL_WIDTH;

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used here for the
/// input cursor and the `preview` figure.
const ACCENT: Color = Color::Red;

/// Fraction of `REFERENCE_TERMINAL_WIDTH` the popup takes. §4b itself calls for an in-place
/// (non-floating) editor rather than a sized dialog, so this borrows `popup::unit`'s own ~88%
/// form width instead — see this module's own doc comment.
const POPUP_WIDTH_PERCENT: u32 = 88;

/// The popup's fixed width in terminal cells, computed once against the reference width rather
/// than the live terminal, mirroring `popup::unit::edit`'s own `POPUP_WIDTH`.
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;

/// Width of the field label column (`preview`, `ledger row`, …) — sized to the longest label,
/// plus a gap.
const LABEL_WIDTH: u16 = "ledger row".len() as u16 + 2;

/// Content rows inside the border: title, its rule, the explain line, the 4-row value box,
/// `preview`/`current`, the "applies to" heading and its 3 lines, a blank spacer, the 5-row
/// on-accept box, the typed command line, a rule, then the footer hints. Trimmed from §4a's own
/// blank-spaced, ruled layout — every optional blank/rule between sections dropped — to keep
/// the popup shorter than the view's 96×30-cell minimum; see `popup_rect`'s own doc comment for
/// what a taller popup would do to the shell chrome around it.
const CONTENT_ROWS: u16 = 1 + 1 + 1 + 4 + 1 + 1 + 1 + 3 + 1 + 5 + 1 + 1 + 1;

/// Total popup height: content plus its top/bottom border.
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

/// The in-place editor popup: `docs/ux/tui/settings/README.md` §4b as a centred floating
/// overlay (see this module's own doc comment for why, over §4b's literal "in the row's
/// position"). No draft state yet — every field is the mockup's own placeholder content, not a
/// real candidate value.
#[derive(Default)]
pub struct EditSettingPopup;

impl EditSettingPopup {
    /// Opens a fresh popup.
    pub fn new() -> Self {
        Self
    }

    /// Renders the floating overlay, centred and sized to its fixed field list, within `area`
    /// (the full terminal area, per `popup::unit::edit`'s own convention).
    pub fn render(&self, frame: &mut Frame, area: Rect) {
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
                Constraint::Length(1), // explain
                Constraint::Length(4), // value box
                Constraint::Length(1), // preview
                Constraint::Length(1), // current
                Constraint::Length(1), // applies to heading
                Constraint::Length(3), // applies to lines
                Constraint::Length(1), // blank spacer
                Constraint::Length(5), // on accept box
                Constraint::Length(1), // typed command line
                Constraint::Length(1), // rule
                Constraint::Length(1), // footer hints
            ])
            .split(inner);

        render_title(frame, rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
        render_field(
            frame,
            rows[2],
            "",
            Line::from("how a negative amount prints everywhere"),
        );
        render_value_box(frame, rows[3]);
        render_preview(frame, rows[4]);
        render_current(frame, rows[5]);
        render_applies_to_heading(frame, rows[6]);
        render_applies_to_lines(frame, rows[7]);
        // rows[8] is left blank — breathing space above the on-accept box.
        render_on_accept_box(frame, rows[9]);
        render_typed_command(frame, rows[10]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[11]);
        render_footer_hints(frame, rows[12]);
    }
}

/// The title row: `negatives` flush left, `general.negatives` dim and right-aligned — matches
/// `popup::unit::edit`'s own "title left, context right" convention.
fn render_title(frame: &mut Frame, area: Rect) {
    let tag = "general.negatives";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    frame.render_widget(Paragraph::new("negatives"), columns[0]);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// One `label   value` row, the label dim and fixed-width — mirrors `popup::unit::edit`'s own
/// `render_field`. `label` is `""` for the explain line, which has no label of its own.
fn render_field(frame: &mut Frame, area: Rect, label: &str, value: Line<'static>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled(label, dim)), columns[0]);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

/// The `value` field's focused box — an accent-bordered `Block` around the enum's segmented
/// row (the selected variant reversed, per §4b's "the selection is a reversed block") and its
/// `←→` hint, mirroring `popup::unit::edit`'s own accent-bordered precision warning box.
fn render_value_box(frame: &mut Frame, area: Rect) {
    let accent = Style::default().fg(ACCENT);
    let block = Block::bordered().border_style(accent);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let reversed = Style::default().add_modifier(Modifier::REVERSED);
    render_field(
        frame,
        lines[0],
        "value",
        Line::from(vec![
            Span::styled("minus", reversed),
            Span::raw(" brackets trailing"),
        ]),
    );
    let dim = Style::default().add_modifier(Modifier::DIM);
    render_field(
        frame,
        lines[1],
        "",
        Line::from(Span::styled("↔ choose · 3 options", dim)),
    );
}

/// The `preview` row: a real figure in the candidate format, per §4b's own "a real figure from
/// the user's data ... not lorem" — styled `ACCENT`, matching the design's own `← accent`
/// marker on this row.
fn render_preview(frame: &mut Frame, area: Rect) {
    let accent = Style::default().fg(ACCENT);
    render_field(
        frame,
        area,
        "preview",
        Line::from(Span::styled("−320 334.10", accent)),
    );
}

/// The `current` row: the value the row would fall back to if there were no override, and
/// whether it is the default — §4b's own "names the current value *and* whether it is the
/// default, in one line".
fn render_current(frame: &mut Frame, area: Rect) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    render_field(
        frame,
        area,
        "current",
        Line::from(vec![
            Span::raw("(320 334.10) "),
            Span::styled("· DEFAULT", dim),
        ]),
    );
}

/// The "applies to" heading, tagged with §4b's own "everywhere an amount prints" — same dim
/// label / dim tag pattern as `view::settings`'s own `render_heading`.
fn render_applies_to_heading(frame: &mut Frame, area: Rect) {
    let tag = "EVERYWHERE AN AMOUNT PRINTS";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled("APPLIES TO", dim)), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The "applies to" box's three lines — net position, one ledger row and the csv-export note,
/// §4b's own "the same candidate shown in the three places it lands, so scope is legible
/// before committing".
fn render_applies_to_lines(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_field(frame, rows[0], "net", Line::from("−320 334.10"));
    render_field(
        frame,
        rows[1],
        "ledger row",
        Line::from("Woolworths −184.20"),
    );
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled("csv export unaffected · machine format", dim)),
        rows[2],
    );
}

/// The "on accept" box — a plain bordered `Block` spelling out the exact write §4b calls for
/// ("`on accept` spells out the write") and the `r` drop-override hint beneath it. §4b's own
/// two-line hint ("drop the override and fall back to the" / "shipped default instead of
/// storing a row") is folded onto one line here to keep the box within this popup's trimmed
/// height budget (`CONTENT_ROWS`'s own doc comment).
fn render_on_accept_box(frame: &mut Frame, area: Rect) {
    let block = Block::bordered();
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // "on accept  upsert settings row"
            Constraint::Length(1), // "general.negatives = "minus""
            Constraint::Length(1), // "r  drop override, falling back to the shipped default"
        ])
        .split(inner);

    render_field(
        frame,
        rows[0],
        "on accept",
        Line::from("upsert settings row"),
    );
    frame.render_widget(Paragraph::new("general.negatives = \"minus\""), rows[1]);
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("r", bold),
            Span::raw("  "),
            Span::styled("drop override, falling back to the shipped default", dim),
        ])),
        rows[2],
    );
}

/// The typed-command echo — §4b's own "the command line echoes the equivalent command", with
/// an accent cursor after the value and the dim "same edit, typed" note.
fn render_typed_command(frame: &mut Frame, area: Rect) {
    let cursor = Style::default().fg(ACCENT);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(":set negatives minus"),
            Span::styled("▌", cursor),
            Span::raw(" "),
            Span::styled("— same edit, typed", dim),
        ])),
        area,
    );
}

/// The window footer hint row: each key bold, its label dim — matches §4b's own "Keys: `←→`
/// choose · `enter` commit · `esc` revert · `r` drop override", `enter` folded into `commit`
/// per `popup::unit::edit`'s own bold-key convention (`^s` there since a unit form has other
/// text fields `enter` would otherwise submit early).
fn render_footer_hints(frame: &mut Frame, area: Rect) {
    const HINTS: &[(&str, &str)] = &[
        ("←→", "choose"),
        ("^s", "commit"),
        ("r", "drop override"),
        ("esc", "cancel"),
    ];

    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let label_style = Style::default().add_modifier(Modifier::DIM);

    let mut spans = Vec::with_capacity(HINTS.len() * 3);
    for (idx, (key, label)) in HINTS.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(*key, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*label, label_style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Computes a centred popup `Rect` sized to `POPUP_HEIGHT`'s fixed field list — mirrors
/// `popup::unit::edit`'s own `popup_rect`. A popup taller than the available height shrinks to
/// fit and its anchor clamps toward `y = 0`, covering the shell's own status line and footer
/// (they render first, but a full-height `Clear` still wipes over them) — `CONTENT_ROWS` is
/// kept well under the view's 96×30-cell minimum specifically to avoid that.
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

    fn render(popup: &EditSettingPopup) -> String {
        let backend = TestBackend::new(96, 34);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area()))
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
        render(&EditSettingPopup::new());
    }

    #[test]
    fn shows_the_title_and_key() {
        let text = render(&EditSettingPopup::new());
        assert!(text.contains("negatives"), "title missing");
        assert!(text.contains("general.negatives"), "dotted key missing");
    }

    #[test]
    fn shows_the_value_choices_and_the_choose_hint() {
        let text = render(&EditSettingPopup::new());
        for variant in ["minus", "brackets", "trailing"] {
            assert!(text.contains(variant), "{variant} option missing");
        }
        assert!(text.contains("↔ choose · 3 options"), "choose hint missing");
    }

    #[test]
    fn the_selected_variant_renders_reversed() {
        let backend = TestBackend::new(96, 34);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| EditSettingPopup::new().render(frame, frame.area()))
            .expect("rendering the popup should not error");

        let buffer = terminal.backend().buffer();
        let row_containing = |needle: &str| -> u16 {
            (0..buffer.area.height)
                .find(|&y| {
                    let mut row = String::new();
                    for x in 0..buffer.area.width {
                        row.push_str(buffer[(x, y)].symbol());
                    }
                    row.contains(needle)
                })
                .unwrap_or_else(|| panic!("no row contains {needle:?}"))
        };
        let row_is_reversed = |y: u16| -> bool {
            (0..buffer.area.width).any(|x| buffer[(x, y)].modifier.contains(Modifier::REVERSED))
        };

        assert!(
            row_is_reversed(row_containing("brackets trailing")),
            "the value row should carry a reversed span on the selected variant"
        );
    }

    #[test]
    fn shows_preview_current_and_applies_to() {
        let text = render(&EditSettingPopup::new());
        assert!(text.contains("preview"), "preview label missing");
        assert!(text.contains("−320 334.10"), "preview value missing");
        assert!(text.contains("current"), "current label missing");
        assert!(text.contains("(320 334.10)"), "current value missing");
        assert!(text.contains("DEFAULT"), "default marker missing");
        assert!(text.contains("APPLIES TO"), "applies to heading missing");
        assert!(text.contains("net"), "net line missing");
        assert!(text.contains("Woolworths"), "ledger row line missing");
        assert!(
            text.contains("csv export unaffected · machine format"),
            "csv export line missing"
        );
    }

    #[test]
    fn shows_the_on_accept_write_and_drop_override_hint() {
        let text = render(&EditSettingPopup::new());
        assert!(text.contains("on accept"), "on accept label missing");
        assert!(
            text.contains("upsert settings row"),
            "on accept verb missing"
        );
        assert!(
            text.contains("general.negatives = \"minus\""),
            "the exact write missing"
        );
        assert!(
            text.contains("drop override, falling back to the shipped default"),
            "drop-override hint missing"
        );
    }

    #[test]
    fn shows_the_typed_command_echo() {
        let text = render(&EditSettingPopup::new());
        assert!(
            text.contains(":set negatives minus"),
            "typed command echo missing"
        );
        assert!(
            text.contains("— same edit, typed"),
            "same-edit note missing"
        );
    }

    #[test]
    fn shows_the_footer_hints() {
        let text = render(&EditSettingPopup::new());
        for key in ["←→", "^s", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
        assert!(text.contains("choose"), "choose label missing");
        assert!(text.contains("commit"), "commit label missing");
        assert!(
            text.contains("drop override"),
            "drop override label missing"
        );
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 34);
        let popup = popup_rect(area);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }

    #[test]
    fn popup_shrinks_to_fit_a_shorter_terminal() {
        let area = Rect::new(0, 0, 96, POPUP_HEIGHT - 2);
        let popup = popup_rect(area);
        assert_eq!(popup.height, area.height);
    }
}
