//! The "delete unit" popup — `:unit delete <code>`, or `d` from the Units view (`docs/ux/mockups/
//! units/README.md` §4d/§4e, "Delete"). Wireframe stage, same as `popup::unit::new`/`edit`:
//! every field renders one of two hardcoded mockup examples verbatim rather than a real,
//! reference-count-driven draft — `Shell` wires up opening and closing it (`Esc`) so the
//! navigation path exists before real reference-count resolution and deletion (`^s`) are
//! built out.
//!
//! §4d/§4e's own rule is "resolve the reference counts before drawing the dialog and branch —
//! do not draw one dialog that fails on submit", so [`DeleteUnitPopup`] is an enum of the two
//! branches rather than one struct: [`DeleteUnitPopup::Refused`] (§4d — something still
//! references the unit) and [`DeleteUnitPopup::Allowed`] (§4e — nothing does, so deletion
//! cascades the unit's price history). With no real resolution wired up yet, `Shell` always
//! opens [`DeleteUnitPopup::refused`] — it matches VDHG, the one row the Units view's own fake
//! data marks selected, and its reference counts (412 transactions, 1 account) are the same
//! ones `popup::unit::edit`'s own mockup already shows for VDHG. `allowed` (mocked against the
//! unreferenced AAPL instead) is fully built and tested but not reachable from a live key yet.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::colours::Colours;
use crate::popup::REFERENCE_TERMINAL_WIDTH;

/// Fraction of `REFERENCE_TERMINAL_WIDTH` the popup takes, per `docs/ux/mockups/units/README.md`
/// "The forms" ("~88% width on the drawing") — matches `popup::unit::new`/`edit`'s own width.
const POPUP_WIDTH_PERCENT: u32 = 88;

/// The popup's fixed width in terminal cells, computed once against the reference width rather
/// than the live terminal, mirroring `popup::unit::new`'s own `POPUP_WIDTH`.
const POPUP_WIDTH: u16 = ((REFERENCE_TERMINAL_WIDTH as u32 * POPUP_WIDTH_PERCENT) / 100) as u16;

/// Width of the reference-count label column (`transactions`, `accounts`, `prices`, `budgets`)
/// — sized to the longest label, plus a gap.
const LABEL_WIDTH: u16 = "transactions".len() as u16 + 1;

/// Content rows inside the border for [`DeleteUnitPopup::Allowed`]: title, its rule, the three
/// reference counts, a blank spacer, the one-line consequence statement, a blank spacer, the
/// three-row type-to-confirm box, the "keeping the history instead?" line, the footer's rule,
/// then the footer itself.
const ALLOWED_CONTENT_ROWS: u16 = 1 + 1 + 3 + 1 + 1 + 1 + 3 + 1 + 1 + 1;

/// Content rows inside the border for [`DeleteUnitPopup::Refused`]: title, its rule, the
/// blocking-rule statement, the four reference counts, a blank spacer, the four-row
/// deactivate-instead box, the footer's rule, then the footer itself.
const REFUSED_CONTENT_ROWS: u16 = 1 + 1 + 1 + 4 + 1 + 4 + 1 + 1;

/// The "delete unit" popup: `docs/ux/mockups/units/README.md` §4d/§4e as a centred floating
/// overlay, same window treatment as `popup::unit::new`/`edit`. No draft state yet — every
/// field is one of the two mockups' own placeholder content, not resolved against a real
/// `Unit`'s reference counts.
pub enum DeleteUnitPopup {
    /// §4d — a transaction or account still references the unit; deletion is refused and only
    /// deactivate is offered.
    Refused,
    /// §4e — nothing references the unit; deletion is allowed and cascades its price history.
    Allowed,
}

impl DeleteUnitPopup {
    /// Opens the refused (§4d) variant.
    pub fn refused() -> Self {
        Self::Refused
    }

    /// Opens the allowed (§4e) variant.
    pub fn allowed() -> Self {
        Self::Allowed
    }

    /// Renders whichever variant this is — see [`render_refused`]/[`render_allowed`] for each
    /// one's own layout.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        match self {
            DeleteUnitPopup::Refused => render_refused(frame, area, c),
            DeleteUnitPopup::Allowed => render_allowed(frame, area, c),
        }
    }
}

/// §4d's own worked example: `cannot delete VDHG`, blocked by 412 transactions and 1 account,
/// with `budgets` and the non-blocking `prices` count shown alongside for completeness.
fn render_refused(frame: &mut Frame<'_>, area: Rect, c: &Colours) {
    let popup = popup_rect(area, REFUSED_CONTENT_ROWS);

    frame.render_widget(Clear, popup);
    let block = Block::bordered();
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title
            Constraint::Length(1), // rule
            Constraint::Length(1), // blocking-rule statement
            Constraint::Length(1), // transactions
            Constraint::Length(1), // accounts
            Constraint::Length(1), // prices
            Constraint::Length(1), // budgets
            Constraint::Length(1), // blank spacer
            Constraint::Length(4), // deactivate-instead box
            Constraint::Length(1), // rule
            Constraint::Length(1), // footer hints
        ])
        .split(inner);

    render_refused_title(frame, rows[0], c);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
    render_blocking_rule(frame, rows[2]);
    render_count(
        frame,
        rows[3],
        "transactions",
        negative_count("412", "Index Fund", c),
        c,
    );
    render_count(
        frame,
        rows[4],
        "accounts",
        negative_count("1", "Index Fund (unit fixed at creation)", c),
        c,
    );
    render_count(
        frame,
        rows[5],
        "prices",
        dim_plain("52 weekly closes", c),
        c,
    );
    render_count(frame, rows[6], "budgets", dim_plain("none", c), c);
    // rows[7] is left blank — breathing space above the deactivate-instead box.
    render_deactivate_instead_box(frame, rows[8], c);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[9]);
    const HINTS: &[(&str, &str)] = &[
        ("x", "deactivate"),
        ("enter", "show the 412 txns"),
        ("esc", "close"),
    ];
    render_footer_hints(frame, rows[10], HINTS, c);
}

/// §4e's own worked example: `delete AAPL`, nothing references it, confirmed by typing the
/// code.
fn render_allowed(frame: &mut Frame<'_>, area: Rect, c: &Colours) {
    let popup = popup_rect(area, ALLOWED_CONTENT_ROWS);

    frame.render_widget(Clear, popup);
    let block = Block::bordered();
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title
            Constraint::Length(1), // rule
            Constraint::Length(1), // transactions
            Constraint::Length(1), // accounts
            Constraint::Length(1), // prices
            Constraint::Length(1), // blank spacer
            Constraint::Length(1), // consequence statement
            Constraint::Length(1), // blank spacer
            Constraint::Length(3), // type-to-confirm box
            Constraint::Length(1), // keeping the history instead?
            Constraint::Length(1), // rule
            Constraint::Length(1), // footer hints
        ])
        .split(inner);

    render_allowed_title(frame, rows[0], c);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
    render_count(
        frame,
        rows[2],
        "transactions",
        dim_hinted("0", "safe to delete", c),
        c,
    );
    render_count(frame, rows[3], "accounts", dim_plain("0", c), c);
    render_price_count(frame, rows[4], c);
    // rows[5] is left blank — breathing space above the consequence statement.
    render_consequence(frame, rows[6]);
    // rows[7] is left blank — breathing space above the type-to-confirm box.
    render_confirm_box(frame, rows[8], c);
    render_deactivate_hint(frame, rows[9], c);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[10]);
    const HINTS: &[(&str, &str)] = &[("^s", "delete"), ("x", "deactivate"), ("esc", "cancel")];
    render_footer_hints(frame, rows[11], HINTS, c);
}

/// The refused title row: `cannot delete VDHG` in `negative`, flush left, the `:unit delete`
/// command dim and right-aligned.
fn render_refused_title(frame: &mut Frame<'_>, area: Rect, c: &Colours) {
    let tag = ":unit delete";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    let negative_bold = c.negative().add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(Span::styled("cannot delete VDHG", negative_bold)),
        columns[0],
    );
    let dim = c.muted();
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The allowed title row: "delete AAPL" flush left, the `:unit delete` command dim and
/// right-aligned — echoing the refused title's own layout.
fn render_allowed_title(frame: &mut Frame<'_>, area: Rect, c: &Colours) {
    let tag = ":unit delete";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    frame.render_widget(Paragraph::new("delete AAPL"), columns[0]);
    let dim = c.muted();
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The rule the whole refused dialog exists to state plainly, full ink weight (not dim).
fn render_blocking_rule(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new("a unit can only be deleted when nothing references it."),
        area,
    );
}

/// One `label   value` reference-count row shared by both variants — the label dim and
/// fixed-width, per the shell's "dim for labels" style role, `value` carrying whatever
/// emphasis that particular count needs.
fn render_count(frame: &mut Frame<'_>, area: Rect, label: &str, value: Line<'static>, c: &Colours) {
    let dim = c.muted();
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled(label, dim)), columns[0]);
    frame.render_widget(Paragraph::new(value), columns[1]);
}

/// A count rendered dim throughout, with a dim `· hint` — the non-blocking rows on either
/// variant (`accounts` with no hint, `prices`/`budgets` on the refused variant).
fn dim_hinted(value: &'static str, hint: &'static str, c: &Colours) -> Line<'static> {
    let dim = c.muted();
    Line::from(Span::styled(format!("{value} · {hint}"), dim))
}

/// A single dim value with no hint at all.
fn dim_plain(value: &'static str, c: &Colours) -> Line<'static> {
    Line::from(Span::styled(value, c.muted()))
}

/// A blocking count: the figure itself in `negative`, its dim `· hint` after — the refused
/// variant's own `transactions`/`accounts` rows, the counts that actually gate deletion.
fn negative_count(count: &'static str, hint: &'static str, c: &Colours) -> Line<'static> {
    let negative = c.negative();
    let dim = c.muted();
    Line::from(vec![
        Span::styled(count, negative),
        Span::raw(" "),
        Span::styled(format!("· {hint}"), dim),
    ])
}

/// The allowed variant's `prices` row: the one reference count that isn't blocking — price
/// history cascades with the unit rather than gating its deletion — so its consequence renders
/// in `negative` rather than dim, per §4e's own "the one consequence spelled out". The inverse
/// emphasis of [`negative_count`] (value dim, hint negative, rather than the other way around),
/// since here the count itself isn't what's alarming.
fn render_price_count(frame: &mut Frame<'_>, area: Rect, c: &Colours) {
    let dim = c.muted();
    let negative = c.negative();

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(LABEL_WIDTH), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(Span::styled("prices", dim)), columns[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("18 weekly closes", dim),
            Span::raw(" "),
            Span::styled("· deleted with the unit", negative),
        ])),
        columns[1],
    );
}

/// The allowed variant's one-line consequence statement, full ink weight (not dim) — the
/// sentence the whole dialog exists to state plainly before the user commits.
fn render_consequence(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new("this removes the unit and its price history."),
        area,
    );
}

/// The allowed variant's type-to-confirm box: a `negative`-bordered `Block` around the `type the
/// code` prompt — §4e's own "confirm by typing the code, not by pressing `y`". The code typed
/// so far renders bold with a trailing `cursor` caret, the same treatment `popup::unit::new`'s
/// own `code` field uses for text still being entered.
fn render_confirm_box(frame: &mut Frame<'_>, area: Rect, c: &Colours) {
    let block = Block::bordered().border_style(c.negative());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let dim = c.muted();
    let bold = Style::default().add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("type the code ", dim),
            Span::styled("AAPL", bold),
            Span::styled("▏", c.cursor()),
        ])),
        inner,
    );
}

/// The allowed variant's soft-alternative hint below the confirm box: dim text with the `x`
/// key rendered as a reversed pill, matching the footer's own key/label convention — §4e keeps
/// offering deactivate even though deletion is allowed here.
fn render_deactivate_hint(frame: &mut Frame<'_>, area: Rect, c: &Colours) {
    let dim = c.muted();
    let key = c.selection();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("keeping the history instead? ", dim),
            Span::styled(" x ", key),
            Span::styled(" deactivate", dim),
        ])),
        area,
    );
}

/// The refused variant's "way out": a plain (not accent) bordered box — unlike the blocking
/// counts above it, this is the positive path, not a warning — offering deactivate and naming
/// what deleting for real would require. §4d's own "the way out, in a box".
fn render_deactivate_instead_box(frame: &mut Frame<'_>, area: Rect, c: &Colours) {
    let block = Block::bordered();
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let key = c.selection();
    let bold = Style::default().add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" x ", key),
            Span::raw(" "),
            Span::styled("deactivate instead", bold),
            Span::raw(" — keeps every record"),
        ])),
        lines[0],
    );

    let dim = c.muted();
    frame.render_widget(
        Paragraph::new(Span::styled(
            "to delete: reassign or remove the 412 transactions first",
            dim,
        )),
        lines[1],
    );
}

/// The window footer hint row: each key bold, its label dim — matching `popup::unit::new`'s
/// own `render_footer_hints` convention, shared here since the refused/allowed variants only
/// differ in which hints they list.
fn render_footer_hints(
    frame: &mut Frame<'_>,
    area: Rect,
    hints: &[(&'static str, &'static str)],
    c: &Colours,
) {
    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let label_style = c.muted();

    let mut spans = Vec::with_capacity(hints.len() * 3);
    for (idx, (key, label)) in hints.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(*key, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*label, label_style));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Computes a centred popup `Rect` sized to `content_rows` plus its top/bottom border — shared
/// by both variants, which differ only in how tall their content is. There's no scrolling body
/// to cap, so the only adjustment is shrinking to fit a terminal narrower or shorter than the
/// popup itself.
fn popup_rect(area: Rect, content_rows: u16) -> Rect {
    let width = POPUP_WIDTH.min(area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;

    let height = (content_rows + 2).min(area.height);
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

    fn render(popup: &DeleteUnitPopup, c: &Colours) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area(), c))
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
    fn refused_renders_without_panicking() {
        let c = &Colours::default();
        render(&DeleteUnitPopup::refused(), c);
    }

    #[test]
    fn allowed_renders_without_panicking() {
        let c = &Colours::default();
        render(&DeleteUnitPopup::allowed(), c);
    }

    #[test]
    fn refused_shows_the_title_and_command_tag() {
        let c = &Colours::default();
        let text = render(&DeleteUnitPopup::refused(), c);
        assert!(text.contains("cannot delete VDHG"), "title missing");
        assert!(text.contains(":unit delete"), "command tag missing");
    }

    #[test]
    fn refused_shows_the_blocking_rule() {
        let c = &Colours::default();
        let text = render(&DeleteUnitPopup::refused(), c);
        assert!(
            text.contains("a unit can only be deleted when nothing references it."),
            "blocking rule missing"
        );
    }

    #[test]
    fn refused_shows_every_reference_count() {
        let c = &Colours::default();
        let text = render(&DeleteUnitPopup::refused(), c);
        for label in ["transactions", "accounts", "prices", "budgets"] {
            assert!(text.contains(label), "{label} label missing");
        }
        assert!(text.contains("412"), "transaction count missing");
        assert!(text.contains("Index Fund"), "account reason missing");
        assert!(text.contains("52 weekly closes"), "price count missing");
        assert!(text.contains("none"), "budget count missing");
    }

    #[test]
    fn refused_shows_the_deactivate_instead_box() {
        let c = &Colours::default();
        let text = render(&DeleteUnitPopup::refused(), c);
        assert!(
            text.contains("deactivate instead"),
            "deactivate offer missing"
        );
        assert!(
            text.contains("to delete: reassign or remove the 412 transactions first"),
            "reassign instruction missing"
        );
    }

    #[test]
    fn refused_shows_the_footer_hints() {
        let c = &Colours::default();
        let text = render(&DeleteUnitPopup::refused(), c);
        for key in ["x", "enter", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
        assert!(text.contains("show the 412 txns"), "enter label missing");
    }

    #[test]
    fn allowed_shows_the_title_and_command_tag() {
        let c = &Colours::default();
        let text = render(&DeleteUnitPopup::allowed(), c);
        assert!(text.contains("delete AAPL"), "title missing");
        assert!(text.contains(":unit delete"), "command tag missing");
    }

    #[test]
    fn allowed_shows_the_reference_counts() {
        let c = &Colours::default();
        let text = render(&DeleteUnitPopup::allowed(), c);
        assert!(text.contains("transactions"), "transactions label missing");
        assert!(
            text.contains("safe to delete"),
            "safe-to-delete hint missing"
        );
        assert!(text.contains("accounts"), "accounts label missing");
        assert!(text.contains("18 weekly closes"), "price count missing");
        assert!(
            text.contains("deleted with the unit"),
            "price cascade consequence missing"
        );
    }

    #[test]
    fn allowed_shows_the_consequence_statement() {
        let c = &Colours::default();
        let text = render(&DeleteUnitPopup::allowed(), c);
        assert!(
            text.contains("this removes the unit and its price history."),
            "consequence statement missing"
        );
    }

    #[test]
    fn allowed_shows_the_type_to_confirm_box() {
        let c = &Colours::default();
        let text = render(&DeleteUnitPopup::allowed(), c);
        assert!(text.contains("type the code"), "confirm prompt missing");
        assert!(text.contains("AAPL"), "typed code missing");
    }

    #[test]
    fn allowed_shows_the_deactivate_hint_and_footer() {
        let c = &Colours::default();
        let text = render(&DeleteUnitPopup::allowed(), c);
        assert!(
            text.contains("keeping the history instead?"),
            "deactivate hint missing"
        );
        for key in ["^s", "x", "esc"] {
            assert!(text.contains(key), "{key} hint missing");
        }
    }

    #[test]
    fn popup_rect_stays_within_the_terminal_area() {
        let area = Rect::new(0, 0, 96, 30);
        let popup = popup_rect(area, REFUSED_CONTENT_ROWS);
        assert!(popup.x + popup.width <= area.width);
        assert!(popup.y + popup.height <= area.height);
    }

    #[test]
    fn popup_shrinks_to_fit_a_shorter_terminal() {
        let area = Rect::new(0, 0, 96, REFUSED_CONTENT_ROWS);
        let popup = popup_rect(area, REFUSED_CONTENT_ROWS);
        assert_eq!(popup.height, area.height);
    }
}
