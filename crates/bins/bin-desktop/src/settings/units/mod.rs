//! The **Units** Settings page (ADR-0037, and its amendment's one folder per page): the Units
//! table and its Price Sources. [`UnitsPage`] reads both straight from [`LedgerStore`], renders
//! itself (`render.rs`), builds the Add, Edit and Delete unit Dialogs (`dialogs/`) and reaches
//! `Shell` only through [`ViewEvent`]. The Dialog host holds them, and they draw, read back and
//! confirm themselves (`dialogs/`, `apply.rs`), handing each outcome back to [`UnitsPage::land`]. The Settings View forwards the page's keys and status to it while the page has
//! focus. At X-Large the page also docks a detail pane (`render::detail_pane`) and keeps a selected row.

pub mod apply;
pub mod dialogs;
pub mod form;
mod render;

pub(super) use render::detail_pane;

use chrono::Local;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, Pixels, Point, Subscription};
use lib_ledger::Domain;
use lib_list_cursor::{resolve, step_id};
use lib_units::{PriceSourceRow, UnitId, UnitRow};

use crate::{
    chrome::{dialog_host::open_dialog, statusline::PageStatus, window_size::WindowSize},
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    settings::{
        SettingsPage,
        hints::{settings_plain_page_hints, settings_units_hints},
    },
    view::event::ViewEvent,
};
use apply::UnitsOutcome;
use dialogs::UnitsDialog;
use form::{DeleteUnitForm, UnitForm};

impl EventEmitter<ViewEvent> for UnitsPage {}

/// Emitted when a click lands on a row, so the Settings View, which owns the page's focus, moves
/// it from the index into the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitsRowClicked;

impl EventEmitter<UnitsRowClicked> for UnitsPage {}

/// The row a context menu is open on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuTarget {
    Unit(UnitId),
    /// An index into the Price Sources table.
    Source(usize),
}

/// An entry of a row's context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    EditUnit,
    DeleteUnit,
    TestSource,
    EditSource,
    DeleteSource,
}

impl MenuAction {
    /// A row's entries in menu order, the destructive one last.
    pub fn for_target(target: MenuTarget) -> &'static [Self] {
        match target {
            MenuTarget::Unit(_) => &[Self::EditUnit, Self::DeleteUnit],
            MenuTarget::Source(_) => &[Self::TestSource, Self::EditSource, Self::DeleteSource],
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::EditUnit => crate::msg::desktop_settings_units_menu_edit_unit(),
            Self::DeleteUnit => crate::msg::desktop_settings_units_menu_delete_unit(),
            Self::TestSource => crate::msg::desktop_settings_units_menu_test_source(),
            Self::EditSource => crate::msg::desktop_settings_units_menu_edit_source(),
            Self::DeleteSource => crate::msg::desktop_settings_units_menu_delete_source(),
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::EditUnit => "edit-unit",
            Self::DeleteUnit => "delete-unit",
            Self::TestSource => "test-source",
            Self::EditSource => "edit-source",
            Self::DeleteSource => "delete-source",
        }
    }

    pub fn is_destructive(self) -> bool {
        matches!(self, Self::DeleteUnit | Self::DeleteSource)
    }
}

/// The context menu open on a row, drawn at the pointer that opened it (window coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowMenu {
    pub target: MenuTarget,
    pub position: Point<Pixels>,
}

/// One line of the detail pane's ACTIVITY: a Units outcome, with the time it landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEntry {
    pub time: String,
    pub text: String,
}

/// The Units page. Below X-Large a row is reached by double-click and right-click; at X-Large the page keeps a
/// selected row, the detail pane beside it carries the actions, and the page's Success outcomes
/// land in the pane's ACTIVITY instead of raising Toasts (DESKTOP-125).
pub struct UnitsPage {
    ledger: Entity<LedgerStore>,
    /// It may name a row that is gone, so read it through [`Self::selected_id`].
    selected: Option<UnitId>,
    /// Whether the window is X-Large, told by the Settings View as it draws the page.
    docked: bool,
    /// This session's outcomes, newest first.
    activity: Vec<ActivityEntry>,
    /// The row menu, if one is open.
    menu: Option<RowMenu>,
    _units_changed: Subscription,
}

impl UnitsPage {
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _units_changed = notify_on(&ledger, Domain::Units, cx);
        Self {
            ledger,
            selected: None,
            docked: false,
            activity: Vec::new(),
            menu: None,
            _units_changed,
        }
    }

    /// Tells the page the Window Size it is drawn at: X-Large docks the detail pane.
    pub fn set_window_size(&mut self, window_size: WindowSize, cx: &mut Context<'_, Self>) {
        let docked = window_size == WindowSize::XLarge;
        if docked != self.docked {
            self.docked = docked;
            cx.notify();
        }
    }

    /// Whether the detail pane is docked, and so whether outcomes go to ACTIVITY, not Toasts.
    pub fn docked(&self) -> bool {
        self.docked
    }

    /// The selected row's id, or the first row's while nothing (or a removed row) is selected.
    fn selected_id(&self, cx: &App) -> Option<UnitId> {
        resolve(self.units(cx).iter().map(|unit| unit.id), self.selected)
    }

    fn selected_unit<'a>(&self, cx: &'a App) -> Option<&'a UnitRow> {
        let id = self.selected_id(cx)?;
        self.units(cx).iter().find(|unit| unit.id == id)
    }

    /// The Price Sources row that values `unit`: they are keyed by the unit's name.
    fn price_source_of<'a>(&self, unit: &UnitRow, cx: &'a App) -> Option<&'a PriceSourceRow> {
        self.price_sources(cx)
            .iter()
            .find(|source| source.unit_name == unit.name)
    }

    /// The Accounts counted in `unit`, in ledger order.
    fn accounts_in<'a>(&self, unit: &UnitRow, cx: &'a App) -> Vec<&'a lib_accounts::Account> {
        self.ledger
            .read(cx)
            .ledger()
            .accounts()
            .iter()
            .filter(|account| account.unit == unit.id)
            .collect()
    }

    /// Lands a confirmed Dialog. The docked detail pane is already in view beside the row, so it
    /// logs the outcome in ACTIVITY, newest first, instead of raising a Toast (DESKTOP-125).
    fn land(&mut self, outcome: UnitsOutcome, cx: &mut Context<'_, Self>) {
        if !self.docked {
            if let Some((kind, text)) = outcome.toast() {
                cx.emit(ViewEvent::RaiseToast { kind, text });
            }
            return;
        }
        let text = match &outcome {
            UnitsOutcome::Saved { code } => crate::msg::desktop_settings_units_activity_saved(code),
            UnitsOutcome::Deleted { code } => {
                crate::msg::desktop_settings_units_activity_deleted(code)
            }
        };
        let time = Local::now().format("%H:%M").to_string();
        self.activity.insert(0, ActivityEntry { time, text });
        cx.notify();
    }

    fn units<'a>(&self, cx: &'a App) -> &'a [UnitRow] {
        self.ledger.read(cx).ledger().units()
    }

    fn price_sources<'a>(&self, cx: &'a App) -> &'a [PriceSourceRow] {
        self.ledger.read(cx).ledger().price_sources()
    }

    // -----------------------------------------------------------------------------------------
    // The interface the Settings View forwards to while the page has focus (ADR-0037)
    // -----------------------------------------------------------------------------------------

    pub fn key_ahead(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// At X-Large the page's bare `n`, `e`, `d` and `t`, each through the handler its button
    /// uses; below it each Dialog opens from its button. Returns whether the key was one of them.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        self.close_menu(cx);
        let modifiers = &keystroke.modifiers;
        if !self.docked
            || modifiers.control
            || modifiers.alt
            || modifiers.platform
            || modifiers.shift
        {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.click_add(cx),
            "e" => self.edit_selected(cx),
            "d" => self.delete_selected(cx),
            "t" => self.test_selected_source(cx),
            _ => return false,
        }
        true
    }

    /// At X-Large `j`/`k` walk the Units; below it there is no selection to move, so the page
    /// takes no movement and the Settings View scrolls its body instead.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) -> bool {
        self.close_menu(cx);
        if !self.docked {
            return false;
        }
        if let Some(step) = movement.step() {
            let ids: Vec<UnitId> = self.units(cx).iter().map(|unit| unit.id).collect();
            self.selected = step_id(&ids, self.selected_id(cx), step);
            cx.notify();
        }
        true
    }

    /// `Esc` closes an open row menu; otherwise it is left to Settings, which steps back to the
    /// index.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        self.close_menu(cx)
    }

    /// The page has no Search: `/` is inert there.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page legend (the unit keys while docked) and the page's fixed note.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: if self.docked {
                settings_units_hints()
            } else {
                settings_plain_page_hints()
            },
            right: SettingsPage::Units.scope_note(),
        })
    }

    /// The heading's meta: how many Units the table holds.
    pub fn scope_note(&self, cx: &App) -> String {
        let count = i64::try_from(self.units(cx).len()).unwrap_or(i64::MAX);
        crate::msg::desktop_settings_scope_units(count)
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// A click on row `index` selects it and hands the page focus.
    fn click_select(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.menu = None;
        if let Some(unit) = self.units(cx).get(index) {
            self.selected = Some(unit.id);
        }
        cx.emit(UnitsRowClicked);
        cx.notify();
    }

    /// A double-click on a Units row opens its Edit dialog, as the keyboard's `e` does.
    fn double_click_unit(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.click_select(index, cx);
        self.click_edit(index, cx);
    }

    /// A right-click on a row opens its menu at the pointer. A Units row is selected as well.
    fn open_menu(
        &mut self,
        target: MenuTarget,
        position: Point<Pixels>,
        cx: &mut Context<'_, Self>,
    ) {
        if let MenuTarget::Unit(id) = target {
            self.selected = Some(id);
        }
        self.menu = Some(RowMenu { target, position });
        cx.emit(UnitsRowClicked);
        cx.notify();
    }

    /// Closes the row menu. Returns whether one was open.
    fn close_menu(&mut self, cx: &mut Context<'_, Self>) -> bool {
        let was_open = self.menu.take().is_some();
        if was_open {
            cx.notify();
        }
        was_open
    }

    /// An entry of the open row menu: closes it and runs the handler its button or key has.
    fn click_menu_action(&mut self, action: MenuAction, cx: &mut Context<'_, Self>) {
        let Some(RowMenu { target, .. }) = self.menu.take() else {
            return;
        };
        cx.notify();
        let unit_index = match target {
            MenuTarget::Unit(id) => self.units(cx).iter().position(|unit| unit.id == id),
            MenuTarget::Source(_) => None,
        };
        match (action, unit_index) {
            (MenuAction::EditUnit, Some(index)) => self.click_edit(index, cx),
            (MenuAction::DeleteUnit, Some(index)) => self.click_delete(index, cx),
            (MenuAction::TestSource, _) => self.not_built(
                crate::msg::desktop_status_test_price_source_not_yet_built(),
                cx,
            ),
            (MenuAction::EditSource, _) => self.not_built(
                crate::msg::desktop_status_edit_price_source_not_yet_built(),
                cx,
            ),
            (MenuAction::DeleteSource, _) => self.not_built(
                crate::msg::desktop_status_delete_price_source_not_yet_built(),
                cx,
            ),
            (MenuAction::EditUnit | MenuAction::DeleteUnit, None) => {}
        }
    }

    fn edit_selected(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(index) = self.selected_index(cx) {
            self.click_edit(index, cx);
        }
    }

    fn delete_selected(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(index) = self.selected_index(cx) {
            self.click_delete(index, cx);
        }
    }

    /// Testing a price source has no backing yet, so it says so on the status line.
    fn test_selected_source(&mut self, cx: &mut Context<'_, Self>) {
        if self
            .selected_unit(cx)
            .is_some_and(|unit| self.price_source_of(unit, cx).is_some())
        {
            self.not_built(
                crate::msg::desktop_status_test_price_source_not_yet_built(),
                cx,
            );
        }
    }

    fn selected_index(&self, cx: &App) -> Option<usize> {
        let id = self.selected_id(cx)?;
        self.units(cx).iter().position(|unit| unit.id == id)
    }

    /// The square **+** beside UNITS opens the Add unit dialog.
    fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        open_dialog(UnitsDialog::Add(UnitForm::default()), Self::land, cx);
    }

    /// Edit unit (double-click, menu or pane) opens the Edit unit dialog filled from the row.
    fn click_edit(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.units(cx).get(index) else {
            return;
        };
        let form = UnitForm::from_row(row);
        open_dialog(UnitsDialog::Edit(row.id, form), Self::land, cx);
    }

    /// A Units row's **delete** opens the destructive Delete unit confirm dialog.
    fn click_delete(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.units(cx).get(index) else {
            return;
        };
        let form = DeleteUnitForm::new(row.code.as_str());
        open_dialog(UnitsDialog::Delete(row.id, form), Self::land, cx);
    }

    /// Price Sources' test, edit, delete and add have no Dialog yet, so they say so on the status
    /// line.
    fn not_built(&mut self, message: String, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(message)));
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// The Units rows. The open Dialog reads itself back (`Dialog::readback`).
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsUnitsSnapshot {
        let units = self.units(cx);
        SettingsUnitsSnapshot {
            unit_codes: units.iter().map(|unit| unit.code.clone()).collect(),
            unit_rows: units
                .iter()
                .map(|unit| {
                    (
                        unit.code.clone(),
                        unit.name.clone(),
                        unit.kind.as_str().to_string(),
                    )
                })
                .collect(),
            selected_code: self.selected_unit(cx).map(|unit| unit.code.clone()),
            menu: self.menu.map(|menu| {
                let row = match menu.target {
                    MenuTarget::Unit(id) => units
                        .iter()
                        .find(|unit| unit.id == id)
                        .map(|unit| format!("unit:{}", unit.code)),
                    MenuTarget::Source(index) => self
                        .price_sources(cx)
                        .get(index)
                        .map(|source| format!("source:{}", source.unit_name)),
                };
                (
                    row.unwrap_or_default(),
                    MenuAction::for_target(menu.target)
                        .iter()
                        .map(|action| action.slug().to_string())
                        .collect(),
                )
            }),
            activity: self
                .activity
                .iter()
                .map(|entry| entry.text.clone())
                .collect(),
        }
    }
}

/// The Units page: codes in table order and `(code, name, kind)` rows.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsUnitsSnapshot {
    pub unit_codes: Vec<String>,
    pub unit_rows: Vec<(String, String, String)>,
    /// The selected row's code, which is the first row's until another is chosen.
    pub selected_code: Option<String>,
    /// The open row menu: the row it is on (`unit:btc`, `source:Bitcoin`) and its entries' slugs
    /// in order.
    pub menu: Option<(String, Vec<String>)>,
    /// ACTIVITY's lines, newest first.
    pub activity: Vec<String>,
}
