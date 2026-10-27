//! The Inventory page's default keys, built on `lib_viper`'s [`Keymap`]. Movement, `new` and `edit`
//! are the flat names every module shares (`lib_viper::keymap`). Remove is the module's own: the
//! flat `delete` defaults to `d` in `[keybindings]`, and the page has always removed on `x`. So are
//! the tree's own commands, adding and reordering an Area and opening and folding a Location.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP,
};

use crate::presenter::InventoryPageEvent;

/// Remove the selected Location or Area, opening the Remove Dialog on it.
pub const REMOVE: &str = "inventory.remove";
/// Add an Area to the selected Location (or the selected Area's), opening the Location.
pub const ADD_AREA: &str = "inventory.add_area";
/// Move the selected Area a place down its Location's tabs; inert on a Location.
pub const MOVE_AREA_DOWN: &str = "inventory.move_area_down";
/// Move the selected Area a place up its Location's tabs; inert on a Location.
pub const MOVE_AREA_UP: &str = "inventory.move_area_up";
/// Open a closed Location, or step into an open one's first Area.
pub const EXPAND: &str = "inventory.expand";
/// Close an open Location, or climb from an Area to its Location.
pub const COLLAPSE: &str = "inventory.collapse";

/// The Inventory page's commands, with today's keys. The arrow, Home/End and Page keys move as
/// `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it.
pub const INVENTORY_PAGE_BINDINGS: &[Binding<InventoryPageEvent>] = &[
    Binding {
        command: MOVE_DOWN,
        chord: "j",
        event: InventoryPageEvent::Move(Step::Next),
    },
    Binding {
        command: MOVE_UP,
        chord: "k",
        event: InventoryPageEvent::Move(Step::Prev),
    },
    Binding {
        command: MOVE_FIRST,
        chord: "g g",
        event: InventoryPageEvent::Move(Step::First),
    },
    Binding {
        command: MOVE_LAST,
        chord: "G",
        event: InventoryPageEvent::Move(Step::Last),
    },
    Binding {
        command: HALF_PAGE_DOWN,
        chord: "ctrl+d",
        event: InventoryPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: HALF_PAGE_UP,
        chord: "ctrl+u",
        event: InventoryPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: ARROW_DOWN,
        chord: "down",
        event: InventoryPageEvent::Move(Step::Next),
    },
    Binding {
        command: ARROW_UP,
        chord: "up",
        event: InventoryPageEvent::Move(Step::Prev),
    },
    Binding {
        command: HOME,
        chord: "home",
        event: InventoryPageEvent::Move(Step::First),
    },
    Binding {
        command: END,
        chord: "end",
        event: InventoryPageEvent::Move(Step::Last),
    },
    Binding {
        command: PAGE_DOWN,
        chord: "pagedown",
        event: InventoryPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: PAGE_UP,
        chord: "pageup",
        event: InventoryPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: NEW,
        chord: "n",
        event: InventoryPageEvent::New,
    },
    Binding {
        command: EDIT,
        chord: "e",
        event: InventoryPageEvent::Edit,
    },
    Binding {
        command: REMOVE,
        chord: "x",
        event: InventoryPageEvent::Remove,
    },
    Binding {
        command: ADD_AREA,
        chord: "a",
        event: InventoryPageEvent::AddArea,
    },
    Binding {
        command: MOVE_AREA_DOWN,
        chord: "J",
        event: InventoryPageEvent::MoveArea(1),
    },
    Binding {
        command: MOVE_AREA_UP,
        chord: "K",
        event: InventoryPageEvent::MoveArea(-1),
    },
    Binding {
        command: EXPAND,
        chord: "right",
        event: InventoryPageEvent::Expand,
    },
    Binding {
        command: COLLAPSE,
        chord: "left",
        event: InventoryPageEvent::Collapse,
    },
];

/// The Inventory page's keymap under `config`'s overrides.
pub fn inventory_page_keymap(config: &KeyBindingConfig) -> Keymap<InventoryPageEvent> {
    Keymap::new(INVENTORY_PAGE_BINDINGS, config)
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
    fn a_default_chord_reaches_its_event() {
        let keymap = inventory_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("n"), Some(InventoryPageEvent::New));
        assert_eq!(keymap.event("e"), Some(InventoryPageEvent::Edit));
        assert_eq!(keymap.event("x"), Some(InventoryPageEvent::Remove));
        assert_eq!(keymap.event("a"), Some(InventoryPageEvent::AddArea));
        assert_eq!(keymap.event("J"), Some(InventoryPageEvent::MoveArea(1)));
        assert_eq!(keymap.event("K"), Some(InventoryPageEvent::MoveArea(-1)));
        assert_eq!(keymap.event("right"), Some(InventoryPageEvent::Expand));
        assert_eq!(keymap.event("left"), Some(InventoryPageEvent::Collapse));
        assert_eq!(
            keymap.event("j"),
            Some(InventoryPageEvent::Move(Step::Next))
        );
        assert_eq!(
            keymap.event("g g"),
            Some(InventoryPageEvent::Move(Step::First))
        );
    }

    #[test]
    fn a_keybindings_override_moves_the_event_to_the_new_chord() {
        let keymap = inventory_page_keymap(&overriding("inventory.add_area", "s"));

        assert_eq!(keymap.event("s"), Some(InventoryPageEvent::AddArea));
        assert_eq!(keymap.event("a"), None, "the default no longer binds");
        // `new` is the flat name every module shares, so one override moves it here too.
        let flat = inventory_page_keymap(&overriding("new", "c"));
        assert_eq!(flat.event("c"), Some(InventoryPageEvent::New));
        assert_eq!(flat.chord(NEW), Some("c"));
    }
}
