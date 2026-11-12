//! The status-line legends of the Settings pages and of the dialogs they open, as [`Hint`]s. They
//! live beside the Settings Views that own the keys, not in `Root`.

use lib_settings::{SettingsFrameEvent, keymap::FOCUS_INDEX};
use lib_shell::Hint;
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP};

/// The keys of a Settings page with no row or field cursor of its own (the info pages and the
/// Institutions table, which is mouse-driven for now). Their chords are read from the frame's
/// keymap, so a `[keybindings]` rebind of the frame's move or focus-index key shows here.
pub(crate) fn settings_plain_page_hints(keymap: &Keymap<SettingsFrameEvent>) -> Vec<Hint> {
    vec![
        Hint::new(
            keymap.hint_chords(&[MOVE_DOWN, MOVE_UP]),
            crate::msg::desktop_hint_scroll(),
        ),
        Hint::new(
            keymap.hint_chord(FOCUS_INDEX),
            crate::msg::desktop_hint_index(),
        ),
    ]
}
