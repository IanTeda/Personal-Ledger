//! The Units Settings page: the Units table above the Price Sources table, one row selected
//! across both, the row menu, and where a confirmed Dialog's outcome lands (a Toast, or ACTIVITY
//! while the detail pane is docked).

use chrono::Local;
use lib_core::{PriceSourceId, UnitId};
use lib_list_cursor::{Step, resolve, step_id};
use lib_locale::Label as _;
use lib_locale::format::format_time_minutes;
use lib_toast::{Notice, ToastKind};
use lib_viper::keymap::{DELETE, EDIT, Keymap, MOVE_DOWN, MOVE_UP, NEW};

use super::units_pane::{DetailPane, PaneInput, detail_pane};
use super::{Clock, UnitChange, UnitsOutcome, ago, joined};
use crate::interactor::ReactivateUnit;
use crate::keymap::{NEXT_TABLE, OPEN, SET_PRICE, TEST_SOURCE};
use crate::protocols::{SharedPorts, UnitsDialog, UnitsRouter};
use crate::{MANUAL_ENTRY, PriceSource, SourceOrigin, UnitRow, last_fetched, msg};

/// A row of either table. One is selected at a time, the Units drawn above the Price Sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitsRow {
    Unit(UnitId),
    Source(PriceSourceId),
}

/// What a listed Price Source is, which decides the actions its row offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// Neither edited nor deleted: it is the fallback every Unit can sit on.
    ManualEntry,
    /// Renamed and retimed, never deleted.
    BuiltIn,
    Custom,
}

impl SourceKind {
    pub(super) fn of(source: &PriceSource<'_>) -> Self {
        match source.origin {
            SourceOrigin::ManualEntry => Self::ManualEntry,
            SourceOrigin::BuiltIn(_) => Self::BuiltIn,
            SourceOrigin::Custom(_) => Self::Custom,
        }
    }
}

/// An action a row offers, in its menu and through its keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// Opens the Unit prices view.
    Prices,
    EditUnit,
    SetPrice,
    ReactivateUnit,
    DeleteUnit,
    TestSource,
    EditSource,
    DeleteSource,
}

impl MenuAction {
    /// A row's actions in menu order, the destructive one last. Every Unit offers its prices
    /// first. A Unit on Manual entry offers Set price, since nothing else prices it; a deactivated
    /// Unit offers Reactivate, since it is hidden from the pickers until then. The Manual entry
    /// row offers nothing and a built-in no Delete.
    pub(super) fn for_row(row: Row<'_>) -> &'static [Self] {
        match row {
            Row::Unit(unit) => match (unit.link.source_id == MANUAL_ENTRY, unit.is_active) {
                (true, true) => &[
                    Self::Prices,
                    Self::EditUnit,
                    Self::SetPrice,
                    Self::DeleteUnit,
                ],
                (true, false) => &[
                    Self::Prices,
                    Self::EditUnit,
                    Self::SetPrice,
                    Self::ReactivateUnit,
                    Self::DeleteUnit,
                ],
                (false, true) => &[Self::Prices, Self::EditUnit, Self::DeleteUnit],
                (false, false) => &[
                    Self::Prices,
                    Self::EditUnit,
                    Self::ReactivateUnit,
                    Self::DeleteUnit,
                ],
            },
            Row::Source(_, SourceKind::Custom) => {
                &[Self::TestSource, Self::EditSource, Self::DeleteSource]
            }
            Row::Source(_, SourceKind::BuiltIn) => &[Self::TestSource, Self::EditSource],
            Row::Source(_, SourceKind::ManualEntry) => &[],
        }
    }

    fn label(self) -> String {
        match self {
            Self::Prices => msg::units_menu_prices(),
            Self::EditUnit => msg::units_menu_edit_unit(),
            Self::SetPrice => msg::units_menu_set_price(),
            Self::ReactivateUnit => msg::units_menu_reactivate_unit(),
            Self::DeleteUnit => msg::units_menu_delete_unit(),
            Self::TestSource => msg::units_menu_test_source(),
            Self::EditSource => msg::units_menu_edit_source(),
            Self::DeleteSource => msg::units_menu_delete_source(),
        }
    }

    /// The label its detail pane button shows, shorter than the menu's.
    pub(super) fn pane_label(self) -> String {
        match self {
            Self::Prices => msg::units_pane_prices(),
            Self::EditUnit => msg::units_pane_edit_unit(),
            Self::SetPrice => msg::units_pane_set_price(),
            Self::ReactivateUnit => msg::units_pane_reactivate_unit(),
            Self::DeleteUnit => msg::units_pane_delete_unit(),
            Self::TestSource => msg::units_pane_test(),
            Self::EditSource => msg::units_pane_edit_source(),
            Self::DeleteSource => msg::units_pane_delete_source(),
        }
    }

    /// A stable name (`edit-unit`), for a Client's element ids and tests.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Prices => "prices",
            Self::EditUnit => "edit-unit",
            Self::SetPrice => "set-price",
            Self::ReactivateUnit => "reactivate-unit",
            Self::DeleteUnit => "delete-unit",
            Self::TestSource => "test-source",
            Self::EditSource => "edit-source",
            Self::DeleteSource => "delete-source",
        }
    }

    fn entry(self) -> MenuEntry {
        MenuEntry {
            action: self,
            label: self.label(),
            destructive: matches!(self, Self::DeleteUnit | Self::DeleteSource),
        }
    }
}

/// A row as the rules about its actions see it.
#[derive(Debug, Clone, Copy)]
pub(super) enum Row<'a> {
    Unit(&'a UnitRow),
    Source(PriceSourceId, SourceKind),
}

/// What the page is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsPageEvent {
    /// `j`/`k`, `g g`, `G`, `Ctrl-d`/`Ctrl-u`: both tables walked as one list.
    Move(Step),
    /// `Tab`: the other table's last-selected row, or its first.
    NextTable,
    /// A click on a row.
    Select(UnitsRow),
    /// A double-click on a row: selects it and opens its Edit Dialog.
    Activate(UnitsRow),
    /// The **+** beside either table, whatever is selected.
    NewUnit,
    NewPriceSource,
    /// A detail pane button: `action` on `row`, as its menu entry runs it.
    Action(MenuAction, UnitsRow),
    /// New, Edit, Delete, Reactivate and Test act on the selected row's table.
    New,
    Edit,
    Delete,
    Reactivate,
    Test,
    SetBase,
    /// `p`: Set price on the selected Unit, when it is on Manual entry.
    SetPrice,
    /// `enter`: the selected Unit's prices view, or the selected Price Source's Edit Dialog.
    Open,
    /// A right-click on a row. A row with no actions opens no menu.
    OpenMenu(UnitsRow),
    CloseMenu,
    /// An entry of the open menu.
    Menu(MenuAction),
    /// Whether the detail pane is docked (X-Large), which sends outcomes to ACTIVITY.
    SetDocked(bool),
    /// The open Dialog closed, with what its confirm did or `None` when cancelled.
    DialogClosed(Option<UnitsOutcome>),
}

/// The page as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitsPageViewModel {
    /// The heading's meta: `3 units · synced`.
    pub scope: String,
    pub unit_columns: UnitColumns,
    pub units: Vec<UnitLine>,
    pub price_sources_heading: String,
    pub source_columns: SourceColumns,
    pub price_sources: Vec<SourceLine>,
    /// The legend under the Units table for the ◆ that marks the base Unit.
    pub base_legend: String,
    /// The status-line legend for the selected row's table, as `(chord, action)`.
    pub hints: Vec<Hint>,
    /// What the selected row offers, for the detail pane's buttons.
    pub selected_actions: Vec<MenuEntry>,
    pub menu: Option<RowMenu>,
    pub docked: bool,
    /// This session's outcomes, newest first, while docked.
    pub activity: Vec<ActivityLine>,
    /// The detail pane, while docked.
    pub pane: Option<DetailPane>,
    pub open_dialog: Option<UnitsDialog>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitColumns {
    pub code: String,
    pub name: String,
    pub kind: String,
    pub price_source: String,
    pub flags: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceColumns {
    pub name: String,
    pub source: String,
    pub last_updated: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitLine {
    pub id: UnitId,
    pub code: String,
    pub name: String,
    /// The Unit Kind's label.
    pub kind: String,
    /// The name of the Price Source the Unit's link names; empty when it is gone.
    pub price_source: String,
    pub is_base: bool,
    /// `base` for the base Unit.
    pub base_flag: Option<String>,
    /// `inactive` for a deactivated Unit, which is drawn dimmed.
    pub inactive_tag: Option<String>,
    /// The kind, then the base and inactive flags that apply: `Currency · base`.
    pub kind_meta: String,
    /// The base and inactive flags that apply, `base · inactive`; empty for neither.
    pub flags: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLine {
    pub id: PriceSourceId,
    pub kind: SourceKind,
    pub name: String,
    /// The address it fetches from, or — for Manual entry.
    pub address: String,
    /// How long ago it last fetched, `Not fetched yet`, or — for Manual entry.
    pub last_updated: String,
    /// Whether it fetches but has not yet, which draws a muted ✗ before LAST UPDATED.
    pub not_fetched: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub chord: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    pub action: MenuAction,
    pub label: String,
    pub destructive: bool,
}

/// The menu open on a row, its entries in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowMenu {
    pub row: UnitsRow,
    pub entries: Vec<MenuEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityLine {
    /// The local time it landed, `HH:MM`.
    pub time: String,
    pub text: String,
}

pub struct UnitsPagePresenter {
    ports: SharedPorts,
    clock: Clock,
    keymap: Keymap<UnitsPageEvent>,
    /// It may name a row that is gone, so it is read through [`Self::selection`].
    selected: Option<UnitsRow>,
    /// Each table's last-selected row, which `Tab` returns to. Either may be gone.
    last_unit: Option<UnitId>,
    last_source: Option<PriceSourceId>,
    menu: Option<UnitsRow>,
    docked: bool,
    activity: Vec<ActivityLine>,
    open_dialog: Option<UnitsDialog>,
}

impl UnitsPagePresenter {
    /// `keymap` is the one the Client looks keys up in, so the legend shows the chords it binds.
    pub fn new(ports: SharedPorts, clock: Clock, keymap: Keymap<UnitsPageEvent>) -> Self {
        Self {
            ports,
            clock,
            keymap,
            selected: None,
            last_unit: None,
            last_source: None,
            menu: None,
            docked: false,
            activity: Vec::new(),
            open_dialog: None,
        }
    }

    pub fn handle(&mut self, event: UnitsPageEvent, router: &mut dyn UnitsRouter) {
        // Any event but the menu's own closes it, as a key or click elsewhere does.
        let menu = self.menu.take();
        match event {
            UnitsPageEvent::Move(step) => {
                let rows = self.rows();
                if let Some(row) = step_id(&rows, self.selection(), step) {
                    self.select(row);
                }
            }
            UnitsPageEvent::NextTable => self.next_table(),
            UnitsPageEvent::Select(row) => self.select(row),
            UnitsPageEvent::Activate(row) => {
                self.select(row);
                self.edit(router);
            }
            UnitsPageEvent::New => {
                let dialog = match self.selection() {
                    Some(UnitsRow::Source(_)) => UnitsDialog::AddPriceSource,
                    _ => UnitsDialog::AddUnit,
                };
                self.open(dialog, router);
            }
            UnitsPageEvent::NewUnit => self.open(UnitsDialog::AddUnit, router),
            UnitsPageEvent::NewPriceSource => self.open(UnitsDialog::AddPriceSource, router),
            UnitsPageEvent::Action(action, row) => self.run(action, row, router),
            UnitsPageEvent::Edit => self.edit(router),
            UnitsPageEvent::Delete => {
                let action = match self.selection() {
                    Some(UnitsRow::Unit(_)) => MenuAction::DeleteUnit,
                    _ => MenuAction::DeleteSource,
                };
                self.run_on_selection(action, router);
            }
            UnitsPageEvent::Reactivate => {
                self.run_on_selection(MenuAction::ReactivateUnit, router);
            }
            UnitsPageEvent::Test => self.test(router),
            UnitsPageEvent::SetBase => self.open(UnitsDialog::SetBaseUnit, router),
            UnitsPageEvent::SetPrice => self.run_on_selection(MenuAction::SetPrice, router),
            UnitsPageEvent::Open => match self.selection() {
                Some(UnitsRow::Unit(_)) => self.run_on_selection(MenuAction::Prices, router),
                _ => self.edit(router),
            },
            UnitsPageEvent::OpenMenu(row) => {
                if self.with_row(row, |row| !MenuAction::for_row(row).is_empty()) == Some(true) {
                    self.select(row);
                    self.menu = Some(row);
                }
            }
            UnitsPageEvent::CloseMenu => {}
            UnitsPageEvent::Menu(action) => {
                if let Some(row) = menu {
                    self.run(action, row, router);
                }
            }
            UnitsPageEvent::SetDocked(docked) => {
                self.docked = docked;
                self.menu = menu;
            }
            UnitsPageEvent::DialogClosed(outcome) => {
                self.open_dialog = None;
                if let Some(outcome) = outcome {
                    self.land(outcome, router);
                }
            }
        }
    }

    pub fn view_model(&self) -> UnitsPageViewModel {
        let ports = self.ports.borrow();
        let units = ports.units().units();
        let catalogue = ports.source_catalogue();
        let base = ports.base_unit().base_unit();
        let sources = catalogue.sources();
        let selection = resolve(Self::rows_of(&units, &sources), self.selected);
        let now = (self.clock)();

        let unit_lines = units
            .iter()
            .map(|unit| {
                let base_flag = (unit.id == base).then(msg::units_flag_base);
                let inactive_tag = (!unit.is_active).then(msg::units_inactive);
                let flags = [base_flag.clone(), inactive_tag.clone()];
                UnitLine {
                    id: unit.id,
                    code: unit.code.clone(),
                    name: unit.name.clone(),
                    kind: unit.kind.label(),
                    price_source: sources
                        .iter()
                        .find(|source| source.id == unit.link.source_id)
                        .map(|source| source.name.to_string())
                        .unwrap_or_default(),
                    is_base: unit.id == base,
                    kind_meta: joined(
                        std::iter::once(unit.kind.label())
                            .chain(flags.clone().into_iter().flatten()),
                    ),
                    flags: joined(flags.into_iter().flatten()),
                    base_flag,
                    inactive_tag,
                    selected: selection == Some(UnitsRow::Unit(unit.id)),
                }
            })
            .collect();
        let source_lines = sources
            .iter()
            .map(|source| {
                let fetched = last_fetched(&units, source.id);
                let fetches = source.endpoint().is_some();
                SourceLine {
                    id: source.id,
                    kind: SourceKind::of(source),
                    name: source.name.to_string(),
                    address: source
                        .endpoint()
                        .map_or_else(msg::units_none, |endpoint| endpoint.address.to_string()),
                    last_updated: match fetched {
                        _ if !fetches => msg::units_none(),
                        Some(at) => ago(at, now),
                        None => msg::units_not_fetched_yet(),
                    },
                    not_fetched: fetches && fetched.is_none(),
                    selected: selection == Some(UnitsRow::Source(source.id)),
                }
            })
            .collect();
        let selected_row = selection.and_then(|selection| match selection {
            UnitsRow::Unit(id) => units.iter().find(|unit| unit.id == id).map(Row::Unit),
            UnitsRow::Source(id) => sources
                .iter()
                .find(|source| source.id == id)
                .map(|source| Row::Source(id, SourceKind::of(source))),
        });
        let entries = |row: Row<'_>| {
            MenuAction::for_row(row)
                .iter()
                .map(|action| action.entry())
                .collect::<Vec<_>>()
        };

        UnitsPageViewModel {
            scope: msg::units_scope(super::count(units.len())),
            unit_columns: UnitColumns {
                code: lib_locale::msg::column_code(),
                name: lib_locale::msg::column_name(),
                kind: lib_locale::msg::column_type(),
                price_source: msg::units_column_price_source(),
                flags: msg::units_column_flags(),
            },
            units: unit_lines,
            price_sources_heading: msg::units_price_sources_heading(),
            source_columns: SourceColumns {
                name: lib_locale::msg::column_name(),
                source: msg::units_column_source(),
                last_updated: msg::units_column_last_updated(),
            },
            price_sources: source_lines,
            base_legend: msg::units_legend_base(),
            hints: self.hints(selected_row),
            selected_actions: selected_row.map(entries).unwrap_or_default(),
            menu: self.menu.and_then(|row| {
                let found = match row {
                    UnitsRow::Unit(id) => units.iter().find(|unit| unit.id == id).map(Row::Unit),
                    UnitsRow::Source(id) => sources
                        .iter()
                        .find(|source| source.id == id)
                        .map(|source| Row::Source(id, SourceKind::of(source))),
                }?;
                Some(RowMenu {
                    row,
                    entries: entries(found),
                })
            }),
            docked: self.docked,
            activity: if self.docked {
                self.activity.clone()
            } else {
                Vec::new()
            },
            pane: self.docked.then(|| {
                detail_pane(&PaneInput {
                    ports: &*ports,
                    units: &units,
                    sources: &sources,
                    base,
                    selection,
                    activity: &self.activity,
                    now,
                })
            }),
            open_dialog: self.open_dialog,
        }
    }

    fn rows_of(units: &[UnitRow], sources: &[PriceSource<'_>]) -> Vec<UnitsRow> {
        units
            .iter()
            .map(|unit| UnitsRow::Unit(unit.id))
            .chain(sources.iter().map(|source| UnitsRow::Source(source.id)))
            .collect()
    }

    /// Every row of both tables in drawn order: the Units, then the Price Sources.
    fn rows(&self) -> Vec<UnitsRow> {
        let ports = self.ports.borrow();
        let units = ports.units().units();
        Self::rows_of(&units, &ports.source_catalogue().sources())
    }

    /// The selected row, or the first while nothing (or a removed row) is selected.
    fn selection(&self) -> Option<UnitsRow> {
        resolve(self.rows(), self.selected)
    }

    /// Selects `row`, remembering it as its table's last row for `Tab`.
    fn select(&mut self, row: UnitsRow) {
        match row {
            UnitsRow::Unit(id) => self.last_unit = Some(id),
            UnitsRow::Source(id) => self.last_source = Some(id),
        }
        self.selected = Some(row);
    }

    /// Selects the other table's last-selected row, or its first while it has none (or that row
    /// is gone). An empty Units table leaves the selection where it is.
    fn next_table(&mut self) {
        let rows = self.rows();
        let target = match self.selection() {
            Some(UnitsRow::Unit(_)) => resolve(
                rows.iter().filter_map(|row| match row {
                    UnitsRow::Source(id) => Some(*id),
                    UnitsRow::Unit(_) => None,
                }),
                self.last_source,
            )
            .map(UnitsRow::Source),
            Some(UnitsRow::Source(_)) => resolve(
                rows.iter().filter_map(|row| match row {
                    UnitsRow::Unit(id) => Some(*id),
                    UnitsRow::Source(_) => None,
                }),
                self.last_unit,
            )
            .map(UnitsRow::Unit),
            None => None,
        };
        if let Some(target) = target {
            self.select(target);
        }
    }

    /// Runs `f` on `row` as the action rules see it, `None` when the row is gone.
    fn with_row<T>(&self, row: UnitsRow, f: impl FnOnce(Row<'_>) -> T) -> Option<T> {
        let ports = self.ports.borrow();
        match row {
            UnitsRow::Unit(id) => ports.units().unit(id).map(|unit| f(Row::Unit(&unit))),
            UnitsRow::Source(id) => {
                let catalogue = ports.source_catalogue();
                let source = catalogue.source(id)?;
                Some(f(Row::Source(id, SourceKind::of(&source))))
            }
        }
    }

    fn edit(&mut self, router: &mut dyn UnitsRouter) {
        let action = match self.selection() {
            Some(UnitsRow::Unit(_)) => MenuAction::EditUnit,
            _ => MenuAction::EditSource,
        };
        self.run_on_selection(action, router);
    }

    fn run_on_selection(&mut self, action: MenuAction, router: &mut dyn UnitsRouter) {
        if let Some(row) = self.selection() {
            self.run(action, row, router);
        }
    }

    /// Runs `action` on `row` when the row offers it, as its menu entry, key or button does.
    fn run(&mut self, action: MenuAction, row: UnitsRow, router: &mut dyn UnitsRouter) {
        let offered = self.with_row(row, |found| MenuAction::for_row(found).contains(&action));
        if offered != Some(true) {
            return;
        }
        match (action, row) {
            (MenuAction::Prices, UnitsRow::Unit(id)) => router.open_prices(id),
            (MenuAction::EditUnit, UnitsRow::Unit(id)) => {
                self.open(UnitsDialog::EditUnit(id), router);
            }
            (MenuAction::SetPrice, UnitsRow::Unit(id)) => {
                self.open(UnitsDialog::SetPrice(id), router);
            }
            (MenuAction::DeleteUnit, UnitsRow::Unit(id)) => {
                self.open(UnitsDialog::DeleteUnit(id), router);
            }
            (MenuAction::ReactivateUnit, UnitsRow::Unit(id)) => self.reactivate(id, router),
            (MenuAction::TestSource, _) => {
                router.notify(Notice::Status(msg::units_status_test_not_yet_built()))
            }
            (MenuAction::EditSource, UnitsRow::Source(id)) => {
                self.open(UnitsDialog::EditPriceSource(id), router);
            }
            (MenuAction::DeleteSource, UnitsRow::Source(id)) => {
                self.open(UnitsDialog::DeletePriceSource(id), router);
            }
            _ => {}
        }
    }

    /// `t`: on a Price Source row its own Test, on a Unit row the Test of the source it links to
    /// (none on Manual entry). Testing has no backing yet, so it says so on the status line.
    fn test(&mut self, router: &mut dyn UnitsRouter) {
        let row = match self.selection() {
            Some(UnitsRow::Unit(id)) => {
                let link = self.ports.borrow().units().unit(id).map(|unit| unit.link);
                match link {
                    Some(link) if link.source_id != MANUAL_ENTRY => {
                        UnitsRow::Source(link.source_id)
                    }
                    _ => return,
                }
            }
            Some(row) => row,
            None => return,
        };
        self.run(MenuAction::TestSource, row, router);
    }

    /// Reactivating loses nothing to undo, so it runs at once, with no Dialog.
    fn reactivate(&mut self, id: UnitId, router: &mut dyn UnitsRouter) {
        let reactivated = {
            let mut ports = self.ports.borrow_mut();
            ReactivateUnit(id).run(ports.split_mut().units)
        };
        if let Ok(unit) = reactivated {
            self.land(
                UnitsOutcome::Unit {
                    change: UnitChange::Reactivated,
                    code: unit.code,
                },
                router,
            );
        }
    }

    fn open(&mut self, dialog: UnitsDialog, router: &mut dyn UnitsRouter) {
        if router.open_dialog(dialog) {
            self.open_dialog = Some(dialog);
        }
    }

    /// The docked detail pane is already in view beside the row, so an outcome goes to ACTIVITY,
    /// newest first, instead of raising a Toast (DESKTOP-125).
    fn land(&mut self, outcome: UnitsOutcome, router: &mut dyn UnitsRouter) {
        if self.docked {
            let time = format_time_minutes((self.clock)().with_timezone(&Local).time());
            self.activity.insert(
                0,
                ActivityLine {
                    time,
                    text: outcome.activity(),
                },
            );
        } else if let Some(text) = outcome.toast() {
            router.notify(Notice::Toast(ToastKind::Success, text));
        }
    }

    /// The legend for `row`'s table, with Set price while `row` offers it.
    fn hints(&self, row: Option<Row<'_>>) -> Vec<Hint> {
        let chord = |command| self.keymap.chord(command).unwrap_or_default().to_string();
        let hint = |chord: String, action: String| Hint { chord, action };
        let moves = format!("{}/{}", chord(MOVE_DOWN), chord(MOVE_UP));
        let on_source = matches!(row, Some(Row::Source(..)));
        let set_price =
            row.is_some_and(|row| MenuAction::for_row(row).contains(&MenuAction::SetPrice));
        let (row, test, other) = if on_source {
            (
                msg::units_hint_price_source(),
                msg::units_hint_test(),
                msg::units_hint_units(),
            )
        } else {
            (
                msg::units_hint_unit(),
                msg::units_hint_test_source(),
                msg::units_hint_price_sources(),
            )
        };
        let mut hints = vec![
            hint(moves, row),
            hint(chord(EDIT), msg::units_hint_edit()),
            hint(chord(DELETE), msg::units_hint_delete()),
            hint(chord(NEW), msg::units_hint_new()),
        ];
        if !on_source {
            hints.push(hint(chord(OPEN), msg::units_hint_prices()));
        }
        if set_price {
            hints.push(hint(chord(SET_PRICE), msg::units_hint_set_price()));
        }
        hints.push(hint(chord(TEST_SOURCE), test));
        hints.push(hint(chord(NEXT_TABLE), other));
        hints
    }
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;

    use super::super::test_support::{self, clock, stub};
    use super::*;
    use crate::fake::{InMemoryPorts, RecordingRouter, RouterCall};
    use crate::keymap::units_page_keymap;
    use crate::presenter::{PaneButton, PaneGroup, SourceGroup};
    use crate::{COINGECKO, FRANKFURTER, YAHOO_FINANCE};

    struct Page {
        ports: std::rc::Rc<std::cell::RefCell<InMemoryPorts>>,
        presenter: UnitsPagePresenter,
        router: RecordingRouter,
    }

    impl Page {
        fn new() -> Self {
            Self::with(KeyBindingConfig::default())
        }

        fn with(config: KeyBindingConfig) -> Self {
            let (ports, shared) = stub();
            Self {
                ports,
                presenter: UnitsPagePresenter::new(shared, clock(), units_page_keymap(&config)),
                router: RecordingRouter::default(),
            }
        }

        fn send(&mut self, event: UnitsPageEvent) -> Vec<RouterCall> {
            self.presenter.handle(event, &mut self.router);
            self.router.take()
        }

        fn view(&self) -> UnitsPageViewModel {
            self.presenter.view_model()
        }

        /// The selected row's text: a Unit's code or a source's name.
        fn selected(&self) -> String {
            let view = self.view();
            view.units
                .iter()
                .find(|unit| unit.selected)
                .map(|unit| unit.code.clone())
                .or_else(|| {
                    view.price_sources
                        .iter()
                        .find(|source| source.selected)
                        .map(|source| source.name.clone())
                })
                .unwrap_or_default()
        }
    }

    #[test]
    fn the_tables_show_the_units_and_every_price_source_with_the_first_row_selected() {
        let page = Page::new();
        let view = page.view();

        let units: Vec<_> = view
            .units
            .iter()
            .map(|unit| {
                (
                    unit.code.as_str(),
                    unit.kind.as_str(),
                    unit.price_source.as_str(),
                )
            })
            .collect();
        assert_eq!(
            units,
            [
                ("aud", "Currency", "Manual entry"),
                ("btc", "Crypto", "CoinGecko"),
                ("vas", "Equity", "Manual entry"),
            ]
        );
        assert_eq!(view.units[0].base_flag.as_deref(), Some("base"));
        assert_eq!(view.units[1].base_flag, None);
        assert_eq!(
            (
                view.units[0].kind_meta.as_str(),
                view.units[0].flags.as_str()
            ),
            ("Currency · base", "base")
        );
        assert_eq!(
            (
                view.units[1].kind_meta.as_str(),
                view.units[1].flags.as_str()
            ),
            ("Crypto", "")
        );
        assert_eq!(view.scope, "3 units · synced");
        let sources: Vec<_> = view
            .price_sources
            .iter()
            .map(|source| {
                (
                    source.name.as_str(),
                    source.address.as_str(),
                    source.last_updated.as_str(),
                )
            })
            .collect();
        assert_eq!(
            sources,
            [
                (
                    "CoinGecko",
                    "https://api.coingecko.com/api/v3",
                    "5 minutes ago"
                ),
                (
                    "Frankfurter",
                    "https://api.frankfurter.app",
                    "Not fetched yet"
                ),
                ("Manual entry", "—", "—"),
                (
                    "Yahoo Finance",
                    "https://query1.finance.yahoo.com",
                    "Not fetched yet"
                ),
            ]
        );
        assert!(view.price_sources[1].not_fetched);
        assert!(!view.price_sources[2].not_fetched);
        assert_eq!(page.selected(), "aud");
    }

    #[test]
    fn moving_walks_both_tables_as_one_list_and_clamps_at_the_ends() {
        let mut page = Page::new();

        page.send(UnitsPageEvent::Move(Step::Prev));
        assert_eq!(page.selected(), "aud");
        for _ in 0..3 {
            page.send(UnitsPageEvent::Move(Step::Next));
        }
        assert_eq!(
            page.selected(),
            "CoinGecko",
            "j on the last Unit reaches the sources"
        );
        page.send(UnitsPageEvent::Move(Step::Last));
        assert_eq!(page.selected(), "Yahoo Finance");
        page.send(UnitsPageEvent::Move(Step::Next));
        assert_eq!(page.selected(), "Yahoo Finance");
        page.send(UnitsPageEvent::Move(Step::First));
        assert_eq!(page.selected(), "aud");
    }

    #[test]
    fn tab_jumps_to_the_other_tables_last_selected_row() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));

        page.send(UnitsPageEvent::NextTable);
        assert_eq!(
            page.selected(),
            "CoinGecko",
            "a table not yet visited starts at its first"
        );
        page.send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));
        page.send(UnitsPageEvent::NextTable);
        assert_eq!(page.selected(), "vas");
        page.send(UnitsPageEvent::NextTable);
        assert_eq!(page.selected(), "Frankfurter");
    }

    #[test]
    fn new_edit_and_delete_open_the_selected_tables_dialogs_through_the_router() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        let open = |dialog| vec![RouterCall::OpenDialog(dialog)];

        assert_eq!(page.send(UnitsPageEvent::New), open(UnitsDialog::AddUnit));
        assert_eq!(page.view().open_dialog, Some(UnitsDialog::AddUnit));
        page.send(UnitsPageEvent::DialogClosed(None));
        assert_eq!(page.view().open_dialog, None);
        assert_eq!(
            page.send(UnitsPageEvent::Edit),
            open(UnitsDialog::EditUnit(UnitId::SEED_BTC))
        );
        assert_eq!(
            page.send(UnitsPageEvent::Delete),
            open(UnitsDialog::DeleteUnit(UnitId::SEED_BTC))
        );
        assert_eq!(
            page.send(UnitsPageEvent::SetBase),
            open(UnitsDialog::SetBaseUnit)
        );

        page.send(UnitsPageEvent::Select(UnitsRow::Source(YAHOO_FINANCE)));
        assert_eq!(
            page.send(UnitsPageEvent::New),
            open(UnitsDialog::AddPriceSource)
        );
        assert_eq!(
            page.send(UnitsPageEvent::Edit),
            open(UnitsDialog::EditPriceSource(YAHOO_FINANCE))
        );
        assert_eq!(
            page.send(UnitsPageEvent::Activate(UnitsRow::Unit(UnitId::SEED_AUD))),
            open(UnitsDialog::EditUnit(UnitId::SEED_AUD)),
            "a double-click selects and edits"
        );
    }

    #[test]
    fn delete_on_a_custom_source_opens_its_dialog_by_key_and_by_menu() {
        let mut page = Page::new();
        let custom = crate::interactor::AddPriceSource {
            name: "Broker".into(),
            endpoint: crate::built_in(COINGECKO).unwrap().endpoint.clone(),
            updates: crate::Updates::Daily,
        }
        .run(&mut page.ports.borrow_mut().price_sources);
        let open = vec![RouterCall::OpenDialog(UnitsDialog::DeletePriceSource(
            custom.id,
        ))];

        page.send(UnitsPageEvent::Select(UnitsRow::Source(custom.id)));
        assert_eq!(page.send(UnitsPageEvent::Delete), open);
        page.send(UnitsPageEvent::OpenMenu(UnitsRow::Source(custom.id)));
        assert_eq!(
            page.send(UnitsPageEvent::Menu(MenuAction::DeleteSource)),
            open
        );
    }

    #[test]
    fn enter_opens_a_units_prices_and_edits_a_price_source() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        assert_eq!(
            page.send(UnitsPageEvent::Open),
            [RouterCall::OpenPrices(UnitId::SEED_BTC)]
        );
        assert_eq!(
            page.selected(),
            "btc",
            "the selection stays for the way back"
        );

        page.send(UnitsPageEvent::Select(UnitsRow::Source(COINGECKO)));
        assert_eq!(
            page.send(UnitsPageEvent::Open),
            [RouterCall::OpenDialog(UnitsDialog::EditPriceSource(
                COINGECKO
            ))]
        );
    }

    #[test]
    fn every_units_menu_and_pane_offer_its_prices() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::OpenMenu(UnitsRow::Unit(UnitId::SEED_BTC)));
        assert_eq!(
            page.send(UnitsPageEvent::Menu(MenuAction::Prices)),
            [RouterCall::OpenPrices(UnitId::SEED_BTC)]
        );
        assert_eq!(
            page.send(UnitsPageEvent::Action(
                MenuAction::Prices,
                UnitsRow::Unit(UnitId::SEED_VAS)
            )),
            [RouterCall::OpenPrices(UnitId::SEED_VAS)]
        );
    }

    #[test]
    fn a_dialog_the_router_could_not_open_is_not_shown_as_open() {
        struct Refusing;
        impl UnitsRouter for Refusing {
            fn open_dialog(&mut self, _dialog: UnitsDialog) -> bool {
                false
            }
            fn close_dialog(&mut self, _outcome: Option<UnitsOutcome>) {}
            fn open_prices(&mut self, _unit: UnitId) {}
            fn close_prices(&mut self) {}
        }
        impl lib_toast::Notify for Refusing {
            fn notify(&mut self, _notice: Notice) {}
        }
        let mut page = Page::new();

        page.presenter.handle(UnitsPageEvent::New, &mut Refusing);

        assert_eq!(page.view().open_dialog, None);
    }

    #[test]
    fn a_unit_in_use_still_opens_its_delete_dialog_which_offers_deactivate() {
        let mut page = Page::new();
        page.ports.borrow_mut().usage.uses.insert(
            UnitId::SEED_BTC,
            crate::UnitUse {
                accounts: 2,
                ..Default::default()
            },
        );
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));

        assert_eq!(
            page.send(UnitsPageEvent::Delete),
            vec![RouterCall::OpenDialog(UnitsDialog::DeleteUnit(
                UnitId::SEED_BTC
            ))]
        );
    }

    #[test]
    fn a_built_in_offers_no_delete_and_manual_entry_offers_nothing() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Select(UnitsRow::Source(COINGECKO)));
        assert_eq!(page.send(UnitsPageEvent::Delete), vec![]);
        let actions: Vec<_> = page
            .view()
            .selected_actions
            .iter()
            .map(|entry| entry.action)
            .collect();
        assert_eq!(actions, [MenuAction::TestSource, MenuAction::EditSource]);

        page.send(UnitsPageEvent::Select(UnitsRow::Source(MANUAL_ENTRY)));
        for event in [
            UnitsPageEvent::Edit,
            UnitsPageEvent::Delete,
            UnitsPageEvent::Test,
        ] {
            assert_eq!(page.send(event), vec![]);
        }
        page.send(UnitsPageEvent::OpenMenu(UnitsRow::Source(MANUAL_ENTRY)));
        assert_eq!(page.view().menu, None);
    }

    #[test]
    fn the_row_menu_lists_a_rows_actions_and_an_entry_runs_on_that_row() {
        let mut page = Page::new();
        page.ports.borrow_mut().units.units[2].is_active = false;

        page.send(UnitsPageEvent::OpenMenu(UnitsRow::Unit(UnitId::SEED_VAS)));
        let menu = page.view().menu.expect("vas has actions");
        let labels: Vec<_> = menu
            .entries
            .iter()
            .map(|entry| entry.label.as_str())
            .collect();
        assert_eq!(
            labels,
            [
                "Prices",
                "Edit unit…",
                "Set price…",
                "Reactivate unit",
                "Delete unit…"
            ]
        );
        assert!(menu.entries[4].destructive);
        assert_eq!(page.selected(), "vas", "opening the menu selects its row");

        assert_eq!(
            page.send(UnitsPageEvent::Menu(MenuAction::DeleteUnit)),
            vec![RouterCall::OpenDialog(UnitsDialog::DeleteUnit(
                UnitId::SEED_VAS
            ))]
        );
        assert_eq!(page.view().menu, None);
    }

    #[test]
    fn a_unit_on_manual_entry_offers_set_price_by_key_menu_and_pane_and_a_fetched_one_does_not() {
        let mut page = docked();
        let open = vec![RouterCall::OpenDialog(UnitsDialog::SetPrice(
            UnitId::SEED_VAS,
        ))];
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));

        assert_eq!(page.send(UnitsPageEvent::SetPrice), open);
        page.send(UnitsPageEvent::OpenMenu(UnitsRow::Unit(UnitId::SEED_VAS)));
        let labels: Vec<_> = page
            .view()
            .menu
            .expect("vas has actions")
            .entries
            .into_iter()
            .map(|entry| entry.label)
            .collect();
        assert_eq!(
            labels,
            ["Prices", "Edit unit…", "Set price…", "Delete unit…"]
        );
        assert_eq!(page.send(UnitsPageEvent::Menu(MenuAction::SetPrice)), open);
        let PaneGroup::Unit(unit) = &pane(&page)[0] else {
            panic!("UNIT first")
        };
        assert_eq!(
            button_slugs(&unit.buttons),
            ["prices", "edit-unit", "set-price", "delete-unit"]
        );
        assert_eq!(
            page.send(unit.buttons[2].event.clone()),
            open,
            "the pane button opens it"
        );

        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        assert_eq!(page.send(UnitsPageEvent::SetPrice), vec![]);
        assert_eq!(
            page.send(UnitsPageEvent::Action(
                MenuAction::SetPrice,
                UnitsRow::Unit(UnitId::SEED_BTC)
            )),
            vec![]
        );
        let actions: Vec<_> = page
            .view()
            .selected_actions
            .iter()
            .map(|entry| entry.action)
            .collect();
        assert_eq!(
            actions,
            [
                MenuAction::Prices,
                MenuAction::EditUnit,
                MenuAction::DeleteUnit
            ]
        );
    }

    #[test]
    fn a_set_price_toasts_its_week_below_x_large_and_shows_in_the_manual_units_pane() {
        let mut page = Page::new();
        let mut vas = page.ports.borrow().units.units[2].clone();
        vas.record_price(
            "101.20".parse().expect("a decimal"),
            test_support::now().date_naive(),
            test_support::now(),
        );
        page.ports.borrow_mut().units.units[2] = vas;
        let set = || {
            UnitsPageEvent::DialogClosed(Some(UnitsOutcome::PriceSet {
                code: "vas".into(),
                week: chrono::NaiveDate::from_ymd_opt(2026, 10, 19).expect("a real date"),
            }))
        };

        assert_eq!(
            page.send(set()),
            vec![RouterCall::Notice(Notice::Toast(
                ToastKind::Success,
                "Set the price of vas for the week of Oct 19, 2026".into()
            ))]
        );
        page.send(UnitsPageEvent::SetDocked(true));
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
        page.send(set());

        assert_eq!(
            page.view().activity[0].text,
            "Set the price of vas for the week of Oct 19, 2026"
        );
        let source = source_group(&pane(&page));
        assert_eq!(
            facts(&source),
            [
                ("Source", "Manual entry"),
                ("Last price", "101.20 aud"),
                ("Last updated", "just now"),
            ]
        );
        assert_eq!(button_slugs(&source.buttons), ["add-source"]);
    }

    #[test]
    fn any_other_event_closes_the_menu() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::OpenMenu(UnitsRow::Source(COINGECKO)));
        assert!(page.view().menu.is_some());

        page.send(UnitsPageEvent::Move(Step::Next));
        assert_eq!(page.view().menu, None);
        assert_eq!(
            page.send(UnitsPageEvent::Menu(MenuAction::EditSource)),
            vec![],
            "with no menu open an entry does nothing"
        );
    }

    #[test]
    fn reactivating_a_deactivated_unit_runs_at_once_and_toasts() {
        let mut page = Page::new();
        page.ports.borrow_mut().units.units[1].is_active = false;
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        assert_eq!(
            page.view().units[1].inactive_tag.as_deref(),
            Some("inactive")
        );
        assert_eq!(page.view().units[1].kind_meta, "Crypto · inactive");
        assert_eq!(page.view().units[1].flags, "inactive");

        let calls = page.send(UnitsPageEvent::Reactivate);

        assert_eq!(
            calls,
            vec![RouterCall::Notice(Notice::Toast(
                ToastKind::Success,
                "Reactivated unit btc".into()
            ))]
        );
        assert!(page.ports.borrow().units.units[1].is_active);
        assert_eq!(
            page.send(UnitsPageEvent::Reactivate),
            vec![],
            "an active Unit has none"
        );
    }

    #[test]
    fn test_says_on_the_status_line_that_it_is_not_built_unless_the_unit_is_priced_by_hand() {
        let mut page = Page::new();
        let not_built = vec![RouterCall::Notice(Notice::Status(
            "test price source — not yet built".to_string(),
        ))];
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        assert_eq!(page.send(UnitsPageEvent::Test), not_built);
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_AUD)));
        assert_eq!(page.send(UnitsPageEvent::Test), vec![]);
    }

    #[test]
    fn an_outcome_toasts_below_x_large_and_goes_to_activity_when_docked() {
        let mut page = Page::new();
        let deleted = || UnitsOutcome::Unit {
            change: UnitChange::Deleted,
            code: "btc".into(),
        };

        assert_eq!(
            page.send(UnitsPageEvent::DialogClosed(Some(deleted()))),
            vec![RouterCall::Notice(Notice::Toast(
                ToastKind::Success,
                "Deleted unit btc".into()
            ))]
        );
        assert_eq!(
            page.send(UnitsPageEvent::DialogClosed(Some(UnitsOutcome::Unit {
                change: UnitChange::Saved,
                code: "eth".into()
            }))),
            vec![],
            "a save raises no Toast"
        );

        page.send(UnitsPageEvent::SetDocked(true));
        assert_eq!(
            page.send(UnitsPageEvent::DialogClosed(Some(deleted()))),
            vec![]
        );
        page.send(UnitsPageEvent::DialogClosed(Some(
            UnitsOutcome::SourceDeleted {
                name: "Broker".into(),
                moved: 1,
            },
        )));
        let lines: Vec<_> = page
            .view()
            .activity
            .into_iter()
            .map(|line| line.text)
            .collect();
        assert_eq!(lines, ["Deleted price source Broker", "Deleted unit btc"]);
    }

    #[test]
    fn the_legend_follows_the_selected_table_and_the_keymaps_chords() {
        let mut config = KeyBindingConfig::default();
        config.bindings.insert("edit".into(), "E".into());
        let mut page = Page::with(config);
        let hints = |page: &Page| -> Vec<(String, String)> {
            page.view()
                .hints
                .into_iter()
                .map(|hint| (hint.chord, hint.action))
                .collect()
        };
        let pairs = |list: &[(&str, &str)]| -> Vec<(String, String)> {
            list.iter()
                .map(|(chord, action)| (chord.to_string(), action.to_string()))
                .collect()
        };

        assert_eq!(
            hints(&page),
            pairs(&[
                ("j/k", "unit"),
                ("E", "edit"),
                ("d", "delete"),
                ("n", "new"),
                ("enter", "prices"),
                ("p", "set price"),
                ("t", "test source"),
                ("tab", "price sources"),
            ])
        );
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        assert!(
            !hints(&page).iter().any(|(_, action)| action == "set price"),
            "btc's source prices it"
        );
        page.send(UnitsPageEvent::NextTable);
        assert_eq!(hints(&page)[0], ("j/k".into(), "price source".into()));
        assert_eq!(hints(&page)[5], ("tab".into(), "units".into()));
    }

    fn docked() -> Page {
        let mut page = Page::new();
        page.send(UnitsPageEvent::SetDocked(true));
        page
    }

    fn pane(page: &Page) -> Vec<PaneGroup> {
        page.view().pane.expect("the page is docked").groups
    }

    fn slugs(groups: &[PaneGroup]) -> Vec<&'static str> {
        groups.iter().map(PaneGroup::slug).collect()
    }

    fn source_group(groups: &[PaneGroup]) -> SourceGroup {
        groups
            .iter()
            .find_map(|group| match group {
                PaneGroup::Source(source) => Some(source.clone()),
                _ => None,
            })
            .expect("a PRICE SOURCE group")
    }

    fn facts(group: &SourceGroup) -> Vec<(&str, &str)> {
        group
            .facts
            .iter()
            .map(|fact| (fact.label.as_str(), fact.value.as_str()))
            .collect()
    }

    fn button_slugs(buttons: &[PaneButton]) -> Vec<&'static str> {
        buttons.iter().map(|button| button.slug).collect()
    }

    #[test]
    fn the_pane_is_built_only_while_docked() {
        let mut page = Page::new();
        assert_eq!(page.view().pane, None);
        page.send(UnitsPageEvent::SetDocked(true));
        assert!(page.view().pane.is_some());
    }

    #[test]
    fn a_unit_on_manual_entry_shows_its_last_price_and_offers_to_add_a_price_source() {
        let page = docked();
        let groups = pane(&page);

        assert_eq!(slugs(&groups), ["unit", "source", "accounts", "activity"]);
        let PaneGroup::Unit(unit) = &groups[0] else {
            panic!("UNIT first")
        };
        assert_eq!(unit.title, "aud · Australian Dollar");
        assert_eq!(unit.meta, "Currency · valued in aud");
        assert_eq!(unit.facts[0].value, "base");
        assert_eq!(
            button_slugs(&unit.buttons),
            ["prices", "edit-unit", "set-price", "delete-unit"]
        );
        let source = source_group(&groups);
        assert_eq!(
            facts(&source),
            [
                ("Source", "Manual entry"),
                ("Last price", "—"),
                ("Last updated", "—"),
            ]
        );
        assert_eq!(button_slugs(&source.buttons), ["add-source"]);
        assert_eq!(source.buttons[0].event, UnitsPageEvent::NewPriceSource);
    }

    #[test]
    fn a_selected_units_price_source_reads_its_link() {
        let mut page = docked();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        let source = source_group(&pane(&page));

        assert_eq!(
            facts(&source),
            [
                ("Source", "CoinGecko"),
                ("URL", "https://api.coingecko.com/api/v3"),
                ("Updates", "Every 15 min"),
                ("Last price", "148,212.40 aud"),
                ("Last updated", "5 minutes ago"),
            ]
        );
        assert_eq!(
            button_slugs(&source.buttons),
            ["test-source", "edit-source"],
            "a built-in is not deleted"
        );
        assert_eq!(
            source.buttons[1].event,
            UnitsPageEvent::Action(MenuAction::EditSource, UnitsRow::Source(COINGECKO))
        );
    }

    #[test]
    fn a_units_accounts_are_counted_and_valued_in_its_quote_unit() {
        let mut page = docked();
        let holding = crate::Holding {
            name: "Cold wallet".into(),
            kind: lib_core::AccountType::Investment,
            balance: lib_core::Money("0.4120".parse().expect("a decimal")),
            transactions: 9,
        };
        page.ports
            .borrow_mut()
            .holdings
            .by_unit
            .insert(UnitId::SEED_BTC, vec![holding]);
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        let groups = pane(&page);

        let PaneGroup::Unit(unit) = &groups[0] else {
            panic!("UNIT first")
        };
        assert_eq!(unit.facts[1].value, "1 account · 9 transactions");
        let PaneGroup::Accounts(accounts) = &groups[2] else {
            panic!("ACCOUNTS third")
        };
        let row = &accounts.rows[0];
        assert_eq!(
            (
                row.name.as_str(),
                row.kind.as_str(),
                row.balance.as_str(),
                row.value.as_deref()
            ),
            (
                "Cold wallet",
                "Investment",
                "0.41200000 btc",
                Some("61,063.51 aud")
            )
        );
        assert_eq!(
            accounts.total.as_ref().map(|total| total.name.as_str()),
            Some("Total")
        );
        assert_eq!(accounts.empty, None);
    }

    #[test]
    fn a_balance_shows_at_its_units_precision_and_a_price_or_value_at_its_quote_units() {
        let mut page = docked();
        {
            let mut ports = page.ports.borrow_mut();
            ports.units.units[0].precision = crate::Precision::new(0).expect("in range");
            ports.units.units[1].precision = crate::Precision::new(3).expect("in range");
            ports.holdings.by_unit.insert(
                UnitId::SEED_BTC,
                vec![crate::Holding {
                    name: "Cold wallet".into(),
                    kind: lib_core::AccountType::Investment,
                    balance: lib_core::Money("0.4125".parse().expect("a decimal")),
                    transactions: 1,
                }],
            );
        }
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        let groups = pane(&page);

        let source = source_group(&groups);
        assert_eq!(source.facts[3].value, "148,212 aud", "aud's 0 places");
        let PaneGroup::Accounts(accounts) = &groups[2] else {
            panic!("ACCOUNTS third")
        };
        // 0.4125 × 148,212.40 = 61,137.615: rounded for display, never as stored.
        assert_eq!(
            (
                accounts.rows[0].balance.as_str(),
                accounts.rows[0].value.as_deref()
            ),
            ("0.413 btc", Some("61,138 aud"))
        );
        page.send(UnitsPageEvent::Select(UnitsRow::Source(COINGECKO)));
        let PaneGroup::Values(values) = &pane(&page)[1] else {
            panic!("VALUES second")
        };
        assert_eq!(values.rows[0].price, "148,212 aud");
    }

    #[test]
    fn a_selected_built_in_shows_its_endpoint_and_the_units_it_values() {
        let mut page = docked();
        page.send(UnitsPageEvent::Select(UnitsRow::Source(COINGECKO)));
        let groups = pane(&page);

        assert_eq!(slugs(&groups), ["source", "values", "activity"]);
        let source = source_group(&groups);
        assert_eq!(
            source.meta.as_deref(),
            Some("built-in · prices Crypto · key not needed")
        );
        assert_eq!(
            facts(&source),
            [
                ("Source", "CoinGecko · built-in"),
                ("Address", "https://api.coingecko.com/api/v3"),
                (
                    "Request",
                    "/simple/price?ids={symbol}&vs_currencies={quote}"
                ),
                ("Price path", "/{symbol}/{quote}"),
                ("Updates", "Every 15 min"),
                ("Last updated", "5 minutes ago"),
            ]
        );
        assert_eq!(
            source.note.as_deref(),
            Some("Built-in sources update with the app, so their address isn't edited here.")
        );
        let PaneGroup::Values(values) = &groups[1] else {
            panic!("VALUES second")
        };
        let row = &values.rows[0];
        assert_eq!(
            (
                row.code.as_str(),
                row.symbol.clone(),
                row.quoted_in.as_str(),
                row.price.as_str()
            ),
            (
                "btc",
                Some(("symbol".to_string(), "bitcoin".to_string())),
                "quoted in aud",
                "148,212.40 aud"
            )
        );
        assert_eq!(row.detail, "symbol bitcoin · quoted in aud");
    }

    #[test]
    fn a_source_no_unit_uses_says_so_and_has_not_fetched() {
        let mut page = docked();
        page.send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));
        let groups = pane(&page);

        let last = source_group(&groups)
            .facts
            .last()
            .map(|fact| fact.value.clone());
        assert_eq!(last.as_deref(), Some("\u{2717} Not fetched yet"));
        let PaneGroup::Values(values) = &groups[1] else {
            panic!("VALUES second")
        };
        assert!(values.rows.is_empty());
        assert_eq!(values.empty.as_deref(), Some("No units use this source."));
    }

    #[test]
    fn manual_entry_shows_its_note_and_the_units_priced_by_hand_with_no_buttons() {
        let mut page = docked();
        page.send(UnitsPageEvent::Select(UnitsRow::Source(MANUAL_ENTRY)));
        let groups = pane(&page);

        let source = source_group(&groups);
        assert_eq!(source.meta, None);
        assert!(source.facts.is_empty());
        assert!(source.buttons.is_empty());
        assert_eq!(source.note.as_deref(), Some("Prices you enter by hand."));
        let PaneGroup::Values(values) = &groups[1] else {
            panic!("VALUES second")
        };
        let codes: Vec<_> = values.rows.iter().map(|row| row.code.as_str()).collect();
        assert_eq!(codes, ["aud", "vas"]);
    }

    #[test]
    fn a_custom_sources_pane_shows_its_key_parameter_but_never_the_key() {
        let mut page = docked();
        let mut endpoint = crate::built_in(COINGECKO).unwrap().endpoint.clone();
        endpoint.auth = crate::PriceSourceAuth::ApiKey {
            placement: crate::KeyPlacement::Param,
            name: "apikey".into(),
        };
        let custom = crate::interactor::AddPriceSource {
            name: "broker".into(),
            endpoint,
            updates: crate::Updates::Daily,
        }
        .run(&mut page.ports.borrow_mut().price_sources);
        page.send(UnitsPageEvent::Select(UnitsRow::Source(custom.id)));
        let source = source_group(&pane(&page));

        assert_eq!(
            source.meta.as_deref(),
            Some("custom · prices every kind · key needed")
        );
        let facts = facts(&source);
        assert!(facts.contains(&("Source", "broker · custom")));
        assert!(facts.contains(&("Authentication", "API key")));
        assert!(facts.contains(&("Key parameter", "apikey")));
        assert_eq!(source.note, None);
        assert_eq!(
            button_slugs(&source.buttons),
            ["test-source", "edit-source", "delete-source"]
        );
    }

    #[test]
    fn the_plus_buttons_open_their_tables_add_dialog_whatever_is_selected() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Select(UnitsRow::Source(COINGECKO)));
        assert_eq!(
            page.send(UnitsPageEvent::NewUnit),
            vec![RouterCall::OpenDialog(UnitsDialog::AddUnit)]
        );
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_AUD)));
        assert_eq!(
            page.send(UnitsPageEvent::NewPriceSource),
            vec![RouterCall::OpenDialog(UnitsDialog::AddPriceSource)]
        );
    }

    #[test]
    fn a_pane_action_runs_on_its_row_only_when_the_row_offers_it() {
        let mut page = Page::new();
        assert_eq!(
            page.send(UnitsPageEvent::Action(
                MenuAction::EditSource,
                UnitsRow::Source(COINGECKO)
            )),
            vec![RouterCall::OpenDialog(UnitsDialog::EditPriceSource(
                COINGECKO
            ))]
        );
        assert_eq!(
            page.send(UnitsPageEvent::Action(
                MenuAction::DeleteSource,
                UnitsRow::Source(COINGECKO)
            )),
            vec![]
        );
    }

    #[test]
    fn a_removed_selection_falls_back_to_the_first_row() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
        page.ports.borrow_mut().units.units.remove(2);

        assert_eq!(page.selected(), "aud");
    }
}
