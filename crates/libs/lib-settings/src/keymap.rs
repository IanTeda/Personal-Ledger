//! The Settings frame's default keys, built on `lib_viper`'s [`Keymap`]: moving through the index,
//! stepping into and out of a page, and at Medium stepping between pages and opening the page
//! picker. Movement and `select` are the flat names every module shares; the rest are the frame's
//! own (`settings.picker`), each on today's key. `esc` is the Shell's `back`, which it hands the
//! frame, so the frame doesn't bind it.
//!
//! While the picker is open the same keys move its highlight and open its chosen page, and every
//! other key the frame binds is swallowed ([`SettingsFramePresenter::takes`]).
//!
//! The This device pages' keys ([`general_keymap`], [`display_keymap`]) walk a page's controls and
//! change the one focused, and on Display toggle its checkbox and choose a Colour Theme. A page's
//! keys are offered before the frame's, so its `h`/`l` win over the frame's `h`.
//!
//! [`SettingsFramePresenter::takes`]: crate::presenter::SettingsFramePresenter::takes

use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, END, FOCUS_NEXT, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME,
    KeyBindingConfig, Keymap, MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, SELECT,
};

use crate::presenter::{
    DisplayEvent, GeneralEvent, LedgerEvent, LogScroll, SettingsFrameEvent, TracingEvent,
};

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

/// On a This device page, the focused control's choice before.
pub const PREVIOUS_CHOICE: &str = "settings.previous_choice";
/// On a This device page, the focused control's choice after.
pub const NEXT_CHOICE: &str = "settings.next_choice";
/// The choice before, on the left arrow.
pub const CHOICE_LEFT: &str = "settings.choice_left";
/// The choice after, on the right arrow.
pub const CHOICE_RIGHT: &str = "settings.choice_right";
/// Ticks or clears the focused checkbox.
pub const TOGGLE: &str = "settings.toggle";
/// On General, with Toasts focused, raises the Toasts test.
pub const TEST_TOASTS: &str = "settings.test_toasts";

/// Settings › General's commands, with today's keys: `j`/`k` walk the controls, `h`/`l` change
/// the one focused and `t` on Toasts raises the Toasts test. None is listed in the palette: each
/// means something only where the focus is, and `:toasts test` is the test's own Command.
pub const GENERAL_BINDINGS: &[Binding<GeneralEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", GeneralEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", GeneralEvent::Move(Step::Prev)),
    Binding::hidden(ARROW_DOWN, "down", GeneralEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", GeneralEvent::Move(Step::Prev)),
    Binding::hidden(PREVIOUS_CHOICE, "h", GeneralEvent::Change(Step::Prev)),
    Binding::hidden(NEXT_CHOICE, "l", GeneralEvent::Change(Step::Next)),
    Binding::hidden(CHOICE_LEFT, "left", GeneralEvent::Change(Step::Prev)),
    Binding::hidden(CHOICE_RIGHT, "right", GeneralEvent::Change(Step::Next)),
    Binding::hidden(TEST_TOASTS, "t", GeneralEvent::TestToasts),
];

/// Settings › General's keymap under `config`'s overrides.
pub fn general_keymap(config: &KeyBindingConfig) -> Keymap<GeneralEvent> {
    Keymap::new(GENERAL_BINDINGS, config)
}

/// Settings › Display's commands, with today's keys: General's, with `enter` and `space` for the
/// checkbox and `enter` to choose a Colour Theme. None is listed in the palette.
///
/// The Colour Theme grid binds the keys that leave it, under the Shell's own command names so one
/// override moves both: focus `tab` and the half-page moves leave the grid, then go on to the
/// Shell, which moves the focus or scrolls the page. `shift+tab` is a key the Shell keeps, so no
/// module sees it; it leaves the grid as the page loses the keyboard. Any other key the grid
/// doesn't take leaves the grid where it is.
pub const DISPLAY_BINDINGS: &[Binding<DisplayEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", DisplayEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", DisplayEvent::Move(Step::Prev)),
    Binding::hidden(ARROW_DOWN, "down", DisplayEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", DisplayEvent::Move(Step::Prev)),
    Binding::hidden(PREVIOUS_CHOICE, "h", DisplayEvent::Change(Step::Prev)),
    Binding::hidden(NEXT_CHOICE, "l", DisplayEvent::Change(Step::Next)),
    Binding::hidden(CHOICE_LEFT, "left", DisplayEvent::Change(Step::Prev)),
    Binding::hidden(CHOICE_RIGHT, "right", DisplayEvent::Change(Step::Next)),
    Binding::hidden(SELECT, "enter", DisplayEvent::Select),
    Binding::hidden(TOGGLE, "space", DisplayEvent::Toggle),
    Binding::hidden(FOCUS_NEXT, "tab", DisplayEvent::LeaveGrid),
    Binding::hidden(HALF_PAGE_DOWN, "ctrl+d", DisplayEvent::LeaveGrid),
    Binding::hidden(HALF_PAGE_UP, "ctrl+u", DisplayEvent::LeaveGrid),
];

/// Settings › Display's keymap under `config`'s overrides.
pub fn display_keymap(config: &KeyBindingConfig) -> Keymap<DisplayEvent> {
    Keymap::new(DISPLAY_BINDINGS, config)
}

/// Settings › Ledger's commands, with today's keys: `j`/`k` step the Base unit select. None is
/// listed in the palette.
pub const LEDGER_BINDINGS: &[Binding<LedgerEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", LedgerEvent::Step(Step::Next)),
    Binding::hidden(MOVE_UP, "k", LedgerEvent::Step(Step::Prev)),
    Binding::hidden(ARROW_DOWN, "down", LedgerEvent::Step(Step::Next)),
    Binding::hidden(ARROW_UP, "up", LedgerEvent::Step(Step::Prev)),
];

/// Settings › Ledger's keymap under `config`'s overrides.
pub fn ledger_keymap(config: &KeyBindingConfig) -> Keymap<LedgerEvent> {
    Keymap::new(LEDGER_BINDINGS, config)
}

/// On Settings › Tracing, a page of the log box down.
pub const PAGE_DOWN: &str = "settings.log_page_down";
/// On Settings › Tracing, a page of the log box up.
pub const PAGE_UP: &str = "settings.log_page_up";
/// On Settings › Tracing, the log box's oldest entry.
pub const OLDEST: &str = "settings.log_oldest";
/// On Settings › Tracing, Clear logs.
pub const CLEAR_LOGS: &str = "settings.clear_logs";

/// Settings › Tracing's commands, with today's keys: `h`/`l` the level, `j`/`k` a line, `J`/`K` a
/// page, `G` the oldest entry and `c` Clear logs. None is listed in the palette.
pub const TRACING_BINDINGS: &[Binding<TracingEvent>] = &[
    Binding::hidden(PREVIOUS_CHOICE, "h", TracingEvent::Level(Step::Prev)),
    Binding::hidden(NEXT_CHOICE, "l", TracingEvent::Level(Step::Next)),
    Binding::hidden(CHOICE_LEFT, "left", TracingEvent::Level(Step::Prev)),
    Binding::hidden(CHOICE_RIGHT, "right", TracingEvent::Level(Step::Next)),
    Binding::hidden(
        MOVE_DOWN,
        "j",
        TracingEvent::Scroll(LogScroll::Line(Step::Next)),
    ),
    Binding::hidden(
        MOVE_UP,
        "k",
        TracingEvent::Scroll(LogScroll::Line(Step::Prev)),
    ),
    Binding::hidden(
        ARROW_DOWN,
        "down",
        TracingEvent::Scroll(LogScroll::Line(Step::Next)),
    ),
    Binding::hidden(
        ARROW_UP,
        "up",
        TracingEvent::Scroll(LogScroll::Line(Step::Prev)),
    ),
    Binding::hidden(
        PAGE_DOWN,
        "J",
        TracingEvent::Scroll(LogScroll::Page(Step::Next)),
    ),
    Binding::hidden(
        PAGE_UP,
        "K",
        TracingEvent::Scroll(LogScroll::Page(Step::Prev)),
    ),
    Binding::hidden(OLDEST, "G", TracingEvent::Scroll(LogScroll::Oldest)),
    Binding::hidden(CLEAR_LOGS, "c", TracingEvent::ClearLogs),
];

/// Settings › Tracing's keymap under `config`'s overrides.
pub fn tracing_keymap(config: &KeyBindingConfig) -> Keymap<TracingEvent> {
    Keymap::new(TRACING_BINDINGS, config)
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
    fn the_general_defaults_are_todays_keys() {
        let keymap = general_keymap(&KeyBindingConfig::default());

        for (chord, event) in [
            ("j", GeneralEvent::Move(Step::Next)),
            ("down", GeneralEvent::Move(Step::Next)),
            ("k", GeneralEvent::Move(Step::Prev)),
            ("h", GeneralEvent::Change(Step::Prev)),
            ("left", GeneralEvent::Change(Step::Prev)),
            ("l", GeneralEvent::Change(Step::Next)),
            ("right", GeneralEvent::Change(Step::Next)),
        ] {
            assert_eq!(keymap.event(chord), Some(event), "{chord}");
        }
        assert_eq!(keymap.event("enter"), None, "General has no checkbox");
        assert_eq!(keymap.listed().count(), 0, "the palette lists none");
    }

    #[test]
    fn the_display_defaults_are_todays_keys() {
        let keymap = display_keymap(&KeyBindingConfig::default());

        for (chord, event) in [
            ("j", DisplayEvent::Move(Step::Next)),
            ("down", DisplayEvent::Move(Step::Next)),
            ("k", DisplayEvent::Move(Step::Prev)),
            ("h", DisplayEvent::Change(Step::Prev)),
            ("left", DisplayEvent::Change(Step::Prev)),
            ("l", DisplayEvent::Change(Step::Next)),
            ("right", DisplayEvent::Change(Step::Next)),
            ("enter", DisplayEvent::Select),
            ("space", DisplayEvent::Toggle),
            ("tab", DisplayEvent::LeaveGrid),
            ("ctrl+d", DisplayEvent::LeaveGrid),
            ("ctrl+u", DisplayEvent::LeaveGrid),
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

    #[test]
    fn the_tracing_defaults_are_todays_keys() {
        let keymap = tracing_keymap(&KeyBindingConfig::default());

        for (chord, event) in [
            ("h", TracingEvent::Level(Step::Prev)),
            ("left", TracingEvent::Level(Step::Prev)),
            ("l", TracingEvent::Level(Step::Next)),
            ("right", TracingEvent::Level(Step::Next)),
            ("j", TracingEvent::Scroll(LogScroll::Line(Step::Next))),
            ("up", TracingEvent::Scroll(LogScroll::Line(Step::Prev))),
            ("J", TracingEvent::Scroll(LogScroll::Page(Step::Next))),
            ("K", TracingEvent::Scroll(LogScroll::Page(Step::Prev))),
            ("G", TracingEvent::Scroll(LogScroll::Oldest)),
            ("c", TracingEvent::ClearLogs),
        ] {
            assert_eq!(keymap.event(chord), Some(event), "{chord}");
        }
        assert_eq!(keymap.listed().count(), 0, "the palette lists none");
    }
}
