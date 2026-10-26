//! The Categories page's default keys, built on `lib_viper`'s [`Keymap`]. Movement, `new`, `edit`
//! and `delete` are the flat names every module shares (`lib_viper::keymap`); the tree's own
//! commands, adding a sub-category and opening and folding a parent, are namespaced by the module.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, DELETE, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP,
};

use crate::presenter::CategoriesPageEvent;

/// Add a sub-category under the selected Category.
pub const NEW_CHILD: &str = "categories.new_child";
/// Open a folded parent, or step into an open one.
pub const EXPAND: &str = "categories.expand";
/// Fold an open parent, or climb from a nested row to its parent.
pub const COLLAPSE: &str = "categories.collapse";

/// The Categories page's commands, with today's keys. The arrow, Home/End and Page keys move as
/// `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it.
pub const CATEGORIES_PAGE_BINDINGS: &[Binding<CategoriesPageEvent>] = &[
    Binding {
        command: MOVE_DOWN,
        chord: "j",
        event: CategoriesPageEvent::Move(Step::Next),
    },
    Binding {
        command: MOVE_UP,
        chord: "k",
        event: CategoriesPageEvent::Move(Step::Prev),
    },
    Binding {
        command: MOVE_FIRST,
        chord: "g g",
        event: CategoriesPageEvent::Move(Step::First),
    },
    Binding {
        command: MOVE_LAST,
        chord: "G",
        event: CategoriesPageEvent::Move(Step::Last),
    },
    Binding {
        command: HALF_PAGE_DOWN,
        chord: "ctrl+d",
        event: CategoriesPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: HALF_PAGE_UP,
        chord: "ctrl+u",
        event: CategoriesPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: ARROW_DOWN,
        chord: "down",
        event: CategoriesPageEvent::Move(Step::Next),
    },
    Binding {
        command: ARROW_UP,
        chord: "up",
        event: CategoriesPageEvent::Move(Step::Prev),
    },
    Binding {
        command: HOME,
        chord: "home",
        event: CategoriesPageEvent::Move(Step::First),
    },
    Binding {
        command: END,
        chord: "end",
        event: CategoriesPageEvent::Move(Step::Last),
    },
    Binding {
        command: PAGE_DOWN,
        chord: "pagedown",
        event: CategoriesPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: PAGE_UP,
        chord: "pageup",
        event: CategoriesPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: NEW,
        chord: "n",
        event: CategoriesPageEvent::New,
    },
    Binding {
        command: EDIT,
        chord: "e",
        event: CategoriesPageEvent::Edit,
    },
    Binding {
        command: DELETE,
        chord: "d",
        event: CategoriesPageEvent::Delete,
    },
    Binding {
        command: NEW_CHILD,
        chord: "N",
        event: CategoriesPageEvent::NewChild,
    },
    Binding {
        command: EXPAND,
        chord: "right",
        event: CategoriesPageEvent::Expand,
    },
    Binding {
        command: COLLAPSE,
        chord: "left",
        event: CategoriesPageEvent::Collapse,
    },
];

/// The Categories page's keymap under `config`'s overrides.
pub fn categories_page_keymap(config: &KeyBindingConfig) -> Keymap<CategoriesPageEvent> {
    Keymap::new(CATEGORIES_PAGE_BINDINGS, config)
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
        let keymap = categories_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("n"), Some(CategoriesPageEvent::New));
        assert_eq!(keymap.event("N"), Some(CategoriesPageEvent::NewChild));
        assert_eq!(keymap.event("e"), Some(CategoriesPageEvent::Edit));
        assert_eq!(keymap.event("d"), Some(CategoriesPageEvent::Delete));
        assert_eq!(keymap.event("right"), Some(CategoriesPageEvent::Expand));
        assert_eq!(keymap.event("left"), Some(CategoriesPageEvent::Collapse));
        assert_eq!(
            keymap.event("j"),
            Some(CategoriesPageEvent::Move(Step::Next))
        );
        assert_eq!(
            keymap.event("g g"),
            Some(CategoriesPageEvent::Move(Step::First))
        );
    }

    #[test]
    fn a_keybindings_override_moves_the_event_to_the_new_chord() {
        let keymap = categories_page_keymap(&overriding("categories.new_child", "s"));

        assert_eq!(keymap.event("s"), Some(CategoriesPageEvent::NewChild));
        assert_eq!(keymap.event("N"), None, "the default no longer binds");
        // `new` is the flat name every module shares, so one override moves it here too.
        let flat = categories_page_keymap(&overriding("new", "a"));
        assert_eq!(flat.event("a"), Some(CategoriesPageEvent::New));
        assert_eq!(flat.chord(NEW), Some("a"));
    }
}
