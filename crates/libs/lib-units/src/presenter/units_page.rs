//! The Units Settings page: the Units table above the Price Sources table, one row selected
//! across both, the row menu, and where a confirmed Dialog's outcome lands (a Toast, or ACTIVITY
//! while the detail pane is docked).

use chrono::{DateTime, Utc};
use lib_core::{Clock, PriceSourceId, UnitId};
use lib_list_cursor::{Step, resolve, step_id};
use lib_locale::Label as _;
use lib_locale::format::format_time_minutes;
use lib_toast::{Notice, ToastKind};
use lib_viper::keymap::{
    DELETE, EDIT, KeyBindingConfig, KeyBound, Keymap, MOVE_DOWN, MOVE_UP, NEW,
};
use lib_viper::presenter::{PagePresenter, Presenter};
use lib_viper::view::{Hint, count};

use super::fetch_text::{failure_name, retry_text, round_failed_toast, wait_begun_toast};
use super::price_history::{HistoryInput, PriceHistory, price_history};
use super::price_source_test::PriceSourceTest;
use super::unit_prices::{UnitPricesEvent, UnitPricesPresenter, UnitPricesViewModel};
use super::units_pane::{DetailPane, PaneInput, amount, detail_pane};
use super::{UnitChange, UnitsOutcome, ago, joined};
use crate::fetching::{FetchFailure, PriceFetching, SharedFetching};
use crate::interactor::{FetchAll, FetchNow, FetchSource, ReactivateUnit};
use crate::keymap::{FETCH_NOW, NEXT_TABLE, OPEN, SET_PRICE, TEST_SOURCE, unit_prices_keymap};
use crate::protocols::{SharedPorts, UnitsDialog, UnitsRouter};
use crate::{
    Error, MANUAL_ENTRY, PriceSource, SourceOrigin, UnitRow, last_fetched, msg, units_by_code,
};

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
    /// Fetch price now: on a Unit its price, on a Price Source every Unit it values.
    FetchNow,
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
    /// row offers nothing and a built-in no Delete. With `fetching`, an active Unit and a Price
    /// Source that fetches offer Fetch price now, ahead of Delete (or Test, on a source).
    pub(super) fn for_row(row: Row<'_>, fetching: bool) -> Vec<Self> {
        match row {
            Row::Unit(unit) => {
                let manual = unit.link.source_id == MANUAL_ENTRY;
                let mut actions = vec![Self::Prices, Self::EditUnit];
                if manual {
                    actions.push(Self::SetPrice);
                }
                if !unit.is_active {
                    actions.push(Self::ReactivateUnit);
                } else if fetching && !manual {
                    actions.push(Self::FetchNow);
                }
                actions.push(Self::DeleteUnit);
                actions
            }
            Row::Source(_, SourceKind::Custom) => {
                let mut actions = Vec::new();
                actions.extend(fetching.then_some(Self::FetchNow));
                actions.extend([Self::TestSource, Self::EditSource, Self::DeleteSource]);
                actions
            }
            Row::Source(_, SourceKind::BuiltIn) => {
                let mut actions = Vec::new();
                actions.extend(fetching.then_some(Self::FetchNow));
                actions.extend([Self::TestSource, Self::EditSource]);
                actions
            }
            Row::Source(_, SourceKind::ManualEntry) => Vec::new(),
        }
    }

    fn label(self) -> String {
        match self {
            Self::Prices => msg::units_menu_prices(),
            Self::EditUnit => msg::units_menu_edit_unit(),
            Self::SetPrice => msg::units_menu_set_price(),
            Self::FetchNow => msg::units_menu_fetch(),
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
            Self::FetchNow => msg::units_pane_fetch(),
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
            Self::FetchNow => "fetch-now",
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
    /// `f`: fetch the selected Unit's price now, when its source fetches and it is active, or on a
    /// Price Source row every Unit that source values.
    FetchNow,
    /// The palette's `units fetch-all`: every fetching source's Units, one round per source.
    FetchAll,
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
    /// A key or click on the Unit prices view, which the page hands the view while it shows.
    Prices(UnitPricesEvent),
}

/// The page as drawn. Not `Eq`: the prices view's chart is drawn from plain numbers.
#[derive(Debug, Clone, PartialEq)]
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
    /// The Unit prices view, while it shows in place of the tables and the detail pane.
    pub prices: Option<UnitPricesViewModel>,
    /// PRICE HISTORY: the selected Unit's, or while a Price Source row is selected the last Unit
    /// selected; `None` with no Units.
    pub history: Option<PriceHistory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitColumns {
    pub code: String,
    pub name: String,
    pub kind: String,
    pub price_source: String,
    pub last: String,
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
    /// The newest Weekly Price's close in the quote Unit at its Precision, `148,212.40 aud`, or —
    /// with none.
    pub last_price: String,
    /// How long ago that price was written, `5 minutes ago`; `None` with no price.
    pub last_age: Option<String>,
    /// Whether that price is Out of date, which draws its age in the warning colour.
    pub out_of_date: bool,
    pub is_base: bool,
    /// `base` for the base Unit.
    pub base_flag: Option<String>,
    /// `inactive` for a deactivated Unit, which is drawn dimmed.
    pub inactive_tag: Option<String>,
    /// `fetch failed` while this Client's last fetch of the Unit failed, at either level.
    pub fetch_failed_tag: Option<String>,
    /// The kind, then the flags that apply: `Currency · base`.
    pub kind_meta: String,
    /// The base, inactive and fetch failed flags that apply, `base · inactive`; empty for none.
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
    /// How long ago it last fetched, `Not fetched yet`, or — for Manual entry; while its last
    /// round failed, the round's reason (`Offline`).
    pub last_updated: String,
    /// Whether its last round failed, which draws LAST UPDATED in the warning colour.
    pub failed: bool,
    /// Whether it fetches but has not yet, which draws a muted ✗ before LAST UPDATED.
    pub not_fetched: bool,
    pub selected: bool,
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
    /// The Unit prices view, while it shows in place of the tables: a state of the page, which
    /// opens it on `MenuAction::Prices` and closes it on the view's `UnitPricesEvent::Back`. The
    /// Client's Router is not told: it draws the view from the view model.
    prices: Option<UnitPricesPresenter>,
    /// The keys the prices view is built with, so its legend shows the chords the Client binds.
    prices_keymap: Keymap<UnitPricesEvent>,
    /// The page's keys while the prices view shows ([`Self::prices_keys`]).
    prices_keys: Keymap<UnitsPageEvent>,
    /// The Client's fetching state, without which nothing is fetched and `f` is offered nowhere.
    fetching: Option<SharedFetching>,
}

impl UnitsPagePresenter {
    /// The page and its prices view with their keys built from `config`, as
    /// [`PagePresenter::rebind`] builds them later.
    pub fn new(ports: SharedPorts, clock: Clock, config: &KeyBindingConfig) -> Self {
        let (keymap, prices_keymap, prices_keys) = Self::keymaps(config);
        Self {
            prices_keys,
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
            prices: None,
            prices_keymap,
            fetching: None,
        }
    }

    /// The page fetching prices through `fetching`, the Client's for its open Ledger; `None` for a
    /// Client with no price feed.
    #[must_use]
    pub fn with_fetching(mut self, fetching: Option<SharedFetching>) -> Self {
        self.fetching = fetching;
        self
    }

    /// Whether `f` fetches `unit`: there is fetching, the Unit is active and its source fetches.
    fn fetches(&self, unit: &UnitRow, sources: &[PriceSource<'_>]) -> bool {
        self.fetching.is_some()
            && unit.is_active
            && sources
                .iter()
                .any(|source| source.id == unit.link.source_id && source.endpoint().is_some())
    }

    /// The page's keys, the prices view's, and the page's while the prices view shows, from
    /// `config`.
    fn keymaps(
        config: &KeyBindingConfig,
    ) -> (
        Keymap<UnitsPageEvent>,
        Keymap<UnitPricesEvent>,
        Keymap<UnitsPageEvent>,
    ) {
        let keymap = crate::keymap::units_page_keymap(config);
        let prices_keymap = unit_prices_keymap(config);
        let prices_keys = Self::prices_keys(&keymap, &prices_keymap);
        (keymap, prices_keymap, prices_keys)
    }

    /// Whether the Unit prices view shows in place of the tables.
    pub fn prices_open(&self) -> bool {
        self.prices.is_some()
    }

    /// The page's keys while the prices view shows: only the view's chords bind, each sending
    /// [`UnitsPageEvent::Prices`]. The page's own commands keep their names but lose their
    /// chords, and come first, so a palette Command reaches the page's event by name, which
    /// [`PagePresenter::shows`] closes the view ahead of, as it did before the view was the
    /// page's (`units.set_price` among them, which both bind).
    fn prices_keys(
        page: &Keymap<UnitsPageEvent>,
        prices: &Keymap<UnitPricesEvent>,
    ) -> Keymap<UnitsPageEvent> {
        page.scoped(|_| false)
            .chain(prices.map(UnitsPageEvent::Prices))
    }

    /// Shows `unit`'s prices view in place of the tables, the selection kept for the way back.
    fn open_prices(&mut self, unit: UnitId) {
        self.prices = Some(
            UnitPricesPresenter::new(
                self.ports.clone(),
                self.clock.clone(),
                self.prices_keymap.clone(),
                unit,
            )
            .with_fetching(self.fetching.clone()),
        );
    }
}

impl Presenter for UnitsPagePresenter {
    type Event = UnitsPageEvent;
    type ViewModel = UnitsPageViewModel;
    type Router<'r> = dyn UnitsRouter + 'r;

    /// Handles `event`, a prices view's on the open view.
    fn handle(&mut self, event: UnitsPageEvent, router: &mut Self::Router<'_>) {
        self.handle_event(event, router);
    }

    fn view_model(&self) -> UnitsPageViewModel {
        self.view()
    }

    /// The prices view's events while it shows, `SetDocked` only for a change, and every other
    /// event of the page.
    fn takes(&self, event: &UnitsPageEvent) -> bool {
        match event {
            UnitsPageEvent::Prices(_) => self.prices.is_some(),
            UnitsPageEvent::SetDocked(docked) => *docked != self.docked,
            _ => true,
        }
    }
}

impl UnitsPagePresenter {
    fn handle_event(&mut self, event: UnitsPageEvent, router: &mut dyn UnitsRouter) {
        // Any event but the menu's own closes it, as a key or click elsewhere does.
        let menu = self.menu.take();
        match event {
            UnitsPageEvent::Move(step) => {
                let rows = self.rows();
                if let Some(row) = step_id(&rows, self.selection_in(&rows), step) {
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
            UnitsPageEvent::FetchNow => self.run_on_selection(MenuAction::FetchNow, router),
            UnitsPageEvent::FetchAll => self.fetch_all(router),
            UnitsPageEvent::Open => match self.selection() {
                Some(UnitsRow::Unit(_)) => self.run_on_selection(MenuAction::Prices, router),
                _ => self.edit(router),
            },
            UnitsPageEvent::OpenMenu(row) => {
                let fetching = self.fetching.is_some();
                if self.with_row(row, |row| !MenuAction::for_row(row, fetching).is_empty())
                    == Some(true)
                {
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
                let dialog = self.open_dialog.take();
                if let Some(outcome) = outcome {
                    if let UnitsOutcome::SourceSaved {
                        key_stored: Some(source),
                        ..
                    } = &outcome
                        && let Some(fetching) = &self.fetching
                    {
                        fetching.borrow_mut().key_stored(*source);
                    }
                    router.notify(Notice::Changed(outcome.changed(dialog)));
                    self.land(outcome, router);
                }
            }
            // The view's Back is the page's to act on: it closes the view, which leaves the
            // selection as it was.
            UnitsPageEvent::Prices(UnitPricesEvent::Back) => self.prices = None,
            UnitsPageEvent::Prices(event) => {
                if let Some(prices) = self.prices.as_mut() {
                    prices.handle(event, router);
                }
            }
        }
    }

    fn view(&self) -> UnitsPageViewModel {
        let ports = self.ports.borrow();
        let units = ports.units().units();
        let catalogue = ports.source_catalogue();
        let base = ports.base_unit().base_unit();
        let sources = catalogue.sources();
        let by_code = units_by_code(&units);
        let selection = resolve(Self::rows_of(&by_code, &sources), self.selected);
        let now = self.clock.now();
        let fetching = self.fetching.as_ref().map(|fetching| fetching.borrow());
        let fetching = fetching.as_deref();
        let source_of = |unit: &UnitRow| {
            sources
                .iter()
                .find(|source| source.id == unit.link.source_id)
        };
        let unit_failed = |unit: &UnitRow| {
            fetching
                .zip(source_of(unit))
                .and_then(|(fetching, source)| fetching.unit_fetch(unit, source))
                .is_some_and(|last| last.failure().is_some())
        };
        let out_of_date = |unit: &UnitRow| {
            unit.link.source_id != MANUAL_ENTRY
                && unit.is_out_of_date(source_of(unit).map(|source| source.updates), now)
        };
        let history = self.history_unit(&by_code, selection).map(|unit| {
            price_history(
                &HistoryInput {
                    units: &units,
                    sources: &sources,
                    base,
                    open_chord: self.keymap.hint_chord(OPEN),
                    today: self.clock.today(),
                    out_of_date: out_of_date(unit),
                    fetch_failed: unit_failed(unit),
                    backfill: source_of(unit).and_then(|source| {
                        no_history_text(source, unit.id == base).or_else(|| {
                            fetching.and_then(|fetching| backfill_text(fetching, unit, source, now))
                        })
                    }),
                },
                unit,
            )
        });

        let unit_lines = by_code
            .iter()
            .map(|unit| {
                let base_flag = (unit.id == base).then(msg::units_flag_base);
                let inactive_tag = (!unit.is_active).then(msg::units_inactive);
                let fetch_failed_tag = unit_failed(unit).then(msg::units_flag_fetch_failed);
                let flags = [
                    base_flag.clone(),
                    inactive_tag.clone(),
                    fetch_failed_tag.clone(),
                ];
                let last = unit.last_price();
                let quote = units.iter().find(|quote| quote.id == unit.link.quote);
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
                    last_price: last
                        .map_or_else(msg::units_none, |price| amount(&price.close, quote)),
                    last_age: if fetching.is_some_and(|fetching| fetching.unit_asked_for(unit.id)) {
                        Some(msg::units_fetching())
                    } else {
                        last.map(|price| ago(price.written_at, now))
                    },
                    out_of_date: out_of_date(unit),
                    is_base: unit.id == base,
                    kind_meta: joined(
                        std::iter::once(unit.kind.label())
                            .chain(flags.clone().into_iter().flatten()),
                    ),
                    flags: joined(flags.into_iter().flatten()),
                    base_flag,
                    inactive_tag,
                    fetch_failed_tag,
                    selected: selection == Some(UnitsRow::Unit(unit.id)),
                }
            })
            .collect();
        let source_lines = sources
            .iter()
            .map(|source| {
                let fetched = last_fetched(&units, source.id);
                let fetches = source.endpoint().is_some();
                let asked = fetching.is_some_and(|fetching| fetching.source_asked_for(source.id));
                let round_failure = fetching
                    .and_then(|fetching| fetching.source_fetch(source))
                    .and_then(|last| last.failure());
                SourceLine {
                    id: source.id,
                    kind: SourceKind::of(source),
                    name: source.name.to_string(),
                    address: source
                        .endpoint()
                        .map_or_else(msg::units_none, |endpoint| endpoint.address.to_string()),
                    last_updated: match (fetched, round_failure) {
                        _ if !fetches => msg::units_none(),
                        _ if asked => msg::units_fetching(),
                        (_, Some(reason)) => failure_name(reason),
                        (Some(at), None) => ago(at, now),
                        (None, None) => msg::units_not_fetched_yet(),
                    },
                    not_fetched: fetches && !asked && fetched.is_none() && round_failure.is_none(),
                    failed: round_failure.is_some(),
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
        let fetching_on = fetching.is_some();
        let entries = |row: Row<'_>| {
            MenuAction::for_row(row, fetching_on)
                .iter()
                .map(|action| action.entry())
                .collect::<Vec<_>>()
        };

        UnitsPageViewModel {
            scope: msg::units_scope(count(units.len())),
            unit_columns: UnitColumns {
                code: lib_locale::msg::column_code(),
                name: lib_locale::msg::column_name(),
                kind: lib_locale::msg::column_type(),
                price_source: msg::units_column_price_source(),
                last: msg::units_column_last(),
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
            hints: self.hints(
                selected_row,
                selected_row.is_some_and(|row| {
                    MenuAction::for_row(row, fetching_on).contains(&MenuAction::FetchNow)
                }),
            ),
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
                    history: history.as_ref(),
                    fetching,
                })
            }),
            open_dialog: self.open_dialog,
            prices: self.prices.as_ref().map(UnitPricesPresenter::view_model),
            history,
        }
    }
}

impl PagePresenter for UnitsPagePresenter {
    type Outcome = UnitsOutcome;

    fn dialog_closed(outcome: Option<UnitsOutcome>) -> Option<UnitsPageEvent> {
        Some(UnitsPageEvent::DialogClosed(outcome))
    }

    /// The prices view's keys while it shows, else the page's.
    fn keymap(&self) -> &Keymap<UnitsPageEvent> {
        if self.prices.is_some() {
            &self.prices_keys
        } else {
            &self.keymap
        }
    }

    /// The page's keys, then the prices view's.
    fn bindings(&self) -> Vec<KeyBound<'_>> {
        self.keymap
            .bindings()
            .chain(self.prices_keymap.bindings())
            .collect()
    }

    /// The page's keys and the prices view's.
    fn rebind(&mut self, keys: &KeyBindingConfig) {
        (self.keymap, self.prices_keymap, self.prices_keys) = Self::keymaps(keys);
    }

    /// A page event while the prices view shows closes the view first, so a palette Command acts
    /// on the tables the person then sees.
    fn shows(&self, event: &UnitsPageEvent) -> Option<UnitsPageEvent> {
        (self.prices.is_some() && !matches!(event, UnitsPageEvent::Prices(_)))
            .then_some(UnitsPageEvent::Prices(UnitPricesEvent::Back))
    }
}

impl UnitsPagePresenter {
    /// `units` are already in code order (see [`units_by_code`]), so the caller sorts once.
    fn rows_of(units: &[&UnitRow], sources: &[PriceSource<'_>]) -> Vec<UnitsRow> {
        units
            .iter()
            .map(|unit| UnitsRow::Unit(unit.id))
            .chain(sources.iter().map(|source| UnitsRow::Source(source.id)))
            .collect()
    }

    /// The Unit PRICE HISTORY shows: the selected one, or while a Price Source row is selected the
    /// last Unit selected, or the first, so the history doesn't jump as the selection leaves the
    /// Units.
    fn history_unit<'a>(
        &self,
        by_code: &[&'a UnitRow],
        selection: Option<UnitsRow>,
    ) -> Option<&'a UnitRow> {
        let id = match selection {
            Some(UnitsRow::Unit(id)) => Some(id),
            _ => resolve(by_code.iter().map(|unit| unit.id), self.last_unit),
        }?;
        by_code.iter().copied().find(|unit| unit.id == id)
    }

    /// Every row of both tables in drawn order: the Units, then the Price Sources.
    fn rows(&self) -> Vec<UnitsRow> {
        let ports = self.ports.borrow();
        let units = ports.units().units();
        Self::rows_of(&units_by_code(&units), &ports.source_catalogue().sources())
    }

    /// The selected row, or the first while nothing (or a removed row) is selected.
    fn selection(&self) -> Option<UnitsRow> {
        self.selection_in(&self.rows())
    }

    /// `selection` over rows the caller has already built, so one input reads the Ledger once.
    fn selection_in(&self, rows: &[UnitsRow]) -> Option<UnitsRow> {
        resolve(rows.iter().copied(), self.selected)
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
        let target = match self.selection_in(&rows) {
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
        let fetching = self.fetching.is_some();
        let offered = self.with_row(row, |found| {
            MenuAction::for_row(found, fetching).contains(&action)
        });
        if offered != Some(true) {
            return;
        }
        match (action, row) {
            (MenuAction::Prices, UnitsRow::Unit(id)) => self.open_prices(id),
            (MenuAction::EditUnit, UnitsRow::Unit(id)) => {
                self.open(UnitsDialog::EditUnit(id), router);
            }
            (MenuAction::FetchNow, UnitsRow::Unit(id)) => self.fetch_now(id, router),
            (MenuAction::FetchNow, UnitsRow::Source(id)) => self.fetch_source(id, router),
            (MenuAction::SetPrice, UnitsRow::Unit(id)) => {
                self.open(UnitsDialog::SetPrice(id), router);
            }
            (MenuAction::DeleteUnit, UnitsRow::Unit(id)) => {
                self.open(UnitsDialog::DeleteUnit(id), router);
            }
            (MenuAction::ReactivateUnit, UnitsRow::Unit(id)) => self.reactivate(id, router),
            (MenuAction::TestSource, UnitsRow::Source(id)) => self.test_source(id, router),
            (MenuAction::EditSource, UnitsRow::Source(id)) => {
                self.open(UnitsDialog::EditPriceSource(id), router);
            }
            (MenuAction::DeleteSource, UnitsRow::Source(id)) => {
                self.open(UnitsDialog::DeletePriceSource(id), router);
            }
            _ => {}
        }
    }

    /// `t`: on a Price Source row its own Test, on a Unit row a test of that Unit's link (none on
    /// Manual entry).
    fn test(&mut self, router: &mut dyn UnitsRouter) {
        match self.selection() {
            Some(UnitsRow::Unit(id)) => {
                let unit = self.ports.borrow().units().unit(id);
                if let Some(unit) = unit.filter(|unit| unit.link.source_id != MANUAL_ENTRY) {
                    self.test_link(&unit, router);
                }
            }
            Some(row @ UnitsRow::Source(_)) => self.run(MenuAction::TestSource, row, router),
            None => {}
        }
    }

    /// Tests source `id` on the first Unit (by code) it values, or with none opens its Edit
    /// Dialog, whose Test says why it can't.
    fn test_source(&mut self, id: PriceSourceId, router: &mut dyn UnitsRouter) {
        if self.fetching.is_none() {
            return;
        }
        let first = super::first_linked(&*self.ports.borrow(), id);
        match first {
            Some(unit) => self.test_link(&unit, router),
            None => self.open(UnitsDialog::EditPriceSource(id), router),
        }
    }

    /// Tests `unit`'s link on its source in the background, writing nothing: its response raises
    /// one Toast. A press while that source's test is out, or with no key where one is needed,
    /// answers at once instead.
    fn test_link(&mut self, unit: &UnitRow, router: &mut dyn UnitsRouter) {
        let Some(fetching) = &self.fetching else {
            return;
        };
        let refusal =
            PriceSourceTest::in_background(fetching, &self.clock, &*self.ports.borrow(), unit);
        if let Some(refusal) = refusal {
            router.notify(refusal);
        }
    }

    /// Asks the Unit's source for its price; the response writes it. A Unit fetching isn't
    /// offered on is left alone. Never undoable, so it raises no `Notice::Changed`. A source
    /// needing a key this Client doesn't have sends nothing and says so in a Warning Toast.
    fn fetch_now(&mut self, id: UnitId, router: &mut dyn UnitsRouter) {
        let Some(fetching) = &self.fetching else {
            return;
        };
        let ports = self.ports.borrow();
        let offered = ports
            .units()
            .unit(id)
            .is_some_and(|unit| self.fetches(&unit, &ports.source_catalogue().sources()));
        if !offered {
            return;
        }
        // Any other refusal is logged by the Interactor; the checks above leave none but a race.
        let now = self.clock.now();
        let sent = FetchNow { id }.run(&*ports, &mut fetching.borrow_mut(), now);
        if sent.is_ok() {
            // Held by its source's `Retry-After`, it says so once, as the wait begins.
            if let Some(toast) = wait_begun_toast(&*ports, &mut fetching.borrow_mut(), now) {
                router.notify(toast);
            }
            return;
        }
        if !matches!(sent, Err(Error::KeyMissing)) {
            return;
        }
        let catalogue = ports.source_catalogue();
        if let Some(source) = ports
            .units()
            .unit(id)
            .and_then(|unit| catalogue.source(unit.link.source_id))
        {
            router.notify(Notice::Toast(
                ToastKind::Warning,
                round_failed_toast(FetchFailure::KeyMissing, source.name),
            ));
        }
    }

    /// Asks one Price Source for every Unit it values, as one round.
    fn fetch_source(&mut self, id: PriceSourceId, router: &mut dyn UnitsRouter) {
        let Some(fetching) = &self.fetching else {
            return;
        };
        let ports = self.ports.borrow();
        let outcome = FetchSource { id }.run(&*ports, &mut fetching.borrow_mut(), self.clock.now());
        self.tell_started(&[(id, outcome)], true, router);
    }

    /// The palette's `units fetch-all`: every fetching source, one round each.
    fn fetch_all(&mut self, router: &mut dyn UnitsRouter) {
        let Some(fetching) = &self.fetching else {
            return;
        };
        let ports = self.ports.borrow();
        let outcomes = FetchAll.run(&*ports, &mut fetching.borrow_mut(), self.clock.now());
        self.tell_started(&outcomes, false, router);
    }

    /// Tells what asking each source came to: an Info Toast as its round starts, an Info Toast for
    /// a source already out and a Warning Toast for a missing key; the rows read *Fetching…* until
    /// it lands. A source no active Unit uses sends nothing; asked alone it says so.
    fn tell_started(
        &self,
        outcomes: &[(PriceSourceId, Result<usize, Error>)],
        alone: bool,
        router: &mut dyn UnitsRouter,
    ) {
        let ports = self.ports.borrow();
        let catalogue = ports.source_catalogue();
        let name = |id| {
            catalogue
                .source(id)
                .map(|source| source.name.to_string())
                .unwrap_or_default()
        };
        for (id, outcome) in outcomes {
            match outcome {
                Ok(0) if alone => {
                    router.notify(Notice::Status(msg::units_status_fetch_no_units(&name(*id))));
                }
                Ok(0) => {}
                Ok(_) => router.notify(Notice::Toast(
                    ToastKind::Info,
                    msg::units_toast_fetching(&name(*id)),
                )),
                Err(Error::AlreadyFetching) => router.notify(Notice::Toast(
                    ToastKind::Info,
                    msg::units_toast_already_fetching(&name(*id)),
                )),
                Err(Error::KeyMissing) => router.notify(Notice::Toast(
                    ToastKind::Warning,
                    round_failed_toast(FetchFailure::KeyMissing, &name(*id)),
                )),
                // Gone or Manual entry: the rules that offer the action leave only a race.
                Err(_) => {}
            }
        }
    }

    /// Reactivating loses nothing to undo, so it runs at once, with no Dialog.
    fn reactivate(&mut self, id: UnitId, router: &mut dyn UnitsRouter) {
        let reactivated = {
            let mut ports = self.ports.borrow_mut();
            ReactivateUnit(id).run(ports.split_mut().units)
        };
        if let Ok(unit) = reactivated {
            router.notify(Notice::Changed(msg::units_changed_reactivated(&unit.code)));
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
            let time = format_time_minutes(self.clock.local_time(self.clock.now()).time());
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

    /// The legend for `row`'s table, with Set price while `row` offers it and Fetch price while
    /// `fetch` (`f` fetches a Unit's price, or a Price Source's Units').
    fn hints(&self, row: Option<Row<'_>>, fetch: bool) -> Vec<Hint> {
        let chord = |command| self.keymap.hint_chord(command);
        let moves = self.keymap.hint_chords(&[MOVE_DOWN, MOVE_UP]);
        let on_source = matches!(row, Some(Row::Source(..)));
        let set_price =
            row.is_some_and(|row| MenuAction::for_row(row, true).contains(&MenuAction::SetPrice));
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
            Hint::new(moves, row),
            Hint::new(chord(EDIT), msg::units_hint_edit()),
            Hint::new(chord(DELETE), msg::units_hint_delete()),
            Hint::new(chord(NEW), msg::units_hint_new()),
        ];
        if !on_source {
            hints.push(Hint::new(chord(OPEN), msg::units_hint_prices()));
        }
        if set_price {
            hints.push(Hint::new(chord(SET_PRICE), msg::units_hint_set_price()));
        }
        if fetch {
            hints.push(Hint::new(chord(FETCH_NOW), msg::units_hint_fetch()));
        }
        hints.push(Hint::new(chord(TEST_SOURCE), test));
        hints.push(Hint::new(chord(NEXT_TABLE), other));
        hints
    }
}

/// What PRICE HISTORY says of a Unit on `source` that never backfills because its source has no
/// history request (ADR-0043's amendment "a Custom source's history request"); `None` for the
/// base Unit, Manual entry, which never fetches, and a source with one.
fn no_history_text(source: &PriceSource<'_>, is_base: bool) -> Option<String> {
    let endpoint = source.endpoint()?;
    (!is_base && endpoint.history.is_none()).then(|| msg::units_history_no_request(source.name))
}

/// What PRICE HISTORY says of `unit`'s backfill at `now`: *Fetching history…* while its history
/// request is out, *History failed · Offline · tries again in 8 minutes* after one failed, else
/// nothing.
fn backfill_text(
    fetching: &PriceFetching,
    unit: &UnitRow,
    source: &PriceSource<'_>,
    now: DateTime<Utc>,
) -> Option<String> {
    if fetching.history_fetching(unit.id) {
        return Some(msg::units_history_backfilling());
    }
    let (reason, retry_at) = fetching.history_failure(unit, source)?;
    Some(msg::units_history_backfill_failed(
        &failure_name(reason),
        &retry_text(retry_at, now),
    ))
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;

    use super::super::test_support::{self, clock, sample};
    use super::*;
    use crate::fake::{InMemoryPorts, RecordingRouter, RouterCall};
    use crate::presenter::{PaneButton, PaneGroup, PriceGroup, SourceGroup, Trend};
    use crate::protocols::UnitRepository as _;
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
            let (ports, shared) = sample();
            Self {
                ports,
                presenter: UnitsPagePresenter::new(shared, clock(), &config),
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
    fn units_are_listed_by_code_ignoring_case_wherever_the_store_holds_them() {
        let page = Page::new();
        let mut first = page.ports.borrow().units.units[2].clone();
        first.code = "Aaa".into();
        // Appended last, as an Undo restore appends.
        page.ports.borrow_mut().units.units.push(first);

        let codes: Vec<_> = page.view().units.iter().map(|u| u.code.clone()).collect();
        assert_eq!(codes, ["Aaa", "AUD", "BTC", "VAS"]);
        assert_eq!(page.selected(), "Aaa");
    }

    #[test]
    fn bindings_hold_the_prices_views_keys_while_it_is_closed() {
        let page = Page::new();
        assert!(!page.presenter.prices_open());

        let bindings = page.presenter.bindings();
        let chord = |command: &str| {
            bindings
                .iter()
                .find(|bound| bound.command == command)
                .map(|bound| bound.chord)
        };

        assert_eq!(chord(OPEN), page.presenter.keymap().chord(OPEN));
        assert!(chord(crate::keymap::EARLIER).is_some_and(|chord| !chord.is_empty()));
        assert!(
            page.presenter
                .keymap()
                .chord(crate::keymap::EARLIER)
                .is_none()
        );
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
                ("AUD", "Currency", "Manual entry"),
                ("BTC", "Crypto", "CoinGecko"),
                ("VAS", "Equity", "Manual entry"),
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
        assert_eq!(view.unit_columns.last, "Last");
        let last: Vec<_> = view
            .units
            .iter()
            .map(|unit| (unit.last_price.as_str(), unit.last_age.as_deref()))
            .collect();
        assert_eq!(
            last,
            [
                ("—", None),
                ("148,212.40 AUD", Some("5 minutes ago")),
                ("—", None),
            ],
            "BTC's newest Weekly Price at AUD's Precision; AUD and VAS have none"
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
                    "https://api.frankfurter.dev",
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
        assert_eq!(page.selected(), "AUD");
    }

    #[test]
    fn moving_walks_both_tables_as_one_list_and_clamps_at_the_ends() {
        let mut page = Page::new();

        page.send(UnitsPageEvent::Move(Step::Prev));
        assert_eq!(page.selected(), "AUD");
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
        assert_eq!(page.selected(), "AUD");
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
        assert_eq!(page.selected(), "VAS");
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
        page.send(UnitsPageEvent::Open);
        assert_eq!(
            page.view().prices.map(|prices| prices.unit),
            Some(UnitId::SEED_BTC)
        );
        assert_eq!(
            page.selected(),
            "BTC",
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
        page.send(UnitsPageEvent::Menu(MenuAction::Prices));
        assert_eq!(
            page.view().prices.map(|prices| prices.unit),
            Some(UnitId::SEED_BTC)
        );
        page.send(UnitsPageEvent::Action(
            MenuAction::Prices,
            UnitsRow::Unit(UnitId::SEED_VAS),
        ));
        assert_eq!(
            page.view().prices.map(|prices| prices.unit),
            Some(UnitId::SEED_VAS)
        );
    }

    #[test]
    fn the_prices_view_is_a_state_of_the_page_with_its_own_keys_and_back_closes_it() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        page.send(UnitsPageEvent::Open);

        assert!(page.presenter.prices_open());
        let prices = page.view().prices.expect("the prices view shows");
        assert_eq!(prices.unit, UnitId::SEED_BTC);
        let keymap = page.presenter.keymap();
        assert_eq!(
            keymap.event("["),
            Some(UnitsPageEvent::Prices(UnitPricesEvent::Earlier))
        );
        assert_eq!(keymap.event("n"), None, "the tables' keys don't bind");
        assert_eq!(
            keymap.event_for(NEW),
            Some(UnitsPageEvent::New),
            "the palette still reaches the page's commands"
        );
        assert_eq!(
            page.presenter.shows(&UnitsPageEvent::New),
            Some(UnitsPageEvent::Prices(UnitPricesEvent::Back))
        );

        page.send(UnitsPageEvent::Prices(UnitPricesEvent::Back));
        assert!(page.view().prices.is_none());
        assert!(
            !page
                .presenter
                .takes(&UnitsPageEvent::Prices(UnitPricesEvent::Back))
        );
        assert_eq!(
            page.presenter.keymap().event("n"),
            Some(UnitsPageEvent::New)
        );
        assert_eq!(page.selected(), "BTC", "the selection is as it was");
    }

    #[test]
    fn the_prices_view_opens_its_set_price_dialog_through_the_page() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Action(
            MenuAction::Prices,
            UnitsRow::Unit(UnitId::SEED_VAS),
        ));

        assert_eq!(
            page.send(UnitsPageEvent::Prices(UnitPricesEvent::SetPrice)),
            [RouterCall::OpenDialog(UnitsDialog::SetPrice(
                UnitId::SEED_VAS
            ))]
        );
    }

    #[test]
    fn docking_is_taken_only_as_a_change() {
        let mut page = Page::new();
        assert!(!page.presenter.takes(&UnitsPageEvent::SetDocked(false)));
        assert!(page.presenter.takes(&UnitsPageEvent::SetDocked(true)));

        page.send(UnitsPageEvent::SetDocked(true));
        assert!(!page.presenter.takes(&UnitsPageEvent::SetDocked(true)));
    }

    #[test]
    fn a_dialog_the_router_could_not_open_is_not_shown_as_open() {
        struct Refusing;
        impl UnitsRouter for Refusing {
            fn open_dialog(&mut self, _dialog: UnitsDialog) -> bool {
                false
            }
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
        let menu = page.view().menu.expect("VAS has actions");
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
        assert_eq!(page.selected(), "VAS", "opening the menu selects its row");

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
            .expect("VAS has actions")
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
                code: "VAS".into(),
                week: chrono::NaiveDate::from_ymd_opt(2026, 10, 19).expect("a real date"),
            }))
        };

        assert_eq!(
            page.send(set()),
            vec![
                RouterCall::Notice(Notice::Changed(
                    "Set the price of VAS for the week of Oct 19, 2026".into()
                )),
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Set the price of VAS for the week of Oct 19, 2026".into()
                ))
            ]
        );
        page.send(UnitsPageEvent::SetDocked(true));
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
        page.send(set());

        assert_eq!(
            page.view().activity[0].text,
            "Set the price of VAS for the week of Oct 19, 2026"
        );
        let source = source_group(&pane(&page));
        assert_eq!(
            facts(&source),
            [
                ("Source", "Manual entry"),
                ("Last price", "101.20 AUD"),
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
            vec![
                RouterCall::Notice(Notice::Changed("Reactivate unit BTC".into())),
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Reactivated unit BTC".into()
                ))
            ]
        );
        assert!(page.ports.borrow().units.units[1].is_active);
        assert_eq!(
            page.send(UnitsPageEvent::Reactivate),
            vec![],
            "an active Unit has none"
        );
    }

    /// History names each change by what it did, which a save tells from the Dialog it came from.
    #[test]
    fn a_unit_saved_in_the_add_dialog_is_named_an_add_and_one_edited_an_edit() {
        let mut page = Page::new();
        let saved = || {
            UnitsPageEvent::DialogClosed(Some(UnitsOutcome::Unit {
                change: UnitChange::Saved,
                code: "NZD".into(),
            }))
        };

        page.send(UnitsPageEvent::NewUnit);
        let added = page.send(saved());
        page.send(UnitsPageEvent::Edit);
        let edited = page.send(saved());

        assert_eq!(
            added,
            [RouterCall::Notice(Notice::Changed("Add unit NZD".into()))]
        );
        assert_eq!(
            edited,
            [RouterCall::Notice(Notice::Changed("Edit unit NZD".into()))]
        );
    }

    #[test]
    fn an_outcome_toasts_below_x_large_and_goes_to_activity_when_docked() {
        let mut page = Page::new();
        let deleted = || UnitsOutcome::Unit {
            change: UnitChange::Deleted,
            code: "BTC".into(),
        };

        let changed = |label: &str| RouterCall::Notice(Notice::Changed(label.into()));
        assert_eq!(
            page.send(UnitsPageEvent::DialogClosed(Some(deleted()))),
            vec![
                changed("Delete unit BTC"),
                RouterCall::Notice(Notice::Toast(ToastKind::Success, "Deleted unit BTC".into()))
            ]
        );
        assert_eq!(
            page.send(UnitsPageEvent::DialogClosed(Some(UnitsOutcome::Unit {
                change: UnitChange::Saved,
                code: "ETH".into()
            }))),
            vec![changed("Edit unit ETH")],
            "a save raises no Toast"
        );

        page.send(UnitsPageEvent::SetDocked(true));
        assert_eq!(
            page.send(UnitsPageEvent::DialogClosed(Some(deleted()))),
            vec![changed("Delete unit BTC")]
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
        assert_eq!(lines, ["Deleted price source Broker", "Deleted unit BTC"]);
    }

    #[test]
    fn the_legend_follows_the_selected_table_and_the_keymaps_chords() {
        let mut page = Page::with(KeyBindingConfig::overriding("edit", "E"));
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
            "BTC's source prices it"
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

        assert_eq!(
            slugs(&groups),
            ["unit", "history", "source", "price", "accounts", "activity"]
        );
        let PaneGroup::Unit(unit) = &groups[0] else {
            panic!("UNIT first")
        };
        assert_eq!(unit.title, "AUD · Australian Dollar");
        assert_eq!(unit.meta, "Currency · valued in AUD");
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
                ("Last price", "148,212.40 AUD"),
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

    fn price_group(groups: &[PaneGroup]) -> PriceGroup {
        groups
            .iter()
            .find_map(|group| match group {
                PaneGroup::Price(price) => Some(price.clone()),
                _ => None,
            })
            .expect("a PRICE group")
    }

    #[test]
    fn a_selected_units_unit_group_shows_its_precision() {
        let mut page = docked();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        let PaneGroup::Unit(unit) = &pane(&page)[0] else {
            panic!("UNIT first")
        };

        let precision = unit
            .facts
            .iter()
            .find(|fact| fact.label == "Precision")
            .map(|fact| fact.value.as_str());
        assert_eq!(precision, Some("8"), "a satoshi's places");
    }

    #[test]
    fn seeded_btc_shows_price_between_its_price_source_and_its_accounts() {
        let mut page = docked();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        let groups = pane(&page);

        assert_eq!(
            slugs(&groups),
            ["unit", "history", "source", "price", "accounts", "activity"]
        );
        let price = price_group(&groups);
        assert_eq!(price.heading, "Price");
        assert_eq!(price.empty, None);
        let labels: Vec<_> = price.facts.iter().map(|fact| fact.label.as_str()).collect();
        assert_eq!(labels, ["Last price", "Week change", "52w range"]);
        assert_eq!(price.facts[0].value, "148,212.40 AUD · 5 minutes ago");
    }

    #[test]
    fn price_reads_the_week_change_and_the_lowest_and_highest_of_the_last_52_weeks() {
        let mut page = docked();
        {
            let money = |text: &str| lib_core::Money(text.parse().expect("a decimal"));
            let week = |y, m, d| chrono::NaiveDate::from_ymd_opt(y, m, d).expect("a valid date");
            let price = |monday, close, high: Option<&str>| crate::WeeklyPrice {
                week: monday,
                close: money(close),
                open: None,
                high: high.map(money),
                low: None,
                written_at: test_support::now() - chrono::Duration::minutes(5),
            };
            let mut ports = page.ports.borrow_mut();
            let vas = ports
                .units
                .units
                .iter_mut()
                .find(|unit| unit.id == UnitId::SEED_VAS)
                .expect("the sample's VAS");
            vas.prices = vec![
                // 53 weeks before the newest: out of the range.
                price(week(2025, 10, 20), "50", None),
                price(week(2026, 10, 12), "100", Some("120")),
                price(week(2026, 10, 19), "110", None),
            ];
        }
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
        let price = price_group(&pane(&page));

        let values: Vec<_> = price.facts.iter().map(|fact| fact.value.as_str()).collect();
        assert_eq!(
            values,
            [
                "110.00 AUD · 5 minutes ago",
                "+10.00 AUD (+10.00%)",
                "100.00 – 120.00 AUD",
            ]
        );
    }

    #[test]
    fn a_unit_with_no_weekly_prices_shows_the_price_groups_empty_message() {
        let mut page = docked();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
        let price = price_group(&pane(&page));

        assert_eq!(price.facts, []);
        assert_eq!(price.empty.as_deref(), Some("No weekly prices yet."));
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
        let PaneGroup::Accounts(accounts) = &groups[4] else {
            panic!("ACCOUNTS fourth")
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
                "0.41200000 BTC",
                Some("61,063.51 AUD")
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
        assert_eq!(source.facts[3].value, "148,212 AUD", "AUD's 0 places");
        let PaneGroup::Accounts(accounts) = &groups[4] else {
            panic!("ACCOUNTS fourth")
        };
        // 0.4125 × 148,212.40 = 61,137.615: rounded for display, never as stored.
        assert_eq!(
            (
                accounts.rows[0].balance.as_str(),
                accounts.rows[0].value.as_deref()
            ),
            ("0.413 BTC", Some("61,138 AUD"))
        );
        page.send(UnitsPageEvent::Select(UnitsRow::Source(COINGECKO)));
        let PaneGroup::Values(values) = &pane(&page)[1] else {
            panic!("VALUES second")
        };
        assert_eq!(values.rows[0].price, "148,212 AUD");
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
            Some("built-in · prices Crypto · key optional")
        );
        assert_eq!(source.attribution.as_deref(), Some("Powered by CoinGecko"));
        assert_eq!(
            facts(&source),
            [
                ("Source", "CoinGecko · built-in"),
                ("Address", "https://api.coingecko.com/api/v3"),
                (
                    "Request",
                    "/simple/price?ids={symbol_lower}&vs_currencies={quote_lower}"
                ),
                ("Price path", "/{symbol_lower}/{quote_lower}"),
                ("API key", "Not set"),
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
                "BTC",
                Some(("symbol".to_string(), "bitcoin".to_string())),
                "quoted in AUD",
                "148,212.40 AUD"
            )
        );
        assert_eq!(row.detail, "symbol bitcoin · quoted in AUD");
    }

    #[test]
    fn a_unit_coingecko_prices_credits_it_and_yahoos_pane_says_its_api_is_unofficial() {
        let mut page = docked();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        assert_eq!(
            source_group(&pane(&page)).attribution.as_deref(),
            Some("Powered by CoinGecko")
        );

        page.send(UnitsPageEvent::Select(UnitsRow::Source(YAHOO_FINANCE)));
        assert_eq!(
            source_group(&pane(&page)).attribution.as_deref(),
            Some("Yahoo Finance's API is unofficial and may stop working.")
        );

        page.send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));
        assert_eq!(source_group(&pane(&page)).attribution, None);
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
        assert_eq!(codes, ["AUD", "VAS"]);
    }

    #[test]
    fn a_custom_sources_pane_shows_its_key_parameter_but_never_the_key() {
        let mut page = docked();
        let mut endpoint = crate::built_in(COINGECKO).unwrap().endpoint.clone();
        endpoint.auth = crate::PriceSourceAuth::ApiKey {
            placement: crate::KeyPlacement::Param,
            name: "apikey".into(),
            optional: false,
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
    fn a_keyed_sources_api_key_fact_says_saved_not_set_or_unreadable_here() {
        use crate::fake::InMemoryKey;
        let mut page = docked();
        page.send(UnitsPageEvent::Select(UnitsRow::Source(COINGECKO)));
        let api_key = |page: &Page| {
            source_group(&pane(page))
                .facts
                .iter()
                .find(|fact| fact.label == "API key")
                .map(|fact| fact.value.clone())
        };
        assert_eq!(api_key(&page).as_deref(), Some("Not set"));

        let saved = InMemoryKey::Saved("CG-1".into());
        page.ports.borrow_mut().keys.keys.insert(COINGECKO, saved);
        assert_eq!(api_key(&page).as_deref(), Some("Saved"));

        let unreadable = InMemoryKey::Unreadable;
        page.ports
            .borrow_mut()
            .keys
            .keys
            .insert(COINGECKO, unreadable);
        assert_eq!(
            api_key(&page).as_deref(),
            Some("Saved, but can't be read on this device")
        );

        page.send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));
        assert_eq!(
            api_key(&page),
            None,
            "a source that takes no key has no fact"
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

        assert_eq!(page.selected(), "AUD");
    }

    /// Fetch now on the page, through the fake transport: no HTTP in `lib-units`.
    mod fetch_now {
        use lib_toast::{Notice, ToastKind};

        use super::*;
        use crate::fake::LedgerPricesFixture;
        use crate::presenter::test_support::prices;
        use crate::{Precision, PriceLink, UnitKind};

        const BODY: &str = r#"{"date":"2026-10-21","base":"EUR","quote":"AUD","rate":1.6512}"#;

        struct Fetching {
            page: Page,
            prices: LedgerPricesFixture,
            eur: UnitId,
        }

        /// The sample with EUR on Frankfurter, quoted in aud, on a page that fetches.
        fn fetching() -> Fetching {
            let prices = prices();
            let unit = UnitRow::new(
                "EUR".into(),
                "Euro".into(),
                UnitKind::Currency,
                Precision::of_kind(UnitKind::Currency),
                PriceLink {
                    source_id: FRANKFURTER,
                    symbol: "EUR".into(),
                    quote: UnitId::SEED_AUD,
                },
            );
            let eur = unit.id;
            prices.ports().borrow_mut().units.units.push(unit);
            let presenter = UnitsPagePresenter::new(
                prices.shared(),
                prices.clock(),
                &KeyBindingConfig::default(),
            )
            .with_fetching(Some(prices.fetching().clone()));
            Fetching {
                page: Page {
                    ports: prices.ports().clone(),
                    presenter,
                    router: RecordingRouter::default(),
                },
                prices,
                eur,
            }
        }

        fn offers_fetch(page: &Page) -> bool {
            page.view()
                .hints
                .iter()
                .any(|hint| hint.chord == "f" && hint.action == "fetch price")
        }

        #[test]
        fn f_on_a_frankfurter_unit_asks_frankfurter_v2_and_its_answer_writes_the_price() {
            let mut on = fetching();
            on.page.send(UnitsPageEvent::Select(UnitsRow::Unit(on.eur)));
            assert!(offers_fetch(&on.page));

            assert_eq!(on.page.send(UnitsPageEvent::FetchNow), vec![]);
            assert_eq!(
                on.prices.transport().urls(),
                ["https://api.frankfurter.dev/v2/rate/EUR/AUD"]
            );

            let notices = on.prices.land(on.prices.transport().answer_last(200, BODY));

            assert_eq!(
                notices,
                [Notice::Toast(
                    ToastKind::Success,
                    "Fetched EUR at 1.65 AUD".into()
                )]
            );
            let line = on
                .page
                .view()
                .units
                .into_iter()
                .find(|unit| unit.code == "EUR")
                .expect("EUR is listed");
            assert_eq!(line.last_price, "1.65 AUD");
            assert_eq!(on.prices.fetching().borrow().in_flight(), 0);
        }

        /// Adds an active `code` on `link`, presses `f` on it, answers the one request sent with
        /// `body` and returns that request and the Unit's last price as listed.
        fn fetch_answered(
            on: &mut Fetching,
            code: &str,
            kind: UnitKind,
            link: PriceLink,
            body: &str,
        ) -> (crate::PriceRequest, String) {
            let unit = UnitRow::new(
                code.into(),
                code.into(),
                kind,
                Precision::of_kind(kind),
                link,
            );
            let id = unit.id;
            on.page.ports.borrow_mut().units.units.push(unit);
            on.page.send(UnitsPageEvent::Select(UnitsRow::Unit(id)));
            assert!(offers_fetch(&on.page), "{code}");
            on.page.send(UnitsPageEvent::FetchNow);
            let requests = on.prices.transport().requests();
            let [(_, request)] = requests.as_slice() else {
                panic!("one request for {code}, not {requests:?}");
            };
            on.prices.land(on.prices.transport().answer_last(200, body));
            let line = on
                .page
                .view()
                .units
                .into_iter()
                .find(|line| line.code == code)
                .expect("listed");
            (request.clone(), line.last_price)
        }

        #[test]
        fn f_on_a_coingecko_unit_asks_keyless_in_lower_case_and_its_answer_writes_the_price() {
            let mut on = fetching();
            let (request, price) = fetch_answered(
                &mut on,
                "ETH",
                UnitKind::Crypto,
                PriceLink {
                    source_id: COINGECKO,
                    symbol: "bitcoin".into(),
                    quote: UnitId::SEED_AUD,
                },
                include_str!("../../fixtures/prices/coingecko_simple_price.json"),
            );

            assert_eq!(
                request.url,
                "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=aud"
            );
            assert_eq!(request.headers, [], "keyless, with no key stored");
            assert_eq!(price, "92,568.00 AUD");
        }

        #[test]
        fn f_on_a_yahoo_unit_sends_its_user_agent_and_its_answer_writes_the_price() {
            let mut on = fetching();
            let (request, price) = fetch_answered(
                &mut on,
                "BHP",
                UnitKind::Equity,
                PriceLink {
                    source_id: YAHOO_FINANCE,
                    symbol: "BHP.AX".into(),
                    quote: UnitId::SEED_AUD,
                },
                include_str!("../../fixtures/prices/yahoo_chart.json"),
            );

            assert_eq!(
                request.headers,
                [(
                    "User-Agent".to_string(),
                    format!("Personal-Ledger/{}", env!("CARGO_PKG_VERSION"))
                )]
            );
            assert_eq!(price, "52.52 AUD");
        }

        #[test]
        fn f_on_a_custom_sources_unit_sends_its_headers_as_entered_and_writes_the_price() {
            let mut on = fetching();
            let source = crate::interactor::AddPriceSource {
                name: "Broker".into(),
                endpoint: crate::Endpoint {
                    address: "https://broker.example".to_string().into(),
                    request: "/quote/{symbol_lower}?in={quote}".to_string().into(),
                    price_path: "/quote/last".to_string().into(),
                    auth: crate::PriceSourceAuth::None,
                    headers: vec![crate::RequestHeader {
                        name: "X-Client".to_string().into(),
                        value: "ledger; v1".to_string().into(),
                    }]
                    .into(),
                    batch: None,
                    history: None,
                },
                updates: crate::Updates::Hourly,
            }
            .run(&mut on.page.ports.borrow_mut().price_sources);

            let (request, price) = fetch_answered(
                &mut on,
                "ABC",
                UnitKind::Equity,
                PriceLink {
                    source_id: source.id,
                    symbol: "ABC".into(),
                    quote: UnitId::SEED_AUD,
                },
                r#"{"quote":{"last":"12.5"}}"#,
            );

            assert_eq!(request.url, "https://broker.example/quote/abc?in=AUD");
            assert_eq!(
                request.headers,
                [("X-Client".to_string(), "ledger; v1".to_string())]
            );
            assert_eq!(price, "12.50 AUD");
        }

        #[test]
        fn f_is_not_offered_and_sends_nothing_on_manual_entry_or_a_deactivated_unit() {
            let mut on = fetching();
            on.page
                .send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
            assert!(!offers_fetch(&on.page));
            on.page.send(UnitsPageEvent::FetchNow);

            on.page
                .ports
                .borrow_mut()
                .units
                .units
                .iter_mut()
                .find(|unit| unit.id == on.eur)
                .expect("eur")
                .is_active = false;
            on.page.send(UnitsPageEvent::Select(UnitsRow::Unit(on.eur)));
            assert!(!offers_fetch(&on.page));
            on.page.send(UnitsPageEvent::FetchNow);

            assert!(on.prices.transport().requests().is_empty());
        }

        fn notices(calls: Vec<RouterCall>) -> Vec<Notice> {
            calls
                .into_iter()
                .filter_map(|call| match call {
                    RouterCall::Notice(notice) => Some(notice),
                    _ => None,
                })
                .collect()
        }

        fn info(text: &str) -> Notice {
            Notice::Toast(ToastKind::Info, text.into())
        }

        #[test]
        fn f_on_a_source_row_fetches_its_units_and_tells_it_started_on_a_toast_alone() {
            let mut on = fetching();
            on.page
                .send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));
            assert!(offers_fetch(&on.page));

            let told = notices(on.page.send(UnitsPageEvent::FetchNow));

            assert_eq!(told, [info("Fetching prices from Frankfurter\u{2026}")]);
            assert_eq!(
                on.prices.transport().urls(),
                ["https://api.frankfurter.dev/v2/rate/EUR/AUD"]
            );
            assert_eq!(
                eur_line(&on.page).last_age.as_deref(),
                Some("Fetching\u{2026}")
            );
            assert_eq!(frankfurter_line(&on.page).last_updated, "Fetching\u{2026}");
        }

        #[test]
        fn a_second_press_while_the_round_is_out_gives_an_info_toast_and_sends_nothing() {
            let mut on = fetching();
            on.page
                .send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));
            on.page.send(UnitsPageEvent::FetchNow);

            let told = notices(on.page.send(UnitsPageEvent::FetchNow));

            assert_eq!(told, [info("Already fetching prices from Frankfurter")]);
            assert_eq!(on.prices.transport().requests().len(), 1);
        }

        #[test]
        fn the_round_landing_writes_one_toast_and_clears_the_rows() {
            let mut on = fetching();
            let usd = UnitRow::new(
                "USD".into(),
                "US dollar".into(),
                UnitKind::Currency,
                Precision::of_kind(UnitKind::Currency),
                PriceLink {
                    source_id: FRANKFURTER,
                    symbol: "USD".into(),
                    quote: UnitId::SEED_AUD,
                },
            );
            on.page.ports.borrow_mut().units.units.push(usd);
            on.page
                .send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));
            on.page.send(UnitsPageEvent::FetchNow);
            let mut notices = on.prices.land(on.prices.transport().answer_last(200, BODY));
            assert_eq!(notices, [], "the round is held until its last response");
            let first = on.prices.transport().requests()[0].0;
            notices.extend(on.prices.land(on.prices.transport().answer(
                first,
                crate::TransportOutcome::Answered {
                    status: 200,
                    body: BODY.into(),
                    retry_after: None,
                },
            )));

            assert_eq!(
                notices,
                [Notice::Toast(
                    ToastKind::Success,
                    "Fetched 2 prices from Frankfurter".into()
                )]
            );
            assert_ne!(
                eur_line(&on.page).last_age.as_deref(),
                Some("Fetching\u{2026}")
            );
            assert_ne!(frankfurter_line(&on.page).last_updated, "Fetching\u{2026}");
        }

        #[test]
        fn fetch_all_starts_a_round_on_every_fetching_source_with_a_toast_each() {
            let mut on = fetching();

            let told = notices(on.page.send(UnitsPageEvent::FetchAll));

            let sources = on.page.view().price_sources;
            assert!(sources.iter().any(|source| source.id == COINGECKO));
            assert_eq!(
                told,
                [
                    info("Fetching prices from CoinGecko\u{2026}"),
                    info("Fetching prices from Frankfurter\u{2026}"),
                ]
            );
            let after = on.prices.land(on.prices.transport().answer_last(200, BODY));
            assert!(
                !after
                    .iter()
                    .any(|notice| matches!(notice, Notice::Status(_))),
                "a landing round leaves the status line to the Shell: {after:?}"
            );
        }

        #[test]
        fn fetch_all_while_every_round_is_out_only_gives_info_toasts() {
            let mut on = fetching();
            on.page.send(UnitsPageEvent::FetchAll);
            let sent = on.prices.transport().requests().len();

            let told = notices(on.page.send(UnitsPageEvent::FetchAll));

            assert!(
                told.iter()
                    .all(|notice| matches!(notice, Notice::Toast(ToastKind::Info, _))),
                "{told:?}"
            );
            assert_eq!(on.prices.transport().requests().len(), sent);
        }

        #[test]
        fn fetch_price_now_is_in_both_row_menus_but_not_on_manual_entry_or_a_deactivated_unit() {
            let mut on = fetching();
            let eur = on.eur;
            let entries = |on: &mut Fetching, row| {
                on.page.send(UnitsPageEvent::OpenMenu(row));
                on.page
                    .view()
                    .menu
                    .map(|menu| {
                        menu.entries
                            .into_iter()
                            .map(|e| e.label)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            };

            assert!(entries(&mut on, UnitsRow::Unit(eur)).contains(&"Fetch price now".into()));
            assert!(
                entries(&mut on, UnitsRow::Source(FRANKFURTER)).contains(&"Fetch price now".into())
            );
            assert!(
                !entries(&mut on, UnitsRow::Unit(UnitId::SEED_VAS))
                    .contains(&"Fetch price now".into())
            );
            assert!(entries(&mut on, UnitsRow::Source(MANUAL_ENTRY)).is_empty());
            on.page
                .ports
                .borrow_mut()
                .units
                .units
                .iter_mut()
                .find(|unit| unit.id == on.eur)
                .expect("eur")
                .is_active = false;
            assert!(!entries(&mut on, UnitsRow::Unit(eur)).contains(&"Fetch price now".into()));
        }

        #[test]
        fn units_fetch_all_is_in_the_palette() {
            let page = UnitsPagePresenter::new(sample().1, clock(), &KeyBindingConfig::default());

            assert_eq!(
                page.keymap().event_for("units.fetch_all"),
                Some(UnitsPageEvent::FetchAll)
            );
            assert!(
                page.keymap()
                    .listed()
                    .any(|listed| listed.name == "units fetch-all")
            );
        }

        fn eur_line(page: &Page) -> UnitLine {
            page.view()
                .units
                .into_iter()
                .find(|unit| unit.code == "EUR")
                .expect("EUR is listed")
        }

        fn frankfurter_line(page: &Page) -> SourceLine {
            page.view()
                .price_sources
                .into_iter()
                .find(|source| source.id == FRANKFURTER)
                .expect("Frankfurter is listed")
        }

        fn last_fetch(group: &SourceGroup) -> Option<String> {
            group
                .facts
                .iter()
                .find(|fact| fact.label == "Last fetch")
                .map(|fact| fact.value.clone())
        }

        /// Fetches EUR now and answers it with `status` and `body`, through the Client's door.
        fn fetch_eur(on: &mut Fetching, status: u16, body: &str) -> Vec<Notice> {
            on.page.send(UnitsPageEvent::Select(UnitsRow::Unit(on.eur)));
            on.page.send(UnitsPageEvent::FetchNow);
            on.prices
                .land(on.prices.transport().answer_last(status, body))
        }

        #[test]
        fn f_held_by_a_retry_after_raises_one_waiting_toast_and_the_pump_adds_nothing() {
            let mut on = fetching();
            on.page.send(UnitsPageEvent::Select(UnitsRow::Unit(on.eur)));
            on.page.send(UnitsPageEvent::FetchNow);
            on.prices
                .land(on.prices.transport().answer_last_with(429, "", Some("40")));

            let told = notices(on.page.send(UnitsPageEvent::FetchNow));
            let pumped = on.prices.pump();

            assert_eq!(told, [info("Waiting for Frankfurter, 40 s\u{2026}")]);
            assert_eq!(pumped, [], "the page already said so");
        }

        #[test]
        fn price_history_shows_a_fetch_nows_backfill_in_place_then_its_failure() {
            let mut on = fetching();
            let today = test_support::now().date_naive();
            on.page
                .ports
                .borrow_mut()
                .units
                .units
                .iter_mut()
                .find(|unit| unit.id == on.eur)
                .expect("eur")
                .created_on = Some(today - chrono::Duration::weeks(3));
            let backfill = |page: &Page| page.view().history.expect("eur's history").backfill;

            let notices = fetch_eur(&mut on, 200, BODY);
            assert_eq!(
                backfill(&on.page).as_deref(),
                Some("Fetching history\u{2026}")
            );
            assert!(
                notices.iter().all(
                    |notice| !matches!(notice, Notice::Toast(_, text) if text.contains("history"))
                ),
                "a backfill never toasts"
            );

            let notices = on.prices.land(on.prices.transport().answer_last(503, ""));

            assert_eq!(
                backfill(&on.page).as_deref(),
                Some("History failed \u{b7} Source error \u{b7} tries again in 10 minutes")
            );
            assert_eq!(notices, [], "no Toast, and no status");
            assert_eq!(
                eur_line(&on.page).fetch_failed_tag,
                None,
                "a history failure never flags the fetch"
            );
        }

        #[test]
        fn a_unit_failure_flags_the_row_the_history_and_the_panes_last_fetch() {
            let mut on = fetching();
            on.page.send(UnitsPageEvent::SetDocked(true));
            let before = eur_line(&on.page);
            assert_eq!(before.fetch_failed_tag, None);
            assert_eq!(
                last_fetch(&source_group(&pane(&on.page))),
                None,
                "not tried yet"
            );

            fetch_eur(&mut on, 404, r#"{"message":"not found"}"#);

            let line = eur_line(&on.page);
            assert_eq!(line.fetch_failed_tag.as_deref(), Some("fetch failed"));
            assert_eq!(line.flags, "fetch failed");
            assert_eq!(line.kind_meta, "Currency · fetch failed");
            assert_eq!(
                last_fetch(&source_group(&pane(&on.page))).as_deref(),
                Some(
                    "Failed just now · Not found: Frankfurter has no EUR in AUD · tries again in 24 hours"
                )
            );
            assert_eq!(
                on.page
                    .view()
                    .history
                    .expect("eur's history")
                    .warning
                    .as_deref(),
                Some("⚠ fetch failed")
            );
            let source = frankfurter_line(&on.page);
            assert!(!source.failed, "a Unit failure stays on its Unit's row");
        }

        #[test]
        fn a_round_failure_shows_its_reason_in_last_updated_until_the_next_success() {
            let mut on = fetching();
            on.page.send(UnitsPageEvent::SetDocked(true));

            fetch_eur(&mut on, 503, "");

            let source = frankfurter_line(&on.page);
            assert_eq!(source.last_updated, "Source error");
            assert!(source.failed);
            assert!(!source.not_fetched);
            on.page
                .send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));
            assert_eq!(
                last_fetch(&source_group(&pane(&on.page))).as_deref(),
                Some(
                    "Failed just now · Source error: Frankfurter answered with an error · tries again in 24 hours"
                )
            );

            fetch_eur(&mut on, 200, BODY);

            let source = frankfurter_line(&on.page);
            assert_eq!(source.last_updated, "just now");
            assert!(!source.failed);
            assert_eq!(eur_line(&on.page).fetch_failed_tag, None);
            on.page
                .send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));
            assert_eq!(
                last_fetch(&source_group(&pane(&on.page))).as_deref(),
                Some("Fetched just now")
            );
        }

        #[test]
        fn a_price_past_twice_its_sources_interval_is_out_of_date_but_never_on_manual_entry() {
            let on = fetching();
            let age = |page: &Page, id: UnitId| {
                page.view()
                    .units
                    .into_iter()
                    .find(|unit| unit.id == id)
                    .expect("listed")
                    .out_of_date
            };
            assert!(
                !age(&on.page, UnitId::SEED_BTC),
                "five minutes on a 15-minute source"
            );
            for unit in &mut on.page.ports.borrow_mut().units.units {
                for price in &mut unit.prices {
                    price.written_at = clock().now() - chrono::Duration::minutes(31);
                }
            }

            assert!(age(&on.page, UnitId::SEED_BTC));
            assert!(!age(&on.page, UnitId::SEED_VAS), "Manual entry");
            assert!(!age(&on.page, on.eur), "no price yet");
            let mut page = on.page;
            page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
            assert_eq!(
                page.view()
                    .history
                    .expect("btc's history")
                    .warning
                    .as_deref(),
                Some("⚠ Out of date")
            );
        }

        #[test]
        fn f_on_a_source_needing_a_key_sends_nothing_and_warns_key_missing() {
            let mut on = fetching();
            let id = PriceSourceId::new();
            on.page.ports.borrow_mut().price_sources.records.push(
                crate::PriceSourceRecord::Custom(crate::CustomSource {
                    id,
                    name: "Rates".into(),
                    endpoint: crate::Endpoint {
                        address: "https://rates.example".into(),
                        request: "/{symbol}".into(),
                        price_path: "/rate".into(),
                        auth: crate::PriceSourceAuth::ApiKey {
                            placement: crate::KeyPlacement::Header,
                            name: "x-key".into(),
                            optional: false,
                        },
                        headers: Vec::new().into(),
                        batch: None,
                        history: None,
                    },
                    updates: crate::Updates::Hourly,
                }),
            );
            on.page
                .ports
                .borrow_mut()
                .units
                .units
                .iter_mut()
                .find(|unit| unit.id == on.eur)
                .expect("eur")
                .link
                .source_id = id;
            on.page.send(UnitsPageEvent::Select(UnitsRow::Unit(on.eur)));

            let calls = on.page.send(UnitsPageEvent::FetchNow);

            assert!(on.prices.transport().requests().is_empty());
            assert_eq!(
                calls,
                [RouterCall::Notice(Notice::Toast(
                    ToastKind::Warning,
                    "Couldn't fetch from Rates: no API key on this device".into()
                ))]
            );
            assert_eq!(
                eur_line(&on.page).fetch_failed_tag.as_deref(),
                Some("fetch failed")
            );
        }

        #[test]
        fn a_page_with_no_fetching_offers_no_f() {
            let mut page = Page::new();
            page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));

            assert!(!offers_fetch(&page));
        }

        /// `t` on a row: a test in the background, answered with one Toast and writing nothing.
        mod testing {
            use super::*;
            use crate::price_source;

            /// Hands the newest request's answer to the fetched-prices Presenter, returning what
            /// it raised.
            fn answer(on: &mut Fetching, status: u16, body: &str) -> Vec<Notice> {
                on.prices
                    .land(on.prices.transport().answer_last(status, body))
            }

            /// Whether the test left any trace: a price on EUR, or a fetch held for it or
            /// Frankfurter.
            fn wrote_nothing(on: &Fetching) -> bool {
                let ports = on.page.ports.borrow();
                let eur = ports.units.units.iter().find(|unit| unit.id == on.eur);
                let records = ports.price_sources.records.clone();
                let source =
                    price_source(&records, "Manual entry", FRANKFURTER).expect("a built-in");
                let fetching = on.prices.fetching().borrow();
                eur.is_some_and(|eur| eur.prices.is_empty())
                    && fetching.source_fetch(&source).is_none()
                    && eur.is_some_and(|eur| fetching.unit_fetch(eur, &source).is_none())
                    && ports.keys.keys.is_empty()
            }

            #[test]
            fn t_on_a_price_source_tests_its_first_unit_and_a_price_raises_one_info_toast() {
                let mut on = fetching();
                on.page
                    .send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));

                assert_eq!(on.page.send(UnitsPageEvent::Test), vec![]);
                assert_eq!(
                    on.prices.transport().urls(),
                    ["https://api.frankfurter.dev/v2/rate/EUR/AUD"]
                );
                assert!(
                    on.page
                        .view()
                        .units
                        .iter()
                        .all(|unit| unit.last_age != Some("Fetching\u{2026}".into())),
                    "a test is no fetch"
                );

                assert_eq!(
                    answer(&mut on, 200, BODY),
                    [Notice::Toast(
                        ToastKind::Info,
                        "Frankfurter works: EUR · 1.65 AUD · 0 ms".into()
                    )]
                );
                assert!(wrote_nothing(&on));
            }

            #[test]
            fn t_on_a_source_valuing_no_unit_opens_its_edit_dialog_and_on_manual_entry_nothing() {
                let mut on = fetching();
                let eur = on.eur;
                on.page
                    .ports
                    .borrow_mut()
                    .units
                    .units
                    .retain(|unit| unit.id != eur);
                on.page
                    .send(UnitsPageEvent::Select(UnitsRow::Source(FRANKFURTER)));

                assert_eq!(
                    on.page.send(UnitsPageEvent::Test),
                    [RouterCall::OpenDialog(UnitsDialog::EditPriceSource(
                        FRANKFURTER
                    ))]
                );
                on.page.send(UnitsPageEvent::DialogClosed(None));
                on.page
                    .send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_AUD)));
                assert_eq!(on.page.send(UnitsPageEvent::Test), vec![]);
                assert!(on.prices.transport().requests().is_empty());
            }
        }
    }

    fn history(page: &Page) -> PriceHistory {
        page.view().history.expect("a PRICE HISTORY")
    }

    #[test]
    fn price_history_shows_the_selected_units_newest_twelve_weeks_with_this_week_provisional() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        let history = history(&page);

        assert_eq!(history.unit, UnitId::SEED_BTC);
        assert_eq!(history.heading, "Price history");
        assert_eq!(history.subtitle, "BTC · CoinGecko");
        assert_eq!(history.flag.as_deref(), Some("this week · provisional"));
        assert_eq!(history.open_hint, "enter all prices");
        let columns = &history.columns;
        assert_eq!(
            [
                &columns.week,
                &columns.open,
                &columns.high,
                &columns.low,
                &columns.close,
                &columns.change
            ],
            ["Week", "Open", "High", "Low", "Close", "Change"]
        );
        assert_eq!(history.rows.len(), 12);
        // The tests' clock reads Wednesday 21 Oct 2026, in the week of Monday 19 Oct.
        let newest = &history.rows[0];
        assert_eq!(newest.label, "Oct 19, 2026");
        assert_eq!(newest.close, "148,212.40");
        assert_eq!(newest.provisional.as_deref(), Some("provisional"));
        assert!(
            newest.open.contains('.') && newest.change.ends_with('%'),
            "{newest:?}"
        );
        assert_eq!(history.rows[1].provisional, None);
        assert_eq!(history.rows[11].label, "Aug 3, 2026");
        for row in &history.rows {
            let expected = if row.change.starts_with('-') || row.change.starts_with('\u{2212}') {
                Trend::Falling
            } else {
                Trend::Rising
            };
            assert_eq!(row.trend, expected, "{row:?}");
        }
        assert_eq!(
            history.count.as_deref(),
            Some("Newest week first · 12 of 52 weeks")
        );
        assert_eq!(
            history.pane_hint.as_deref(),
            Some("enter opens all 52 weeks")
        );
        assert_eq!(history.sparkline.len(), 52);
        assert_eq!(history.sparkline.last(), Some(&148_212.40));
        assert_eq!(history.empty, None);
    }

    #[test]
    fn price_history_keeps_the_last_unit_while_a_price_source_is_selected() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        page.send(UnitsPageEvent::NextTable);
        assert!(
            page.view()
                .price_sources
                .iter()
                .any(|source| source.selected)
        );
        assert_eq!(history(&page).unit, UnitId::SEED_BTC);

        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
        assert_eq!(history(&page).unit, UnitId::SEED_VAS);
    }

    #[test]
    fn a_unit_on_a_custom_source_with_no_history_request_says_its_past_weeks_fill_only_by_hand() {
        let mut page = Page::new();
        let endpoint = crate::Endpoint {
            address: "https://prices.example".to_string().into(),
            request: "/{symbol}".to_string().into(),
            price_path: "/price".to_string().into(),
            auth: crate::PriceSourceAuth::None,
            headers: Vec::new().into(),
            batch: None,
            history: None,
        };
        let source = {
            let mut ports = page.ports.borrow_mut();
            let source = crate::add_custom_source(
                &mut ports.price_sources.records,
                "Prices".into(),
                endpoint,
                crate::Updates::Daily,
            );
            let vas = ports
                .units
                .units
                .iter_mut()
                .find(|unit| unit.id == UnitId::SEED_VAS)
                .expect("VAS is on the sample");
            vas.link.source_id = source.id;
            source
        };

        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
        assert_eq!(
            history(&page).backfill.as_deref(),
            Some("Prices has no history request, so past weeks fill only by hand")
        );

        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        assert_eq!(history(&page).backfill, None, "CoinGecko has one");

        {
            let mut ports = page.ports.borrow_mut();
            let mut endpoint = source.endpoint.clone();
            endpoint.history = Some(crate::HistoryRef::from(crate::HistoryEndpoint {
                request: "/h/{symbol}".to_string().into(),
                series_path: "".into(),
                date_path: "/d".to_string().into(),
                price_path: "/p".to_string().into(),
                shape: crate::SeriesShape::Points,
            }));
            crate::edit_custom_source(
                &mut ports.price_sources.records,
                source.id,
                "Prices".into(),
                crate::Updates::Daily,
                Some(endpoint),
            );
        }
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
        assert_eq!(history(&page).backfill, None, "once it has one");
    }

    #[test]
    fn price_history_says_why_the_base_and_an_unpriced_unit_have_none() {
        let mut page = Page::new();
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_AUD)));
        let base = history(&page);
        assert_eq!(base.subtitle, "AUD");
        assert_eq!(
            base.empty.as_deref(),
            Some("AUD is the base unit: everything is valued in it, so it has no price.")
        );
        assert!(base.rows.is_empty() && base.count.is_none() && base.flag.is_none());

        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
        let vas = history(&page);
        assert_eq!(vas.subtitle, "VAS · Manual entry");
        assert_eq!(
            vas.empty.as_deref(),
            Some("VAS has no prices yet. A fetch or backfill will fill this.")
        );
        assert!(vas.pane_hint.is_none() && vas.sparkline.is_empty());
    }

    #[test]
    fn a_manual_entry_units_history_shows_only_its_priced_weeks_with_no_open_high_or_low() {
        let mut page = Page::new();
        let mut vas = page
            .ports
            .borrow()
            .units
            .unit(UnitId::SEED_VAS)
            .expect("VAS");
        let at = test_support::now();
        let day = |d| chrono::NaiveDate::from_ymd_opt(2026, 10, d).expect("a valid date");
        vas.set_price(
            lib_core::Money(bigdecimal::BigDecimal::from(42)),
            day(5),
            day(21),
            at,
        );
        vas.set_price(
            lib_core::Money(bigdecimal::BigDecimal::from(40)),
            day(21),
            day(21),
            at,
        );
        vas.set_price(
            lib_core::Money(bigdecimal::BigDecimal::from(41)),
            day(21),
            day(21),
            at,
        );
        page.ports.borrow_mut().units.upsert_unit(vas);
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_VAS)));
        let history = history(&page);

        let weeks: Vec<_> = history.rows.iter().map(|row| row.label.as_str()).collect();
        assert_eq!(weeks, ["Oct 19, 2026", "Oct 5, 2026"]);
        let newest = &history.rows[0];
        assert_eq!(newest.close, "41.00");
        assert_eq!(
            (
                newest.open.as_str(),
                newest.high.as_str(),
                newest.low.as_str()
            ),
            ("—", "—", "—")
        );
        assert_eq!(newest.change, "—", "no week before it to compare");
        assert_eq!(newest.trend, Trend::Flat);
        assert_eq!(
            history.count.as_deref(),
            Some("Newest week first · 2 of 2 weeks")
        );
    }

    #[test]
    fn docked_the_pane_carries_price_history_after_unit_and_rebinding_open_renames_its_hints() {
        let mut page = Page::with(KeyBindingConfig::overriding("units.open", "o"));
        page.send(UnitsPageEvent::SetDocked(true));
        page.send(UnitsPageEvent::Select(UnitsRow::Unit(UnitId::SEED_BTC)));
        let groups = pane(&page);

        let PaneGroup::History(group) = &groups[1] else {
            panic!("PRICE HISTORY after UNIT: {:?}", slugs(&groups))
        };
        assert_eq!(Some(&**group), page.view().history.as_ref());
        assert_eq!(group.open_hint, "o all prices");
        assert_eq!(group.pane_hint.as_deref(), Some("o opens all 52 weeks"));

        page.send(UnitsPageEvent::NextTable);
        assert!(
            !slugs(&pane(&page)).contains(&"history"),
            "a Price Source's pane has none"
        );
    }
}
