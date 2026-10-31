//! The Import step's default keys, built on `lib_viper`'s [`Keymap`]: one keymap for the rows and
//! one for while a row's select is open, since a key means something else then. The commands every
//! module shares keep their flat names; those only the Import has are namespaced
//! (`import.pick_payee`). `back` is bound in both, and the Client hands the Presenter
//! [`ImportEvent::Cancel`] for it, as the Shell keeps `esc`.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, BACK, Binding, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::presenter::{ImportEvent, RowSelect};

pub const PICK_PAYEE: &str = "import.pick_payee";
pub const PICK_CATEGORY: &str = "import.pick_category";
pub const NEW_PAYEE: &str = "import.new_payee";
pub const REMEMBER: &str = "import.remember";
pub const CONTINUE: &str = "import.continue";
/// `space` chooses in an open select as `enter` does, under its own name so an override can move it.
pub const CHOOSE: &str = "import.choose";

/// The rows' commands. `n` is `import.new_payee`, namespaced rather than the flat `new`, which the
/// Clients bind to their own add key.
pub const IMPORT_BINDINGS: &[Binding<ImportEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", ImportEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", ImportEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", ImportEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", ImportEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        ImportEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(HALF_PAGE_UP, "ctrl+u", ImportEvent::Move(Step::HalfPageUp)),
    Binding::hidden(ARROW_DOWN, "down", ImportEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", ImportEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", ImportEvent::Move(Step::First)),
    Binding::hidden(END, "end", ImportEvent::Move(Step::Last)),
    Binding::hidden(PAGE_DOWN, "pagedown", ImportEvent::Move(Step::HalfPageDown)),
    Binding::hidden(PAGE_UP, "pageup", ImportEvent::Move(Step::HalfPageUp)),
    Binding::hidden(PICK_PAYEE, "p", ImportEvent::Open(RowSelect::Payee)),
    Binding::hidden(PICK_CATEGORY, "c", ImportEvent::Open(RowSelect::Category)),
    Binding::hidden(NEW_PAYEE, "n", ImportEvent::NewPayee),
    Binding::hidden(REMEMBER, "r", ImportEvent::ToggleRemember),
    Binding::hidden(CONTINUE, "enter", ImportEvent::Continue),
    Binding::hidden(BACK, "esc", ImportEvent::Cancel),
];

/// While a select is open: the highlight, choose and close. Anything else is swallowed by the
/// Client, as an open list swallows it in the Dialogs.
pub const IMPORT_SELECT_BINDINGS: &[Binding<ImportEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", ImportEvent::Highlight(Step::Next)),
    Binding::hidden(MOVE_UP, "k", ImportEvent::Highlight(Step::Prev)),
    Binding::hidden(ARROW_DOWN, "down", ImportEvent::Highlight(Step::Next)),
    Binding::hidden(ARROW_UP, "up", ImportEvent::Highlight(Step::Prev)),
    Binding::hidden(SELECT, "enter", ImportEvent::Commit),
    Binding::hidden(CHOOSE, "space", ImportEvent::Commit),
    Binding::hidden(BACK, "esc", ImportEvent::Cancel),
];

/// The rows' keymap under `config`'s overrides.
pub fn import_keymap(config: &KeyBindingConfig) -> Keymap<ImportEvent> {
    Keymap::new(IMPORT_BINDINGS, config)
}

/// The open select's keymap under `config`'s overrides.
pub fn import_select_keymap(config: &KeyBindingConfig) -> Keymap<ImportEvent> {
    Keymap::new(IMPORT_SELECT_BINDINGS, config)
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
    fn the_default_chords_reach_their_events() {
        let rows = import_keymap(&KeyBindingConfig::default());
        assert_eq!(rows.event("j"), Some(ImportEvent::Move(Step::Next)));
        assert_eq!(rows.event("g g"), Some(ImportEvent::Move(Step::First)));
        assert_eq!(rows.event("p"), Some(ImportEvent::Open(RowSelect::Payee)));
        assert_eq!(
            rows.event("c"),
            Some(ImportEvent::Open(RowSelect::Category))
        );
        assert_eq!(rows.event("n"), Some(ImportEvent::NewPayee));
        assert_eq!(rows.event("r"), Some(ImportEvent::ToggleRemember));
        assert_eq!(rows.event("enter"), Some(ImportEvent::Continue));
        assert_eq!(rows.event("esc"), Some(ImportEvent::Cancel));

        let select = import_select_keymap(&KeyBindingConfig::default());
        assert_eq!(select.event("enter"), Some(ImportEvent::Commit));
        assert_eq!(select.event("space"), Some(ImportEvent::Commit));
        assert_eq!(
            select.event("down"),
            Some(ImportEvent::Highlight(Step::Next))
        );
        assert_eq!(
            select.event("p"),
            None,
            "an open select swallows the rows' keys"
        );
    }

    #[test]
    fn a_keybindings_override_moves_a_namespaced_and_a_shared_command() {
        let moved = import_keymap(&overriding(NEW_PAYEE, "N"));
        assert_eq!(moved.event("N"), Some(ImportEvent::NewPayee));
        assert_eq!(moved.event("n"), None);

        let moved = import_keymap(&overriding(MOVE_DOWN, "ctrl+n"));
        assert_eq!(moved.event("ctrl+n"), Some(ImportEvent::Move(Step::Next)));
        assert_eq!(moved.chord(MOVE_DOWN), Some("ctrl+n"));
    }
}
