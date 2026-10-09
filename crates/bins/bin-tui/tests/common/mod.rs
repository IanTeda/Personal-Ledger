//! Shared harness for the TUI's in-process keyboard tests (ADR-0035). Each test builds a
//! hermetic `Shell`, feeds it key sequences the way the terminal would, and reads the drawn
//! screen back from ratatui's `TestBackend`. Nothing here touches the database, the terminal
//! or the user's config. What a test may assert on (screen text, Colour Roles, and when a
//! `#[doc(hidden)]` accessor is allowed) is set out in ADR-0035.

#![expect(
    clippy::panic,
    reason = "test support: an unknown key token is a typo in the test and should fail it loudly"
)]

use bin_tui::{Shell, build_shell, colours::Colours, event::Event, locale};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_config::KeyBindingConfig;
use ratatui::{Terminal, backend::TestBackend};

/// The fixed terminal size every test draws at, so layouts never depend on the host.
pub const WIDTH: u16 = 96;
pub const HEIGHT: u16 = 30;

/// A `Shell` on the default key bindings and Colours, with the Locale pinned to `en-US`.
pub fn shell() -> Shell {
    locale::init_for_tests();
    build_shell(KeyBindingConfig::default(), Colours::default())
}

/// Sends a whitespace-separated key sequence, one key per token: a single character is that
/// character, and `Esc`, `Enter` or `Tab` name the special keys (e.g. `"g a"`, `": Esc"`).
pub fn press(shell: &mut Shell, keys: &str) {
    for token in keys.split_whitespace() {
        let code = match token {
            "Esc" => KeyCode::Esc,
            "Enter" => KeyCode::Enter,
            "Tab" => KeyCode::Tab,
            other => {
                let mut chars = other.chars();
                let (Some(c), None) = (chars.next(), chars.next()) else {
                    panic!("unknown key token `{other}`: use one character or Esc/Enter/Tab");
                };
                KeyCode::Char(c)
            }
        };
        shell.handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
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
