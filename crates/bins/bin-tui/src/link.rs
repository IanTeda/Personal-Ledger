//! What the TUI hands to the world outside the terminal: a URL to the browser and text to the
//! clipboard (Settings › About's links, Settings › Logs' lines). `Shell` holds one [`Outside`],
//! the [`System`] one unless a test swaps in its own, so no test opens a browser or touches the
//! clipboard.

use std::io::{self, Write as _};
use std::process::{Command, Stdio};

use base64::Engine as _;

/// The browser and the clipboard, behind one seam.
pub trait Outside {
    /// Opens `url` in the default browser.
    fn open(&self, url: &str) -> io::Result<()>;
    /// Puts `text` on the clipboard.
    fn copy(&self, text: &str) -> io::Result<()>;
}

/// The real browser and clipboard.
#[derive(Debug, Default)]
pub struct System;

impl Outside for System {
    /// Hands `url` to the platform's opener, detached and silenced: the TUI owns the terminal, so
    /// the opener's own output must not land on it.
    fn open(&self, url: &str) -> io::Result<()> {
        let mut command = if cfg!(target_os = "macos") {
            Command::new("open")
        } else if cfg!(target_os = "windows") {
            let mut command = Command::new("cmd");
            command.args(["/C", "start", ""]);
            command
        } else {
            Command::new("xdg-open")
        };
        command
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(drop)
    }

    /// Asks the terminal to set the clipboard (OSC 52), which works over SSH and needs no
    /// clipboard library. A terminal that doesn't support it ignores the sequence.
    fn copy(&self, text: &str) -> io::Result<()> {
        let mut stdout = io::stdout();
        stdout.write_all(osc52(text).as_bytes())?;
        stdout.flush()
    }
}

/// The OSC 52 sequence that sets the clipboard to `text`.
fn osc52(text: &str) -> String {
    format!(
        "\x1b]52;c;{}\x07",
        base64::engine::general_purpose::STANDARD.encode(text)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc52_wraps_the_text_base64_encoded() {
        assert_eq!(osc52("hi"), "\x1b]52;c;aGk=\x07");
    }
}
