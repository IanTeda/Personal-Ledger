//! The Settings frame's default keys, built on `lib_viper`'s [`Keymap`]: moving through the index,
//! stepping into and out of a page, and at Medium stepping between pages and opening the page
//! picker. Movement and `select` are the flat names every module shares; the rest are the frame's
//! own (`settings.picker`), each on today's key. `esc` is the Shell's `back`, which it hands the
//! frame, so the frame doesn't bind it.
//!
//! While the picker is open the same keys move its highlight and open its chosen page, and every
//! other key the frame binds is swallowed ([`SettingsFramePresenter::takes`]).
//!
//! [`SettingsFramePresenter::takes`]: crate::presenter::SettingsFramePresenter::takes

use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, END, HOME, KeyBindingConfig, Keymap, MOVE_DOWN, MOVE_FIRST,
    MOVE_LAST, MOVE_UP, SELECT,
};

use crate::presenter::SettingsFrameEvent;

/// From the index into the page on show.
pub const ENTER_PAGE: &str = "settings.enter_page";
/// From the index into the page on show, on the right arrow.
pub const ARROW_RIGHT: &str = "settings.arrow_right";
/// From the page back to the index.
pub const FOCUS_INDEX: &str = "settings.focus_index";
/// From the page back to the index, on the left arrow.
pub const ARROW_LEFT: &str = "settings.arrow_left";
/// At Medium, the page before in index order.
pub const PREVIOUS_PAGE: &str = "settings.previous_page";
/// At Medium, the page after in index order.
pub const NEXT_PAGE: &str = "settings.next_page";
/// At Medium, the page picker under the page's name.
pub const PICKER: &str = "settings.picker";

/// The frame's commands, with today's keys. None is listed in the palette: each is movement or
/// means something only where the focus is, and `settings <page>` already opens a page by name.
pub const SETTINGS_FRAME_BINDINGS: &[Binding<SettingsFrameEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", SettingsFrameEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", SettingsFrameEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", SettingsFrameEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", SettingsFrameEvent::Move(Step::Last)),
    Binding::hidden(ARROW_DOWN, "down", SettingsFrameEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", SettingsFrameEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", SettingsFrameEvent::Move(Step::First)),
    Binding::hidden(END, "end", SettingsFrameEvent::Move(Step::Last)),
    Binding::hidden(SELECT, "enter", SettingsFrameEvent::Select),
    Binding::hidden(ENTER_PAGE, "l", SettingsFrameEvent::EnterPage),
    Binding::hidden(ARROW_RIGHT, "right", SettingsFrameEvent::EnterPage),
    Binding::hidden(FOCUS_INDEX, "h", SettingsFrameEvent::FocusIndex),
    Binding::hidden(ARROW_LEFT, "left", SettingsFrameEvent::FocusIndex),
    Binding::hidden(PREVIOUS_PAGE, "[", SettingsFrameEvent::StepPage(Step::Prev)),
    Binding::hidden(NEXT_PAGE, "]", SettingsFrameEvent::StepPage(Step::Next)),
    Binding::hidden(PICKER, "p", SettingsFrameEvent::OpenPicker),
];

/// The frame's keymap under `config`'s overrides.
pub fn settings_frame_keymap(config: &KeyBindingConfig) -> Keymap<SettingsFrameEvent> {
    Keymap::new(SETTINGS_FRAME_BINDINGS, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_todays_keys() {
        let keymap = settings_frame_keymap(&KeyBindingConfig::default());

        for (chord, event) in [
            ("j", SettingsFrameEvent::Move(Step::Next)),
            ("k", SettingsFrameEvent::Move(Step::Prev)),
            ("g g", SettingsFrameEvent::Move(Step::First)),
            ("G", SettingsFrameEvent::Move(Step::Last)),
            ("enter", SettingsFrameEvent::Select),
            ("l", SettingsFrameEvent::EnterPage),
            ("right", SettingsFrameEvent::EnterPage),
            ("h", SettingsFrameEvent::FocusIndex),
            ("left", SettingsFrameEvent::FocusIndex),
            ("[", SettingsFrameEvent::StepPage(Step::Prev)),
            ("]", SettingsFrameEvent::StepPage(Step::Next)),
            ("p", SettingsFrameEvent::OpenPicker),
        ] {
            assert_eq!(keymap.event(chord), Some(event), "{chord}");
        }
        assert_eq!(keymap.listed().count(), 0, "the palette lists none");
    }

    #[test]
    fn keybindings_rebinds_a_frame_command() {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(PICKER.to_string(), "ctrl+p".to_string());
        let keymap = settings_frame_keymap(&config);

        assert_eq!(keymap.event("ctrl+p"), Some(SettingsFrameEvent::OpenPicker));
        assert_eq!(keymap.event("p"), None);
    }
}
