//! The top-level application: owns terminal lifecycle, the async event loop, and which screen
//! is active — the "Elm" half of ADR-0003's hybrid architecture
//! (`docs/adr/0003-hybrid-tea-component-tui-architecture.md`).

use std::time::Duration;

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use tokio::sync::mpsc;

use crate::{
    action::Action,
    event::{Event, EventHandler},
    screen::{
        Screen, candlestick_chart::CandlestickChartScreen, categories::CategoriesScreen,
        divergent_chart::DivergentChartScreen, doughnut_chart::DoughnutChartScreen,
        line_chart::LineChartScreen, table::TableScreen,
    },
    tui::Tui,
};

/// How often an [`Action::Tick`] fires in the absence of input.
const TICK_RATE: Duration = Duration::from_millis(250);

/// Owns terminal lifecycle and the set of demo screens, and drives the async event loop.
pub struct App {
    screens: Vec<Box<dyn Screen>>,
    current: usize,
    should_quit: bool,
    /// Where background tasks spawned by a screen's `init()` (e.g. the SQLite feasibility
    /// demo's load) report their results back as an [`Action`].
    action_rx: mpsc::UnboundedReceiver<Action>,
}

impl App {
    /// Creates the app with every demo screen registered, starting on the first, and gives
    /// each screen a chance to kick off any background work via `init()`.
    pub fn new() -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let mut screens: Vec<Box<dyn Screen>> = vec![
            Box::new(LineChartScreen::new()),
            Box::new(DoughnutChartScreen::new()),
            Box::new(CandlestickChartScreen::new()),
            Box::new(DivergentChartScreen::new()),
            Box::new(TableScreen::new()),
            Box::new(CategoriesScreen::new()),
        ];
        for screen in &mut screens {
            screen.init(action_tx.clone());
        }

        Self {
            screens,
            current: 0,
            should_quit: false,
            action_rx,
        }
    }

    /// Runs the app until the user quits.
    pub async fn run(&mut self) -> std::io::Result<()> {
        let mut tui = Tui::new()?;
        let mut events = EventHandler::new(TICK_RATE);

        tui.draw(|frame| self.draw(frame))?;

        loop {
            let action = tokio::select! {
                event = events.next() => match event.and_then(Self::map_event) {
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

    /// Translates a raw terminal event into an [`Action`], if any.
    fn map_event(event: Event) -> Option<Action> {
        match event {
            Event::Tick => Some(Action::Tick),
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
                KeyCode::Tab => Some(Action::NextScreen),
                KeyCode::BackTab => Some(Action::PrevScreen),
                _ => None,
            },
        }
    }

    /// Applies an [`Action`] to application and screen state.
    fn update(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::NextScreen => self.current = (self.current + 1) % self.screens.len(),
            Action::PrevScreen => {
                self.current = (self.current + self.screens.len() - 1) % self.screens.len();
            }
            Action::Tick => self.screens[self.current].update(&action),
            // A background load can finish while any screen is active, and other screens
            // ignore it via their `update`'s default `_ => {}` arm.
            Action::CategoriesLoaded(_) | Action::CategoriesLoadFailed(_) => {
                for screen in &mut self.screens {
                    screen.update(&action);
                }
            }
        }
    }

    /// Renders the tab bar and the active screen.
    fn draw(&self, frame: &mut Frame) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(frame.area());

        let tabs: Vec<Span> = self
            .screens
            .iter()
            .enumerate()
            .flat_map(|(index, screen)| {
                let style = if index == self.current {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                [
                    Span::styled(format!(" {}: {} ", index + 1, screen.title()), style),
                    Span::raw(" "),
                ]
            })
            .chain(std::iter::once(Span::raw(
                "— Tab/Shift+Tab: switch, q: quit",
            )))
            .collect();
        frame.render_widget(Paragraph::new(Line::from(tabs)), rows[0]);

        self.screens[self.current].view(frame, rows[1]);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    // `App::new()` calls each screen's `init()`, and `CategoriesScreen::init()` spawns a
    // background task via `tokio::spawn` — that needs an active runtime, hence `tokio::test`
    // rather than a plain `#[test]`.
    #[tokio::test]
    async fn renders_every_screen_without_panicking() {
        let mut app = App::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        for index in 0..app.screens.len() {
            app.current = index;
            terminal
                .draw(|frame| app.draw(frame))
                .expect("drawing the tab bar and active screen should not error");
        }
    }
}
