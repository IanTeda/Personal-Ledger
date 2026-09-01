//! The top-level application: owns terminal lifecycle, the async event loop, and the
//! currently-active screen — the "Elm" half of ADR-0003's hybrid architecture
//! (`docs/adr/0003-hybrid-tea-component-tui-architecture.md`).

use std::time::Duration;

use crossterm::event::KeyCode;

use crate::{
    action::Action,
    event::{Event, EventHandler},
    screen::{Screen, line_chart::LineChartScreen},
    tui::Tui,
};

/// How often an [`Action::Tick`] fires in the absence of input.
const TICK_RATE: Duration = Duration::from_millis(250);

/// Owns terminal lifecycle and the currently-active screen, and drives the async event loop.
pub struct App {
    screen: Box<dyn Screen>,
    should_quit: bool,
}

impl App {
    /// Creates the app, starting on the line chart demo screen.
    pub fn new() -> Self {
        Self {
            screen: Box::new(LineChartScreen::new()),
            should_quit: false,
        }
    }

    /// Runs the app until the user quits.
    pub async fn run(&mut self) -> std::io::Result<()> {
        let mut tui = Tui::new()?;
        let mut events = EventHandler::new(TICK_RATE);

        tui.draw(|frame| self.screen.view(frame, frame.area()))?;

        while let Some(event) = events.next().await {
            if let Some(action) = Self::map_event(event) {
                self.update(action);
            }
            if self.should_quit {
                break;
            }
            tui.draw(|frame| self.screen.view(frame, frame.area()))?;
        }

        Ok(())
    }

    /// Translates a raw terminal event into an [`Action`], if any.
    fn map_event(event: Event) -> Option<Action> {
        match event {
            Event::Tick => Some(Action::Tick),
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
                _ => None,
            },
        }
    }

    /// Applies an [`Action`] to application and screen state.
    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Tick => self.screen.update(&action),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
