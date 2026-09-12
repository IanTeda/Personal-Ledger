//! The base-unit guard overlay — `enter` from the Settings view on `general.base_unit`
//! (`docs/ux/tui/settings/README.md` §4c, "Base unit guard"). Wireframe stage, same as
//! `popup::settings::edit`: every field renders §4c's own `AUD → USD` worked example verbatim
//! — including the resolved impact figures and the typed confirmation — rather than a real
//! query against the (not yet built) settings registry and database. `Shell` wires up opening
//! and closing it (`Esc`) so the navigation path exists before the resolver and the commit
//! itself are built out.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::popup::REFERENCE_TERMINAL_WIDTH;

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used here for the
/// `missing rates` fact and the input cursor.
const ACCENT: Color = Color::Red;

/// Fraction of `REFERENCE_TERMINAL_WIDTH` the popup takes, per §4c's own "~78% width" — matches
/// the command popup's own width, distinct from a unit form's ~88%.
const POPUP_WIDTH_PERCENT: u32 = 78;

/// The popup's fixed width in terminal cells, computed once against the reference width rather
/// than the live terminal, mirroring `popup::unit::edit`'s own `POPUP_WIDTH`.
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;

/// Width of the fact label column (`transactions`, `missing rates`, …) — sized to the longest
/// label, plus a gap.
const LABEL_WIDTH: u16 = "missing rates".len() as u16 + 1;

/// Content rows inside the border: title, its rule, the 3-line prose, a blank spacer, the 6
/// impact facts, a blank spacer, the 4-row missing-rates box, the 3-row typed confirmation box
/// (directly beneath it — no blank spacer, to keep the popup shorter than the view's 96×30-cell
/// minimum; see `popup_rect`'s own doc comment for what a taller popup would do to the shell
/// chrome around it), a rule, then the footer hints.
const CONTENT_ROWS: u16 = 1 + 1 + 3 + 1 + 6 + 1 + 4 + 3 + 1 + 1;

/// Total popup height: content plus its top/bottom border.
const POPUP_HEIGHT: u16 = CONTENT_ROWS + 2;

/// The base-unit guard: `docs/ux/tui/settings/README.md` §4c as a centred floating overlay,
/// same window treatment as `popup::command` and `popup::unit`. No real resolver yet — every
/// figure is the mockup's own placeholder content, not a query against the user's data.
#[derive(Default)]
pub struct BaseUnitGuardPopup;

impl BaseUnitGuardPopup {
    /// Opens a fresh popup.
    pub fn new() -> Self {
        Self
    }

    /// Renders the floating overlay, anchored in the top third of `area` (the full terminal
    /// area) per §4c's own "anchored in the top third" — distinct from `popup::unit`'s forms,
    /// which centre vertically too but via the same `popup_rect` shape.
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
                Constraint::Length(3), // prose
                Constraint::Length(1), // blank spacer
                Constraint::Length(6), // impact facts
                Constraint::Length(1), // blank spacer
                Constraint::Length(4), // missing-rates box
                Constraint::Length(3), // typed confirmation box
                Constraint::Length(1), // rule
                Constraint::Length(1), // footer hints
            ])
            .split(inner);

        render_title(frame, rows[0]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
        render_prose(frame, rows[2]);
        // rows[3] is left blank — breathing space above the impact facts.
        render_impact_facts(frame, rows[4]);
        // rows[5] is left blank — breathing space above the missing-rates box.
        render_missing_rates_box(frame, rows[6]);
        render_confirmation_box(frame, rows[7]);
        frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[8]);
        render_footer_hints(frame, rows[9]);
    }
}

/// The title row: `base unit  AUD → USD` flush left, `:set base` dim and right-aligned — §4c's
/// own "Header: `base unit AUD → USD` with `:set base` right-aligned".
fn render_title(frame: &mut Frame, area: Rect) {
    let tag = ":set base";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    frame.render_widget(Paragraph::new("base unit  AUD → USD"), columns[0]);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The body's three prose lines, stating the cost before the facts do — §4c's own "prose
/// first ... then the facts".
fn render_prose(frame: &mut Frame, area: Rect) {
    let lines = [
        "transactions are stored in their own units and are",
        "not touched. Every reported total is re-converted",
        "at the weekly USD close.",
    ];
    frame.render_widget(Paragraph::new(lines.join("\n")), area);
}

/// One `label   value` fact row, the label dim and fixed-width; `accent` styles the value for
/// the one fact §4c marks (`missing rates`).
fn render_fact(frame: &mut Frame, area: Rect, label: &str, value: Line<'static>, accent: bool) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled(label, dim)), columns[0]);
    let value = if accent {
        value.style(Style::default().fg(ACCENT))
    } else {
        value
    };
    frame.render_widget(Paragraph::new(value), columns[1]);
}

/// The six resolved impact figures — §4c's own worked example verbatim, each "resolved by
/// query" against real data in the eventual build.
fn render_impact_facts(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_fact(
        frame,
        rows[0],
        "transactions",
        Line::from("412 · unchanged"),
        false,
    );
    render_fact(
        frame,
        rows[1],
        "accounts",
        Line::from("11 · 4 already in USD"),
        false,
    );
    render_fact(
        frame,
        rows[2],
        "re-converted",
        Line::from("18 months of totals"),
        false,
    );
    render_fact(
        frame,
        rows[3],
        "missing rates",
        Line::from("6 weeks · nov 25 – dec 25"),
        true,
    );
    render_fact(
        frame,
        rows[4],
        "one write",
        Line::from("general.base_unit = \"USD\""),
        false,
    );
    let dim = Style::default().add_modifier(Modifier::DIM);
    render_fact(
        frame,
        rows[5],
        "then",
        Line::from(vec![
            Span::raw("clears the cached totals "),
            Span::styled("· u undoes it", dim),
        ]),
        false,
    );
}

/// The missing-rates box — a plain bordered `Block` naming the gap this dialog exists to
/// prevent and offering the fix as a first-class option, per §4c's own "name the count and the
/// range, then offer the fix as a first-class option rather than an error".
fn render_missing_rates_box(frame: &mut Frame, area: Rect) {
    let block = Block::bordered();
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    frame.render_widget(
        Paragraph::new("those 6 weeks report as gaps until priced."),
        rows[0],
    );
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("i", bold),
            Span::raw(" "),
            Span::styled("import USD closes first — :price import", dim),
        ])),
        rows[1],
    );
}

/// The typed-confirmation box — an accent-bordered focused input, matching the delete
/// confirmation convention on the unit forms per §4c's own "matching the delete-confirmation
/// convention on the unit forms".
fn render_confirmation_box(frame: &mut Frame, area: Rect) {
    let accent = Style::default().fg(ACCENT);
    let block = Block::bordered().border_style(accent);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("type the unit  "),
            Span::raw("USD"),
            Span::styled("▌", accent),
        ])),
        inner,
    );
}

/// The window footer hint row: each key bold, its label dim — §4c's own "Keys: `enter` commit
/// and re-convert · `i` import first · `esc` cancel".
fn render_footer_hints(frame: &mut Frame, area: Rect) {
    const HINTS: &[(&str, &str)] = &[
        ("enter", "commit and re-convert"),
        ("i", "import first"),
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

/// Computes a popup `Rect` anchored in the top third of `area`, per §4c's own "anchored in the
/// top third" — otherwise mirrors `popup::unit::edit`'s own `popup_rect`. A popup taller than
/// the available height shrinks to fit and its anchor clamps toward `y = 0`, covering the
/// shell's own status line and footer (they render first, but a full-height `Clear` still
/// wipes over them) — `CONTENT_ROWS` is kept well under the view's 96×30-cell minimum
/// specifically to avoid that.
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

    fn render(popup: &BaseUnitGuardPopup) -> String {
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
        render(&BaseUnitGuardPopup::new());
    }

    #[test]
    fn shows_the_title_and_command_tag() {
        let text = render(&BaseUnitGuardPopup::new());
        assert!(text.contains("base unit"), "title missing");
        assert!(text.contains("AUD → USD"), "conversion arrow missing");
        assert!(text.contains(":set base"), "command tag missing");
    }

    #[test]
    fn shows_the_prose() {
        let text = render(&BaseUnitGuardPopup::new());
        assert!(
            text.contains("transactions are stored in their own units and are"),
            "prose line 1 missing"
        );
        assert!(
            text.contains("at the weekly USD close."),
            "prose line 3 missing"
        );
    }

    #[test]
    fn shows_every_impact_fact() {
        let text = render(&BaseUnitGuardPopup::new());
        for (label, value) in [
            ("transactions", "412 · unchanged"),
            ("accounts", "11 · 4 already in USD"),
            ("re-converted", "18 months of totals"),
            ("missing rates", "6 weeks · nov 25 – dec 25"),
            ("one write", "general.base_unit = \"USD\""),
        ] {
            assert!(text.contains(label), "{label} label missing");
            assert!(text.contains(value), "{label}'s value missing");
        }
        assert!(text.contains("then"), "then label missing");
        assert!(
            text.contains("clears the cached totals"),
            "then value missing"
        );
        assert!(text.contains("u undoes it"), "undo hint missing");
    }

    #[test]
    fn missing_rates_fact_renders_in_the_accent_colour() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| BaseUnitGuardPopup::new().render(frame, frame.area()))
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
        let row_has_accent =
            |y: u16| -> bool { (0..buffer.area.width).any(|x| buffer[(x, y)].fg == ACCENT) };

        assert!(
            row_has_accent(row_containing("missing rates")),
            "missing rates fact should render in the accent colour"
        );
    }

    #[test]
    fn shows_the_missing_rates_box_and_import_hint() {
        let text = render(&BaseUnitGuardPopup::new());
        assert!(
            text.contains("those 6 weeks report as gaps until priced."),
            "missing-rates box line missing"
        );
        assert!(
            text.contains("import USD closes first — :price import"),
            "import-first hint missing"
        );
    }

    #[test]
    fn shows_the_typed_confirmation_input() {
        let text = render(&BaseUnitGuardPopup::new());
        assert!(text.contains("type the unit"), "confirmation label missing");
        assert!(text.contains("USD"), "typed unit missing");
    }

    #[test]
    fn shows_the_footer_hints() {
        let text = render(&BaseUnitGuardPopup::new());
        assert!(text.contains("enter"), "enter hint missing");
        assert!(
            text.contains("commit and re-convert"),
            "commit label missing"
        );
        assert!(text.contains("import first"), "import first label missing");
        assert!(text.contains("cancel"), "cancel label missing");
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

    #[test]
    fn popup_is_narrower_than_a_unit_form() {
        // §4c's own "~78% width" is narrower than `popup::unit`'s ~88% — this is what keeps
        // the guard visually distinct from an ordinary form.
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area);
        assert!(popup.width < 88 * 96 / 100);
    }
}
