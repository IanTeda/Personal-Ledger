//! The full-window drop overlay: while files are dragged over the window, a dashed frame says
//! that dropping adds them to the Inbox, on every destination. Purely presentational -- the drop
//! itself is `Shell::drop_documents`.
//!
//! gpui raises its active drag on `FileDropEvent::Entered` and clears it on `Exited`, and applies
//! `group_drag_over` styles at paint time, so the overlay needs no state of its own and no
//! re-render: it is always laid out, transparent, and turns opaque while an `ExternalPaths` drag
//! is over the group. It never occludes the mouse, so it cannot steal the drop from the root.

use gpui::{AnyElement, App, ExternalPaths, div, prelude::*, px};

use crate::theme::{color, type_scale};

/// The group name the Shell root declares with `.group(..)` so the overlay can key its style off it.
pub const GROUP: &str = "shell-drop-group";

/// The dashed overlay, to be appended as the last child of the `GROUP` root.
pub fn render(cx: &App) -> AnyElement {
    div()
        .id("documents-drop-overlay")
        .absolute()
        .inset_0()
        .opacity(0.0)
        .group_drag_over::<ExternalPaths>(GROUP, |style| style.opacity(1.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(color::scrim(cx))
        .child(
            div()
                .m(px(16.0))
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .border(px(2.0))
                .border_dashed()
                .border_color(color::foreground(cx))
                .text_size(type_scale::BODY)
                .text_color(color::foreground(cx))
                .child(crate::msg::desktop_documents_drop_overlay()),
        )
        .into_any_element()
}
