//! The Bills page's default keys, built on `lib_viper`'s [`Keymap`]. The commands every module
//! shares keep their flat names from `lib_viper::keymap`; those only Bills has are namespaced here
//! (`bills.pay`). Each is today's key, checked against both Clients' global keys by their clash
//! tests.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    Binding, EDIT, HALF_PAGE_DOWN, HALF_PAGE_UP, Keymap, MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP,
    NEW,
};

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

const fn schedule(
    command: &'static str,
    chord: &'static str,
    event: ScheduleEvent,
) -> Binding<BillsPageEvent> {
    Binding {
        command,
        chord,
        event: BillsPageEvent::Schedule(event),
    }
}

/// The Bills page's commands, with today's keys. A Schedule command does nothing on the Planner,
/// and `edit` nothing on the Schedule.
pub const BILLS_PAGE_BINDINGS: &[Binding<BillsPageEvent>] = &[
    Binding {
        command: MOVE_DOWN,
        chord: "j",
        event: BillsPageEvent::Move(Step::Next),
    },
    Binding {
        command: MOVE_UP,
        chord: "k",
        event: BillsPageEvent::Move(Step::Prev),
    },
    Binding {
        command: MOVE_FIRST,
        chord: "g g",
        event: BillsPageEvent::Move(Step::First),
    },
    Binding {
        command: MOVE_LAST,
        chord: "G",
        event: BillsPageEvent::Move(Step::Last),
    },
    Binding {
        command: HALF_PAGE_DOWN,
        chord: "ctrl+d",
        event: BillsPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: HALF_PAGE_UP,
        chord: "ctrl+u",
        event: BillsPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: NEW,
        chord: "n",
        event: BillsPageEvent::New,
    },
    Binding {
        command: EDIT,
        chord: "e",
        event: BillsPageEvent::Planner(PlannerEvent::Edit),
    },
    Binding {
        command: NEXT_TAB,
        chord: "tab",
        event: BillsPageEvent::NextTab,
    },
    schedule(PAY, "p", ScheduleEvent::Pay),
    schedule(SKIP, "s", ScheduleEvent::Skip),
    schedule(PREVIOUS_PERIOD, "[", ScheduleEvent::PrevPeriod),
    schedule(NEXT_PERIOD, "]", ScheduleEvent::NextPeriod),
    schedule(ALL, "0", ScheduleEvent::ToggleAll),
    schedule(FILTERS, "f", ScheduleEvent::CycleFilter),
    schedule(CHIP_PAID, "1", ScheduleEvent::ToggleChip(BillStatus::Paid)),
    schedule(
        CHIP_SKIPPED,
        "2",
        ScheduleEvent::ToggleChip(BillStatus::Skipped),
    ),
    schedule(
        CHIP_OVERDUE,
        "3",
        ScheduleEvent::ToggleChip(BillStatus::Overdue),
    ),
    schedule(CHIP_DUE, "4", ScheduleEvent::ToggleChip(BillStatus::Due)),
    schedule(
        CHIP_UPCOMING,
        "5",
        ScheduleEvent::ToggleChip(BillStatus::Upcoming),
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
