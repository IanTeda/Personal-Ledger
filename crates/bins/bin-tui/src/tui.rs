//! Terminal setup and teardown.
//!
//! Wraps a [`ratatui::Terminal`] on a [`CrosstermBackend`], entering raw mode and the
//! alternate screen on construction and restoring the terminal on [`Drop`] — including on a
//! panic, so a crash never leaves the user's shell in raw mode.

use std::io::{self, Stdout};

use crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        supports_keyboard_enhancement,
    },
};
use ratatui::{
    Terminal,
    backend::{Backend as _, ClearType, CrosstermBackend},
};

/// The terminal type this crate renders to: `crossterm` on `stdout`.
pub type Backend = CrosstermBackend<Stdout>;

/// Owns the terminal for the lifetime of the app, restoring it on drop.
pub struct Tui {
    terminal: Terminal<Backend>,
    /// Whether the keyboard enhancement flags below were actually pushed, so `Drop` only
    /// pops what it pushed.
    keyboard_enhancement: bool,
}

impl Tui {
    /// Enters raw mode and the alternate screen, ready to draw. Clears the screen before
    /// returning — ratatui's diffing only ever overwrites cells it knows changed since its
    /// *own* last draw, so without this, whatever the terminal emulator left in the alternate
    /// screen buffer (leftover output, a resize revealing untouched rows) can show through
    /// as stray artifacts until something else happens to redraw that exact cell.
    ///
    /// Clears via the backend's own `clear_region` directly, not `Terminal::clear` -- the
    /// latter snapshots and restores the cursor position around the clear, which is a real
    /// blocking ANSI round-trip (`ESC[6n`) to the terminal. That's meaningless here (nothing's
    /// been drawn yet, so there's no cursor position worth preserving) and, chained right after
    /// `supports_keyboard_enhancement`'s own round-trip above, was enough to make CI's smoke
    /// test (which runs against `script`'s fake PTY -- a real pty device, but nothing on the
    /// other end answers escape queries) fail with "the cursor position could not be read
    /// within a normal duration" on Linux/macOS runners (Windows never round-trips at all:
    /// `supports_keyboard_enhancement` is hardcoded `Ok(false)` there). Confirmed by direct
    /// reproduction of the exact CI command locally.
    ///
    /// Also opts into the Kitty keyboard protocol's `DISAMBIGUATE_ESCAPE_CODES`, when the
    /// terminal supports it: legacy terminal encoding reduces any `Ctrl+<key>` chord to the
    /// same control code as some other key (e.g. `Ctrl+;` and `Esc` are both `0x1B`) on a
    /// terminal that doesn't speak the enhanced protocol at all (a plain `xterm`, most Linux
    /// VTs, some multiplexer configurations). The command popup's own default
    /// `open_command_popup` binding is a bare `:` now (`docs/navigation-design.md`, converged onto
    /// from the old default `Ctrl+;` by "Wire `KeyBindingConfig` into `bin-tui`'s Shell/View
    /// global key handling", issue #160), so it no longer needs this -- but `quit`'s hardcoded
    /// `Ctrl+C` still benefits, and so would any user-remapped `Ctrl+<key>` binding via
    /// `KeyBindingConfig`. `supports_keyboard_enhancement` probes the terminal first so
    /// nothing is pushed where it wouldn't be understood.
    ///
    /// This doesn't request `REPORT_ALTERNATE_KEYS` -- nothing in `Shell`'s global key
    /// handling cares whether `Shift` was also held to produce a given character, so there's
    /// no need for the terminal to disambiguate that.
    pub fn new() -> crate::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let keyboard_enhancement = supports_keyboard_enhancement().unwrap_or(false);
        if keyboard_enhancement {
            execute!(
                stdout,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )?;
        }

        let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        terminal.backend_mut().clear_region(ClearType::All)?;
        Ok(Self {
            terminal,
            keyboard_enhancement,
        })
    }

    /// Draws one frame via the given closure.
    pub fn draw(
        &mut self,
        render: impl FnOnce(&mut ratatui::Frame),
    ) -> crate::Result<ratatui::CompletedFrame<'_>> {
        self.terminal.draw(render).map_err(Into::into)
    }

    /// Clears the terminal. Call this on a resize — the newly-revealed rows may still hold
    /// whatever the terminal emulator had there before, and ratatui's diffing has no way to
    /// know that without being told explicitly.
    pub fn clear(&mut self) -> crate::Result<()> {
        self.terminal.clear().map_err(Into::into)
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        // Best-effort: a failure here shouldn't panic during unwind (e.g. on a prior panic),
        // so errors are swallowed rather than propagated. Reverse setup order: pop the
        // keyboard flags (if pushed) while still in the alternate screen, then leave it.
        if self.keyboard_enhancement {
            let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
        }
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}
