//! The Budgets page's default keys, built on `lib_viper`'s [`Keymap`]. The commands every module
//! shares keep their flat names from `lib_viper::keymap`; those only Budgets has are namespaced here
//! (`budgets.fill`). Each is today's key, checked against both Clients' global keys by their clash
//! tests.
//!
//! The tabs share some keys, each to an event of its own: `[`/`]` step the Progress month or the
//! Plan and History range, `h`/`l` the Plan cell or the History month, and `x` clears a Plan cell
//! but exports History. Such a command is bound once per tab under one name, so one override moves
//! it on every tab, and the page takes the event for the tab shown ([`Keymap::events`]). The
//! Budget-wide verbs the palette lists (`budgets duplicate`, `budgets archive`, …) have no key until
//! `[keybindings]` gives them one.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::msg;
use crate::presenter::{BudgetsPageEvent, BudgetsTab, HistoryEvent, PlanEvent, ProgressEvent};

pub const NEXT_TAB: &str = "budgets.next_tab";
pub const SHOW_PROGRESS: &str = "budgets.show_progress";
pub const SHOW_PLAN: &str = "budgets.show_plan";
pub const SHOW_HISTORY: &str = "budgets.show_history";
pub const SWITCH: &str = "budgets.switch";
pub const EDIT_BUDGET: &str = "budgets.edit_budget";
pub const MANAGE: &str = "budgets.manage";
pub const DUPLICATE: &str = "budgets.duplicate";
pub const SET_DEFAULT: &str = "budgets.set_default";
pub const ARCHIVE: &str = "budgets.archive";
pub const RESTORE: &str = "budgets.restore";
pub const PREVIOUS_PERIOD: &str = "budgets.previous_period";
pub const NEXT_PERIOD: &str = "budgets.next_period";
pub const STOP: &str = "budgets.stop";
pub const BUDGET_CATEGORY: &str = "budgets.budget_category";
pub const COLUMN_LEFT: &str = "budgets.column_left";
pub const COLUMN_RIGHT: &str = "budgets.column_right";
pub const ARROW_LEFT: &str = "budgets.arrow_left";
pub const ARROW_RIGHT: &str = "budgets.arrow_right";
pub const EDIT_CELL: &str = "budgets.edit_cell";
pub const ROLLOVER: &str = "budgets.rollover";
pub const FILL: &str = "budgets.fill";
pub const CLEAR: &str = "budgets.clear";
pub const CLEAR_BACKSPACE: &str = "budgets.clear_backspace";
pub const ZERO: &str = "budgets.zero";
pub const EXPORT: &str = "budgets.export";

use BudgetsPageEvent::{History, Plan, Progress};

/// The Budgets page's commands, with today's keys. The moves and `select` act on the tab shown;
/// the arrow, Home/End and Page keys move as `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each
/// under its own name so an override can move it.
pub const BUDGETS_PAGE_BINDINGS: &[Binding<BudgetsPageEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", BudgetsPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", BudgetsPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", BudgetsPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", BudgetsPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        BudgetsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        BudgetsPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", BudgetsPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", BudgetsPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", BudgetsPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", BudgetsPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        BudgetsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(PAGE_UP, "pageup", BudgetsPageEvent::Move(Step::HalfPageUp)),
    Binding::hidden(SELECT, "enter", BudgetsPageEvent::Open),
    Binding::listed(
        NEXT_TAB,
        "tab",
        BudgetsPageEvent::NextTab,
        "budgets next-tab",
        msg::budgets_command_next_tab,
    ),
    Binding::listed(
        SHOW_PROGRESS,
        "1",
        BudgetsPageEvent::ShowTab(BudgetsTab::Progress),
        "budgets progress",
        msg::budgets_command_progress,
    ),
    Binding::listed(
        SHOW_PLAN,
        "2",
        BudgetsPageEvent::ShowTab(BudgetsTab::Plan),
        "budgets plan",
        msg::budgets_command_plan,
    ),
    Binding::listed(
        SHOW_HISTORY,
        "3",
        BudgetsPageEvent::ShowTab(BudgetsTab::History),
        "budgets history",
        msg::budgets_command_history,
    ),
    // Bare `b` is the Shell's rail toggle, so the Switcher takes the shifted key (#400).
    Binding::listed(
        SWITCH,
        "B",
        BudgetsPageEvent::Switch,
        "budgets switch",
        msg::budgets_command_switch,
    ),
    Binding::listed(
        NEW,
        "n",
        BudgetsPageEvent::New,
        "budgets new",
        msg::budgets_command_new,
    ),
    Binding::listed(
        EDIT_BUDGET,
        "",
        BudgetsPageEvent::EditBudget,
        "budgets edit",
        msg::budgets_command_edit,
    ),
    Binding::listed(
        MANAGE,
        "",
        BudgetsPageEvent::Manage,
        "budgets manage",
        msg::budgets_command_manage,
    ),
    Binding::listed(
        DUPLICATE,
        "",
        BudgetsPageEvent::Duplicate,
        "budgets duplicate",
        msg::budgets_command_duplicate,
    ),
    Binding::listed(
        SET_DEFAULT,
        "",
        BudgetsPageEvent::SetDefault,
        "budgets set-default",
        msg::budgets_command_set_default,
    ),
    Binding::listed(
        ARCHIVE,
        "",
        BudgetsPageEvent::Archive,
        "budgets archive",
        msg::budgets_command_archive,
    ),
    Binding::listed(
        RESTORE,
        "",
        BudgetsPageEvent::Restore,
        "budgets restore",
        msg::budgets_command_restore,
    ),
    // Progress.
    Binding::listed(
        PREVIOUS_PERIOD,
        "[",
        Progress(ProgressEvent::PreviousPeriod),
        "budgets previous-period",
        msg::budgets_command_previous_period,
    ),
    Binding::listed(
        NEXT_PERIOD,
        "]",
        Progress(ProgressEvent::NextPeriod),
        "budgets next-period",
        msg::budgets_command_next_period,
    ),
    Binding::hidden(EDIT, "e", Progress(ProgressEvent::EditAmount)),
    Binding::hidden(STOP, "s", Progress(ProgressEvent::Stop)),
    Binding::listed(
        BUDGET_CATEGORY,
        "c",
        Progress(ProgressEvent::BudgetCategory),
        "budgets budget-category",
        msg::budgets_command_budget_category,
    ),
    // Plan.
    Binding::hidden(PREVIOUS_PERIOD, "[", Plan(PlanEvent::PreviousRange)),
    Binding::hidden(NEXT_PERIOD, "]", Plan(PlanEvent::NextRange)),
    Binding::hidden(BUDGET_CATEGORY, "c", Plan(PlanEvent::BudgetCategory)),
    Binding::hidden(COLUMN_LEFT, "h", Plan(PlanEvent::ColumnLeft)),
    Binding::hidden(COLUMN_RIGHT, "l", Plan(PlanEvent::ColumnRight)),
    Binding::hidden(ARROW_LEFT, "left", Plan(PlanEvent::ColumnLeft)),
    Binding::hidden(ARROW_RIGHT, "right", Plan(PlanEvent::ColumnRight)),
    Binding::hidden(EDIT_CELL, "i", Plan(PlanEvent::StartEdit)),
    Binding::hidden(ROLLOVER, "r", Plan(PlanEvent::CycleRollover)),
    Binding::listed(
        FILL,
        "f",
        Plan(PlanEvent::Fill),
        "budgets fill",
        msg::budgets_command_fill,
    ),
    Binding::hidden(CLEAR, "x", Plan(PlanEvent::Clear)),
    Binding::hidden(CLEAR_BACKSPACE, "backspace", Plan(PlanEvent::Clear)),
    Binding::hidden(ZERO, "0", Plan(PlanEvent::Zero)),
    // History.
    Binding::hidden(PREVIOUS_PERIOD, "[", History(HistoryEvent::PreviousRange)),
    Binding::hidden(NEXT_PERIOD, "]", History(HistoryEvent::NextRange)),
    Binding::hidden(COLUMN_LEFT, "h", History(HistoryEvent::MonthLeft)),
    Binding::hidden(COLUMN_RIGHT, "l", History(HistoryEvent::MonthRight)),
    Binding::hidden(ARROW_LEFT, "left", History(HistoryEvent::MonthLeft)),
    Binding::hidden(ARROW_RIGHT, "right", History(HistoryEvent::MonthRight)),
    Binding::listed(
        EXPORT,
        "x",
        History(HistoryEvent::Export),
        "budgets export",
        msg::budgets_command_export,
    ),
];

/// The Budgets page's keymap under `config`'s overrides.
pub fn budgets_page_keymap(config: &KeyBindingConfig) -> Keymap<BudgetsPageEvent> {
    Keymap::new(BUDGETS_PAGE_BINDINGS, config)
}

/// The keymap on `tab`: a chord reaches the tab's own bindings and those every tab takes, so a key
/// another tab binds goes on to the Shell, while the palette finds every command by name, `tab`'s
/// own binding of it first and else its first, whose tab the page shows first.
pub fn tab_keymap(keymap: &Keymap<BudgetsPageEvent>, tab: BudgetsTab) -> Keymap<BudgetsPageEvent> {
    keymap.scoped(|event| event.tab().is_none_or(|own| own == tab))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overriding(command: &str, chord: &str) -> KeyBindingConfig {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(command.to_string(), chord.to_string());
        config
    }

    fn defaults() -> Keymap<BudgetsPageEvent> {
        budgets_page_keymap(&KeyBindingConfig::default())
    }

    #[test]
    fn a_shared_chord_reaches_the_event_of_the_tab_shown() {
        let keymap = defaults();

        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::Plan).event("x"),
            Some(Plan(PlanEvent::Clear))
        );
        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::History).event("x"),
            Some(History(HistoryEvent::Export))
        );
        assert_eq!(tab_keymap(&keymap, BudgetsTab::Progress).event("x"), None);
        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::Progress).event("["),
            Some(Progress(ProgressEvent::PreviousPeriod))
        );
        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::History).event("j"),
            Some(BudgetsPageEvent::Move(Step::Next))
        );
        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::History).event("c"),
            None,
            "History budgets no Category"
        );
    }

    #[test]
    fn the_palette_runs_a_shared_command_on_the_tab_shown_or_its_first_tab() {
        let keymap = defaults();

        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::History).event_for(NEXT_PERIOD),
            Some(History(HistoryEvent::NextRange))
        );
        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::Progress).event_for(FILL),
            Some(Plan(PlanEvent::Fill))
        );
        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::Plan).event_for(ARCHIVE),
            Some(BudgetsPageEvent::Archive)
        );
        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::Plan).event_for("budgets.nothing"),
            None
        );
    }

    #[test]
    fn an_override_moves_a_command_on_every_tab_that_binds_it() {
        let keymap = budgets_page_keymap(&overriding(NEXT_PERIOD, "ctrl+n"));

        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::Plan).event("ctrl+n"),
            Some(Plan(PlanEvent::NextRange))
        );
        assert_eq!(
            tab_keymap(&keymap, BudgetsTab::Progress).event("ctrl+n"),
            Some(Progress(ProgressEvent::NextPeriod))
        );
        assert_eq!(tab_keymap(&keymap, BudgetsTab::Plan).event("]"), None);
        let bound = budgets_page_keymap(&overriding(ARCHIVE, "A"));
        assert_eq!(
            tab_keymap(&bound, BudgetsTab::Progress).event("A"),
            Some(BudgetsPageEvent::Archive)
        );
    }

    #[test]
    fn the_budget_wide_verbs_have_no_key_until_one_is_bound() {
        let keymap = defaults();
        for command in [
            EDIT_BUDGET,
            MANAGE,
            DUPLICATE,
            SET_DEFAULT,
            ARCHIVE,
            RESTORE,
        ] {
            assert_eq!(keymap.chord(command), Some(""), "{command}");
        }
        assert_eq!(keymap.event(""), None);
    }
}
