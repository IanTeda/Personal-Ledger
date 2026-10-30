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

use crate::msg;

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
    Binding::hidden(MOVE_DOWN, "j", InventoryPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", InventoryPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", InventoryPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", InventoryPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        InventoryPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        InventoryPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", InventoryPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", InventoryPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", InventoryPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", InventoryPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        InventoryPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        PAGE_UP,
        "pageup",
        InventoryPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::listed(
        NEW,
        "n",
        InventoryPageEvent::New,
        "inventory new",
        msg::inventory_command_new,
    ),
    Binding::listed(
        EDIT,
        "e",
        InventoryPageEvent::Edit,
        "inventory edit",
        msg::inventory_command_edit,
    ),
    Binding::listed(
        REMOVE,
        "x",
        InventoryPageEvent::Remove,
        "inventory remove",
        msg::inventory_command_remove,
    ),
    Binding::listed(
        ADD_AREA,
        "a",
        InventoryPageEvent::AddArea,
        "inventory add-area",
        msg::inventory_command_add_area,
    ),
    Binding::hidden(MOVE_AREA_DOWN, "J", InventoryPageEvent::MoveArea(1)),
    Binding::hidden(MOVE_AREA_UP, "K", InventoryPageEvent::MoveArea(-1)),
    Binding::hidden(EXPAND, "right", InventoryPageEvent::Expand),
    Binding::hidden(COLLAPSE, "left", InventoryPageEvent::Collapse),
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
