//! Shared harness for the TUI's in-process keyboard tests (ADR-0035). Each test builds a
//! hermetic `Shell`, feeds it key sequences the way the terminal would, and reads the drawn
//! screen back from ratatui's `TestBackend`. Nothing here touches the database, the terminal
//! or the user's config: the Units sit on `lib-units`' in-memory ports. What a test may assert on (screen text, Colour Roles, and when a
//! `#[doc(hidden)]` accessor is allowed) is set out in ADR-0035.

#![expect(
    clippy::panic,
    reason = "test support: an unknown key token is a typo in the test and should fail it loudly"
)]

use std::sync::mpsc;

use bin_tui::{Shell, build_shell, colours::Colours, event::Event, locale};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_config::KeyBindingConfig;
use lib_database::{UnitsStore, WriteFailure};
use lib_units::fake::InMemoryPorts;
use ratatui::{Terminal, backend::TestBackend};

/// The fixed terminal size every test draws at, so layouts never depend on the host.
pub const WIDTH: u16 = 96;
pub const HEIGHT: u16 = 30;

/// A `Shell` on the default key bindings and Colours, with the Locale pinned to `en-US` and the
/// Units on the stub seed in memory.
pub fn shell() -> Shell {
    shell_with_failures().0
}

/// [`shell`], with the sender of its writer failure channel, so a test can report a failed write.
pub fn shell_with_failures() -> (Shell, mpsc::Sender<WriteFailure>) {
    shell_on(Box::new(InMemoryPorts::stub(chrono::Utc::now())))
}

/// A `Shell` whose Units are kept in `units`.
pub fn shell_on(units: Box<dyn UnitsStore>) -> (Shell, mpsc::Sender<WriteFailure>) {
    locale::init_for_tests();
    let (report, failures) = mpsc::channel();
    let shell = build_shell(
        KeyBindingConfig::default(),
        Colours::default(),
        units,
        failures,
    );
    (shell, report)
}

/// Sends a whitespace-separated key sequence, one key per token: a single character is that
/// character, and `Esc`, `Enter`, `Tab`, `Backspace`, `Left`, `Right`, `Up` or `Down` name the
/// special keys (e.g. `"g a"`, `": Esc"`).
pub fn press(shell: &mut Shell, keys: &str) {
    for token in keys.split_whitespace() {
        let code = match token {
            "Esc" => KeyCode::Esc,
            "Enter" => KeyCode::Enter,
            "Tab" => KeyCode::Tab,
            "Backspace" => KeyCode::Backspace,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            other => {
                let mut chars = other.chars();
                let (Some(c), None) = (chars.next(), chars.next()) else {
                    panic!("unknown key token `{other}`: use one character or a named key");
                };
                KeyCode::Char(c)
            }
        };
        shell.handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }
}

/// Types `text` one character at a time, spaces included, as into a focused text field.
pub fn type_text(shell: &mut Shell, text: &str) {
    for c in text.chars() {
        shell.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char(c),
            KeyModifiers::NONE,
        )));
    }
}

/// Every row of the frame as one string, drawn at [`WIDTH`] by [`HEIGHT`].
pub fn screen(shell: &mut Shell) -> String {
    let mut terminal =
        Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("test backend should initialise");
    terminal
        .draw(|frame| shell.draw(frame))
        .expect("drawing the shell should not error");

    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}
