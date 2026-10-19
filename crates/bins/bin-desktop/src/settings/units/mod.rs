//! The **Units** Settings page (ADR-0037, and its amendment's one folder per page): the Units
//! table and its Price Sources. [`UnitsPage`] reads both straight from [`LedgerStore`], renders
//! itself (`render.rs`), builds the Add, Edit and Delete unit and price source Dialogs
//! (`dialogs/`) and reaches `Shell` only through [`ViewEvent`]. The Dialog host holds them, and
//! they draw, read back and confirm themselves (`dialogs/`, `apply.rs`, `source_apply.rs`), handing
//! each outcome back to [`UnitsPage::land`] or [`UnitsPage::land_source`]. The Settings View
//! forwards the page's keys and status to it while the page has focus. One row is selected at a
//! time across both tables ([`UnitsSelection`]); at X-Large the page also docks a detail pane
//! (`render::detail_pane`) that follows it.

pub mod apply;
pub mod dialogs;
pub mod form;
mod pane;
mod render;
pub mod source_apply;
pub mod source_form;

pub(super) use render::detail_pane;

use chrono::Local;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, Pixels, Point, Subscription};
use lib_ledger::Domain;
use lib_list_cursor::{resolve, step_id};
use lib_units::{MANUAL_ENTRY, PriceSource, PriceSourceId, SourceOrigin, UnitId, UnitRow};

use crate::{
    chrome::{dialog_host::open_dialog, statusline::PageStatus, window_size::WindowSize},
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    settings::{SettingsPage, hints::settings_units_hints},
    view::event::ViewEvent,
};
use apply::UnitsOutcome;
use dialogs::{UnitsDialog, sources::PriceSourceDialog};
use form::{DeleteUnitForm, UnitForm};
use source_apply::SourceOutcome;
use source_form::{BuiltInSourceForm, CustomSourceForm, DeleteSourceForm};

impl EventEmitter<ViewEvent> for UnitsPage {}

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
    fn of(source: &PriceSource<'_>) -> Self {
        match source.origin {
            SourceOrigin::ManualEntry => Self::ManualEntry,
            SourceOrigin::BuiltIn(_) => Self::BuiltIn,
            SourceOrigin::Custom(_) => Self::Custom,
        }
    }
}

/// A source's LAST UPDATED cell.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LastUpdated {
    /// Manual entry, which never fetches: drawn —.
    Never,
    /// A source that hasn't fetched a price yet: drawn as a muted ✗ labelled "Not fetched yet".
    NotFetched,
    /// How long ago it last fetched.
    Ago(String),
}

/// A Price Source as the page draws it, its text resolved for the Locale.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceRow {
    id: PriceSourceId,
    kind: SourceKind,
    name: String,
    /// The address it fetches from; `None` for Manual entry, which never fetches.
    address: Option<String>,
    last_updated: LastUpdated,
}

/// The ✗ a source that hasn't fetched yet shows in LAST UPDATED. A glyph, not a Message: its
/// label is.
const NOT_FETCHED_GLYPH: &str = "\u{2717}";

impl SourceRow {
    fn new(source: &PriceSource<'_>, ledger: &lib_ledger::Ledger) -> Self {
        let last_updated = match source.endpoint() {
            None => LastUpdated::Never,
            Some(_) => ledger
                .last_fetched(source.id)
                .map_or(LastUpdated::NotFetched, |fetched| {
                    LastUpdated::Ago(crate::navigation::explorer::format_modified(
                        fetched.into(),
                        std::time::SystemTime::now(),
                    ))
                }),
        };
        Self {
            id: source.id,
            kind: SourceKind::of(source),
            name: source.name.to_string(),
            address: source
                .endpoint()
                .map(|endpoint| endpoint.address.to_string()),
            last_updated,
        }
    }

    /// LAST UPDATED as text: the age, the not-fetched label, or — for Manual entry.
    fn last_updated_text(&self) -> String {
        match &self.last_updated {
            LastUpdated::Never => crate::msg::desktop_settings_units_pane_none(),
            LastUpdated::NotFetched => crate::msg::desktop_settings_units_not_fetched_yet(),
            LastUpdated::Ago(ago) => ago.clone(),
        }
    }
}

/// Emitted when a click lands on a row, so the Settings View, which owns the page's focus, moves
/// it from the index into the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitsRowClicked;

impl EventEmitter<UnitsRowClicked> for UnitsPage {}

/// The selected row: one at a time across both tables, UNITS drawn above PRICE SOURCES.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitsSelection {
    Unit(UnitId),
    Source(PriceSourceId),
}

/// The row a context menu is open on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuTarget {
    Unit(UnitId),
    Source(PriceSourceId, SourceKind),
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
            MenuTarget::Source(_, SourceKind::Custom) => {
                &[Self::TestSource, Self::EditSource, Self::DeleteSource]
            }
            MenuTarget::Source(_, SourceKind::BuiltIn) => &[Self::TestSource, Self::EditSource],
            // Manual entry has no actions, so it opens no menu.
            MenuTarget::Source(_, SourceKind::ManualEntry) => &[],
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

/// A group of the X-Large detail pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneGroup {
    Unit,
    PriceSource,
    Accounts,
    Values,
    Activity,
}

impl PaneGroup {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::PriceSource => "source",
            Self::Accounts => "accounts",
            Self::Values => "values",
            Self::Activity => "activity",
        }
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

/// The Units page. At every Window Size it keeps one selected row across both tables, which `j`/`k`
/// walk as one list and `Tab` jumps between; at X-Large the detail pane beside it also carries the
/// actions, and the page's Success outcomes land in the pane's ACTIVITY instead of raising Toasts
/// (DESKTOP-125).
pub struct UnitsPage {
    ledger: Entity<LedgerStore>,
    /// It may name a row that is gone, so read it through [`Self::selection`].
    selected: Option<UnitsSelection>,
    /// Each table's last-selected row, which `Tab` returns to. Either may be gone.
    last_unit: Option<UnitId>,
    last_source: Option<PriceSourceId>,
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
            last_unit: None,
            last_source: None,
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

    /// Every row of both tables in drawn order: the Units, then the Price Sources.
    fn rows(&self, cx: &App) -> Vec<UnitsSelection> {
        let units = self
            .units(cx)
            .iter()
            .map(|unit| UnitsSelection::Unit(unit.id));
        let sources = self
            .price_sources(cx)
            .into_iter()
            .map(|source| UnitsSelection::Source(source.id));
        units.chain(sources).collect()
    }

    /// The selected row, or the first row while nothing (or a removed row) is selected.
    fn selection(&self, cx: &App) -> Option<UnitsSelection> {
        resolve(self.rows(cx), self.selected)
    }

    /// Selects `selection`, remembering it as its table's last row for `Tab`.
    fn select(&mut self, selection: UnitsSelection) {
        match selection {
            UnitsSelection::Unit(id) => self.last_unit = Some(id),
            UnitsSelection::Source(id) => self.last_source = Some(id),
        }
        self.selected = Some(selection);
    }

    fn selected_unit<'a>(&self, cx: &'a App) -> Option<&'a UnitRow> {
        let UnitsSelection::Unit(id) = self.selection(cx)? else {
            return None;
        };
        self.units(cx).iter().find(|unit| unit.id == id)
    }

    fn selected_source(&self, cx: &App) -> Option<SourceRow> {
        let UnitsSelection::Source(id) = self.selection(cx)? else {
            return None;
        };
        self.price_sources(cx)
            .into_iter()
            .find(|source| source.id == id)
    }

    /// The Price Source that fetches `unit`'s price, or `None` while it sits on Manual entry.
    fn price_source_of(&self, unit: &UnitRow, cx: &App) -> Option<SourceRow> {
        if unit.link.source_id == MANUAL_ENTRY {
            return None;
        }
        let ledger = self.ledger.read(cx).ledger();
        let manual = lib_locale::msg::price_source_manual_entry();
        let source = ledger.price_source(&manual, unit.link.source_id)?;
        Some(SourceRow::new(&source, ledger))
    }

    /// A selected Unit's PRICE SOURCE group, or `None` while it sits on Manual entry.
    fn linked_source(&self, unit: &UnitRow, cx: &App) -> Option<pane::LinkedSource> {
        if unit.link.source_id == MANUAL_ENTRY {
            return None;
        }
        let ledger = self.ledger.read(cx).ledger();
        let manual = lib_locale::msg::price_source_manual_entry();
        let source = ledger.price_source(&manual, unit.link.source_id)?;
        let row = SourceRow::new(&source, ledger);
        Some(pane::linked_source(&source, row, unit, ledger.units()))
    }

    /// The selected Price Source's PRICE SOURCE and VALUES groups, or `None` while a Unit is
    /// selected.
    fn source_detail(&self, cx: &App) -> Option<pane::SourceDetail> {
        let UnitsSelection::Source(id) = self.selection(cx)? else {
            return None;
        };
        let ledger = self.ledger.read(cx).ledger();
        let manual = lib_locale::msg::price_source_manual_entry();
        let source = ledger.price_source(&manual, id)?;
        let row = SourceRow::new(&source, ledger);
        Some(pane::source_detail(&source, row, ledger.units()))
    }

    /// The pane's groups in drawn order: a Unit's UNIT, PRICE SOURCE and ACCOUNTS, or a Price
    /// Source's PRICE SOURCE and VALUES (DESKTOP-140), then ACTIVITY.
    fn pane_groups(&self, cx: &App) -> &'static [PaneGroup] {
        match self.selection(cx) {
            Some(UnitsSelection::Unit(_)) => &[
                PaneGroup::Unit,
                PaneGroup::PriceSource,
                PaneGroup::Accounts,
                PaneGroup::Activity,
            ],
            Some(UnitsSelection::Source(_)) => &[
                PaneGroup::PriceSource,
                PaneGroup::Values,
                PaneGroup::Activity,
            ],
            None => &[PaneGroup::Activity],
        }
    }

    /// The PRICE SOURCE group's buttons as slugs: the source's own menu entries, or `add-source`
    /// for a Unit on Manual entry.
    fn pane_source_buttons(&self, cx: &App) -> Vec<String> {
        let row = match self.selection(cx) {
            Some(UnitsSelection::Unit(_)) => {
                let Some(unit) = self.selected_unit(cx) else {
                    return Vec::new();
                };
                match self.price_source_of(unit, cx) {
                    Some(row) => row,
                    None => return vec!["add-source".to_string()],
                }
            }
            Some(UnitsSelection::Source(_)) => match self.selected_source(cx) {
                Some(row) => row,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };
        MenuAction::for_target(MenuTarget::Source(row.id, row.kind))
            .iter()
            .map(|action| action.slug().to_string())
            .collect()
    }

    /// The name of the Price Source `id`, empty when it is gone.
    fn source_name(&self, id: PriceSourceId, cx: &App) -> String {
        let manual = lib_locale::msg::price_source_manual_entry();
        self.ledger
            .read(cx)
            .ledger()
            .price_source(&manual, id)
            .map(|source| source.name.to_string())
            .unwrap_or_default()
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
        self.log_activity(text, cx);
    }

    /// Lands a confirmed price source Dialog: a saved or deleted source toasts below X-Large and
    /// goes to ACTIVITY at it, as a Unit's outcomes do.
    fn land_source(&mut self, outcome: SourceOutcome, cx: &mut Context<'_, Self>) {
        if self.docked {
            self.log_activity(outcome.activity(), cx);
        } else {
            let (kind, text) = outcome.toast();
            cx.emit(ViewEvent::RaiseToast { kind, text });
        }
    }

    fn log_activity(&mut self, text: String, cx: &mut Context<'_, Self>) {
        let time = Local::now().format("%H:%M").to_string();
        self.activity.insert(0, ActivityEntry { time, text });
        cx.notify();
    }

    fn units<'a>(&self, cx: &'a App) -> &'a [UnitRow] {
        self.ledger.read(cx).ledger().units()
    }

    /// Every Price Source in the Ledger's order, resolved for drawing.
    fn price_sources(&self, cx: &App) -> Vec<SourceRow> {
        let ledger = self.ledger.read(cx).ledger();
        let manual = lib_locale::msg::price_source_manual_entry();
        ledger
            .price_sources(&manual)
            .iter()
            .map(|source| SourceRow::new(source, ledger))
            .collect()
    }

    // -----------------------------------------------------------------------------------------
    // The interface the Settings View forwards to while the page has focus (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// `Tab` jumps to the other table, claimed ahead of the router as Bills claims it for its
    /// tabs; `Shift-Tab` is left to cycle the focus zones.
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if keystroke.key != "tab"
            || modifiers.shift
            || modifiers.control
            || modifiers.alt
            || modifiers.platform
        {
            return false;
        }
        self.close_menu(cx);
        self.jump_table(cx);
        true
    }

    /// Selects the other table's last-selected row, or its first while it has none (or that row
    /// is gone). An empty Units table leaves the selection where it is.
    fn jump_table(&mut self, cx: &mut Context<'_, Self>) {
        let target = match self.selection(cx) {
            Some(UnitsSelection::Unit(id)) => {
                self.last_unit = Some(id);
                let ids = self.price_sources(cx).into_iter().map(|source| source.id);
                resolve(ids, self.last_source).map(UnitsSelection::Source)
            }
            Some(UnitsSelection::Source(id)) => {
                self.last_source = Some(id);
                let ids = self.units(cx).iter().map(|unit| unit.id);
                resolve(ids, self.last_unit).map(UnitsSelection::Unit)
            }
            None => None,
        };
        if let Some(target) = target {
            self.select(target);
            cx.notify();
        }
    }

    /// The page's bare `n`, `e`, `d` and `t` at every Window Size, each acting on the selected
    /// row's table through the handler its button or menu entry uses. Manual entry takes no `e`,
    /// `d` or `t`, and a built-in no `d`. Returns whether the key was one of them.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        self.close_menu(cx);
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        let on_source = matches!(self.selection(cx), Some(UnitsSelection::Source(_)));
        match (keystroke.key.as_str(), on_source) {
            ("n", false) => self.click_add(cx),
            ("e", false) => self.edit_selected(cx),
            ("d", false) => self.delete_selected(cx),
            ("t", false) => self.test_selected_source(cx),
            ("n", true) => self.click_add_source(cx),
            ("e", true) => self.edit_selected_source(cx),
            ("d", true) => self.delete_selected_source(cx),
            ("t", true) => self.test_selected_row_source(cx),
            _ => return false,
        }
        true
    }

    /// `j`/`k`, `g g`, `G` and `Ctrl-d`/`Ctrl-u` walk both tables as one list, at every Window
    /// Size: `j` on the last Unit selects the first Price Source.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) -> bool {
        self.close_menu(cx);
        if let Some(step) = movement.step()
            && let Some(selection) = step_id(&self.rows(cx), self.selection(cx), step)
        {
            self.select(selection);
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

    /// The page legend, by the selected row's table, and the page's fixed note.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        let on_source = matches!(self.selection(cx), Some(UnitsSelection::Source(_)));
        Some(PageStatus {
            hints: settings_units_hints(on_source),
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
            self.select(UnitsSelection::Unit(unit.id));
        }
        cx.emit(UnitsRowClicked);
        cx.notify();
    }

    /// A click on Price Sources row `index` selects it and hands the page focus.
    fn click_select_source(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.menu = None;
        if let Some(source) = self.price_sources(cx).get(index) {
            self.select(UnitsSelection::Source(source.id));
        }
        cx.emit(UnitsRowClicked);
        cx.notify();
    }

    /// A double-click on a Units row opens its Edit dialog, as the keyboard's `e` does.
    fn double_click_unit(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.click_select(index, cx);
        self.click_edit(index, cx);
    }

    /// A double-click on a Price Sources row selects it and opens its Edit dialog; Manual entry
    /// has none.
    fn double_click_source(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.click_select_source(index, cx);
        if let Some(source) = self.price_sources(cx).get(index) {
            self.edit_source(source.id, cx);
        }
    }

    /// A right-click on a row opens its menu at the pointer and selects the row. A row with no
    /// actions (Manual entry) opens none.
    fn open_menu(
        &mut self,
        target: MenuTarget,
        position: Point<Pixels>,
        cx: &mut Context<'_, Self>,
    ) {
        if MenuAction::for_target(target).is_empty() {
            self.close_menu(cx);
            return;
        }
        self.select(match target {
            MenuTarget::Unit(id) => UnitsSelection::Unit(id),
            MenuTarget::Source(id, _) => UnitsSelection::Source(id),
        });
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
            MenuTarget::Source(..) => None,
        };
        match (action, unit_index, target) {
            (MenuAction::EditUnit, Some(index), _) => self.click_edit(index, cx),
            (MenuAction::DeleteUnit, Some(index), _) => self.click_delete(index, cx),
            (MenuAction::TestSource, ..) => self.not_built(
                crate::msg::desktop_status_test_price_source_not_yet_built(),
                cx,
            ),
            (MenuAction::EditSource, _, MenuTarget::Source(id, _)) => self.edit_source(id, cx),
            (MenuAction::DeleteSource, _, MenuTarget::Source(id, _)) => self.delete_source(id, cx),
            (MenuAction::EditUnit | MenuAction::DeleteUnit, None, _)
            | (MenuAction::EditSource | MenuAction::DeleteSource, _, MenuTarget::Unit(_)) => {}
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

    fn edit_selected_source(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(source) = self.selected_source(cx) {
            self.edit_source(source.id, cx);
        }
    }

    fn delete_selected_source(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(source) = self.selected_source(cx) {
            self.delete_source(source.id, cx);
        }
    }

    /// `t` on a Price Sources row: testing has no backing yet, and Manual entry never fetches.
    fn test_selected_row_source(&mut self, cx: &mut Context<'_, Self>) {
        if self
            .selected_source(cx)
            .is_some_and(|source| source.kind != SourceKind::ManualEntry)
        {
            self.not_built(
                crate::msg::desktop_status_test_price_source_not_yet_built(),
                cx,
            );
        }
    }

    /// `t` on a Units row tests its linked source: testing has no backing yet, so it says so on
    /// the status line.
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

    /// The selected Unit's row index; `None` while a Price Source is selected.
    fn selected_index(&self, cx: &App) -> Option<usize> {
        let id = self.selected_unit(cx)?.id;
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

    /// The square **+** beside PRICE SOURCES opens Add price source for a Custom source: the
    /// built-ins are always listed, so there is none to add.
    fn click_add_source(&mut self, cx: &mut Context<'_, Self>) {
        open_dialog(
            PriceSourceDialog::Add(CustomSourceForm::default()),
            Self::land_source,
            cx,
        );
    }

    /// Edit price source: a built-in's name and Updates (16z·edit), or a Custom source's every
    /// field (16z·custom). Manual entry has nothing to edit.
    fn edit_source(&mut self, id: PriceSourceId, cx: &mut Context<'_, Self>) {
        let manual = lib_locale::msg::price_source_manual_entry();
        let Some(source) = self.ledger.read(cx).ledger().price_source(&manual, id) else {
            return;
        };
        let dialog = match source.origin {
            SourceOrigin::ManualEntry => return,
            SourceOrigin::BuiltIn(_) => PriceSourceDialog::EditBuiltIn(
                id,
                BuiltInSourceForm::new(source.name, source.updates),
            ),
            SourceOrigin::Custom(custom) => {
                PriceSourceDialog::EditCustom(id, CustomSourceForm::from_source(custom))
            }
        };
        open_dialog(dialog, Self::land_source, cx);
    }

    /// Delete price source (16z·del), for a Custom source only.
    fn delete_source(&mut self, id: PriceSourceId, cx: &mut Context<'_, Self>) {
        let manual = lib_locale::msg::price_source_manual_entry();
        let Some(source) = self.ledger.read(cx).ledger().price_source(&manual, id) else {
            return;
        };
        if !matches!(source.origin, SourceOrigin::Custom(_)) {
            return;
        }
        let form = DeleteSourceForm::new(source.name);
        open_dialog(PriceSourceDialog::Delete(id, form), Self::land_source, cx);
    }

    /// Testing a price source has no backing yet (fetching is undecided), so it says so on the
    /// status line.
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
            source_rows: self
                .price_sources(cx)
                .iter()
                .map(|source| {
                    let last_updated = match source.last_updated {
                        LastUpdated::NotFetched => {
                            format!("{NOT_FETCHED_GLYPH} {}", source.last_updated_text())
                        }
                        _ => source.last_updated_text(),
                    };
                    (
                        source.name.clone(),
                        source
                            .address
                            .clone()
                            .unwrap_or_else(crate::msg::desktop_settings_units_pane_none),
                        last_updated,
                    )
                })
                .collect(),
            selected_code: self.selected_unit(cx).map(|unit| unit.code.clone()),
            selected_source: self.selected_source(cx).map(|source| source.name),
            menu: self.menu.map(|menu| {
                let row = match menu.target {
                    MenuTarget::Unit(id) => units
                        .iter()
                        .find(|unit| unit.id == id)
                        .map(|unit| format!("unit:{}", unit.code)),
                    MenuTarget::Source(id, _) => self
                        .price_sources(cx)
                        .iter()
                        .find(|source| source.id == id)
                        .map(|source| format!("source:{}", source.name)),
                };
                (
                    row.unwrap_or_default(),
                    MenuAction::for_target(menu.target)
                        .iter()
                        .map(|action| action.slug().to_string())
                        .collect(),
                )
            }),
            pane_groups: if self.docked {
                self.pane_groups(cx)
                    .iter()
                    .map(|group| group.slug().to_string())
                    .collect()
            } else {
                Vec::new()
            },
            pane_source_buttons: if self.docked {
                self.pane_source_buttons(cx)
            } else {
                Vec::new()
            },
            source_facts: self
                .selected_unit(cx)
                .and_then(|unit| self.linked_source(unit, cx))
                .map(|linked| linked.facts)
                .or_else(|| self.source_detail(cx).map(|detail| detail.facts))
                .unwrap_or_default()
                .into_iter()
                .map(|fact| (fact.label, fact.value))
                .collect(),
            source_meta: self.source_detail(cx).and_then(|detail| detail.meta),
            source_note: self.source_detail(cx).and_then(|detail| detail.note),
            source_values: self
                .source_detail(cx)
                .map(|detail| {
                    detail
                        .values
                        .iter()
                        .map(|value| {
                            (
                                format!("{} \u{b7} {}", value.code, value.name),
                                value.detail(),
                                value.price.clone(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            account_values: self
                .selected_unit(cx)
                .map(|unit| {
                    let units = self.units(cx);
                    self.accounts_in(unit, cx)
                        .iter()
                        .filter_map(|account| pane::base_value(&account.balance, unit, units))
                        .collect()
                })
                .unwrap_or_default(),
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
    /// The Price Sources rows in table order: `(name, source, last updated)`, as drawn.
    pub source_rows: Vec<(String, String, String)>,
    /// The selected Unit's code, which is the first row's until another is chosen; `None` while
    /// a Price Source is selected.
    pub selected_code: Option<String>,
    /// The selected Price Source's name; `None` while a Unit is selected.
    pub selected_source: Option<String>,
    /// The open row menu: the row it is on (`unit:btc`, `source:Bitcoin`) and its entries' slugs
    /// in order.
    pub menu: Option<(String, Vec<String>)>,
    /// The docked pane's groups in drawn order (`unit`, `source`, `accounts`, `values`,
    /// `activity`); empty below X-Large.
    pub pane_groups: Vec<String>,
    /// The docked pane's PRICE SOURCE buttons (`test-source`, `edit-source`, `delete-source`, or
    /// `add-source`); empty below X-Large.
    pub pane_source_buttons: Vec<String>,
    /// The pane's PRICE SOURCE fact rows for the selection as `(label, value)`: the selected
    /// Unit's linked source (DESKTOP-125) or the selected Price Source (DESKTOP-140). Empty for a
    /// Unit or source on Manual entry.
    pub source_facts: Vec<(String, String)>,
    /// The selected Price Source's meta line; `None` for Manual entry or a selected Unit.
    pub source_meta: Option<String>,
    /// The note under the selected Price Source's fact rows.
    pub source_note: Option<String>,
    /// The selected Price Source's VALUES rows: `(code · name, symbol and quote, last price)`.
    pub source_values: Vec<(String, String, String)>,
    /// The selected Unit's accounts' values in the base Unit, for those it can price.
    pub account_values: Vec<String>,
    /// ACTIVITY's lines, newest first.
    pub activity: Vec<String>,
}
