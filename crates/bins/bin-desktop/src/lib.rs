//! Personal Ledger desktop library. Everything the `desktop` binary does lives here so the
//! integration tests under `tests/` can build `Shell` through the same [`build_shell`] the app
//! does; `main.rs` is only config, tracing and a call to [`run`]. Boots into `Shell`, the
//! persistent-rail, command-palette-driven navigation chrome `docs/ux/desktop/README.md`
//! specifies, in place of the feasibility cycle's flat `TabBar` screen-cycling (ADR-0016).
//! `feasibility_demo` is kept as a module so it still compiles -- see the ADR -- but is no
//! longer referenced here; its `gpui-component` chart-widget patterns are reused by the real
//! Dashboard view as that work lands (issue #148).

mod accounts;
mod assets;
mod bill_form;
mod bill_history;
mod bills;
mod budget_form;
mod budgets;
mod categories;
pub mod colours;
mod command;
mod dialog;
mod document_types;
mod documents;
mod documents_form;
mod documents_picker;
mod error;
mod explorer;
mod feasibility_demo;
mod format;
mod help;
mod icon;
mod import;
#[doc(hidden)]
pub mod key_router;
mod limit_form;
pub mod locale;
pub mod nav;
mod palette;
mod pay_form;
mod payees;
pub mod persistence;
mod rail;
mod select;
mod settings;
pub mod shell;
mod statusline;
mod tags;
mod theme;
mod toast;
mod topbar;
mod transaction_chips;
mod transaction_filter_form;
mod transaction_query;
mod transaction_rows;
mod transactions;
mod view;

/// This bin's own Messages, generated at build time from `i18n/<locale>/*.ftl`.
mod msg {
    include!(concat!(env!("OUT_DIR"), "/msg.rs"));
}

use std::borrow::Cow;

use gpui::{
    App, Application, Bounds, Context, FocusHandle, TitlebarOptions, WindowBounds, WindowOptions,
    point, prelude::*, px, size,
};

pub use error::Error;
use persistence::{PersistedState, WindowGeometry};
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

/// Keybinding overrides from config that `Shell` needs at construction.
pub struct ShellBindings {
    pub dismiss_toasts: String,
    pub toast_history: Option<String>,
}

/// Builds `Shell` from persisted state the way the app does, so tests construct it identically.
/// `today` is a parameter so tests can pin the date the seeded stub data and scopes hang off.
pub fn build_shell(
    persisted: PersistedState,
    bindings: ShellBindings,
    today: chrono::NaiveDate,
    focus_handle: FocusHandle,
    cx: &mut Context<'_, Shell>,
) -> Shell {
    let mut nav = nav::NavState::new();
    nav.set_noun(persisted.noun);
    nav.set_primary_rail(if persisted.start_sidebar_minimised {
        nav::RailMode::Collapsed
    } else {
        persisted.primary_rail
    });
    let documents_state = (
        documents::DocumentsMode::from_id(persisted.documents_mode.as_deref()),
        documents::LibraryScope::from_id(
            persisted.documents_scope.as_deref().unwrap_or("all"),
            today,
        ),
        documents::LibrarySort::from_id(persisted.documents_sort.as_deref()),
    );
    let settings_page = persisted
        .settings_page
        .as_deref()
        .map(settings::SettingsSection::from_id)
        .unwrap_or_default();

    let mut shell = Shell::with_today(nav, focus_handle, today);
    shell.set_start_sidebar_minimised(persisted.start_sidebar_minimised);
    shell.set_settings_page(settings_page);
    shell.set_explorer_filters(persisted.explorer_filters);
    shell.set_documents_state(documents_state.0, documents_state.1, documents_state.2);
    shell.set_dismiss_toasts_binding(bindings.dismiss_toasts);
    shell.set_toast_history_binding(bindings.toast_history);
    shell.start_toast_clock(cx);
    shell
}

/// Opens the main window and runs the app until it quits. Tracing is already initialised.
pub fn run(config: &lib_config::Config) {
    config.theme_config().warn_invalid();
    let theme_overrides = config.theme_config().overrides.clone();
    let dismiss_toasts_binding = config
        .keybindings_config()
        .key_for("dismiss_toasts")
        .unwrap_or(key_router::DEFAULT_DISMISS_TOASTS)
        .to_string();
    let toast_history_binding = config
        .keybindings_config()
        .key_for("toast_history")
        .map(str::to_string);

    // Resolved once, before any window opens; the Locale never changes at runtime.
    let (requested_locale, locale_source) = config.personal_ledger_config().resolved_locale();
    let locale = locale::init(requested_locale, locale_source);

    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            gpui_component::set_locale(locale::gpui_component_tag(locale));
            colours::init(theme_overrides, cx);

            // Registered once, before any window opens, so every `Font { family: "Archivo".into(),
            // .. }` request resolves against the bundled weights rather than a fallback.
            // A failure here only costs the bundled face -- text still renders in gpui's
            // fallback font -- so it's reported rather than stopping the app.
            if let Err(error) = cx.text_system().add_fonts(vec![
                Cow::Borrowed(ARCHIVO_REGULAR),
                Cow::Borrowed(ARCHIVO_SEMIBOLD),
                Cow::Borrowed(ARCHIVO_EXTRA_BOLD),
            ]) {
                tracing::error!(%error, "Failed to register the bundled Archivo fonts");
            }

            // `noun`/`primary_rail`/window geometry survive restart (`docs/ux/desktop/README.md`'s
            // "State machine"); everything else in `NavState` starts fresh every launch, so there's
            // nothing else to seed here.
            let persisted = persistence::load();
            let window_geometry = persisted.window;

            // Restore the last saved window geometry; otherwise 1280x800 centered, the handoff's
            // own window size (`docs/ux/desktop/01-shell/README.md`, option 1a).
            let bounds = match window_geometry {
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

            let opened = cx.open_window(
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
                    // System follows the OS light/dark setting live. Held for the window's
                    // life, which is the app's, hence `detach()`.
                    window
                        .observe_window_appearance(|window, cx| {
                            colours::set_system(window.appearance(), cx);
                        })
                        .detach();
                    cx.new(|cx| {
                        build_shell(
                            persisted,
                            ShellBindings {
                                dismiss_toasts: dismiss_toasts_binding,
                                toast_history: toast_history_binding,
                            },
                            chrono::Local::now().date_naive(),
                            focus_handle,
                            cx,
                        )
                    })
                },
            );
            // Without a window there is nothing to run, so report and quit rather than panic.
            let window_handle = match opened {
                Ok(handle) => handle,
                Err(error) => {
                    tracing::error!(%error, "Failed to open the main window");
                    cx.quit();
                    return;
                }
            };

            // No action mutates `NavState` yet (the keybinding/rail-toggle tickets do), so this
            // currently ever only re-saves whatever `persistence::load` produced -- registered now
            // anyway so the mechanism is real and exercised, rather than added later as an
            // afterthought. `on_app_quit`'s `Subscription` must outlive this closure to stay
            // registered, hence `detach()` (mirroring `cx.spawn(..).detach()` elsewhere in this
            // crate) rather than binding and dropping it.
            cx.on_app_quit(move |cx| {
                let _ = window_handle.update(cx, |shell, window, _cx| {
                    let bounds = window.bounds();
                    let state = PersistedState {
                        noun: shell.nav().noun(),
                        primary_rail: shell.nav().primary_rail(),
                        start_sidebar_minimised: shell.start_sidebar_minimised(),
                        settings_page: Some(shell.settings_page().id().to_string()),
                        explorer_filters: shell.explorer_filters(),
                        documents_mode: Some(shell.documents_persisted().0.id().to_string()),
                        documents_scope: Some(shell.documents_persisted().1.id()),
                        documents_sort: Some(shell.documents_persisted().2.id().to_string()),
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
}
