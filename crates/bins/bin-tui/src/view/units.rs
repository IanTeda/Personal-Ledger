//! The Units `View`, hosted by `Shell` (ADR-0013). Wireframe stage: four labelled, bordered
//! placeholder boxes matching the pane structure and proportions from
//! `docs/ux/tui/units/README.md` §4a — unit list and summary in a ~36-col left column,
//! weekly close candlestick and weekly prices filling the rest — so the screen's overall
//! layout can be checked and adjusted before any one region's real widget content (list,
//! summary detail, candlestick chart, price table, forms) is built out.

use chandelier::{Candle, CandleSeries, CandlestickChart};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crate::view::{Action, View};

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used here for
/// a negative weekly `Δ%`.
const ACCENT: Color = Color::Red;

/// Width of the left column — the unit list and its summary. Widened past §4a's own "~36
/// cols" for less cramped `TYPE`/summary-value columns and the summary box's own horizontal
/// padding; the right column still has plenty of room for the candlestick chart and weekly
/// prices table.
const LEFT_COLUMN_WIDTH: u16 = 46;

/// Width of the unit list's `CODE` column, per §4a's column set.
const UNIT_CODE_WIDTH: u16 = 6;

/// Width of the unit list's right-aligned, tabular `LAST` column, per §4a's column set.
const UNIT_LAST_WIDTH: u16 = 9;

/// Total number of fake units behind the list, deliberately larger than the fake rows
/// actually shown — so the scrollbar renders a partial thumb with track beyond it, showing
/// there's more to scroll to than what's listed, rather than a full, non-scrollable track.
const FAKE_UNIT_TOTAL: usize = 20;

/// Width of the summary box's secondary-field label column, e.g. `"symbol · priced in   "` —
/// wide enough for the longest label plus a run of spaces before the value starts.
const SUMMARY_LABEL_WIDTH: usize = 21;

/// Number of highlighted, bold figures at the top of the summary box
/// (`UnitSummary::figures`).
const SUMMARY_FIGURE_COUNT: usize = 3;

/// Number of secondary label/value field rows in the summary box (`UnitSummary::fields`).
const SUMMARY_FIELD_COUNT: usize = 6;

/// Rows inside the summary box's border: code/type, name, a blank spacer, the highlighted
/// figures, another blank spacer, then the secondary fields.
const SUMMARY_BOX_CONTENT_HEIGHT: u16 =
    2 + 1 + SUMMARY_FIGURE_COUNT as u16 + 1 + SUMMARY_FIELD_COUNT as u16;

/// Total height of the summary section — the "SUMMARY" heading, its rule, and the
/// bordered box (border plus its content rows). Fixed rather than left to stretch, so the
/// box sizes tightly to its content and any extra terminal height goes to the unit list box
/// above instead of sitting as blank space at the bottom of the summary box.
const SUMMARY_SECTION_HEIGHT: u16 = 2 + SUMMARY_BOX_CONTENT_HEIGHT + 2;

/// Height of the weekly close section: heading, its rule, ~11 rows of candles, and the
/// price/time axis rows drawn by `chandelier`'s `CandlestickChart`.
const WEEKLY_CLOSE_HEIGHT: u16 = 17;

/// Number of fake weekly candles behind the weekly close chart — roughly the "12 months" of
/// weekly OHLC from §4a; `CandlestickChart` autoscales and draws the most recent that fit.
const FAKE_WEEK_COUNT: usize = 52;

/// Height of the command/keybind hints section: heading, its rule, the single-key hints row,
/// then one row per `COMMAND_HINTS` line.
const KEYBIND_HINTS_HEIGHT: u16 = 1 + 1 + 1 + COMMAND_HINTS.len() as u16;

/// The single-key hints for navigating and acting on the highlighted unit, bold key / dim
/// label — matches the shell footer's "key bolded to stand out from its label" convention and
/// the design reference's own key-hint bar.
const KEY_HINTS: [(&str, &str); 7] = [
    ("j/k", "unit"),
    ("Tab", "list ↔ prices"),
    ("n", "new"),
    ("e", "edit"),
    ("d", "delete"),
    ("p", "set price"),
    ("[ ]", "12m window"),
];

/// The `:` command hints, matching §4a's own "Command hints" example verbatim.
const COMMAND_HINTS: [&str; 3] = [
    ":unit new <code> <type> <precision>  — add a unit",
    ":unit edit VDHG  — name, symbol, precision, active",
    ":price set VDHG <date> <close>  ·  :price import <path.csv>",
];

/// The fake "units held" figure, shared between the summary's `UNITS HELD` figure and the
/// weekly prices heading's tag, so the two stay consistent.
const FAKE_UNITS_HELD: &str = "561.204";

/// `FAKE_UNITS_HELD` as a plain number, for computing the generated weekly price rows' market
/// value (`close * FAKE_UNITS_HELD_QTY`). Keep the two in sync.
const FAKE_UNITS_HELD_QTY: f64 = 561.204;

/// Width of the weekly prices table's `W/C` column.
const WEEKLY_PRICE_DATE_WIDTH: u16 = 7;

/// Width of the weekly prices table's `CLOSE` column.
const WEEKLY_PRICE_CLOSE_WIDTH: u16 = 7;

/// Width of the weekly prices table's `Δ%` column.
const WEEKLY_PRICE_CHANGE_WIDTH: u16 = 8;

/// A trivial placeholder Units `View`: four labelled boxes proving the §4a pane layout.
#[derive(Default)]
pub struct UnitsView;

impl UnitsView {
    pub fn new() -> Self {
        Self
    }
}

impl View for UnitsView {
    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        // rows[0] is left blank — breathing space between the shell's title bar and the
        // unit list / weekly close boxes, matching the dashboard's own spacer rows.
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(LEFT_COLUMN_WIDTH), Constraint::Min(0)])
            .spacing(2)
            .split(rows[1]);

        render_left_column(frame, columns[0]);
        render_price_history(frame, columns[1]);
    }

    fn title(&self) -> &'static str {
        "Units"
    }
}

/// The left column: the unit list above its summary, per §4a ("Left column ~36 cols holds
/// the list above the summary"). The summary section is sized tightly to its content, so the
/// unit list box takes whatever height is left over.
fn render_left_column(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(SUMMARY_SECTION_HEIGHT),
        ])
        .split(area);

    render_unit_list(frame, rows[0], &fake_units());
    render_summary(frame, rows[1], &fake_summary());
}

/// One row in the unit list — matches §4a's own example rows
/// (`docs/ux/tui/units/README.md`): base currency, another currency, the selected unit, two
/// more units, then an inactive one.
struct UnitListRow {
    code: &'static str,
    /// The qualifier-bearing type cell, e.g. `"currency · base"`, `"share · inactive"`.
    kind: &'static str,
    last: &'static str,
    selected: bool,
    /// Inactive units render dim throughout, per §4a.
    inactive: bool,
}

/// The fake rows behind the unit list — §4a's own worked example, verbatim.
fn fake_units() -> Vec<UnitListRow> {
    vec![
        UnitListRow {
            code: "AUD",
            kind: "currency · base",
            last: "1.0000",
            selected: false,
            inactive: false,
        },
        UnitListRow {
            code: "USD",
            kind: "currency",
            last: "0.6612",
            selected: false,
            inactive: false,
        },
        UnitListRow {
            code: "VDHG",
            kind: "etf",
            last: "72.41",
            selected: true,
            inactive: false,
        },
        UnitListRow {
            code: "VAS",
            kind: "etf",
            last: "104.82",
            selected: false,
            inactive: false,
        },
        UnitListRow {
            code: "BTC",
            kind: "crypto",
            last: "168 402",
            selected: false,
            inactive: false,
        },
        UnitListRow {
            code: "AAPL",
            kind: "share · inactive",
            last: "341.06",
            selected: false,
            inactive: true,
        },
    ]
}

/// The unit list (top left): a "UNITS · N OF total" heading over a `CODE`/`TYPE`/`LAST`
/// table, a scrollbar riding the right edge of the row area so it reads as scrollable even
/// though every fake row here fits on screen. No border, echoing the design reference's
/// borderless `.pane` — only the summary box beneath it gets one.
fn render_unit_list(frame: &mut Frame, area: Rect, units: &[UnitListRow]) {
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .spacing(1) // breathing space between the LAST column and the scrollbar
        .split(area);
    let content_area = split[0];
    let scrollbar_column = split[1];

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // heading
            Constraint::Length(1), // rule
            Constraint::Length(1), // column header
            Constraint::Min(0),    // rows
        ])
        .split(content_area);

    render_unit_list_heading(frame, sections[0], units.len());
    frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
    render_unit_list_column_header(frame, sections[2]);
    render_unit_rows(frame, sections[3], units);

    let rows_scrollbar_area = Rect {
        y: sections[3].y,
        height: sections[3].height,
        ..scrollbar_column
    };
    let mut scrollbar_state = ScrollbarState::new(FAKE_UNIT_TOTAL)
        .viewport_content_length(units.len())
        .position(0);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None);
    frame.render_stateful_widget(scrollbar, rows_scrollbar_area, &mut scrollbar_state);
}

/// The "UNITS · N OF total" heading, echoing the summary heading's dim label / dim tag
/// pattern.
fn render_unit_list_heading(frame: &mut Frame, area: Rect, visible: usize) {
    let tag = format!("{visible} OF {FAKE_UNIT_TOTAL}");
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled("UNITS", dim)), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The `CODE`/`TYPE`/`LAST` column header row, dim, per §4a's column set.
fn render_unit_list_column_header(frame: &mut Frame, area: Rect) {
    let columns = unit_row_columns(area);
    let dim = Style::default().add_modifier(Modifier::DIM);

    frame.render_widget(Paragraph::new(Span::styled("CODE", dim)), columns[0]);
    frame.render_widget(Paragraph::new(Span::styled("TYPE", dim)), columns[1]);
    frame.render_widget(
        Paragraph::new(Span::styled("LAST", dim)).alignment(Alignment::Right),
        columns[2],
    );
}

/// One row per unit, top to bottom.
fn render_unit_rows(frame: &mut Frame, area: Rect, units: &[UnitListRow]) {
    // Capped to however many rows actually fit `area` — asking `Layout::split` for more
    // `Length(1)` rows than available height makes it visibly skip/overlap rows rather than
    // truncate cleanly.
    let visible = units.len().min(area.height as usize);
    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), visible).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    for (unit, row) in units.iter().zip(rows.iter()) {
        render_unit_row(frame, *row, unit);
    }
}

/// One unit row: `CODE`/`TYPE`/`LAST` per §4a's column set. Type and last are dim unless the
/// row is selected; the code is dim only when the unit is inactive ("inactive units render
/// dim throughout"). The selected row reverses instead, per the shell's "reversed for the
/// selected row" style role.
fn render_unit_row(frame: &mut Frame, area: Rect, unit: &UnitListRow) {
    if unit.selected {
        frame.render_widget(
            Block::new().style(Style::default().add_modifier(Modifier::REVERSED)),
            area,
        );
    }

    let dim = Style::default().add_modifier(Modifier::DIM);
    let muted = if unit.selected { Style::default() } else { dim };
    let code_style = if unit.inactive { dim } else { Style::default() };

    let columns = unit_row_columns(area);
    frame.render_widget(
        Paragraph::new(Span::styled(unit.code, code_style)),
        columns[0],
    );
    frame.render_widget(Paragraph::new(Span::styled(unit.kind, muted)), columns[1]);
    frame.render_widget(
        Paragraph::new(Span::styled(unit.last, muted)).alignment(Alignment::Right),
        columns[2],
    );
}

/// Splits a unit list row (or its column header) into `CODE` / `TYPE` (fill) / `LAST`
/// columns, per §4a's column set.
fn unit_row_columns(area: Rect) -> [Rect; 3] {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(UNIT_CODE_WIDTH),
            Constraint::Min(0),
            Constraint::Length(UNIT_LAST_WIDTH),
        ])
        .spacing(1)
        .split(area);
    [columns[0], columns[1], columns[2]]
}

/// One unit's fake summary detail. All fake data: no unit selection or `lib-database` access
/// wired up yet, per this module's wireframe stage.
struct UnitSummary {
    code: &'static str,
    /// The type tag shown right-aligned on the code row, e.g. `"etf"`.
    kind: &'static str,
    name: &'static str,
    /// The three highlighted, bold, right-aligned figures below the name — units held, last
    /// market price, and last market value.
    figures: [SummaryFigure; SUMMARY_FIGURE_COUNT],
    /// Secondary label/value pairs rendered in order, one per row, below the figures.
    fields: [(&'static str, &'static str); SUMMARY_FIELD_COUNT],
}

/// One highlighted figure at the top of the summary box: a dim label on the left, a bold
/// value — with an optional dim suffix, e.g. the currency code — right-aligned.
struct SummaryFigure {
    label: &'static str,
    value: &'static str,
    suffix: Option<&'static str>,
}

/// The fake summary behind the summary box.
fn fake_summary() -> UnitSummary {
    UnitSummary {
        code: "VDHG",
        kind: "etf",
        name: "Vanguard Diversified High Growth",
        figures: [
            SummaryFigure {
                label: "UNITS HELD",
                value: FAKE_UNITS_HELD,
                suffix: None,
            },
            SummaryFigure {
                label: "LAST MARKET PRICE",
                value: "72.41",
                suffix: Some("AUD"),
            },
            SummaryFigure {
                label: "LAST MARKET VALUE",
                value: "40 637.79",
                suffix: None,
            },
        ],
        fields: [
            ("priced", "31 aug · weekly close"),
            ("week change", "+0.42% +169.87"),
            ("52w range", "64.18 – 73.90"),
            ("symbol · priced in", "VDHG.AX · AUD"),
            ("precision", "3 · price 4"),
            ("accounts", "1 · Index Fund"),
        ],
    }
}

/// The summary section (bottom left): a "SUMMARY" heading over a bordered `Block`, echoing
/// the `.hd` + `.box` pair in §4a's design reference — the box carries no title of its own,
/// since the heading above it already names the section.
fn render_summary(frame: &mut Frame, area: Rect, summary: &UnitSummary) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    render_summary_heading(frame, sections[0]);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
    render_summary_box(frame, sections[2], summary);
}

/// The heading above the summary box: "SUMMARY", dim.
fn render_summary_heading(frame: &mut Frame, area: Rect) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled("SUMMARY", dim)), area);
}

/// The bordered `Block` beneath the summary heading: code and type on the first line, the
/// unit's name on the second, a rule, the three highlighted figures, another rule, then the
/// secondary label/value fields. Labels render dim, per the shell's "dim for labels" style
/// role.
fn render_summary_box(frame: &mut Frame, area: Rect, summary: &UnitSummary) {
    let block = Block::bordered().padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), SUMMARY_BOX_CONTENT_HEIGHT as usize).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);

    render_code_and_kind(frame, rows[0], summary.code, summary.kind);
    frame.render_widget(Paragraph::new(summary.name), rows[1]);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[2]);

    for (index, figure) in summary.figures.iter().enumerate() {
        render_summary_figure(frame, rows[3 + index], figure);
    }
    frame.render_widget(
        Block::new().borders(Borders::BOTTOM),
        rows[3 + summary.figures.len()],
    );

    let dim = Style::default().add_modifier(Modifier::DIM);
    let fields_start = 3 + summary.figures.len() + 1;
    for (index, (label, value)) in summary.fields.iter().enumerate() {
        frame.render_widget(
            summary_field_line(label, value, dim),
            rows[fields_start + index],
        );
    }
}

/// One highlighted figure row: a dim label on the left, a bold value — with an optional dim
/// suffix, e.g. the currency code — right-aligned.
fn render_summary_figure(frame: &mut Frame, area: Rect, figure: &SummaryFigure) {
    let value_width =
        figure.value.chars().count() + figure.suffix.map_or(0, |suffix| suffix.chars().count() + 1);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(value_width as u16)])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled(figure.label, dim)), columns[0]);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let mut value = vec![Span::styled(figure.value, bold)];
    if let Some(suffix) = figure.suffix {
        value.push(Span::raw(" "));
        value.push(Span::styled(suffix, dim));
    }
    frame.render_widget(
        Paragraph::new(Line::from(value)).alignment(Alignment::Right),
        columns[1],
    );
}

/// One `label   value` row, the label padded to `SUMMARY_LABEL_WIDTH` and dimmed.
fn summary_field_line<'a>(label: &'a str, value: &'a str, label_style: Style) -> Paragraph<'a> {
    Paragraph::new(Line::from(vec![
        Span::styled(format!("{label:<SUMMARY_LABEL_WIDTH$}"), label_style),
        Span::raw(value),
    ]))
}

/// The code/type row: the unit code left, its type tag (`"etf"`, `"currency"`, ...)
/// right-aligned and dimmed.
fn render_code_and_kind(frame: &mut Frame, area: Rect, code: &str, kind: &str) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(kind.chars().count() as u16),
        ])
        .split(area);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    frame.render_widget(Paragraph::new(Span::styled(code, bold)), columns[0]);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled(kind, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The right column: the weekly close candlestick above the weekly prices table, pagination
/// row and command hints, per §4a ("the rest is price history").
fn render_price_history(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(WEEKLY_CLOSE_HEIGHT),
            Constraint::Length(1), // blank spacer
            Constraint::Min(0),
            Constraint::Length(KEYBIND_HINTS_HEIGHT),
            Constraint::Length(1), // blank spacer, above the shell's footer
        ])
        .split(area);

    render_weekly_close(frame, rows[0], &fake_weekly_candles());
    // rows[1] is left blank — breathing space between weekly close and weekly prices.
    render_weekly_prices(frame, rows[2], &fake_weekly_prices());
    render_keybind_hints(frame, rows[3]);
    // rows[4] is left blank — breathing space between the hints box and the shell footer.
}

/// One week's fake close/change/value row for the weekly prices table, newest week first.
struct WeeklyPriceRow {
    /// The week-commencing date, e.g. `"31 aug"`.
    week_commencing: String,
    close: String,
    /// The signed weekly change, e.g. `"+0.42"` or `"-0.61"`.
    change_percent: String,
    market_value: String,
    /// A negative week renders `change_percent` in the accent colour.
    is_negative: bool,
}

/// The fake weekly prices behind the table. The first ten rows are §4a's own worked example
/// (VDHG), verbatim; the rest are generated continuing backward from there (deterministic
/// xorshift PRNG, the same technique as `fake_weekly_candles`) out to `FAKE_WEEK_COUNT`, so
/// the table fills any reasonably tall terminal instead of trailing off into blank rows.
fn fake_weekly_prices() -> Vec<WeeklyPriceRow> {
    let mut rows: Vec<WeeklyPriceRow> = [
        ("31 aug", "72.41", "+0.42", "40 637.79", false),
        ("24 aug", "72.11", "-0.61", "40 469.42", true),
        ("17 aug", "72.55", "+1.08", "40 716.34", false),
        ("10 aug", "71.78", "+0.94", "40 284.20", false),
        ("03 aug", "71.11", "-0.28", "39 908.16", true),
        ("27 jul", "71.31", "+1.44", "40 020.42", false),
        ("20 jul", "70.30", "+0.63", "39 453.63", false),
        ("13 jul", "69.86", "-0.90", "39 206.69", true),
        ("06 jul", "70.49", "+2.11", "39 560.28", false),
        ("29 jun", "69.03", "+0.34", "38 740.93", false),
    ]
    .into_iter()
    .map(
        |(week_commencing, close, change_percent, market_value, is_negative)| WeeklyPriceRow {
            week_commencing: week_commencing.to_string(),
            close: close.to_string(),
            change_percent: change_percent.to_string(),
            market_value: market_value.to_string(),
            is_negative,
        },
    )
    .collect();

    let mut week_commencing =
        chrono::NaiveDate::from_ymd_opt(2025, 6, 29).expect("29 june 2025 is a valid date");
    let mut close = 69.03_f64;
    let mut seed: u64 = 29;
    for _ in rows.len()..FAKE_WEEK_COUNT {
        week_commencing -= chrono::Duration::weeks(1);

        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let change_percent = (seed % 400) as f64 / 100.0 - 2.0; // -2.00..=2.00
        let previous_close = (close / (1.0 + change_percent / 100.0)).max(1.0);

        rows.push(WeeklyPriceRow {
            week_commencing: week_commencing.format("%d %b").to_string().to_lowercase(),
            close: format!("{previous_close:.2}"),
            change_percent: format!("{change_percent:+.2}"),
            market_value: format_market_value(previous_close * FAKE_UNITS_HELD_QTY),
            is_negative: change_percent < 0.0,
        });

        close = previous_close;
    }

    rows
}

/// Formats a dollar amount with a space thousands separator and two decimal places, e.g.
/// `40637.79` -> `"40 637.79"`, matching the weekly prices table's own fake figures.
fn format_market_value(amount: f64) -> String {
    let whole = amount.trunc().abs() as i64;
    let cents = ((amount.abs() - whole as f64) * 100.0).round() as u32;

    let digits = whole.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().rev().enumerate() {
        if index != 0 && index % 3 == 0 {
            grouped.push(' ');
        }
        grouped.push(ch);
    }
    let grouped: String = grouped.chars().rev().collect();

    format!("{grouped}.{cents:02}")
}

/// The weekly prices table (middle right): a "WEEKLY PRICES · W/C MONDAY · N UNITS" heading
/// over a `W/C`/`CLOSE`/`Δ%`/`MARKET VALUE` table, newest week first, with a scrollbar on the
/// right edge since more weeks exist than fit on screen (`FAKE_WEEK_COUNT`). No border,
/// echoing the unit list's borderless pane convention.
fn render_weekly_prices(frame: &mut Frame, area: Rect, rows: &[WeeklyPriceRow]) {
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .spacing(1)
        .split(area);
    let content_area = split[0];
    let scrollbar_column = split[1];

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // heading
            Constraint::Length(1), // rule
            Constraint::Length(1), // column header
            Constraint::Min(0),    // rows
        ])
        .split(content_area);

    render_weekly_prices_heading(frame, sections[0]);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
    render_weekly_prices_column_header(frame, sections[2]);
    render_weekly_price_rows(frame, sections[3], rows);

    let rows_scrollbar_area = Rect {
        y: sections[3].y,
        height: sections[3].height,
        ..scrollbar_column
    };
    let mut scrollbar_state = ScrollbarState::new(FAKE_WEEK_COUNT)
        .viewport_content_length(rows.len())
        .position(0);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None);
    frame.render_stateful_widget(scrollbar, rows_scrollbar_area, &mut scrollbar_state);
}

/// The "WEEKLY PRICES · W/C MONDAY · N UNITS" heading, echoing the unit list and summary
/// headings' dim label / dim tag pattern.
fn render_weekly_prices_heading(frame: &mut Frame, area: Rect) {
    let tag = format!("W/C MONDAY · {FAKE_UNITS_HELD} UNITS");
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled("WEEKLY PRICES", dim)),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The `W/C`/`CLOSE`/`Δ%`/`MARKET VALUE` column header row, dim.
fn render_weekly_prices_column_header(frame: &mut Frame, area: Rect) {
    let columns = weekly_price_columns(area);
    let dim = Style::default().add_modifier(Modifier::DIM);

    frame.render_widget(Paragraph::new(Span::styled("W/C", dim)), columns[0]);
    frame.render_widget(Paragraph::new(Span::styled("CLOSE", dim)), columns[1]);
    frame.render_widget(Paragraph::new(Span::styled("Δ%", dim)), columns[2]);
    frame.render_widget(
        Paragraph::new(Span::styled("MARKET VALUE", dim)).alignment(Alignment::Right),
        columns[3],
    );
}

/// One row per week, newest first; the first (most recent) row renders reversed, per the
/// shell's "reversed for the selected row" style role. Capped to however many rows actually
/// fit `area` — asking `Layout::split` for more `Length(1)` rows than available height makes
/// it visibly skip/overlap rows rather than truncate cleanly, and the scrollbar already
/// signals that more weeks exist than fit on screen.
fn render_weekly_price_rows(frame: &mut Frame, area: Rect, rows: &[WeeklyPriceRow]) {
    let visible = rows.len().min(area.height as usize);
    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), visible).collect();
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    for (index, (row, row_area)) in rows.iter().zip(row_areas.iter()).enumerate() {
        render_weekly_price_row(frame, *row_area, row, index == 0);
    }
}

/// One weekly price row: `W/C`/`CLOSE`/`Δ%`/`MARKET VALUE` per the pasted design reference. A
/// negative `Δ%` renders in the accent colour; the selected (most recent) row reverses.
fn render_weekly_price_row(frame: &mut Frame, area: Rect, row: &WeeklyPriceRow, selected: bool) {
    if selected {
        frame.render_widget(
            Block::new().style(Style::default().add_modifier(Modifier::REVERSED)),
            area,
        );
    }

    let change_style = if row.is_negative {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };

    let columns = weekly_price_columns(area);
    frame.render_widget(Paragraph::new(row.week_commencing.as_str()), columns[0]);
    frame.render_widget(Paragraph::new(row.close.as_str()), columns[1]);
    frame.render_widget(
        Paragraph::new(Span::styled(row.change_percent.as_str(), change_style)),
        columns[2],
    );
    frame.render_widget(
        Paragraph::new(row.market_value.as_str()).alignment(Alignment::Right),
        columns[3],
    );
}

/// Splits a weekly price row (or its column header) into `W/C` / `CLOSE` / `Δ%` /
/// `MARKET VALUE` (fill, right-aligned) columns.
fn weekly_price_columns(area: Rect) -> [Rect; 4] {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(WEEKLY_PRICE_DATE_WIDTH),
            Constraint::Length(WEEKLY_PRICE_CLOSE_WIDTH),
            Constraint::Length(WEEKLY_PRICE_CHANGE_WIDTH),
            Constraint::Min(0),
        ])
        .spacing(2)
        .split(area);
    [columns[0], columns[1], columns[2], columns[3]]
}

/// The command/keybind hints box (bottom right): the single-key hints for navigating and
/// acting on the highlighted unit, then the `:` commands that do the same from the command
/// line — how to use the units screen at a glance.
fn render_keybind_hints(frame: &mut Frame, area: Rect) {
    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), 3 + COMMAND_HINTS.len()).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled("KEYS · COMMANDS", dim)),
        rows[0],
    );
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

    render_key_hints(frame, rows[2]);

    for (index, line) in COMMAND_HINTS.iter().enumerate() {
        frame.render_widget(Paragraph::new(Span::styled(*line, dim)), rows[3 + index]);
    }
}

/// `KEY_HINTS` as one line: each key bold, its label dim, separated by a dim `·`.
fn render_key_hints(frame: &mut Frame, area: Rect) {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().add_modifier(Modifier::DIM);

    let mut spans = Vec::new();
    for (index, (key, label)) in KEY_HINTS.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  ·  ", dim));
        }
        spans.push(Span::styled(*key, bold));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*label, dim));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// The weekly close candlestick chart (top right): `chandelier`'s `CandlestickChart`
/// (ADR-0002, `crates/bins/bin-tui/src/screen/candlestick_chart.rs`) over deterministic fake
/// weekly OHLC data — real price history isn't wired up yet.
fn render_weekly_close(frame: &mut Frame, area: Rect, candles: &[Candle]) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // heading
            Constraint::Length(1), // rule
            Constraint::Min(0),    // chart
        ])
        .split(area);

    render_weekly_close_heading(frame, rows[0]);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

    let chart = CandlestickChart::new(CandleSeries::new(candles));
    frame.render_widget(chart, rows[2]);
}

/// The "WEEKLY CLOSE · VDHG" heading, echoing the other boxes' dim label / dim tag pattern.
fn render_weekly_close_heading(frame: &mut Frame, area: Rect) {
    let tag = "SEP 25 – SEP 26 · CANDLESTICK";
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled("WEEKLY CLOSE · VDHG", dim)),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// Deterministic fake weekly OHLC data for the weekly close chart — a slow upward random
/// walk from roughly the summary's own `52w range` low toward its `last price`. Same tiny
/// xorshift PRNG technique as `dashboard.rs`'s fake net-worth series, so results are stable
/// across runs and platforms.
fn fake_weekly_candles() -> Vec<Candle> {
    let mut close = 64.18_f64;
    let mut seed: u64 = 7;
    (0..FAKE_WEEK_COUNT)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let wobble = (seed % 200) as f64 / 100.0 - 1.0; // -1.0..=1.0
            let open = close;
            close = (open + 0.16 + wobble * 0.6).max(1.0);
            let spread = (seed % 50) as f64 / 100.0;
            let high = open.max(close) + spread;
            let low = (open.min(close) - spread).max(0.5);
            Candle::new(open, high, low, close)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn render(view: &UnitsView) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("drawing the units view should not error");

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
        render(&UnitsView::new());
    }

    #[test]
    fn shows_all_four_region_boxes() {
        let text = render(&UnitsView::new());

        assert!(text.contains("UNITS"), "unit list heading missing");
        assert!(text.contains("SUMMARY"), "summary heading missing");
        assert!(
            text.contains("WEEKLY CLOSE"),
            "weekly close heading missing"
        );
        assert!(
            text.contains("WEEKLY PRICES"),
            "weekly prices heading missing"
        );
    }

    #[test]
    fn weekly_close_box_shows_a_candlestick_chart() {
        let text = render(&UnitsView::new());

        assert!(text.contains("WEEKLY CLOSE · VDHG"), "heading missing");
        // Candle body / wick glyphs `chandelier` draws with — confirms an actual chart
        // rendered rather than a bare bordered block.
        assert!(
            text.contains(['█', '▄', '▀', '│', '╷', '╵']),
            "candlestick glyphs missing"
        );
    }

    #[test]
    fn weekly_prices_shows_the_heading_tag_column_header_and_the_worked_example_rows() {
        // Taller than the 96x30 minimum: the worked example's first ten weeks need more
        // height than the minimum leaves for the weekly prices table (the scrollbar covers
        // that case at minimum size — see `weekly_prices_shows_a_scrollbar`).
        let backend = TestBackend::new(96, 45);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| UnitsView::new().view(frame, frame.area()))
            .expect("drawing the units view should not error");
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }

        assert!(
            text.contains("W/C MONDAY · 561.204 UNITS"),
            "heading tag missing"
        );
        assert!(text.contains("W/C"), "W/C column header missing");
        assert!(text.contains("CLOSE"), "CLOSE column header missing");
        assert!(text.contains("Δ%"), "Δ% column header missing");
        assert!(
            text.contains("MARKET VALUE"),
            "MARKET VALUE column header missing"
        );

        // §4a's own worked example — the first ten rows `fake_weekly_prices` starts from,
        // verbatim, before it continues generating earlier weeks.
        for row in fake_weekly_prices().into_iter().take(10) {
            assert!(
                text.contains(row.week_commencing.as_str()),
                "{}'s date missing",
                row.week_commencing
            );
            assert!(
                text.contains(row.close.as_str()),
                "{}'s close missing",
                row.week_commencing
            );
            assert!(
                text.contains(row.change_percent.as_str()),
                "{}'s change missing",
                row.week_commencing
            );
            assert!(
                text.contains(row.market_value.as_str()),
                "{}'s market value missing",
                row.week_commencing
            );
        }
    }

    #[test]
    fn weekly_prices_table_fills_the_available_height_with_no_trailing_blank_rows() {
        // Tall enough that the ten worked-example rows alone wouldn't fill it — confirms
        // `fake_weekly_prices`'s generated rows (past the first ten) actually reach the
        // bottom of the table's own space rather than leaving it short.
        let backend = TestBackend::new(96, 60);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| UnitsView::new().view(frame, frame.area()))
            .expect("drawing the units view should not error");

        let buffer = terminal.backend().buffer();
        let row_text = |y: u16| -> String {
            let mut row = String::new();
            for x in 0..buffer.area.width {
                row.push_str(buffer[(x, y)].symbol());
            }
            row
        };
        let row_index = |needle: &str| -> u16 {
            (0..buffer.area.height)
                .find(|&y| row_text(y).contains(needle))
                .unwrap_or_else(|| panic!("no row contains {needle:?}"))
        };

        let column_header_row = row_index("MARKET VALUE");
        let keys_heading_row = row_index("KEYS · COMMANDS");
        let last_price_row = keys_heading_row - 1;

        assert!(
            last_price_row > column_header_row,
            "expected at least one weekly price row"
        );
        assert!(
            !row_text(last_price_row).trim().is_empty(),
            "the row right before the next section should still hold price data, not be blank"
        );
    }

    #[test]
    fn weekly_prices_reverses_the_first_row_and_accents_negative_change() {
        // Taller than the 96x30 minimum: at minimum size the weekly prices table now only has
        // room for one row (the candlestick chart and the spacer above the shell footer both
        // grew), too few to check a non-selected row and a negative one against a selected,
        // positive one.
        let backend = TestBackend::new(96, 45);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| UnitsView::new().view(frame, frame.area()))
            .expect("drawing the units view should not error");

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
        let row_has_accent =
            |y: u16| -> bool { (0..buffer.area.width).any(|x| buffer[(x, y)].fg == ACCENT) };

        assert!(
            row_is_reversed(row_containing("31 aug")),
            "most recent week's row should render reversed"
        );
        assert!(
            !row_is_reversed(row_containing("24 aug")),
            "non-selected row should not render reversed"
        );
        assert!(
            row_has_accent(row_containing("-0.61")),
            "negative change should render in the accent colour"
        );
        assert!(
            !row_has_accent(row_containing("+0.42")),
            "positive change should not render in the accent colour"
        );
    }

    #[test]
    fn weekly_prices_shows_a_scrollbar() {
        let text = render(&UnitsView::new());

        assert!(
            text.contains(['║', '█']),
            "weekly prices scrollbar track or thumb missing"
        );
    }

    #[test]
    fn keybind_hints_box_shows_keys_and_commands() {
        // Wider than the 96-col minimum's right column: the key hints no longer wrap onto a
        // second row (removed along with the border, to close the gap it left underneath), so
        // the full line needs more width to avoid truncating the last hint.
        let backend = TestBackend::new(160, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| UnitsView::new().view(frame, frame.area()))
            .expect("drawing the units view should not error");
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }

        assert!(text.contains("KEYS · COMMANDS"), "heading missing");
        for (key, _) in KEY_HINTS {
            assert!(text.contains(key), "{key} key hint missing");
        }
        // The full command lines are longer than the 96-col minimum's right column, so check
        // a prefix short enough to survive truncation rather than the whole line.
        assert!(text.contains(":unit new"), "unit-new command hint missing");
        assert!(
            text.contains(":unit edit VDHG"),
            "unit-edit command hint missing"
        );
        assert!(
            text.contains(":price set VDHG"),
            "price-set command hint missing"
        );
    }

    #[test]
    fn unit_list_shows_the_heading_tag_and_column_header() {
        let text = render(&UnitsView::new());

        assert!(text.contains("6 OF 20"), "unit count tag missing");
        assert!(text.contains("CODE"), "code column header missing");
        assert!(text.contains("TYPE"), "type column header missing");
        assert!(text.contains("LAST"), "last column header missing");
    }

    #[test]
    fn unit_list_shows_every_fake_row() {
        let text = render(&UnitsView::new());

        for unit in fake_units() {
            assert!(text.contains(unit.code), "{}'s code missing", unit.code);
            assert!(text.contains(unit.kind), "{}'s type missing", unit.code);
            assert!(
                text.contains(unit.last),
                "{}'s last price missing",
                unit.code
            );
        }
    }

    #[test]
    fn unit_list_reverses_the_selected_row_and_dims_the_inactive_one() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| UnitsView::new().view(frame, frame.area()))
            .expect("drawing the units view should not error");

        let buffer = terminal.backend().buffer();
        // Scoped to the left column only: "VDHG" also appears in the weekly close heading on
        // the right, and the candlestick chart legitimately uses `REVERSED` cells of its own
        // (sub-cell partial-block rendering) — both would otherwise be mistaken for unit list
        // row content/styling on a whole-buffer-width scan.
        let row_containing = |needle: &str| -> u16 {
            (0..buffer.area.height)
                .find(|&y| {
                    let mut row = String::new();
                    for x in 0..LEFT_COLUMN_WIDTH {
                        row.push_str(buffer[(x, y)].symbol());
                    }
                    row.contains(needle)
                })
                .unwrap_or_else(|| panic!("no row contains {needle:?}"))
        };
        let row_is_reversed = |y: u16| -> bool {
            (0..LEFT_COLUMN_WIDTH).any(|x| buffer[(x, y)].modifier.contains(Modifier::REVERSED))
        };
        let row_is_dim = |y: u16| -> bool {
            (0..LEFT_COLUMN_WIDTH).any(|x| buffer[(x, y)].modifier.contains(Modifier::DIM))
        };

        assert!(
            row_is_reversed(row_containing("VDHG")),
            "selected row should render reversed"
        );
        assert!(
            !row_is_reversed(row_containing("AUD")),
            "non-selected row should not render reversed"
        );
        assert!(
            row_is_dim(row_containing("AAPL")),
            "inactive row should render dim throughout"
        );
    }

    #[test]
    fn unit_list_shows_a_scrollbar() {
        let text = render(&UnitsView::new());

        // The double-vertical symbol set's track/thumb glyphs — confirms the `Scrollbar`
        // widget actually rendered rather than leaving the column blank.
        assert!(
            text.contains(['║', '█']),
            "scrollbar track or thumb missing"
        );
    }

    #[test]
    fn extra_terminal_height_grows_the_unit_list_not_the_summary_box() {
        // Taller than the 96x30 minimum: the summary section should stay pinned to its exact
        // content height at the bottom of the left column, with the unit list box above it
        // absorbing all of the extra height instead.
        let height = 45;
        let backend = TestBackend::new(96, height);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| UnitsView::new().view(frame, frame.area()))
            .expect("drawing the units view should not error");

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

        let summary_heading_row = row_containing("SUMMARY");
        assert_eq!(
            summary_heading_row,
            height - SUMMARY_SECTION_HEIGHT,
            "summary section should be pinned to its fixed height at the bottom of the column"
        );

        // No blank rows between the last field row and the summary box's bottom border — the
        // section's last row is that border, since the section's fixed height is exactly
        // heading + rule + box border + content rows.
        let last_field_row = row_containing("1 · Index Fund");
        let box_bottom_border = summary_heading_row + SUMMARY_SECTION_HEIGHT - 1;
        assert_eq!(
            last_field_row + 1,
            box_bottom_border,
            "summary box should end immediately after the last field row, with no blank rows \
             between"
        );
    }

    #[test]
    fn summary_shows_code_kind_name_and_all_fields() {
        let text = render(&UnitsView::new());

        assert!(text.contains("VDHG"), "unit code missing");
        assert!(text.contains("etf"), "type tag missing");
        assert!(
            text.contains("Vanguard Diversified High Growth"),
            "unit name missing"
        );
        for (label, value) in fake_summary().fields {
            assert!(text.contains(label), "{label} label missing");
            assert!(text.contains(value), "{label}'s value missing");
        }
        for figure in fake_summary().figures {
            assert!(
                text.contains(figure.label),
                "{} label missing",
                figure.label
            );
            assert!(
                text.contains(figure.value),
                "{}'s value missing",
                figure.label
            );
            if let Some(suffix) = figure.suffix {
                assert!(text.contains(suffix), "{}'s suffix missing", figure.label);
            }
        }
    }

    #[test]
    fn summary_box_has_a_rule_around_the_highlighted_figures() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| UnitsView::new().view(frame, frame.area()))
            .expect("drawing the units view should not error");

        let buffer = terminal.backend().buffer();
        let row_index = |needle: &str| -> usize {
            (0..buffer.area.height)
                .position(|y| {
                    let mut row = String::new();
                    for x in 0..buffer.area.width {
                        row.push_str(buffer[(x, y)].symbol());
                    }
                    row.contains(needle)
                })
                .unwrap_or_else(|| panic!("no row contains {needle:?}"))
        };
        // Scoped to the left column: the row also carries the box's own left/right border and
        // padding, so this checks for a solid run of the rule glyph rather than every cell.
        let row_is_a_rule = |y: usize| -> bool {
            (0..LEFT_COLUMN_WIDTH)
                .filter(|&x| buffer[(x, y as u16)].symbol() == "─")
                .count()
                > 20
        };

        let name_row = row_index("Vanguard Diversified High Growth");
        let units_held_row = row_index("UNITS HELD");
        assert_eq!(
            units_held_row - name_row,
            2,
            "expected exactly one row between the name and the figures"
        );
        assert!(
            row_is_a_rule(name_row + 1),
            "expected a rule between the name and the figures"
        );

        let last_market_value_row = row_index("LAST MARKET VALUE");
        let priced_row = row_index("priced");
        assert_eq!(
            priced_row - last_market_value_row,
            2,
            "expected exactly one row between the figures and the secondary fields"
        );
        assert!(
            row_is_a_rule(last_market_value_row + 1),
            "expected a rule between the figures and the secondary fields"
        );
    }

    #[test]
    fn summary_figures_render_bold() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| UnitsView::new().view(frame, frame.area()))
            .expect("drawing the units view should not error");

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
        let row_has_bold = |y: u16| -> bool {
            (0..buffer.area.width).any(|x| buffer[(x, y)].modifier.contains(Modifier::BOLD))
        };

        for figure in fake_summary().figures {
            // Look up the row by its label, not its value — "72.41" also appears
            // (unbolded) in the unit list above, on the selected VDHG row.
            assert!(
                row_has_bold(row_containing(figure.label)),
                "{}'s figure should render bold",
                figure.label
            );
        }
    }

    #[test]
    fn title_is_units() {
        assert_eq!(UnitsView::new().title(), "Units");
    }
}
