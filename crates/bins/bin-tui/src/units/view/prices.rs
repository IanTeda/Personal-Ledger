//! The Unit prices view, drawn from `lib-units`' [`UnitPricesViewModel`] in place of the Units
//! View's columns: the title and window over the PRICE figures, the WEEKLY CLOSE chart, then the
//! WEEKLY PRICES table, newest week first, scrolled to keep the selected week in view.
//!
//! The chart is drawn cell by cell: a week with an open, high and low is a candle (a `│` wick
//! from high to low behind a `█` body from open to close, a falling week's in the negative
//! colour), and any other week a `─` tick at its close.

use lib_locale::format::upper;
use lib_units::presenter::{ChartWeek, PriceRow, UnitPricesViewModel};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::colours::Colours;

/// Height of the chart's plot, in rows.
const CHART_HEIGHT: u16 = 10;

/// Width of a fact's label column.
const FACT_LABEL_WIDTH: usize = 22;

/// Width of the table's WEEK column and of each price column.
const WEEK_WIDTH: u16 = 14;
const PRICE_WIDTH: u16 = 14;

const WICK: &str = "\u{2502}";
const BODY: &str = "\u{2588}";
const TICK: &str = "\u{2500}";

pub(super) fn render(frame: &mut Frame<'_>, area: Rect, view: &UnitPricesViewModel, c: &Colours) {
    let facts = u16::try_from(view.facts.len()).unwrap_or(0);
    let [_, title, _, facts_area, _, chart_area, _, table_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(facts),
        Constraint::Length(1),
        Constraint::Length(CHART_HEIGHT + 2),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(area);

    heading(
        frame,
        title,
        Span::styled(
            view.title.as_str(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        &view.window,
        c,
    );
    let dim = c.muted();
    for (index, fact) in view.facts.iter().enumerate() {
        let row = Rect {
            y: facts_area.y + u16::try_from(index).unwrap_or(0),
            height: 1,
            ..facts_area
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{:<FACT_LABEL_WIDTH$}", fact.label), dim),
                Span::raw(fact.value.as_str()),
            ])),
            row,
        );
    }

    let [chart_heading, chart_rule, plot] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(chart_area);
    heading(
        frame,
        chart_heading,
        Span::styled(upper(&view.chart_heading), dim),
        "",
        c,
    );
    frame.render_widget(Block::new().borders(Borders::BOTTOM), chart_rule);

    let [table_heading, table_rule, body] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(table_area);
    heading(
        frame,
        table_heading,
        Span::styled(upper(&view.table_heading), dim),
        "",
        c,
    );
    frame.render_widget(Block::new().borders(Borders::BOTTOM), table_rule);

    if let Some(empty) = &view.empty {
        frame.render_widget(Paragraph::new(Span::styled(empty.as_str(), dim)), plot);
        frame.render_widget(Paragraph::new(Span::styled(empty.as_str(), dim)), body);
        return;
    }
    chart(frame.buffer_mut(), plot, &view.chart, c);
    table(frame, body, view, c);
}

/// A heading row: `left` and a dim `right` at the far edge.
fn heading(frame: &mut Frame<'_>, area: Rect, left: Span<'_>, right: &str, c: &Colours) {
    let [left_area, right_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(u16::try_from(right.chars().count()).unwrap_or(0)),
    ])
    .areas(area);
    frame.render_widget(Paragraph::new(left), left_area);
    frame.render_widget(
        Paragraph::new(Span::styled(right.to_string(), c.muted())).alignment(Alignment::Right),
        right_area,
    );
}

/// Plots `weeks` across `area`, one column per week where they fit, scaled to the window's lowest
/// low and highest high.
fn chart(buffer: &mut Buffer, area: Rect, weeks: &[ChartWeek], c: &Colours) {
    if weeks.is_empty() || area.width == 0 || area.height == 0 {
        return;
    }
    let low = weeks
        .iter()
        .map(|week| week.low)
        .fold(f64::INFINITY, f64::min);
    let high = weeks
        .iter()
        .map(|week| week.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let span = (high - low).max(f64::EPSILON);
    let rows = f64::from(area.height - 1);
    let y = |value: f64| {
        let offset = ((high - value) / span * rows).round().clamp(0.0, rows);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..=rows, a u16"
        )]
        let offset = offset as u16;
        area.y + offset
    };
    let count = weeks.len();
    let width = usize::from(area.width);
    for (index, week) in weeks.iter().enumerate() {
        // Spread evenly; when there are more weeks than columns, a later week overdraws.
        let column = index * width / count;
        let x = area.x + u16::try_from(column).unwrap_or(0);
        let falling = week.close < week.open;
        let style = if falling {
            c.negative()
        } else {
            Style::default()
        };
        if !week.candle {
            buffer.set_string(x, y(week.close), TICK, style);
            continue;
        }
        for row in y(week.high)..=y(week.low) {
            buffer.set_string(x, row, WICK, c.muted());
        }
        for row in y(week.open.max(week.close))..=y(week.open.min(week.close)) {
            buffer.set_string(x, row, BODY, style);
        }
    }
}

/// The weekly prices table: a header, then the rows that fit, scrolled to show the selection.
fn table(frame: &mut Frame<'_>, area: Rect, view: &UnitPricesViewModel, c: &Colours) {
    let [header, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let columns = &view.columns;
    let dim = c.muted();
    let labels = [
        &columns.week,
        &columns.open,
        &columns.high,
        &columns.low,
        &columns.close,
        &columns.change,
    ];
    for (index, (label, cell)) in labels.into_iter().zip(cells(header)).enumerate() {
        let alignment = if index == 0 {
            Alignment::Left
        } else {
            Alignment::Right
        };
        frame.render_widget(
            Paragraph::new(Span::styled(upper(label), dim)).alignment(alignment),
            cell,
        );
    }

    let visible = usize::from(body.height);
    let selected = view.rows.iter().position(|row| row.selected);
    let offset = match selected {
        Some(index) if visible > 0 && index >= visible => index + 1 - visible,
        _ => 0,
    };
    for (index, row) in view.rows.iter().skip(offset).take(visible).enumerate() {
        let area = Rect {
            y: body.y + u16::try_from(index).unwrap_or(0),
            height: 1,
            ..body
        };
        price_row(frame, area, row, c);
    }
}

fn price_row(frame: &mut Frame<'_>, area: Rect, row: &PriceRow, c: &Colours) {
    if row.selected {
        frame.render_widget(Block::new().style(c.selection()), area);
    }
    let change = if row.falling {
        if row.selected {
            c.negative_on_selection()
        } else {
            c.negative()
        }
    } else {
        Style::default()
    };
    let texts = [
        (row.label.as_str(), Style::default()),
        (row.open.as_str(), Style::default()),
        (row.high.as_str(), Style::default()),
        (row.low.as_str(), Style::default()),
        (
            row.close.as_str(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        (row.change.as_str(), change),
    ];
    for (index, ((text, style), cell)) in texts.into_iter().zip(cells(area)).enumerate() {
        let alignment = if index == 0 {
            Alignment::Left
        } else {
            Alignment::Right
        };
        frame.render_widget(
            Paragraph::new(Span::styled(text, style)).alignment(alignment),
            cell,
        );
    }
}

/// WEEK, OPEN, HIGH, LOW, CLOSE and CHANGE, left to right.
fn cells(area: Rect) -> [Rect; 6] {
    Layout::horizontal([
        Constraint::Length(WEEK_WIDTH),
        Constraint::Length(PRICE_WIDTH),
        Constraint::Length(PRICE_WIDTH),
        Constraint::Length(PRICE_WIDTH),
        Constraint::Length(PRICE_WIDTH),
        Constraint::Length(PRICE_WIDTH),
    ])
    .spacing(1)
    .areas(area)
}
