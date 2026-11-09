//! Draws the **About** section (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state"): static
//! project info, no interactive elements beyond the links -- no `Root` state or click handlers
//! to thread through, unlike every other real section built so far.
//!
//! The project facts are shared with the `?` help overlay (`crate::view::help::facts`). The
//! version string is `env!("CARGO_PKG_VERSION")`; the release date has no build-time source (no
//! build.rs stamps one), so it stays the mockup's own static value. "Built with" names this
//! binary's actual stack (Rust + GPUI + SQLite), not the mockup's copy-pasted web stack.

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_settings::AboutViewModel;

use crate::{theme::color, view::help};

/// The page drawn from `model`.
pub(super) fn page(model: &AboutViewModel, cx: &App) -> AnyElement {
    let model = model.clone();
    let version = div()
        .flex()
        .gap(px(4.0))
        .debug_selector(|| "settings-about-version".to_string())
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(model.version),
        )
        .child(model.released);
    let built_with = div()
        .text_color(color::faint_text(cx))
        .child(model.built_with);
    let before_author = div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(version)
        .child(built_with)
        .into_any_element();

    div()
        .w_full()
        .child(help::facts(Some(before_author), None, cx))
        .into_any_element()
}
