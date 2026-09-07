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
use ratatui::{Terminal, backend::CrosstermBackend};

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
    /// Also opts into the Kitty keyboard protocol's `DISAMBIGUATE_ESCAPE_CODES`, when the
    /// terminal supports it: legacy terminal encoding reduces `Ctrl+;` to the same control
    /// code as `Esc` (both are `0x1B`), which is exactly the combination the command palette
    /// (`Shell`) binds — without this, `Ctrl+;` is unreliable on terminals that don't speak
    /// the enhanced protocol at all (a plain `xterm`, most Linux VTs, some multiplexer
    /// configurations). `supports_keyboard_enhancement` probes the terminal first so nothing
    /// is pushed where it wouldn't be understood.
    ///
    /// This doesn't request `REPORT_ALTERNATE_KEYS` — `shell::is_open_palette` doesn't care
    /// whether `Shift` was also held (physically producing `:` rather than `;`), so there's
    /// no need for the terminal to disambiguate that.
    pub fn new() -> io::Result<Self> {
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
        terminal.clear()?;
        Ok(Self {
            terminal,
            keyboard_enhancement,
        })
    }

    /// Draws one frame via the given closure.
    pub fn draw(
        &mut self,
        render: impl FnOnce(&mut ratatui::Frame),
    ) -> io::Result<ratatui::CompletedFrame<'_>> {
        self.terminal.draw(render)
    }

    /// Clears the terminal. Call this on a resize — the newly-revealed rows may still hold
    /// whatever the terminal emulator had there before, and ratatui's diffing has no way to
    /// know that without being told explicitly.
    pub fn clear(&mut self) -> io::Result<()> {
        self.terminal.clear()
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
