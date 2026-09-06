//! `Shell` — owns terminal lifecycle, the async event loop, and hosts one active `View`
//! (ADR-0013, `docs/adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md`). Replaces the
//! breadcrumb-stack `App` (`app.rs`, left compiling but disconnected from `main.rs`) for the
//! shell chrome and dashboard being rebuilt against `docs/ux/shell/README.md`: a status line,
//! one full-bleed view region, and a keybind hint bar — no breadcrumb, no navigation stack.

use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
};
use tokio::sync::mpsc;

use crate::{
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
        }
    }

    /// Runs the shell until the user quits.
    pub async fn run(&mut self) -> std::io::Result<()> {
        let mut tui = Tui::new()?;
        let mut events = EventHandler::new(TICK_RATE);

        tui.draw(|frame| self.draw(frame))?;

        loop {
            let action = tokio::select! {
                event = events.next() => match event.and_then(|event| self.map_event(event)) {
                    Some(action) => action,
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

    /// Translates a raw terminal event into an [`Action`]: the active view gets first
    /// refusal via `handle_key` before falling back to `Ctrl+C`, the one truly-global key.
    fn map_event(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::Tick => Some(Action::Tick),
            Event::Key(key) => {
                if is_hard_quit(key) {
                    return Some(Action::Quit);
                }
                self.view.handle_key(key)
            }
        }
    }

    /// Applies an [`Action`] to shell state.
    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Tick => self.view.update(&action),
        }
    }

    /// Renders the three-row shell chrome — status line, full-bleed view region, keybind
    /// hint bar — around the active view, per `docs/ux/shell/README.md`.
    fn draw(&self, frame: &mut Frame) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        frame.render_widget(
            Paragraph::new(Line::from(format!(
                " 📒 Personal Ledger | {} ",
                self.view.title()
            )))
            .style(Style::default().add_modifier(Modifier::REVERSED)),
            rows[0],
        );

        self.view.view(frame, rows[1]);

        frame.render_widget(
            Paragraph::new(" : command · / search · ? help ")
                .style(Style::default().bg(Color::Rgb(211, 211, 211))),
            rows[2],
        );
    }
}

/// `Ctrl+C` — the one key that always quits immediately, regardless of the active view.
fn is_hard_quit(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c'))
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
}
