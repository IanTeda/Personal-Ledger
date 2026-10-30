//! The Categories page's default keys, built on `lib_viper`'s [`Keymap`]. Movement, `new`, `edit`
//! and `delete` are the flat names every module shares (`lib_viper::keymap`); the tree's own
//! commands, adding a sub-category and opening and folding a parent, are namespaced by the module.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, DELETE, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP,
};

use crate::msg;

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
    Binding::hidden(MOVE_DOWN, "j", CategoriesPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", CategoriesPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", CategoriesPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", CategoriesPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        CategoriesPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        CategoriesPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", CategoriesPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", CategoriesPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", CategoriesPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", CategoriesPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        CategoriesPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        PAGE_UP,
        "pageup",
        CategoriesPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::listed(
        NEW,
        "n",
        CategoriesPageEvent::New,
        "categories new",
        msg::categories_command_new,
    ),
    Binding::listed(
        EDIT,
        "e",
        CategoriesPageEvent::Edit,
        "categories edit",
        msg::categories_command_edit,
    ),
    Binding::listed(
        DELETE,
        "d",
        CategoriesPageEvent::Delete,
        "categories delete",
        msg::categories_command_delete,
    ),
    Binding::listed(
        NEW_CHILD,
        "N",
        CategoriesPageEvent::NewChild,
        "categories new-child",
        msg::categories_command_new_child,
    ),
    Binding::hidden(EXPAND, "right", CategoriesPageEvent::Expand),
    Binding::hidden(COLLAPSE, "left", CategoriesPageEvent::Collapse),
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
