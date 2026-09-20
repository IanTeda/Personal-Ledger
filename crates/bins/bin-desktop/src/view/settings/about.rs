//! The **About** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state"): static
//! project info, no interactive elements beyond the links -- no `Shell` state or click handlers
//! to thread through, unlike every other real section built so far.
//!
//! The project facts are shared with the `?` help overlay (`crate::view::help::facts`). The
//! version string is `env!("CARGO_PKG_VERSION")`; the release date has no build-time source (no
//! build.rs stamps one), so it stays the mockup's own static value. "Built with" names this
//! binary's actual stack (Rust + GPUI + SQLite), not the mockup's copy-pasted web stack.

use gpui::{AnyElement, div, prelude::*, px};

use crate::{theme::color, view::help};

pub fn render() -> AnyElement {
    let version = div()
        .flex()
        .gap(px(4.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(format!("Personal Ledger v{}", env!("CARGO_PKG_VERSION"))),
        )
        .child("(08 September 2026)");
    let built_with = div()
        .text_color(color::INK_TERTIARY)
        .child("Built with Rust + GPUI + SQLite");
    let before_author = div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(version)
        .child(built_with)
        .into_any_element();

    div()
        .w_full()
        .child(help::facts(Some(before_author), None))
        .into_any_element()
}
