//! The Settings destination's Entity (ADR-0032). [`SettingsView`] owns the View state that
//! belongs to Settings alone: the page on show, where keyboard focus sits, the Display page's
//! focused control, and the Documents and Inventory pages' selections. Preferences that Chrome
//! and other Views read stay on `Shell` (ADR-0032's "Shell keeps what is cross-cutting").

use std::collections::HashSet;

use gpui::{Context, EventEmitter, ListOffset, ListState, px};
use lib_institutions::{InstitutionRow, InstitutionService};
use lib_units::{PriceSourceRow, UnitRow, UnitService};

use crate::{
    settings::{
        SettingsFocus, SettingsSection,
        tracing_log::{LogChange, LogView},
    },
    view::settings::inventory::InventoryRow,
};

/// The Settings View's own state. The page is the last one visited, so it survives leaving and
/// re-entering Settings, as `g s` relies on. The Tracing page's log mirror and its list live here
/// too: the capture feed in `Shell` pushes into them, and only this page reads them.
pub struct SettingsView {
    selected_section: SettingsSection,
    focus: SettingsFocus,
    display_field: Option<usize>,
    documents_selected: Option<u32>,
    inventory_selected: Option<InventoryRow>,
    inventory_expanded: HashSet<u32>,
    /// The Colour Theme card the keyboard is on, while the Display page's grid has focus.
    colour_theme_focus: Option<usize>,
    log: LogView,
    log_list: ListState,
}

impl SettingsView {
    /// `log` mirrors the live capture; `log_list` is the virtualised list that shows it, and must
    /// be kept in step with `log` by [`Self::pull_log`] and [`Self::replace_log`].
    pub fn new(log: LogView, log_list: ListState) -> Self {
        Self {
            selected_section: SettingsSection::default(),
            focus: SettingsFocus::default(),
            display_field: None,
            documents_selected: None,
            inventory_selected: None,
            inventory_expanded: HashSet::new(),
            colour_theme_focus: None,
            log,
            log_list,
        }
    }

    pub fn colour_theme_focus(&self) -> Option<usize> {
        self.colour_theme_focus
    }

    pub fn set_colour_theme_focus(&mut self, cx: &mut Context<'_, Self>, focus: Option<usize>) {
        self.colour_theme_focus = focus;
        cx.notify();
    }

    pub fn log(&self) -> &LogView {
        &self.log
    }

    pub fn log_list(&self) -> &ListState {
        &self.log_list
    }

    /// Hands the page a new capture, opening on `log`'s level, and restarts the list at its top.
    pub fn replace_log(&mut self, cx: &mut Context<'_, Self>, log: LogView) {
        self.log = log;
        self.log_list.reset(self.log.visible().len());
        cx.notify();
    }

    /// Pulls new entries from the capture and mirrors them onto the list. Notifies only when
    /// something changed, so an idle capture doesn't redraw the page.
    pub fn pull_log(&mut self, cx: &mut Context<'_, Self>) -> LogChange {
        let change = self.log.pull();
        if change != LogChange::default() {
            self.mirror_log_change(change);
            cx.notify();
        }
        change
    }

    /// Mirrors a pull onto the list: evicted rows leave the bottom, new ones arrive at the top.
    /// A reader at the very top keeps seeing the newest; one scrolled down stays where they are.
    fn mirror_log_change(&mut self, change: LogChange) {
        let list = &self.log_list;
        let top = list.logical_scroll_top();
        let at_top = top.item_ix == 0 && top.offset_in_item <= px(0.0);
        let count = list.item_count();
        let evicted = change.evicted.min(count);
        list.splice(count - evicted..count, 0);
        list.splice(0..0, change.added);
        if at_top {
            list.scroll_to(ListOffset::default());
        }
    }

    /// Edits the log mirror in place (its level and clear), then redraws. The list is reset to
    /// the mirror's visible rows, as a level or clear changes which rows show.
    pub fn update_log(&mut self, cx: &mut Context<'_, Self>, edit: impl FnOnce(&mut LogView)) {
        edit(&mut self.log);
        self.log_list.reset(self.log.visible().len());
        cx.notify();
    }

    pub fn selected_section(&self) -> SettingsSection {
        self.selected_section
    }

    pub fn focus(&self) -> SettingsFocus {
        self.focus
    }

    pub fn display_field(&self) -> Option<usize> {
        self.display_field
    }

    pub fn documents_selected(&self) -> Option<u32> {
        self.documents_selected
    }

    pub fn inventory_selected(&self) -> Option<InventoryRow> {
        self.inventory_selected
    }

    /// The Properties shown open on the Inventory page.
    pub fn inventory_expanded(&self) -> &HashSet<u32> {
        &self.inventory_expanded
    }

    pub fn set_selected_section(&mut self, cx: &mut Context<'_, Self>, section: SettingsSection) {
        self.selected_section = section;
        cx.notify();
    }

    pub fn set_focus(&mut self, cx: &mut Context<'_, Self>, focus: SettingsFocus) {
        self.focus = focus;
        cx.notify();
    }

    pub fn set_display_field(&mut self, cx: &mut Context<'_, Self>, field: Option<usize>) {
        self.display_field = field;
        cx.notify();
    }

    pub fn set_documents_selected(&mut self, cx: &mut Context<'_, Self>, id: Option<u32>) {
        self.documents_selected = id;
        cx.notify();
    }

    pub fn set_inventory_selected(
        &mut self,
        cx: &mut Context<'_, Self>,
        row: Option<InventoryRow>,
    ) {
        self.inventory_selected = row;
        cx.notify();
    }

    /// Shows Property `id` open. Returns whether it was closed before, so the caller can tell a
    /// real change from a no-op.
    pub fn expand(&mut self, cx: &mut Context<'_, Self>, id: u32) -> bool {
        let inserted = self.inventory_expanded.insert(id);
        cx.notify();
        inserted
    }

    /// Shows Property `id` closed. Returns whether it was open before.
    pub fn collapse(&mut self, cx: &mut Context<'_, Self>, id: u32) -> bool {
        let removed = self.inventory_expanded.remove(&id);
        cx.notify();
        removed
    }

    /// Edits the expanded Properties in place, for the fold and disclosure rules to run on.
    pub fn update_inventory_expanded(
        &mut self,
        cx: &mut Context<'_, Self>,
        edit: impl FnOnce(&mut HashSet<u32>),
    ) {
        edit(&mut self.inventory_expanded);
        cx.notify();
    }
}

/// Emitted by the Units and Institutions stores after a write lands, so `Shell` can refresh the
/// Views that read the rows (ADR-0032's View events).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerDataEvent {
    Changed,
}

/// The shared Units table and Price Sources. Settings edits them, and Accounts and the
/// Institutions dialog read them, so a change is seen everywhere on the next render.
pub struct UnitsStore {
    service: UnitService,
}

impl EventEmitter<LedgerDataEvent> for UnitsStore {}

impl UnitsStore {
    /// A store over the seeded Units and Price Sources, which the Desktop takes from
    /// `lib_units::default_units` and `lib_units::default_price_sources`.
    pub fn new(units: Vec<UnitRow>, price_sources: Vec<PriceSourceRow>) -> Self {
        Self {
            service: UnitService::new(units, price_sources),
        }
    }

    pub fn units(&self) -> &[UnitRow] {
        self.service.units()
    }

    pub fn unit(&self, index: usize) -> Option<&UnitRow> {
        self.service.unit(index)
    }

    pub fn price_sources(&self) -> &[PriceSourceRow] {
        self.service.price_sources()
    }

    pub fn add_unit(
        &mut self,
        cx: &mut Context<'_, Self>,
        code: String,
        name: String,
        kind: String,
    ) {
        self.service.add_unit(code, name, kind);
        cx.emit(LedgerDataEvent::Changed);
    }

    /// Returns `false` when `index` is out of bounds, and emits nothing.
    pub fn edit_unit(
        &mut self,
        cx: &mut Context<'_, Self>,
        index: usize,
        code: String,
        name: String,
        kind: String,
    ) -> bool {
        let edited = self.service.edit_unit(index, code, name, kind);
        if edited {
            cx.emit(LedgerDataEvent::Changed);
        }
        edited
    }

    pub fn remove_unit(
        &mut self,
        cx: &mut Context<'_, Self>,
        index: usize,
        code: &str,
    ) -> Option<UnitRow> {
        let removed = self.service.remove_unit(index, code);
        if removed.is_some() {
            cx.emit(LedgerDataEvent::Changed);
        }
        removed
    }
}

/// The shared Institutions table. Settings edits it, and the Accounts form reads it.
pub struct InstitutionsStore {
    service: InstitutionService,
}

impl EventEmitter<LedgerDataEvent> for InstitutionsStore {}

impl InstitutionsStore {
    pub fn new(institutions: Vec<InstitutionRow>) -> Self {
        Self {
            service: InstitutionService::new(institutions),
        }
    }

    pub fn institutions(&self) -> &[InstitutionRow] {
        self.service.institutions()
    }

    pub fn add(&mut self, cx: &mut Context<'_, Self>, name: String, account_type: String) {
        self.service.add(name, account_type);
        cx.emit(LedgerDataEvent::Changed);
    }
}
