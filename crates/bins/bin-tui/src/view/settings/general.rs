//! Settings › General (TUI-019, editing in place TUI-020): ledger name, owner, financial year
//! starts and base unit, then the THIS LEDGER box.
//!
//! Ledger name, owner and the financial year's start have no Preference columns yet, so they
//! start on TUI-019's worked values and edit for the session only ([`SettingsValues`](super::field::SettingsValues)). The base
//! unit is the Units' base, read from `lib-units`' base Unit port on the Open Ledger's shared ports
//! on each draw. Committing a new one opens the base unit guard (TUI-021,
//! `popup::settings::guard`), which sets it through `lib-units`' Set base unit Presenter.
//!
//! THIS LEDGER's counts are derived on each draw: the Units from the Units store, and accounts,
//! transactions and institutions from the Ledger the Units ports read usage from, the stub Ledger
//! until those domains have adapters (#746).

use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use lib_list_cursor::Step;
use lib_units::UnitsPorts;
use lib_units::presenter::SetBaseUnitPresenter;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Span,
    widgets::{Block, Paragraph, Wrap},
};

use super::field::{FINANCIAL_YEAR_STARTS, SettingField, financial_year, month_label};
use super::keys::{self, Binding, Press};
use super::{Context, FieldRow, Open, list, render_form};
use crate::colours::Colours;
use crate::msg;
use crate::popup::settings::edit::{Draft, FieldEditor, Segment};
use crate::units::TuiPorts;
use crate::view::Action;

/// The fields, in the page's order.
const FIELDS: [SettingField; 4] = [
    SettingField::LedgerName,
    SettingField::Owner,
    SettingField::FinancialYearStart,
    SettingField::BaseUnit,
];

#[derive(Debug, Clone, Copy)]
enum Command {
    Row,
    Open,
}

/// The page's keys and its hint bar, one table (#898).
const KEYS: &[Binding<Command>] = &[
    Binding {
        keys: keys::ROW_KEYS,
        label: "j/k",
        hint: msg::tui_settings_hint_field,
        command: Command::Row,
    },
    Binding {
        keys: &[Press::key(KeyCode::Enter)],
        label: "enter",
        hint: msg::tui_settings_hint_edit,
        command: Command::Open,
    },
];

/// General's state: the selected field.
#[derive(Default)]
pub(super) struct GeneralPage {
    selected: usize,
}

impl GeneralPage {
    /// `enter` opens the selected field in place.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        Some(match keys::lookup(KEYS, key)? {
            Command::Row => {
                self.step(list::motion(key)?);
                Action::NoOp
            }
            Command::Open => Action::SetSetting(FIELDS[self.selected], None),
        })
    }

    pub(super) fn hints() -> Vec<(String, String)> {
        keys::hints(KEYS)
    }

    pub(super) fn step(&mut self, step: Step) {
        self.selected = lib_list_cursor::step(self.selected, FIELDS.len(), step);
    }

    /// Selects `field`'s row, if it is on this page.
    pub(super) fn select(&mut self, field: SettingField) {
        if let Some(index) = FIELDS.iter().position(|row| *row == field) {
            self.selected = index;
        }
    }

    /// `focused`, the selected field is reversed; an `editor` open on one of its fields draws in
    /// that row's place.
    pub(super) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        focused: bool,
        context: &Context<'_>,
        c: &Colours,
    ) {
        let rows: Vec<FieldRow> = FIELDS.iter().map(|field| row(*field, context)).collect();
        let open = context.editor.map(|editor| Open {
            editor,
            sub_rows: sub_rows(editor, context),
        });
        let rest = render_form(
            frame,
            area,
            &rows,
            focused.then_some(self.selected),
            open,
            c,
        );
        let [_, summary, _, note] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .areas(rest);
        if let Some(units) = context.units {
            render_this_ledger(frame, summary, units, c);
        }
        frame.render_widget(
            Paragraph::new(Span::styled(msg::tui_settings_general_note(), c.muted()))
                .wrap(Wrap { trim: true }),
            note,
        );
    }
}

/// One field's row at rest.
fn row(field: SettingField, context: &Context<'_>) -> FieldRow {
    let values = context.values;
    let value = match field {
        SettingField::LedgerName => values.ledger_name.clone(),
        SettingField::Owner => values.owner.clone(),
        SettingField::FinancialYearStart => month_label(values.financial_year_start),
        _ => context.units.map(base_unit_label).unwrap_or_default(),
    };
    FieldRow {
        glyph: None,
        field,
        value,
        note: String::new(),
    }
}

/// The base Unit as `aud — Australian Dollar`.
fn base_unit_label(units: &TuiPorts) -> String {
    let ports = units.borrow();
    let base = ports.base_unit().base_unit();
    ports
        .units()
        .unit(base)
        .map(|unit| lib_units::msg::units_base_unit_option(&unit.code, &unit.name))
        .unwrap_or_default()
}

/// What opening `field` in place starts from: its draft and the old value's label. `None` for a
/// field General doesn't hold, or the base unit without a Ledger's Units.
pub(super) fn draft(field: SettingField, context: &Context<'_>) -> Option<(Draft, String)> {
    let values = context.values;
    Some(match field {
        SettingField::LedgerName | SettingField::Owner => {
            let text = values.get(field)?;
            (Draft::Text(text.clone()), text)
        }
        SettingField::FinancialYearStart => {
            let segments = FINANCIAL_YEAR_STARTS
                .iter()
                .map(|(value, month)| Segment {
                    value: (*value).to_string(),
                    label: month_label(*month),
                })
                .collect();
            let chosen = FINANCIAL_YEAR_STARTS
                .iter()
                .position(|(_, month)| *month == values.financial_year_start)
                .unwrap_or_default();
            (
                Draft::Choice { segments, chosen },
                month_label(values.financial_year_start),
            )
        }
        SettingField::BaseUnit => base_unit_draft(context.units?)?,
        _ => return None,
    })
}

/// The base unit's segments: the codes of the Units `lib-units`' Set base unit Presenter offers,
/// so the TUI keeps no rule of its own about which Units can be the base.
fn base_unit_draft(units: &TuiPorts) -> Option<(Draft, String)> {
    let shared: lib_units::SharedPorts = units.clone();
    let options = SetBaseUnitPresenter::new(shared).view_model().base.options;
    let ports = units.borrow();
    let base = ports.base_unit().base_unit();
    let code = |id| ports.units().unit(id).map(|unit| unit.code);
    let segments: Vec<Segment> = options
        .iter()
        .filter_map(|choice| {
            code(choice.value).map(|code| Segment {
                value: code.clone(),
                label: code,
            })
        })
        .collect();
    let was = code(base)?;
    let chosen = segments
        .iter()
        .position(|segment| segment.value == was)
        .unwrap_or_default();
    Some((Draft::Choice { segments, chosen }, was))
}

/// The open field's sub-rows: what the draft does, and what it was.
fn sub_rows(editor: &FieldEditor, context: &Context<'_>) -> Vec<(String, String)> {
    let session = (
        msg::tui_settings_edit_on_enter(),
        msg::tui_settings_edit_holds_for_session(),
    );
    match editor.field() {
        SettingField::FinancialYearStart => {
            let today = Local::now().date_naive();
            let month = FINANCIAL_YEAR_STARTS
                .iter()
                .find(|(value, _)| *value == editor.value())
                .map_or(context.values.financial_year_start, |(_, month)| *month);
            let range = |month| {
                let (first, last) = financial_year(today, month);
                msg::tui_settings_edit_year_range(
                    &crate::format::date(first),
                    &crate::format::date(last),
                )
            };
            vec![
                (msg::tui_settings_edit_this_year(), range(month)),
                (
                    msg::tui_settings_edit_was(),
                    msg::tui_settings_edit_was_year(
                        editor.was(),
                        &range(context.values.financial_year_start),
                    ),
                ),
                (
                    msg::tui_settings_edit_changes(),
                    msg::tui_settings_edit_financial_year_changes(),
                ),
                session,
            ]
        }
        SettingField::BaseUnit => {
            let chosen = context
                .units
                .and_then(|units| {
                    let ports = units.borrow();
                    ports
                        .units()
                        .units()
                        .into_iter()
                        .find(|unit| unit.code == editor.value())
                        .map(|unit| lib_units::msg::units_base_unit_option(&unit.code, &unit.name))
                })
                .unwrap_or_default();
            vec![
                (msg::tui_settings_edit_chosen(), chosen),
                (msg::tui_settings_edit_was(), editor.was().to_string()),
                (
                    msg::tui_settings_edit_on_enter(),
                    msg::tui_settings_edit_base_on_enter(),
                ),
            ]
        }
        _ => vec![
            (msg::tui_settings_edit_was(), editor.was().to_string()),
            session,
        ],
    }
}

/// THIS LEDGER: a 4-cell box of counts, each derived on this draw.
fn render_this_ledger(frame: &mut Frame<'_>, area: Rect, units: &TuiPorts, c: &Colours) {
    let ports = units.borrow();
    let ledger = ports.ledger();
    let cells = [
        (
            msg::tui_settings_general_accounts(),
            ledger.accounts().len(),
        ),
        (
            msg::tui_settings_general_transactions(),
            ledger.transactions().len(),
        ),
        (
            msg::tui_settings_general_units(),
            ports.units().units().len(),
        ),
        (
            msg::tui_settings_general_institutions(),
            ledger.institutions().len(),
        ),
    ];
    let block = Block::bordered()
        .border_style(c.muted())
        .title(Span::styled(
            msg::tui_settings_general_this_ledger(),
            c.muted(),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let columns = Layout::horizontal([Constraint::Ratio(1, 4); 4]).split(inner);
    for ((label, count), column) in cells.into_iter().zip(columns.iter()) {
        let [label_area, count_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(*column);
        frame.render_widget(Paragraph::new(Span::styled(label, c.muted())), label_area);
        let count = i64::try_from(count).unwrap_or(i64::MAX);
        frame.render_widget(Paragraph::new(crate::format::count(count)), count_area);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;
    use crate::view::View;
    use crate::view::settings::{SettingsPage, SettingsView, tests::render_at};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn enter_opens_the_selected_field_by_its_action() {
        let mut page = GeneralPage::default();
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::SetSetting(SettingField::LedgerName, None))
        );
        page.handle_key(key(KeyCode::Char('G')));
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::SetSetting(SettingField::BaseUnit, None))
        );
        page.handle_key(key(KeyCode::Char('j')));
        assert_eq!(FIELDS[page.selected], SettingField::BaseUnit);
    }

    #[test]
    fn it_draws_its_four_fields_and_the_ledger_s_counts() {
        crate::locale::init_for_tests();
        let mut view = SettingsView::with_units(&crate::view::settings::tests::stub_units());
        view.update(&Action::OpenSettingsPage(SettingsPage::General));
        let text = render_at(&view, 96, 30);
        for needle in [
            "ledger name",
            "Teda household",
            "owner",
            "financial year starts",
            "july",
            "base unit",
            "aud — Australian Dollar",
            "this ledger",
            "transactions",
            "institutions",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
    }

    #[test]
    fn the_hint_bar_is_the_keys_the_page_takes() {
        keys::tests::assert_the_hints_are_the_keys(KEYS, GeneralPage::hints(), |key| {
            GeneralPage::default().handle_key(key)
        });
    }
}
