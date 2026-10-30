//! Shared harness for the TUI's in-process keyboard tests (ADR-0035). Each test builds a
//! hermetic `Shell`, feeds it key sequences the way the terminal would, and reads the drawn
//! screen back from ratatui's `TestBackend`. Nothing here touches the terminal or the user's
//! config, and only a test that runs `:open` or `:new` touches a Ledger file: [`shell`]'s Units sit
//! on `lib-units`' in-memory ports, beside the other domains' in-memory stub stores. What a test may assert on (screen text, Colour Roles, and when a
//! `#[doc(hidden)]` accessor is allowed) is set out in ADR-0035.

#![expect(
    clippy::panic,
    reason = "test support: an unknown key token is a typo in the test and should fail it loudly"
)]

use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc;

use bin_tui::{Shell, build_shell, colours::Colours, event::Event, locale};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_config::KeyBindingConfig;
use lib_database::{Commit, FakeStores, InMemoryCommit, Stores, UnitsStore, WriteFailure};
use lib_units::fake::InMemoryPorts;
use ratatui::{Terminal, backend::TestBackend};

/// The fixed terminal size every test draws at, so layouts never depend on the host.
pub const WIDTH: u16 = 96;
pub const HEIGHT: u16 = 30;

/// A `Shell` on the default key bindings and Colours, with the Locale pinned to `en-US` and a
/// Ledger open on the stub seed's Units in memory.
pub fn shell() -> Shell {
    shell_with_failures().0
}

/// [`shell`], with the sender of its writer failure channel, so a test can report a failed write.
pub fn shell_with_failures() -> (Shell, mpsc::Sender<WriteFailure>) {
    shell_on(
        Box::new(InMemoryPorts::stub(chrono::Utc::now())),
        Rc::new(InMemoryCommit),
    )
}

/// A `Shell` with a Ledger open whose Units are kept in `units`, committed through `commit`, and
/// every other domain's stub seed beside them, so the stub's Accounts and Locations use its Units.
pub fn shell_on(
    units: Box<dyn UnitsStore>,
    commit: Rc<dyn Commit>,
) -> (Shell, mpsc::Sender<WriteFailure>) {
    let (report, failures) = mpsc::channel();
    let (stub, _) = Stores::fake(FakeStores::stub(chrono::Local::now().date_naive()));
    let stores = Stores {
        units,
        commit,
        failures,
        ..stub
    };
    let shell = cold_shell(None).with_open_ledger("test.pldb", stores);
    (shell, report)
}

/// A Ledger file at `path`, created and seeded as `:new` would, then changed by `edit` and saved.
pub fn ledger_file_with(path: &Path, edit: impl FnOnce(&mut lib_database::Storage)) {
    let storage = lib_database::open_storage(path, chrono::Local::now().date_naive(), "Personal");
    assert!(storage.is_ok(), "a new Ledger opens");
    let Ok(mut storage) = storage else { return };
    edit(&mut storage);
    Commit::commit(&storage.pending);
    assert!(storage.writer.flush().is_ok(), "the writer is running");
    let failed = storage.failures.try_recv();
    assert!(failed.is_err(), "the edit was written: {failed:?}");
}

/// A `Shell` as `main` starts it: no Ledger open, and `file` the configured Ledger file a bare
/// `:open` opens, if any.
pub fn cold_shell(file: Option<&Path>) -> Shell {
    locale::init_for_tests();
    build_shell(KeyBindingConfig::default(), Colours::default(), file)
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

/// Runs `command` through the command popup: `:`, the command as typed, then `Enter`.
pub fn run_command(shell: &mut Shell, command: &str) {
    press(shell, ":");
    type_text(shell, command);
    press(shell, "Enter");
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
