//! `Shell` — owns terminal lifecycle, the async event loop, and hosts one active `View`
//! (ADR-0013, `docs/adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md`). Replaces the
//! breadcrumb-stack `App` (`app.rs`, left compiling but disconnected from `main.rs`) for the
//! shell chrome and dashboard being rebuilt against `docs/ux/tui/README.md`: a status line,
//! one full-bleed view region, and a keybind hint bar — no breadcrumb, no navigation stack.

use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tokio::sync::mpsc;

use crate::{
    command_palette::{CommandPalette, Dim},
    event::{Event, EventHandler},
    tui::Tui,
    view::{Action, View, dashboard::DashboardView},
};

/// How often an [`Action::Tick`] fires in the absence of input.
const TICK_RATE: Duration = Duration::from_millis(250);

/// Owns terminal lifecycle and the single active `View`, and drives the async event loop.
pub struct Shell {
    view: Box<dyn View>,
    should_quit: bool,
    /// Where a view's `init()` (e.g. a background load) reports results back as an
    /// [`Action`].
    action_rx: mpsc::UnboundedReceiver<Action>,
    /// The command palette overlay (`docs/ux/tui/README.md` §3a) — `Some` while open. Owned
    /// here rather than by the active `View`: it floats over whatever view is on screen and
    /// intercepts keys before the view sees them, per `view/mod.rs`'s "shell's own command
    /// window" note.
    command_palette: Option<CommandPalette>,
}

impl Shell {
    /// Creates the shell with the placeholder Dashboard as its active view.
    pub fn new() -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let mut view: Box<dyn View> = Box::new(DashboardView::new());
        view.init(action_tx);

        Self {
            view,
            should_quit: false,
            action_rx,
            command_palette: None,
        }
    }

    /// Runs the shell until the user quits.
    pub async fn run(&mut self) -> std::io::Result<()> {
        let mut tui = Tui::new()?;
        let mut events = EventHandler::new(TICK_RATE);

        tui.draw(|frame| self.draw(frame))?;

        loop {
            let action = tokio::select! {
                event = events.next() => match event {
                    // A resize can reveal rows the terminal emulator never had ratatui-drawn
                    // content in; clear before the next draw rather than risk stray artifacts.
                    Some(Event::Resize) => {
                        tui.clear()?;
                        continue;
                    }
                    Some(event) => match self.map_event(event) {
                        Some(action) => action,
                        None => continue,
                    },
                    None => continue,
                },
                Some(action) = self.action_rx.recv() => action,
            };
            self.update(action);
            if self.should_quit {
                break;
            }
            tui.draw(|frame| self.draw(frame))?;
        }

        Ok(())
    }

    /// Translates a raw terminal event into an [`Action`]. Precedence: `Ctrl+C` always quits;
    /// then, while the command palette is open, it takes every other key over the active
    /// view (per §3a, the view behind it is inert while it's up); otherwise `Ctrl+;` opens
    /// the palette, and anything left falls to the active view's own `handle_key`.
    /// `Event::Resize` never reaches here — `run` intercepts it directly to clear the
    /// terminal, since that's a `Tui`-level concern with no `Action` of its own.
    fn map_event(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::Tick => Some(Action::Tick),
            Event::Key(key) => {
                if is_hard_quit(key) {
                    return Some(Action::Quit);
                }
                if self.command_palette.is_some() {
                    return map_palette_key(key);
                }
                if is_open_palette(key) {
                    return Some(Action::OpenPalette);
                }
                self.view.handle_key(key)
            }
            Event::Resize => None,
        }
    }

    /// Applies an [`Action`] to shell state.
    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Tick => self.view.update(&action),
            Action::OpenPalette => self.command_palette = Some(CommandPalette::new()),
            Action::ClosePalette => self.command_palette = None,
            Action::PaletteInput(c) => {
                if let Some(palette) = &mut self.command_palette {
                    palette.push_char(c);
                }
            }
            Action::PaletteBackspace => {
                if let Some(palette) = &mut self.command_palette {
                    palette.backspace();
                }
            }
            Action::PaletteMoveUp => {
                if let Some(palette) = &mut self.command_palette {
                    palette.move_up();
                }
            }
            Action::PaletteMoveDown => {
                if let Some(palette) = &mut self.command_palette {
                    palette.move_down();
                }
            }
        }
    }

    /// Renders the three-row shell chrome — status line, full-bleed view region, keybind
    /// hint bar — around the active view, per `docs/ux/tui/README.md`.
    fn draw(&self, frame: &mut Frame) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        let palette_open = self.command_palette.is_some();

        // Header Frame — the status line names the mode whenever it isn't the resting
        // NORMAL state, per `docs/ux/tui/README.md`'s "show the mode ... whenever it is not
        // NORMAL".
        let mode = if palette_open { " · COMMAND" } else { "" };
        frame.render_widget(
            Paragraph::new(Line::from(format!(
                " 📒 Personal Ledger | {}{mode} ",
                self.view.title()
            )))
            .style(Style::default().add_modifier(Modifier::REVERSED)),
            rows[0],
        );

        // Screen Frame / View
        self.view.view(frame, rows[1]);

        // Footer Frame — each keybind's key is bolded to stand out from its label. While the
        // palette is open the whole bar greys out and gains its own close hint, per §3a.
        let footer = if palette_open {
            Line::from(" : command · / search · ? help · esc close command window ")
                .style(Style::default().fg(Color::DarkGray))
        } else {
            let key = Style::default().add_modifier(Modifier::BOLD);
            Line::from(vec![
                Span::raw(" "),
                Span::styled(":", key),
                Span::raw(" command · "),
                Span::styled("/", key),
                Span::raw(" search · "),
                Span::styled("?", key),
                Span::raw(" help "),
            ])
        };
        frame.render_widget(
            Paragraph::new(footer).style(Style::default().bg(Color::Rgb(211, 211, 211))),
            rows[2],
        );

        // Command palette overlay — dims the view behind it (never hides it) and floats over
        // the whole frame, per §3a.
        if let Some(palette) = &self.command_palette {
            frame.render_widget(Dim, rows[1]);
            palette.render(frame, frame.area());
        }
    }
}

/// `Ctrl+C` — the one key that always quits immediately, regardless of the active view.
fn is_hard_quit(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c'))
}

/// `Ctrl+;` — opens the command palette from anywhere, shift optional. `Tui` requests
/// `DISAMBIGUATE_ESCAPE_CODES` so a Kitty-protocol terminal reports the unshifted key as
/// `Char(';')` regardless of whether `Shift` is also held (physically producing `:`); `':'`
/// is matched too as a defensive fallback for a terminal or layout that reports the shifted
/// symbol instead. Either way `Shift`'s presence is ignored.
fn is_open_palette(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char(';') | KeyCode::Char(':'))
}

/// Routes a key while the command palette is open. `Ctrl+;` toggles it shut again; `Esc`
/// closes it; typing, `Backspace` and `↑`/`↓` drive the input buffer and selection. Any other
/// key (e.g. `Enter`, `Tab` — running a command and completion are the action-registry's
/// "later ticket", per `view/mod.rs`) is swallowed without effect, since the palette owns
/// every key while it's up.
fn map_palette_key(key: KeyEvent) -> Option<Action> {
    if is_open_palette(key) {
        return Some(Action::ClosePalette);
    }
    match key.code {
        KeyCode::Esc => Some(Action::ClosePalette),
        KeyCode::Up => Some(Action::PaletteMoveUp),
        KeyCode::Down => Some(Action::PaletteMoveDown),
        KeyCode::Backspace => Some(Action::PaletteBackspace),
        KeyCode::Char(c) => Some(Action::PaletteInput(c)),
        _ => None,
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_the_three_row_shell_layout_without_panicking() {
        let shell = Shell::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell should not error");
    }

    #[test]
    fn ctrl_c_is_recognised_as_a_hard_quit() {
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(is_hard_quit(ctrl_c));

        let plain_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        assert!(!is_hard_quit(plain_c));
    }

    #[test]
    fn ctrl_c_quits_the_shell() {
        let mut shell = Shell::new();
        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+c always maps to an action");
        shell.update(action);
        assert!(shell.should_quit);
    }

    #[test]
    fn tick_reaches_the_active_view() {
        let mut shell = Shell::new();
        let action = shell
            .map_event(Event::Tick)
            .expect("a tick always maps to an action");
        assert_eq!(action, Action::Tick);
        shell.update(action);
        assert!(!shell.should_quit);
    }

    #[test]
    fn resize_maps_to_no_action() {
        // `run` intercepts `Event::Resize` directly to clear the terminal, before it would
        // ever reach `map_event` — this just documents that `map_event` itself treats it as
        // unmapped, keeping the match exhaustive without inventing a `Resize` action.
        let mut shell = Shell::new();
        assert_eq!(shell.map_event(Event::Resize), None);
    }

    #[test]
    fn ctrl_semicolon_is_recognised_as_open_palette() {
        let ctrl_semicolon = KeyEvent::new(KeyCode::Char(';'), KeyModifiers::CONTROL);
        assert!(is_open_palette(ctrl_semicolon));

        let plain_semicolon = KeyEvent::new(KeyCode::Char(';'), KeyModifiers::NONE);
        assert!(!is_open_palette(plain_semicolon));
    }

    #[test]
    fn ctrl_semicolon_with_shift_held_is_also_recognised_as_open_palette() {
        // Whether `Shift` is also held (physically producing `:` rather than `;`) doesn't
        // matter — see `is_open_palette`'s doc.
        let ctrl_shift_semicolon = KeyEvent::new(
            KeyCode::Char(';'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert!(is_open_palette(ctrl_shift_semicolon));

        let ctrl_colon = KeyEvent::new(KeyCode::Char(':'), KeyModifiers::CONTROL);
        assert!(is_open_palette(ctrl_colon));
    }

    #[test]
    fn ctrl_colon_opens_the_command_palette() {
        let mut shell = Shell::new();
        assert!(shell.command_palette.is_none());

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char(':'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+: always maps to an action");
        shell.update(action);

        assert!(shell.command_palette.is_some());
    }

    #[test]
    fn ctrl_colon_again_closes_an_open_palette() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPalette);

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char(':'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+: while open always maps to an action");
        shell.update(action);

        assert!(shell.command_palette.is_none());
    }

    #[test]
    fn esc_closes_an_open_palette() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPalette);

        let action = shell
            .map_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)))
            .expect("esc while open always maps to an action");
        shell.update(action);

        assert!(shell.command_palette.is_none());
    }

    #[test]
    fn typing_while_the_palette_is_open_never_reaches_the_view() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPalette);

        let action = shell.map_event(Event::Key(KeyEvent::new(
            KeyCode::Char('b'),
            KeyModifiers::NONE,
        )));
        assert_eq!(action, Some(Action::PaletteInput('b')));
    }

    #[test]
    fn ctrl_c_still_quits_while_the_palette_is_open() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPalette);

        let action = shell
            .map_event(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            )))
            .expect("ctrl+c always maps to an action");
        shell.update(action);

        assert!(shell.should_quit);
    }

    #[test]
    fn renders_the_open_command_palette_without_panicking() {
        let mut shell = Shell::new();
        shell.update(Action::OpenPalette);

        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| shell.draw(frame))
            .expect("drawing the shell with the palette open should not error");
    }
}
