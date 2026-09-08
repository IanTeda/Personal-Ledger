//! The "edit unit" popup — `:unit edit <code>`, or `e` from the Units view (`docs/ux/tui/
//! units/README.md` §4c, "Edit"). Wireframe stage, same as `popup::unit::new`: every field
//! renders the mockup's placeholder content verbatim — including a unit already mid-edit, with
//! its lowered `qty precision` warning already showing — rather than a real, editable draft.
//! `Shell` wires up opening and closing it (`Esc`) so the navigation path exists before field
//! editing and saving (`^s`) are built out.
//!
//! Locked fields render dim, with their reason where the mockup states one (`code`, `type`) —
//! per §4c's own "locked fields are shown, not hidden — with the reason attached". The
//! `🔒` glyph has no ASCII fallback wired up yet; §4c's own note that it needs one, behind the
//! same config flag as the status glyphs, is later work.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::popup::REFERENCE_TERMINAL_WIDTH;

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used here for
/// the input cursor and the precision-lowering warning box.
const ACCENT: Color = Color::Red;

/// The lock glyph marking a field `docs/ux/tui/units/README.md` §4c locks against editing —
/// shares the `docs/ux/tui/units/README.md` "Style" section's `🔒` with the summary box
/// (`view/units.rs` is fake-data only there, so it doesn't render one yet).
const LOCK: &str = "🔒";

/// Fraction of `REFERENCE_TERMINAL_WIDTH` the popup takes, per `docs/ux/tui/units/README.md`
/// "The forms" ("~88% width on the drawing") — matches `popup::unit::new`'s own width.
const POPUP_WIDTH_PERCENT: u32 = 88;

/// The popup's fixed width in terminal cells, computed once against the reference width rather
/// than the live terminal, mirroring `popup::unit::new`'s own `POPUP_WIDTH`.
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;

/// Width of the field label column (`code`, `qty precision`, …) — sized to the longest label,
/// plus a gap.
const LABEL_WIDTH: u16 = "qty precision".len() as u16 + 1;

/// Content rows inside the border: title, its rule, the seven fields, a blank spacer, the
/// four-row precision warning box, the footer's rule, then the footer itself.
const CONTENT_ROWS: u16 = 1 + 1 + 7 + 1 + 4 + 1 + 1;

/// Total popup height: content plus its top/bottom border.
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

/// The "edit unit" popup: `docs/ux/tui/units/README.md` §4c as a centred floating overlay,
/// same window treatment as `popup::unit::new`. No draft state yet — every field is the
/// mockup's own placeholder content, not a real value read from the edited `Unit`.
#[derive(Default)]
pub struct EditUnitPopup;

impl EditUnitPopup {
    /// Opens a fresh popup.
    pub fn new() -> Self {
        Self
    }

    /// Renders the floating overlay, centred and sized to its fixed field list, within `area`
    /// (the full terminal area — the popup floats over the shell's status line and footer too,
    /// per §3a's "centred floating overlay", which §4c's own forms reuse verbatim).
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
                Constraint::Length(1), // code
                Constraint::Length(1), // name
                Constraint::Length(1), // type
                Constraint::Length(1), // symbol
                Constraint::Length(1), // priced in
                Constraint::Length(1), // qty precision
                Constraint::Length(1), // active
                Constraint::Length(1), // blank spacer
                Constraint::Length(4), // precision warning box
                Constraint::Length(1), // rule
                Constraint::Length(1), // footer hints
            ])
            .split(inner);

        render_title(frame, rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
        render_locked_field(
            frame,
            rows[2],
            "code",
            "VDHG",
            Some("referenced by 412 transactions"),
        );
        render_field(frame, rows[3], "name", name_value());
        render_locked_field(
            frame,
            rows[4],
            "type",
            "etf",
            Some("fixed while prices exist"),
        );
        render_field(frame, rows[5], "symbol", Line::from("VDHG.AX"));
        render_locked_field(frame, rows[6], "priced in", "AUD", None);
        render_field(frame, rows[7], "qty precision", qty_precision_value());
        render_field(frame, rows[8], "active", active_value());
        // rows[9] is left blank — breathing space above the precision warning.
        render_precision_warning(frame, rows[10]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[11]);
        render_footer_hints(frame, rows[12]);
    }
}

/// The title row: "edit unit" flush left, the reference counts dim and right-aligned — §4c's
/// own "the title row carries the reference counts (`1 account · 412 txns · 52 prices`)".
fn render_title(frame: &mut Frame, area: Rect) {
    let tag = "1 account · 412 txns · 52 prices";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    frame.render_widget(Paragraph::new("edit unit"), columns[0]);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// One `label   value` row: the label dim and fixed-width, per the shell's "dim for labels"
/// style role, the value/control filling the rest.
fn render_field(frame: &mut Frame, area: Rect, label: &str, value: Line<'static>) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled(label, dim)), columns[0]);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

/// One locked `label   value 🔒 reason` row: label and the entire value dim throughout — §4c's
/// own "locked fields are shown, not hidden — with the reason attached" — `reason` is `None`
/// for a field the mockup locks without stating why (`priced in`).
fn render_locked_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &'static str,
    reason: Option<&'static str>,
) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let mut spans = vec![Span::raw(value), Span::raw(" "), Span::raw(LOCK)];
    if let Some(reason) = reason {
        spans.push(Span::raw(" "));
        spans.push(Span::raw(reason));
    }
    render_field(frame, area, label, Line::from(spans).style(dim));
}

/// The `name` field's value: free text with a trailing accent cursor — the one field the
/// mockup shows mid-typing (unlike the locked fields around it).
fn name_value() -> Line<'static> {
    let cursor = Style::default().fg(ACCENT);
    Line::from(vec![
        Span::raw("Vanguard Diversified High Growth"),
        Span::styled("▌", cursor),
    ])
}

/// The `qty precision` field's value: the current value, an arrow to the lowered draft, then
/// the dim `· was <original>` hint — the field driving the precision warning box below it.
fn qty_precision_value() -> Line<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    Line::from(vec![
        Span::raw("3 → 2"),
        Span::raw(" "),
        Span::styled("· was 3", dim),
    ])
}

/// The `active` field's value: the checkbox, then the dim hint framing it as the soft
/// alternative to deletion — §4c's own "the soft alternative to deletion".
fn active_value() -> Line<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    Line::from(vec![
        Span::raw("[×]"),
        Span::raw(" "),
        Span::styled("· clear to hide from pickers", dim),
    ])
}

/// The precision-lowering warning: a focused, accent-bordered box around its two lines — §4c's
/// own "lowering `qty precision` warns before it saves, in a focused box", distinct from
/// `popup::unit::new`'s own plain (unboxed) permanence warning since this one only shows up
/// conditionally, tied to the specific field change above it.
fn render_precision_warning(frame: &mut Frame, area: Rect) {
    let accent = Style::default().fg(ACCENT);
    let block = Block::bordered().border_style(accent);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "lowering precision rounds 1 holding: 561.204 → 561.20",
            accent,
        )),
        lines[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            "transactions are not rewritten — rounding applies",
            accent,
        )),
        lines[1],
    );
}

/// The window footer hint row: each key bold, its label dim — matching `popup::unit::new`'s
/// own `render_footer_hints` convention, minus the source-testing/pull-prices hints that only
/// apply at creation.
fn render_footer_hints(frame: &mut Frame, area: Rect) {
    const HINTS: &[(&str, &str)] = &[("tab", "next field"), ("^s", "save"), ("esc", "cancel")];

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
/// `popup::unit::new`'s own `popup_rect`; there's no scrolling body to cap, so the only
/// adjustment is shrinking to fit a terminal narrower or shorter than the popup itself.
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

    fn render(popup: &EditUnitPopup) -> String {
        let backend = TestBackend::new(96, 30);
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
        render(&EditUnitPopup::new());
    }

    #[test]
    fn shows_the_title_and_reference_counts() {
        let text = render(&EditUnitPopup::new());
        assert!(text.contains("edit unit"), "title missing");
        assert!(
            text.contains("1 account · 412 txns · 52 prices"),
            "reference counts missing"
        );
    }

    #[test]
    fn shows_every_field_label() {
        let text = render(&EditUnitPopup::new());
        for label in [
            "code",
            "name",
            "type",
            "symbol",
            "priced in",
            "qty precision",
            "active",
        ] {
            assert!(text.contains(label), "{label} label missing");
        }
    }

    #[test]
    fn shows_the_lock_glyph_and_reasons_on_locked_fields() {
        let text = render(&EditUnitPopup::new());
        assert_eq!(
            text.matches(LOCK).count(),
            3,
            "expected a lock glyph on code, type and priced in"
        );
        assert!(
            text.contains("referenced by 412 transactions"),
            "code's lock reason missing"
        );
        assert!(
            text.contains("fixed while prices exist"),
            "type's lock reason missing"
        );
    }

    #[test]
    fn shows_the_precision_warning_box() {
        let text = render(&EditUnitPopup::new());
        assert!(
            text.contains("lowering precision rounds 1 holding: 561.204 → 561.20"),
            "warning line 1 missing"
        );
        assert!(
            text.contains("transactions are not rewritten — rounding applies"),
            "warning line 2 missing"
        );
    }

    #[test]
    fn shows_the_footer_hints() {
        let text = render(&EditUnitPopup::new());
        for key in ["tab", "^s", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
        assert!(
            !text.contains("^t") && !text.contains("^p"),
            "edit form shouldn't carry the new form's source-testing/pull-prices hints"
        );
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
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
