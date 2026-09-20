//! Personal Ledger desktop entry point. Boots into `Shell`, the persistent-rail, command-
//! palette-driven navigation chrome `docs/ux/desktop/README.md` specifies, in place of the
//! feasibility cycle's flat `TabBar` screen-cycling (ADR-0016). `feasibility_demo` is kept as
//! a workspace module so it still compiles -- see the ADR -- but is no longer referenced
//! here; its `gpui-component` chart-widget patterns are reused by the real Dashboard view as
//! that work lands (issue #148).

mod accounts;
mod assets;
mod categories;
mod command;
mod dialog;
mod error;
mod explorer;
mod feasibility_demo;
mod format;
mod icon;
mod key_router;
mod nav;
mod palette;
mod payees;
mod persistence;
mod rail;
mod select;
mod settings;
mod shell;
mod statusline;
mod tags;
mod theme;
mod topbar;
mod transaction_query;
mod transaction_rows;
mod transactions;
mod view;

use std::borrow::Cow;

use clap::Parser;
use gpui::{
    App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, point, prelude::*, px,
    size,
};

pub use error::Error;
use persistence::WindowGeometry;
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
    let mut config = lib_config::Config::parse(cli.config.path.as_deref())?;
    cli.config.apply_overrides(&mut config);
    let telemetry_level = Some(&config.personal_ledger_config().log());
    let log_file_path = config.personal_ledger_config().log_file_path();
    // Held for the lifetime of `main` -- dropping it stops the background worker that
    // flushes buffered log lines to `log_file_path` (when configured).
    let _log_guard = lib_tracing::init(telemetry_level, log_file_path)?;

    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
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

            // `noun`/`primary_rail`/window geometry survive restart (`docs/ux/desktop/README.md`'s
            // "State machine"); everything else in `NavState` starts fresh every launch, so there's
            // nothing else to seed here.
            let persisted = persistence::load();

            let mut nav = nav::NavState::new();
            nav.set_noun(persisted.noun);
            nav.set_primary_rail(persisted.primary_rail);

            // Restore the last saved window geometry; otherwise 1280x800 centered, the handoff's
            // own window size (`docs/ux/desktop/Shell & Navigation/README.md`, option 1a).
            let bounds = match persisted.window {
                Some(WindowGeometry {
                    x,
                    y,
                    width,
                    height,
                }) => Bounds {
                    origin: point(px(x), px(y)),
                    size: size(px(width), px(height)),
                },
                None => Bounds::centered(None, size(px(1280.0), px(800.0)), cx),
            };

            let window_handle = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        // Wayland app_id / X11 WM_CLASS -- without this the window reports an
                        // empty class, which leaves window-manager rules (workspace assignment,
                        // etc.) with nothing to match on. Mirrors the packager identifier in
                        // this crate's Cargo.toml (`[package.metadata.packager]`).
                        app_id: Some("au.id.teda.personal-ledger.desktop".into()),
                        titlebar: Some(TitlebarOptions {
                            title: Some("Personal Ledger".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    move |window, cx| {
                        let focus_handle = cx.focus_handle();
                        // Grabs keyboard focus for the shell's own key handling (issue #149)
                        // as soon as the window opens -- nothing else in the window competes
                        // for it yet.
                        window.focus(&focus_handle);
                        cx.new(|_cx| Shell::new(nav, focus_handle))
                    },
                )
                .expect("desktop window must open");

            // No action mutates `NavState` yet (the keybinding/rail-toggle tickets do), so this
            // currently ever only re-saves whatever `persistence::load` produced -- registered now
            // anyway so the mechanism is real and exercised, rather than added later as an
            // afterthought. `on_app_quit`'s `Subscription` must outlive this closure to stay
            // registered, hence `detach()` (mirroring `cx.spawn(..).detach()` elsewhere in this
            // crate) rather than binding and dropping it.
            cx.on_app_quit(move |cx| {
                let _ = window_handle.update(cx, |shell, window, _cx| {
                    let bounds = window.bounds();
                    let state = persistence::PersistedState {
                        noun: shell.nav().noun(),
                        primary_rail: shell.nav().primary_rail(),
                        window: Some(WindowGeometry {
                            x: f32::from(bounds.origin.x),
                            y: f32::from(bounds.origin.y),
                            width: f32::from(bounds.size.width),
                            height: f32::from(bounds.size.height),
                        }),
                    };
                    let _ = persistence::save(&state);
                });
                async {}
            })
            .detach();

            cx.activate(true);
        });

    Ok(())
}
