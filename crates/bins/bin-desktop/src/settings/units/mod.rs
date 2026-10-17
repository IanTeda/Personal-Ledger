//! The **Units** Settings page (ADR-0037, and its amendment's one folder per page): the Units
//! table and its Price Sources. [`UnitsPage`] reads both straight from [`LedgerStore`], renders
//! itself (`render.rs`), builds the Add, Edit and Delete unit Dialogs (`dialogs/`) and reaches
//! `Shell` only through [`ViewEvent`]; `Shell`'s Dialog host draws them and confirms them through
//! `apply.rs`. The Settings View forwards the page's keys and status to it while the page has
//! focus. At X-Large the page also docks a detail pane (`render::detail_pane`) and keeps a selected row.

pub mod apply;
pub mod dialogs;
pub mod form;
mod render;

pub(super) use render::detail_pane;

use chrono::Local;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_ledger::Domain;
use lib_list_cursor::{resolve, step_id};
use lib_units::{PriceSourceRow, UnitId, UnitRow};

use crate::{
    chrome::{dialog_host::OpenDialog, statusline::PageStatus, window_size::WindowSize},
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

/// One line of the detail pane's ACTIVITY: a Units outcome, with the time it landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEntry {
    pub time: String,
    pub text: String,
}

/// The Units page. Below X-Large every row is reached by its buttons; at X-Large the page keeps a
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

    /// Records a landed Dialog's outcome in ACTIVITY, newest first.
    pub fn land(&mut self, outcome: &UnitsOutcome, cx: &mut Context<'_, Self>) {
        let text = match outcome {
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

    /// The page has nothing of its own to close: `Esc` is left to Settings, which steps back to
    /// the index.
    pub fn escape(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
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
        if let Some(unit) = self.units(cx).get(index) {
            self.selected = Some(unit.id);
        }
        cx.emit(UnitsRowClicked);
        cx.notify();
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

    /// **+ Add unit** opens the Add unit dialog.
    fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        open_dialog(UnitsDialog::Add(UnitForm::default()), cx);
    }

    /// A Units row's **edit** opens the Edit unit dialog filled from the row.
    fn click_edit(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.units(cx).get(index) else {
            return;
        };
        let form = UnitForm::from_row(row);
        open_dialog(UnitsDialog::Edit(row.id, form), cx);
    }

    /// A Units row's **delete** opens the destructive Delete unit confirm dialog.
    fn click_delete(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.units(cx).get(index) else {
            return;
        };
        let form = DeleteUnitForm::new(row.code.as_str());
        open_dialog(UnitsDialog::Delete(row.id, form), cx);
    }

    /// Price Sources' test, edit, delete and add have no Dialog yet, so they say so on the status
    /// line.
    fn not_built(&mut self, message: String, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(message)));
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// The Units rows. The open Dialog's fields are `Shell`'s, which fills them in.
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
            unit_dialog_name: None,
            unit_dialog_kind: None,
            unit_dialog_field: None,
            selected_code: self.selected_unit(cx).map(|unit| unit.code.clone()),
            activity: self
                .activity
                .iter()
                .map(|entry| entry.text.clone())
                .collect(),
        }
    }
}

/// The Units page: codes in table order, `(code, name, kind)` rows, and the Add/Edit unit dialog's
/// Name draft, Type (`UnitKind` variant name) and focused field (`Code` or `Name`).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsUnitsSnapshot {
    pub unit_codes: Vec<String>,
    pub unit_rows: Vec<(String, String, String)>,
    pub unit_dialog_name: Option<String>,
    pub unit_dialog_kind: Option<String>,
    pub unit_dialog_field: Option<String>,
    /// The selected row's code, which is the first row's until another is chosen.
    pub selected_code: Option<String>,
    /// ACTIVITY's lines, newest first.
    pub activity: Vec<String>,
}

/// What the open Units Dialog shows, for the Settings snapshots.
#[doc(hidden)]
pub struct UnitsDialogSnapshot {
    /// `AddUnit`, `EditUnit` or `DeleteUnit`.
    pub label: &'static str,
    /// The Code draft, on Add and Edit.
    pub code: Option<String>,
    /// The typed-back confirmation, on Delete.
    pub confirm: Option<String>,
    /// The Add/Edit form's Name draft, Type and focused field.
    pub form: Option<(String, String, String)>,
}

impl UnitsDialog {
    #[doc(hidden)]
    pub fn snapshot(&self) -> UnitsDialogSnapshot {
        let (label, form) = match self {
            Self::Add(form) => ("AddUnit", form),
            Self::Edit(_, form) => ("EditUnit", form),
            Self::Delete(_, form) => {
                return UnitsDialogSnapshot {
                    label: "DeleteUnit",
                    code: None,
                    confirm: Some(form.confirm_input.text().to_string()),
                    form: None,
                };
            }
        };
        UnitsDialogSnapshot {
            label,
            code: Some(form.code.text().to_string()),
            confirm: None,
            form: Some((
                form.name.text().to_string(),
                format!("{:?}", form.kind),
                format!("{:?}", form.focused_field),
            )),
        }
    }
}

/// Asks `Shell`'s Dialog host to open a Units Dialog.
fn open_dialog(dialog: UnitsDialog, cx: &mut Context<'_, UnitsPage>) {
    cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Units(dialog))));
}
