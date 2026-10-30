//! The Bills page's default keys, built on `lib_viper`'s [`Keymap`]. The commands every module
//! shares keep their flat names from `lib_viper::keymap`; those only Bills has are namespaced here
//! (`bills.pay`). Each is today's key, checked against both Clients' global keys by their clash
//! tests.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::msg;

use crate::BillStatus;
use crate::presenter::{BillsPageEvent, PlannerEvent, ScheduleEvent};

pub const NEXT_TAB: &str = "bills.next_tab";
pub const PAY: &str = "bills.pay";
pub const SKIP: &str = "bills.skip";
pub const PREVIOUS_PERIOD: &str = "bills.previous_period";
pub const NEXT_PERIOD: &str = "bills.next_period";
pub const ALL: &str = "bills.all";
pub const FILTERS: &str = "bills.filters";
pub const CHIP_PAID: &str = "bills.chip_paid";
pub const CHIP_SKIPPED: &str = "bills.chip_skipped";
pub const CHIP_OVERDUE: &str = "bills.chip_overdue";
pub const CHIP_DUE: &str = "bills.chip_due";
pub const CHIP_UPCOMING: &str = "bills.chip_upcoming";

/// A Schedule command, which the palette lists as `name`.
const fn schedule(
    command: &'static str,
    chord: &'static str,
    event: ScheduleEvent,
    name: &'static str,
    description: fn() -> String,
) -> Binding<BillsPageEvent> {
    Binding::listed(
        command,
        chord,
        BillsPageEvent::Schedule(event),
        name,
        description,
    )
}

/// The Bills page's commands, with today's keys. A Schedule command does nothing on the Planner,
/// and `edit` nothing on the Schedule. The arrow, Home/End and Page keys move as `j`/`k`, `g g`/`G`
/// and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it; `select` opens the
/// row.
pub const BILLS_PAGE_BINDINGS: &[Binding<BillsPageEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", BillsPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", BillsPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", BillsPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", BillsPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        BillsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        BillsPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", BillsPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", BillsPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", BillsPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", BillsPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        BillsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(PAGE_UP, "pageup", BillsPageEvent::Move(Step::HalfPageUp)),
    Binding::hidden(SELECT, "enter", BillsPageEvent::Open),
    Binding::listed(
        NEW,
        "n",
        BillsPageEvent::New,
        "bills new",
        msg::bills_command_new,
    ),
    Binding::listed(
        EDIT,
        "e",
        BillsPageEvent::Planner(PlannerEvent::Edit),
        "bills edit",
        msg::bills_command_edit,
    ),
    Binding::listed(
        NEXT_TAB,
        "tab",
        BillsPageEvent::NextTab,
        "bills next-tab",
        msg::bills_command_next_tab,
    ),
    schedule(
        PAY,
        "p",
        ScheduleEvent::Pay,
        "bills pay",
        msg::bills_command_pay,
    ),
    schedule(
        SKIP,
        "s",
        ScheduleEvent::Skip,
        "bills skip",
        msg::bills_command_skip,
    ),
    schedule(
        PREVIOUS_PERIOD,
        "[",
        ScheduleEvent::PrevPeriod,
        "bills previous-period",
        msg::bills_command_previous_period,
    ),
    schedule(
        NEXT_PERIOD,
        "]",
        ScheduleEvent::NextPeriod,
        "bills next-period",
        msg::bills_command_next_period,
    ),
    schedule(
        ALL,
        "0",
        ScheduleEvent::ToggleAll,
        "bills all",
        msg::bills_command_all,
    ),
    schedule(
        FILTERS,
        "f",
        ScheduleEvent::CycleFilter,
        "bills filters",
        msg::bills_command_filters,
    ),
    schedule(
        CHIP_PAID,
        "1",
        ScheduleEvent::ToggleChip(BillStatus::Paid),
        "bills show-paid",
        msg::bills_command_show_paid,
    ),
    schedule(
        CHIP_SKIPPED,
        "2",
        ScheduleEvent::ToggleChip(BillStatus::Skipped),
        "bills show-skipped",
        msg::bills_command_show_skipped,
    ),
    schedule(
        CHIP_OVERDUE,
        "3",
        ScheduleEvent::ToggleChip(BillStatus::Overdue),
        "bills show-overdue",
        msg::bills_command_show_overdue,
    ),
    schedule(
        CHIP_DUE,
        "4",
        ScheduleEvent::ToggleChip(BillStatus::Due),
        "bills show-due",
        msg::bills_command_show_due,
    ),
    schedule(
        CHIP_UPCOMING,
        "5",
        ScheduleEvent::ToggleChip(BillStatus::Upcoming),
        "bills show-upcoming",
        msg::bills_command_show_upcoming,
    ),
];

/// The Bills page's keymap under `config`'s overrides.
pub fn bills_page_keymap(config: &KeyBindingConfig) -> Keymap<BillsPageEvent> {
    Keymap::new(BILLS_PAGE_BINDINGS, config)
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

    #[test]
    fn a_default_chord_reaches_its_event_for_a_shared_and_a_namespaced_command() {
        let keymap = bills_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("n"), Some(BillsPageEvent::New));
        assert_eq!(
            keymap.event("p"),
            Some(BillsPageEvent::Schedule(ScheduleEvent::Pay))
        );
        assert_eq!(
            keymap.event("3"),
            Some(BillsPageEvent::Schedule(ScheduleEvent::ToggleChip(
                BillStatus::Overdue
            )))
        );
        assert_eq!(keymap.event("q"), None);
    }

    #[test]
    fn the_arrows_move_and_enter_opens() {
        let keymap = bills_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("up"), Some(BillsPageEvent::Move(Step::Prev)));
        assert_eq!(
            keymap.event("home"),
            Some(BillsPageEvent::Move(Step::First))
        );
        assert_eq!(
            keymap.event("pagedown"),
            Some(BillsPageEvent::Move(Step::HalfPageDown))
        );
        assert_eq!(keymap.event("enter"), Some(BillsPageEvent::Open));
        let moved = bills_page_keymap(&overriding("arrow_up", "ctrl+p"));
        assert_eq!(
            moved.event("ctrl+p"),
            Some(BillsPageEvent::Move(Step::Prev))
        );
        assert_eq!(moved.event("up"), None);
    }

    #[test]
    fn a_keybindings_override_of_a_namespaced_name_moves_only_that_command() {
        let keymap = bills_page_keymap(&overriding("bills.pay", "P"));

        assert_eq!(
            keymap.event("P"),
            Some(BillsPageEvent::Schedule(ScheduleEvent::Pay))
        );
        assert_eq!(keymap.event("p"), None);
        // `pay` alone names no Bills command, so it can't move this one.
        let flat = bills_page_keymap(&overriding("pay", "x"));
        assert_eq!(
            flat.event("p"),
            Some(BillsPageEvent::Schedule(ScheduleEvent::Pay))
        );
    }
}
