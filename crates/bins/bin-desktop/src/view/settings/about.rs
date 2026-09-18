//! The **About** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state"): static
//! version info, no interactive elements -- no `Shell` state or click handlers to thread through,
//! unlike every other real section built so far.
//!
//! The version string is `env!("CARGO_PKG_VERSION")`, not a hardcoded copy of it -- the ticket's
//! own body: a fact already available at compile time isn't dummy data to invent. The release
//! date has no equivalent build-time source (no build.rs stamps one), so it stays the mockup's
//! own static value. "Built with Rust + SQLite + React" in the raw mockup markup is a copy-paste
//! artifact from a hypothetical web frontend elsewhere in this handoff's own docs -- corrected to
//! this binary's actual stack (Rust + GPUI + SQLite, via `lib_database`) rather than repeated
//! verbatim, since a static info panel that's actively wrong about its own stack defeats the
//! point of the section.

use gpui::{AnyElement, SharedString, div, prelude::*, px};

use crate::theme::color;

const COLUMN_WIDTH: gpui::Pixels = px(300.0);

pub fn render() -> AnyElement {
    div()
        .w(COLUMN_WIDTH)
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .text_size(px(12.0))
        .child(fact_row("personal-ledger", env!("CARGO_PKG_VERSION")))
        .child(fact_row("released", "08 sep 2026"))
        .child(
            div()
                .text_color(color::INK_TERTIARY)
                .child("Built with Rust + GPUI + SQLite"),
        )
        .into_any_element()
}

fn fact_row(label: &'static str, value: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .gap(px(4.0))
        .child(div().text_color(color::INK_SECONDARY).child(label))
        .child("\u{2014}")
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(value.into()),
        )
}
