//! TUI feasibility demos entry point (see the "TUI App feasibility" Wayfinder map, GitHub
//! issue #7). Currently shows the line chart demo (FC-TUI-002); later tickets add more
//! screens to this same binary. Press `q` or `Esc` to quit.

mod action;
mod app;
mod event;
mod screen;
mod tui;

use app::App;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    App::new().run().await
}
