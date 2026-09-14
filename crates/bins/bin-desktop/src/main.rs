//! Personal Ledger desktop entry point. Boots into `Shell`, the persistent-rail, command-
//! palette-driven navigation chrome `docs/ux/desktop/README.md` specifies, in place of the
//! feasibility cycle's flat `TabBar` screen-cycling (ADR-0016). `feasibility_demo` is kept as
//! a workspace module so it still compiles -- see the ADR -- but is no longer referenced
//! here; its `gpui-component` chart-widget patterns are reused by the real Dashboard view as
//! that work lands (issue #148).

mod error;
mod feasibility_demo;
mod shell;
mod theme;

use std::borrow::Cow;

use clap::Parser;
use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};

pub use error::Error;
use shell::Shell;

/// Crate Result type alias used across the Desktop binary.
///
/// Use `crate::Result<T>` for functions that return `T` or a `crate::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// Archivo (400/600/800), bundled at compile time rather than fetched from Google Fonts at
/// runtime the way the handoff's own mockup does -- the client is local-first and must render
/// offline. See `crates/bins/bin-desktop/assets/fonts/OFL.txt` for the license.
const ARCHIVO_REGULAR: &[u8] = include_bytes!("../assets/fonts/Archivo-Regular.ttf");
const ARCHIVO_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Archivo-SemiBold.ttf");
const ARCHIVO_EXTRA_BOLD: &[u8] = include_bytes!("../assets/fonts/Archivo-ExtraBold.ttf");

/// Personal Ledger Desktop.
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    config: lib_config::ConfigArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = lib_config::Config::parse(cli.config.path.as_deref())?;
    let telemetry_level = Some(&config.telemetry_config().level());
    let log_file_path = config.telemetry_config().log_file_path();
    // Held for the lifetime of `main` -- dropping it stops the background worker that
    // flushes buffered log lines to `log_file_path` (when configured).
    let _log_guard = lib_tracing::init(telemetry_level, log_file_path)?;

    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);

        // Registered once, before any window opens, so every `Font { family: "Archivo".into(),
        // .. }` request resolves against the bundled weights rather than a fallback -- a
        // missing/corrupt bundled font is a build-time problem, not a recoverable one, hence
        // `expect` (matching `open_window`'s own `unwrap` below in this same closure).
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(ARCHIVO_REGULAR),
                Cow::Borrowed(ARCHIVO_SEMIBOLD),
                Cow::Borrowed(ARCHIVO_EXTRA_BOLD),
            ])
            .expect("bundled Archivo fonts must parse");

        // 1280x800 is the handoff's own window size (`docs/ux/desktop/Shell & Navigation/
        // README.md`, option 1a) -- kept here even though the chrome it describes isn't
        // built yet, so the window opens at the right size from the start.
        let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_window, cx| cx.new(|_cx| Shell::new()),
        )
        .unwrap();
        cx.activate(true);
    });

    Ok(())
}
