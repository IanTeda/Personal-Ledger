//! The "new unit" popup — `:unit new`, or `n` from the Units view (`docs/ux/tui/units/
//! README.md` §4b, "Add"). Wireframe stage: every field renders the mockup's placeholder
//! content verbatim rather than a real, editable draft — `Shell` wires up opening and closing
//! it (`Esc`) so the navigation path exists before field editing, source testing (`^t`) and
//! creation (`^s`/`^p`) are built out.
//!
//! The `source` field and the `^t test source` / `^p create & pull prices` hints aren't in
//! §4b's own field table (`code`/`name`/`type`/`symbol`/`priced in`/`qty precision`/`price
//! precision`/`active`) or the checked-in `Ledger TUI Units.dc.html` — they're from the
//! mockup this popup was built against, a newer iteration than either.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::popup::REFERENCE_TERMINAL_WIDTH;

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used here for
/// the input cursor and the permanence warning.
const ACCENT: Color = Color::Red;

/// Fraction of `REFERENCE_TERMINAL_WIDTH` the popup takes, per `docs/ux/tui/units/README.md`
/// "The forms" ("~88% width on the drawing") — wider than `popup::command`'s 78%, since a form
/// carries a label column plus a hint on most rows.
const POPUP_WIDTH_PERCENT: u32 = 88;

/// The popup's fixed width in terminal cells, computed once against the reference width rather
/// than the live terminal, mirroring `popup::command`'s own `POPUP_WIDTH`.
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;

/// Width of the field label column (`code`, `price precision`, …) — sized to the longest
/// label, plus a gap.
const LABEL_WIDTH: u16 = "price precision".len() as u16 + 1;

/// Content rows inside the border: title, its rule, the nine fields, a blank spacer, the
/// two-line permanence warning, the footer's rule, then the footer itself.
const CONTENT_ROWS: u16 = 1 + 1 + 9 + 1 + 2 + 1 + 1;

/// Total popup height: content plus its top/bottom border.
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

/// The "new unit" popup: `docs/ux/tui/units/README.md` §4b as a centred floating overlay,
/// same window treatment as `popup::command`. No draft state yet — every field is the
/// mockup's own placeholder content, not a real value the user has typed.
#[derive(Default)]
pub struct NewUnitPopup;

impl NewUnitPopup {
    /// Opens a fresh popup.
    pub fn new() -> Self {
        Self
    }

    /// Renders the floating overlay, centred and sized to its fixed field list, within `area`
    /// (the full terminal area — the popup floats over the shell's status line and footer too,
    /// per §3a's "centred floating overlay", which §4b's own forms reuse verbatim).
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
                Constraint::Length(1), // source
                Constraint::Length(1), // priced in
                Constraint::Length(1), // qty precision
                Constraint::Length(1), // price precision
                Constraint::Length(1), // active
                Constraint::Length(1), // blank spacer
                Constraint::Length(1), // warning line 1
                Constraint::Length(1), // warning line 2
                Constraint::Length(1), // rule
                Constraint::Length(1), // footer hints
            ])
            .split(inner);

        render_title(frame, rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
        render_field(frame, rows[2], "code", code_value());
        render_field(
            frame,
            rows[3],
            "name",
            Line::from("VGS Intl Shares Index ETF"),
        );
        render_field(frame, rows[4], "type", type_value());
        render_field(
            frame,
            rows[5],
            "symbol",
            hinted("VGS.AX", "optional, your own reference"),
        );
        render_field(frame, rows[6], "source", source_value());
        render_field(
            frame,
            rows[7],
            "priced in",
            hinted("AUD", "tab to pick another unit"),
        );
        render_field(
            frame,
            rows[8],
            "qty precision",
            hinted("3", "decimals held"),
        );
        render_field(frame, rows[9], "price precision", hinted("4", "permanent"));
        render_field(frame, rows[10], "active", Line::from("[×]"));
        // rows[11] is left blank — breathing space above the permanence warning.
        render_warning(
            frame,
            rows[12],
            "CODE AND PRICE PRECISION CANNOT CHANGE ONCE",
        );
        render_warning(frame, rows[13], "A TRANSACTION EXISTS");
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[14]);
        render_footer_hints(frame, rows[15]);
    }
}

/// The title row: "new unit" flush left, the `:unit new` command dim and right-aligned —
/// echoing the command popup's own prompt row, per §4b's "title row left, context right".
fn render_title(frame: &mut Frame, area: Rect) {
    let tag = ":unit new";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    frame.render_widget(Paragraph::new("new unit"), columns[0]);
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

/// A plain value followed by a dim `· hint` — the pattern most fields use (`symbol`, `priced
/// in`, `qty precision`, `price precision`).
fn hinted(value: &'static str, hint: &'static str) -> Line<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    Line::from(vec![
        Span::raw(value),
        Span::raw(" "),
        Span::styled(format!("· {hint}"), dim),
    ])
}

/// The `code` field's value: the placeholder code, an accent block cursor, then its dim rule
/// hint — the one field the mockup shows mid-typing.
fn code_value() -> Line<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let cursor = Style::default().fg(ACCENT);
    Line::from(vec![
        Span::raw("VGS"),
        Span::styled("▌", cursor),
        Span::raw(" "),
        Span::styled("· uppercase, unique, permanent", dim),
    ])
}

/// The `type` field's value: a segmented `currency / etf / share / crypto` control with the
/// selected option rendered as a reversed pill, then the dim `· ↔` toggle hint.
fn type_value() -> Line<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let selected = Style::default().add_modifier(Modifier::REVERSED);
    Line::from(vec![
        Span::raw("currency "),
        Span::styled(" etf ", selected),
        Span::raw(" share crypto "),
        Span::styled("· ↔", dim),
    ])
}

/// The `source` field's value: the current price-feed source, then the dim list of
/// alternatives and the `↔` toggle hint.
fn source_value() -> Line<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    Line::from(vec![
        Span::raw("yahoo"),
        Span::raw(" "),
        Span::styled("· manual · yahoo · csv url · ↔", dim),
    ])
}

/// One line of the permanence warning, in the accent — the mockup's own emphasis for a
/// constraint that can only be honoured at creation.
fn render_warning(frame: &mut Frame, area: Rect, text: &'static str) {
    frame.render_widget(
        Paragraph::new(Span::styled(text, Style::default().fg(ACCENT))),
        area,
    );
}

/// The window footer hint row: each key bold, its label dim — matching the command popup's own
/// `footer_hint_line` convention.
fn render_footer_hints(frame: &mut Frame, area: Rect) {
    const HINTS: &[(&str, &str)] = &[
        ("tab", "next field"),
        ("^t", "test source"),
        ("^s", "create"),
        ("^p", "create & pull prices"),
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

/// Computes a centred popup `Rect` sized to `POPUP_HEIGHT`'s fixed field list — unlike
/// `popup::command`'s own `popup_rect`, there's no scrolling body to cap, so the only
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

    fn render(popup: &NewUnitPopup) -> String {
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
        render(&NewUnitPopup::new());
    }

    #[test]
    fn shows_the_title_and_command_tag() {
        let text = render(&NewUnitPopup::new());
        assert!(text.contains("new unit"), "title missing");
        assert!(text.contains(":unit new"), "command tag missing");
    }

    #[test]
    fn shows_every_field_label() {
        let text = render(&NewUnitPopup::new());
        for label in [
            "code",
            "name",
            "type",
            "symbol",
            "source",
            "priced in",
            "qty precision",
            "price precision",
            "active",
        ] {
            assert!(text.contains(label), "{label} label missing");
        }
    }

    #[test]
    fn shows_the_permanence_warning() {
        let text = render(&NewUnitPopup::new());
        assert!(
            text.contains("CODE AND PRICE PRECISION CANNOT CHANGE ONCE"),
            "warning line 1 missing"
        );
        assert!(
            text.contains("A TRANSACTION EXISTS"),
            "warning line 2 missing"
        );
    }

    #[test]
    fn shows_the_footer_hints() {
        let text = render(&NewUnitPopup::new());
        for key in ["tab", "^t", "^s", "^p", "esc"] {
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

    #[test]
    fn popup_shrinks_to_fit_a_shorter_terminal() {
        let area = Rect::new(0, 0, 96, POPUP_HEIGHT - 2);
        let popup = popup_rect(area);
        assert_eq!(popup.height, area.height);
    }
}
