//! Terminal setup and teardown.
//!
//! Wraps a [`ratatui::Terminal`] on a [`CrosstermBackend`], entering raw mode and the
//! alternate screen on construction and restoring the terminal on [`Drop`] — including on a
//! panic, so a crash never leaves the user's shell in raw mode.

use std::io::{self, Stdout};

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

/// The terminal type this crate renders to: `crossterm` on `stdout`.
pub type Backend = CrosstermBackend<Stdout>;

/// Owns the terminal for the lifetime of the app, restoring it on drop.
pub struct Tui {
    terminal: Terminal<Backend>,
}

impl Tui {
    /// Enters raw mode and the alternate screen, ready to draw.
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(Self { terminal })
    }

    /// Draws one frame via the given closure.
    pub fn draw(
        &mut self,
        render: impl FnOnce(&mut ratatui::Frame),
    ) -> io::Result<ratatui::CompletedFrame<'_>> {
        self.terminal.draw(render)
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        // Best-effort: a failure here shouldn't panic during unwind (e.g. on a prior panic),
        // so errors are swallowed rather than propagated.
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}
