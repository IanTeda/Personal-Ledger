//! The Settings `View`, hosted by `Shell` (ADR-0013). Wireframe stage: the two-pane layout
//! from `docs/ux/tui/settings/README.md` §4a ("Settings at rest") — a fixed 28-col left pane
//! (groups list, "where values live" box, reset block) beside a `Min(0)` right pane (settings
//! list, selected explainer, settings table, command hint row) — so the overall pane structure
//! and proportions can be checked before the database-backed registry, in-place editor (§4b)
//! and base-unit guard (§4c) are built out.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::view::{Action, View};

/// The theme's one accent colour, per `docs/ux/tui/README.md`'s style table — used here for
/// the override dot, the "3 overridden" figure and a setting's `consequence` warning.
const ACCENT: Color = Color::Red;

/// Width of the left pane — "left 28 cols fixed" per §4a's terminal geometry.
const LEFT_COLUMN_WIDTH: u16 = 28;

/// Rows inside the groups list: a "GROUPS" heading, its rule, then one row per group.
const GROUPS_SECTION_HEIGHT: u16 = 1 + 1 + GROUPS.len() as u16;

/// Rows inside the "where values live" box: a heading (carrying the `H log` tag), its rule,
/// then the nine lines of §4a's own worked example — title, rule, three override/default/
/// last-commit facts, rule, then the two bootstrap-config lines.
const WHERE_VALUES_SECTION_HEIGHT: u16 = 1 + 1 + 9;

/// Rows inside the reset block: a heading (carrying the "deletes the row" note), its rule,
/// then the `r`/`R` key hint lines.
const RESET_SECTION_HEIGHT: u16 = 1 + 1 + 2;

/// Rows inside the settings list: a heading, its rule, the column header, then one row per
/// fake `general` setting.
const SETTINGS_LIST_HEIGHT: u16 = 1 + 1 + 1 + SETTINGS.len() as u16;

/// Rows inside the "selected" box: a heading, its rule, one line of `explain` prose, a rule,
/// then the three ruled facts (`default`, `accepts`, `changing it`).
const SELECTED_SECTION_HEIGHT: u16 = 1 + 1 + 1 + 1 + 3;

/// Rows inside the "settings table" block: a heading (carrying the "N rows · M shown" tag),
/// its rule, the column header, then the rows actually shown — truncated per §4a's own "3
/// rows · 2 shown" worked example, the scrollbar signalling the rest.
const SETTINGS_TABLE_SHOWN: usize = 2;
const SETTINGS_TABLE_HEIGHT: u16 = 1 + 1 + 1 + SETTINGS_TABLE_SHOWN as u16;

/// Width of the settings list's 1-col override gutter — `·` in the accent where a row exists
/// in `settings`, blank otherwise.
const GUTTER_WIDTH: u16 = 1;

/// Width of the settings list's `SETTING` column, per §4a's column set.
const SETTING_LABEL_WIDTH: u16 = 18;

/// Width of the settings list's `VALUE` column, per §4a's column set.
const SETTING_VALUE_WIDTH: u16 = 14;

/// Label column width inside the "where values live" box, wide enough for its longest label
/// (`"last commit"`) plus a run of spaces before the value starts — the same fixed-column
/// technique as `view::units`'s `SUMMARY_LABEL_WIDTH`.
const WHERE_VALUES_LABEL_WIDTH: usize = 14;

/// Label column width inside the "selected" box's facts, wide enough for its longest label
/// (`"changing it"`).
const SELECTED_FACT_LABEL_WIDTH: usize = 13;

/// A trivial placeholder Settings `View`: the §4a "at rest" wireframe over static fake data —
/// no database-backed registry, no in-place editor, no base-unit guard yet.
#[derive(Default)]
pub struct SettingsView;

impl SettingsView {
    pub fn new() -> Self {
        Self
    }
}

impl View for SettingsView {
    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        // rows[0] is left blank — breathing space between the shell's title bar and the
        // panes, matching the dashboard's and units view's own spacer rows.
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(LEFT_COLUMN_WIDTH), Constraint::Min(0)])
            .spacing(2)
            .split(rows[1]);

        render_left_pane(frame, columns[0]);
        render_right_pane(frame, columns[1]);
    }

    fn title(&self) -> &'static str {
        "Settings"
    }
}

/// The left pane, top to bottom: the groups list, the "where values live" box, the reset
/// block — §4a's own order.
fn render_left_pane(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(GROUPS_SECTION_HEIGHT),
            Constraint::Length(1), // spacer
            Constraint::Length(WHERE_VALUES_SECTION_HEIGHT),
            Constraint::Length(1), // spacer
            Constraint::Length(RESET_SECTION_HEIGHT),
        ])
        .split(area);

    render_groups(frame, rows[0]);
    render_where_values_live(frame, rows[2]);
    render_reset(frame, rows[4]);
}

/// One row in the groups list — a group `label` plus its right-aligned override-eligible
/// count, `"—"` for `about` (a view, not a group, per §4a).
struct GroupRow {
    label: &'static str,
    count: &'static str,
    selected: bool,
}

/// §4a's own worked example, verbatim — `general` selected, matching the status line's own
/// `settings / general` breadcrumb.
const GROUPS: &[GroupRow] = &[
    GroupRow {
        label: "general",
        count: "8",
        selected: true,
    },
    GroupRow {
        label: "display",
        count: "7",
        selected: false,
    },
    GroupRow {
        label: "units & prices",
        count: "6",
        selected: false,
    },
    GroupRow {
        label: "files & backup",
        count: "5",
        selected: false,
    },
    GroupRow {
        label: "reconcile",
        count: "4",
        selected: false,
    },
    GroupRow {
        label: "keys",
        count: "12",
        selected: false,
    },
    GroupRow {
        label: "about",
        count: "—",
        selected: false,
    },
];

/// The groups list: a "GROUPS" heading over the seven group rows, the selected row a
/// full-width reversed block per §4a.
fn render_groups(frame: &mut Frame, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // heading
            Constraint::Length(1), // rule
            Constraint::Min(0),    // rows
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled("GROUPS", dim)), sections[0]);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);

    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), GROUPS.len()).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(sections[2]);

    for (group, row) in GROUPS.iter().zip(rows.iter()) {
        render_group_row(frame, *row, group);
    }
}

/// One group row: label flush left, count right-aligned. The selected group (`general`)
/// reverses full width, per the shell's "reversed for the selected row" style role.
fn render_group_row(frame: &mut Frame, area: Rect, group: &GroupRow) {
    if group.selected {
        frame.render_widget(
            Block::new().style(Style::default().add_modifier(Modifier::REVERSED)),
            area,
        );
    }

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(group.count.chars().count() as u16),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(group.label), columns[0]);
    frame.render_widget(
        Paragraph::new(group.count).alignment(Alignment::Right),
        columns[1],
    );
}

/// One `label   value` line inside the "where values live" box, the value styled `ACCENT`
/// only for the `overrides` row — the "the red dot ... is the presence of the override" model
/// stated on screen, per §4a.
struct WhereValuesFact {
    label: &'static str,
    value: &'static str,
    accent: bool,
}

/// The "where values live" box: heading, rule, then §4a's own worked example verbatim — the
/// model stated on screen, because a user cannot otherwise tell where a value came from.
fn render_where_values_live(frame: &mut Frame, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // heading
            Constraint::Length(1), // rule
            Constraint::Min(0),    // content
        ])
        .split(area);

    render_heading(frame, sections[0], "WHERE VALUES LIVE", "H log");
    frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);

    let dim = Style::default().add_modifier(Modifier::DIM);
    let facts: [WhereValuesFact; 3] = [
        WhereValuesFact {
            label: "overrides",
            value: "3 rows",
            accent: true,
        },
        WhereValuesFact {
            label: "defaults",
            value: "42 in code",
            accent: false,
        },
        WhereValuesFact {
            label: "last commit",
            value: "14:02",
            accent: false,
        },
    ];
    let bootstrap = WhereValuesFact {
        label: "bootstrap",
        value: "4 keys",
        accent: false,
    };

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(Span::styled("ledger.db · table", dim)),
        Line::from("settings"),
        rule(sections[2].width),
    ];
    lines.extend(facts.iter().map(where_values_fact_line));
    lines.push(rule(sections[2].width));
    lines.push(where_values_fact_line(&bootstrap));
    lines.push(Line::from(Span::styled("ledger.toml · read-only", dim)));

    frame.render_widget(Paragraph::new(lines), sections[2]);
}

/// One `label   value` line, the label padded to `WHERE_VALUES_LABEL_WIDTH` and dimmed; the
/// value renders in `ACCENT` when `fact.accent` is set (the `overrides` row).
fn where_values_fact_line(fact: &WhereValuesFact) -> Line<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let value_style = if fact.accent {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(format!("{:<WHERE_VALUES_LABEL_WIDTH$}", fact.label), dim),
        Span::styled(fact.value, value_style),
    ])
}

/// The reset block: a "RESET" heading carrying the "deletes the row" note — the important
/// word, since it tells the user reset is a deletion, not a write — then the `r`/`R` key
/// hints, bold key / dim label matching the shell footer's own convention.
fn render_reset(frame: &mut Frame, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // heading
            Constraint::Length(1), // rule
            Constraint::Length(1), // "r  this setting"
            Constraint::Length(1), // "R  whole group"
        ])
        .split(area);

    render_heading(frame, sections[0], "RESET", "deletes the row");
    frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
    frame.render_widget(key_hint_line("r", "this setting"), sections[2]);
    frame.render_widget(key_hint_line("R", "whole group"), sections[3]);
}

/// One bold-key / dim-label line, matching `view::units`'s own `KEY_HINTS` convention.
fn key_hint_line(key: &'static str, label: &'static str) -> Paragraph<'static> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().add_modifier(Modifier::DIM);
    Paragraph::new(Line::from(vec![
        Span::styled(key, bold),
        Span::raw("  "),
        Span::styled(label, dim),
    ]))
}

/// A dim label / dim tag heading row, matching `view::units`'s own heading convention —
/// shared by every box in this view. `tag` need not be `'static` — the settings table
/// heading's own tag is computed fresh each render from `TABLE_ROWS.len()`.
fn render_heading(frame: &mut Frame, area: Rect, label: &str, tag: &str) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(tag.chars().count() as u16),
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled(label, dim)), columns[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(tag, dim)).alignment(Alignment::Right),
        columns[1],
    );
}

/// A full-width `─` rule, for the literal rule lines drawn inside the "where values live"
/// box's own text (as distinct from the `Borders::BOTTOM` rules separating each box's
/// heading from its content).
fn rule(width: u16) -> Line<'static> {
    Line::from("─".repeat(width as usize))
}

/// The right pane, top to bottom: the settings list, the "selected" explainer, the settings
/// table block, the command hint row — §4a's own order.
fn render_right_pane(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(SETTINGS_LIST_HEIGHT),
            Constraint::Length(1), // spacer
            Constraint::Length(SELECTED_SECTION_HEIGHT),
            Constraint::Length(1), // spacer
            Constraint::Length(SETTINGS_TABLE_HEIGHT),
            Constraint::Min(0),
        ])
        .split(area);

    render_settings_list(frame, rows[0]);
    render_selected(frame, rows[2]);
    render_settings_table(frame, rows[4]);
    render_command_hint(frame, rows[5]);
}

/// One row in the settings list — matches §4a's own eight `general` settings verbatim.
/// `VALUE` renders as the setting **will appear in the app**, not as it is stored.
struct SettingRow {
    /// `true` when a row exists in `settings` for this key (the override dot).
    overridden: bool,
    setting: &'static str,
    value: &'static str,
    note: &'static str,
    selected: bool,
}

/// §4a's own worked example: the eight `general` settings, `base unit` selected (the
/// `selected` box below explains it).
const SETTINGS: &[SettingRow] = &[
    SettingRow {
        overridden: true,
        setting: "base unit",
        value: "AUD",
        note: "every total converts to this",
        selected: true,
    },
    SettingRow {
        overridden: false,
        setting: "fiscal year starts",
        value: "01 jul",
        note: "drives year-to-date and reports",
        selected: false,
    },
    SettingRow {
        overridden: false,
        setting: "week starts",
        value: "monday",
        note: "w/c label on weekly prices",
        selected: false,
    },
    SettingRow {
        overridden: true,
        setting: "date input",
        value: "dd/mm/yyyy",
        note: "accepted when typing a date",
        selected: false,
    },
    SettingRow {
        overridden: false,
        setting: "number format",
        value: "1 234.56",
        note: "space groups · dot decimal",
        selected: false,
    },
    SettingRow {
        overridden: true,
        setting: "negatives",
        value: "−1 234.56",
        note: "minus · brackets · trailing",
        selected: false,
    },
    SettingRow {
        overridden: false,
        setting: "confirm deletes",
        value: "type the name",
        note: "off falls back to y/n",
        selected: false,
    },
    SettingRow {
        overridden: false,
        setting: "undo depth",
        value: "50 commands",
        note: "kept in the database",
        selected: false,
    },
];

/// The settings list: a "SETTINGS" heading tagged with the focused group and its count, over
/// the gutter/`SETTING`/`VALUE`/`NOTE` column set, per §4a.
fn render_settings_list(frame: &mut Frame, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // heading
            Constraint::Length(1), // rule
            Constraint::Length(1), // column header
            Constraint::Min(0),    // rows
        ])
        .split(area);

    render_heading(frame, sections[0], "SETTINGS", "GENERAL · 8");
    frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
    render_settings_column_header(frame, sections[2]);
    render_setting_rows(frame, sections[3]);
}

/// The gutter/`SETTING`/`VALUE`/`NOTE` column header row, dim — the gutter carries no label.
fn render_settings_column_header(frame: &mut Frame, area: Rect) {
    let columns = setting_row_columns(area);
    let dim = Style::default().add_modifier(Modifier::DIM);

    frame.render_widget(Paragraph::new(Span::styled("SETTING", dim)), columns[1]);
    frame.render_widget(Paragraph::new(Span::styled("VALUE", dim)), columns[2]);
    frame.render_widget(Paragraph::new(Span::styled("NOTE", dim)), columns[3]);
}

/// One row per `general` setting, capped to however many rows actually fit `area` — the same
/// defensive cap `view::units`'s own row renderers use.
fn render_setting_rows(frame: &mut Frame, area: Rect) {
    let visible = SETTINGS.len().min(area.height as usize);
    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), visible).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    for (setting, row) in SETTINGS.iter().zip(rows.iter()) {
        render_setting_row(frame, *row, setting);
    }
}

/// One setting row: the override gutter, `SETTING`, `VALUE` (dim unless selected) and `NOTE`
/// (always dim). The selected row (`base unit`) reverses full width.
fn render_setting_row(frame: &mut Frame, area: Rect, setting: &SettingRow) {
    if setting.selected {
        frame.render_widget(
            Block::new().style(Style::default().add_modifier(Modifier::REVERSED)),
            area,
        );
    }

    let dim = Style::default().add_modifier(Modifier::DIM);
    let muted = if setting.selected {
        Style::default()
    } else {
        dim
    };

    let columns = setting_row_columns(area);
    if setting.overridden {
        frame.render_widget(
            Paragraph::new(Span::styled("·", Style::default().fg(ACCENT))),
            columns[0],
        );
    }
    frame.render_widget(Paragraph::new(setting.setting), columns[1]);
    frame.render_widget(
        Paragraph::new(Span::styled(setting.value, muted)),
        columns[2],
    );
    frame.render_widget(Paragraph::new(Span::styled(setting.note, dim)), columns[3]);
}

/// Splits a settings list row (or its column header) into the gutter / `SETTING` / `VALUE` /
/// `NOTE` (fill) columns, per §4a's column set.
fn setting_row_columns(area: Rect) -> [Rect; 4] {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(GUTTER_WIDTH),
            Constraint::Length(SETTING_LABEL_WIDTH),
            Constraint::Length(SETTING_VALUE_WIDTH),
            Constraint::Min(0),
        ])
        .spacing(1)
        .split(area);
    [columns[0], columns[1], columns[2], columns[3]]
}

/// One `label   value` fact inside the "selected" box, `accent` set for the one fact that
/// names a `consequence` (`changing it`, present only where the registry entry has one).
struct SelectedFact {
    label: &'static str,
    value: &'static str,
    accent: bool,
}

/// The "selected" box's `explain` prose for `general.base_unit`, matching the registry
/// entry's own `explain` field — the two lines that name what the setting does before its
/// facts do.
const SELECTED_EXPLAIN: &str = "which unit every total, chart and report converts into";

/// The "selected" box's facts for `general.base_unit`, resolved against live data in the real
/// build (`accepts` is a query, not static text) — here, §4a's own worked example.
const SELECTED_FACTS: &[SelectedFact] = &[
    SelectedFact {
        label: "default",
        value: "USD",
        accent: false,
    },
    SelectedFact {
        label: "accepts",
        value: "any active currency unit — 2 available",
        accent: false,
    },
    SelectedFact {
        label: "changing it",
        value: "re-converts every historical total",
        accent: true,
    },
];

/// The "selected" box: a heading tagged with the highlighted setting, the `explain` prose,
/// then the ruled facts block — `default` / `accepts` / `changing it`, per §4a.
fn render_selected(frame: &mut Frame, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // heading
            Constraint::Length(1), // rule
            Constraint::Length(1), // explain
            Constraint::Length(1), // rule
            Constraint::Min(0),    // facts
        ])
        .split(area);

    render_heading(frame, sections[0], "SELECTED", "BASE UNIT");
    frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);
    frame.render_widget(Paragraph::new(SELECTED_EXPLAIN), sections[2]);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[3]);

    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), SELECTED_FACTS.len()).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(sections[4]);
    for (fact, row) in SELECTED_FACTS.iter().zip(rows.iter()) {
        frame.render_widget(selected_fact_line(fact), *row);
    }
}

/// One `label   value` line, the label padded to `SELECTED_FACT_LABEL_WIDTH` and dimmed; the
/// value renders in `ACCENT` when `fact.accent` is set (`changing it`).
fn selected_fact_line(fact: &SelectedFact) -> Paragraph<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let value_style = if fact.accent {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };
    Paragraph::new(Line::from(vec![
        Span::styled(format!("{:<SELECTED_FACT_LABEL_WIDTH$}", fact.label), dim),
        Span::styled(fact.value.to_string(), value_style),
    ]))
}

/// One row in the "settings table" block — the actual `settings` row behind a list entry,
/// rendered in its **stored** form (unlike the settings list's `VALUE` column, which renders
/// as the setting will appear in the app).
struct TableRow {
    key: &'static str,
    value: &'static str,
    changed: &'static str,
}

/// §4a's own worked example rows — `general.base_unit` and `general.date_input` verbatim,
/// plus `general.negatives` (the settings list's third `●`) to make up the "3 rows" the
/// heading's tag counts, only two of which fit `SETTINGS_TABLE_SHOWN`.
const TABLE_ROWS: &[TableRow] = &[
    TableRow {
        key: "general.base_unit",
        value: "AUD",
        changed: "14:02 today",
    },
    TableRow {
        key: "general.date_input",
        value: "dmy",
        changed: "02 sep",
    },
    TableRow {
        key: "general.negatives",
        value: "minus",
        changed: "29 aug",
    },
];

/// The "settings table" block: the actual override rows behind the settings list, `KEY` /
/// `VALUE` / `CHANGED`, values in the stored form — the answer to "what will `git diff` on my
/// ledger show" now that there is no config file to diff. A scrollbar rides the right edge
/// since more rows exist than `SETTINGS_TABLE_SHOWN` — signalling the truncation the heading's
/// tag already states in words.
fn render_settings_table(frame: &mut Frame, area: Rect) {
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

    let tag = format!("{} rows · {} shown", TABLE_ROWS.len(), SETTINGS_TABLE_SHOWN);
    render_heading(frame, sections[0], "SETTINGS TABLE", &tag);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), sections[1]);

    let dim = Style::default().add_modifier(Modifier::DIM);
    let columns = table_row_columns(sections[2]);
    frame.render_widget(Paragraph::new(Span::styled("KEY", dim)), columns[0]);
    frame.render_widget(Paragraph::new(Span::styled("VALUE", dim)), columns[1]);
    frame.render_widget(Paragraph::new(Span::styled("CHANGED", dim)), columns[2]);

    let shown = &TABLE_ROWS[..TABLE_ROWS.len().min(sections[3].height as usize)];
    let row_constraints: Vec<Constraint> =
        std::iter::repeat_n(Constraint::Length(1), shown.len()).collect();
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(sections[3]);
    for (row, area) in shown.iter().zip(row_areas.iter()) {
        let columns = table_row_columns(*area);
        frame.render_widget(Paragraph::new(row.key), columns[0]);
        frame.render_widget(Paragraph::new(row.value), columns[1]);
        frame.render_widget(Paragraph::new(row.changed), columns[2]);
    }

    let rows_scrollbar_area = Rect {
        y: sections[3].y,
        height: sections[3].height,
        ..scrollbar_column
    };
    let mut scrollbar_state = ScrollbarState::new(TABLE_ROWS.len())
        .viewport_content_length(shown.len())
        .position(0);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None);
    frame.render_stateful_widget(scrollbar, rows_scrollbar_area, &mut scrollbar_state);
}

/// Splits a settings-table row (or its column header) into `KEY` / `VALUE` / `CHANGED` (fill)
/// columns.
fn table_row_columns(area: Rect) -> [Rect; 3] {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),
            Constraint::Length(12),
            Constraint::Min(0),
        ])
        .spacing(2)
        .split(area);
    [columns[0], columns[1], columns[2]]
}

/// The command hint row: dim, bottom of the pane, matching §4a's own example verbatim — every
/// setting must be settable both ways, and this is how the user learns the `:set` form.
const COMMAND_HINT: &str = ":set base <unit> · :set negatives brackets · :settings log";

fn render_command_hint(frame: &mut Frame, area: Rect) {
    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled(COMMAND_HINT, dim)), area);
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn render(view: &SettingsView) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("drawing the settings view should not error");

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
        render(&SettingsView::new());
    }

    #[test]
    fn title_is_settings() {
        assert_eq!(SettingsView::new().title(), "Settings");
    }

    #[test]
    fn shows_every_pane_heading() {
        let text = render(&SettingsView::new());

        assert!(text.contains("GROUPS"), "groups heading missing");
        assert!(
            text.contains("WHERE VALUES LIVE"),
            "where-values-live heading missing"
        );
        assert!(text.contains("RESET"), "reset heading missing");
        assert!(text.contains("SETTINGS"), "settings list heading missing");
        assert!(text.contains("SELECTED"), "selected heading missing");
        assert!(
            text.contains("SETTINGS TABLE"),
            "settings table heading missing"
        );
    }

    #[test]
    fn shows_every_group_and_every_general_setting() {
        let text = render(&SettingsView::new());

        for group in GROUPS {
            assert!(text.contains(group.label), "{} group missing", group.label);
        }
        for setting in SETTINGS {
            assert!(
                text.contains(setting.setting),
                "{} setting missing",
                setting.setting
            );
            assert!(
                text.contains(setting.value),
                "{}'s value missing",
                setting.setting
            );
        }
    }

    #[test]
    fn general_group_row_reverses_and_others_do_not() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| SettingsView::new().view(frame, frame.area()))
            .expect("drawing the settings view should not error");

        let buffer = terminal.backend().buffer();
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

        assert!(
            row_is_reversed(row_containing("general")),
            "the selected group row should reverse"
        );
        assert!(
            !row_is_reversed(row_containing("display")),
            "a non-selected group row should not reverse"
        );
    }

    #[test]
    fn base_unit_row_reverses_and_shows_the_override_dot() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| SettingsView::new().view(frame, frame.area()))
            .expect("drawing the settings view should not error");

        let buffer = terminal.backend().buffer();
        let row_containing = |needle: &str| -> u16 {
            (0..buffer.area.height)
                .find(|&y| {
                    let mut row = String::new();
                    for x in LEFT_COLUMN_WIDTH + 2..buffer.area.width {
                        row.push_str(buffer[(x, y)].symbol());
                    }
                    row.contains(needle)
                })
                .unwrap_or_else(|| panic!("no row contains {needle:?}"))
        };
        let row_is_reversed = |y: u16| -> bool {
            (LEFT_COLUMN_WIDTH + 2..buffer.area.width)
                .any(|x| buffer[(x, y)].modifier.contains(Modifier::REVERSED))
        };
        let row_has_accent = |y: u16| -> bool {
            (LEFT_COLUMN_WIDTH + 2..buffer.area.width).any(|x| buffer[(x, y)].fg == ACCENT)
        };

        let base_unit_row = row_containing("base unit");
        assert!(
            row_is_reversed(base_unit_row),
            "base unit row should reverse"
        );
        assert!(
            row_has_accent(base_unit_row),
            "base unit row should show the accent override dot"
        );

        let fiscal_year_row = row_containing("fiscal year starts");
        assert!(
            !row_is_reversed(fiscal_year_row),
            "a non-selected setting row should not reverse"
        );
        assert!(
            !row_has_accent(fiscal_year_row),
            "a non-overridden setting row should not show the override dot"
        );
    }

    #[test]
    fn selected_box_shows_explain_and_facts() {
        let text = render(&SettingsView::new());

        assert!(text.contains(SELECTED_EXPLAIN), "explain prose missing");
        for fact in SELECTED_FACTS {
            assert!(text.contains(fact.label), "{} fact missing", fact.label);
            assert!(text.contains(fact.value), "{}'s value missing", fact.label);
        }
    }

    #[test]
    fn settings_table_shows_the_truncation_tag_and_its_scrollbar() {
        let text = render(&SettingsView::new());

        assert!(
            text.contains("3 rows · 2 shown"),
            "settings table truncation tag missing"
        );
        assert!(
            text.contains(TABLE_ROWS[0].key),
            "first settings table row missing"
        );
        assert!(
            text.contains(TABLE_ROWS[1].key),
            "second settings table row missing"
        );
        assert!(
            text.contains(['║', '█']),
            "settings table scrollbar track or thumb missing"
        );
    }

    #[test]
    fn shows_the_command_hint_row() {
        let text = render(&SettingsView::new());

        assert!(text.contains(COMMAND_HINT), "command hint row missing");
    }
}
