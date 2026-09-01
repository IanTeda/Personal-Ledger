//! Async event source, multiplexing a periodic tick and crossterm input onto one channel —
//! the `tokio::select!`-based pattern ADR-0003 locks in
//! (`docs/adr/0003-hybrid-tea-component-tui-architecture.md`).

use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent, KeyEventKind};
use futures_util::StreamExt;
use tokio::sync::mpsc;

/// Something the application's event loop reacts to.
#[derive(Debug, Clone)]
pub enum Event {
    /// A periodic tick, independent of any input.
    Tick,
    /// A key was pressed (crossterm also reports releases on some backends; those are
    /// filtered out before this variant is produced).
    Key(KeyEvent),
}

/// Runs a background task that multiplexes a tick interval and crossterm's input stream,
/// forwarding both onto a channel the application reads from.
pub struct EventHandler {
    receiver: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    /// Spawns the background polling task and returns a handle to receive its events.
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        tokio::spawn(Self::run(tick_rate, sender));
        Self { receiver }
    }

    async fn run(tick_rate: Duration, sender: mpsc::UnboundedSender<Event>) {
        let mut reader = EventStream::new();
        let mut tick = tokio::time::interval(tick_rate);
        loop {
            let tick_delay = tick.tick();
            let crossterm_event = reader.next();
            tokio::select! {
                _ = tick_delay => {
                    if sender.send(Event::Tick).is_err() {
                        break;
                    }
                }
                maybe_event = crossterm_event => {
                    match maybe_event {
                        Some(Ok(CrosstermEvent::Key(key))) if key.kind == KeyEventKind::Press => {
                            if sender.send(Event::Key(key)).is_err() {
                                break;
                            }
                        }
                        Some(Ok(_)) | None => {}
                        // The input stream errored; nothing more will arrive, so stop.
                        Some(Err(_)) => break,
                    }
                }
            }
        }
    }

    /// Awaits the next event from the background task.
    pub async fn next(&mut self) -> Option<Event> {
        self.receiver.recv().await
    }
}
