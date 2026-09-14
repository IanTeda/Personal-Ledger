//! `Shell` -- the desktop window's root render tree (`docs/ux/desktop/README.md`'s
//! "Component tree"), replacing `feasibility_demo::DesktopApp`'s `TabBar`-driven screen
//! cycling as the real navigation entry point (ADR-0016).
//!
//! This is the skeleton only: `NavState` (the primary/context rail state machine, issue
//! #147), the design tokens (issue #146), the top bar, both rails, the status line, and the
//! command palette are separate build tickets on the
//! [Desktop Shell & Navigation](https://github.com/IanTeda/Personal-Ledger/issues/144) map.
//! `Shell` renders a single placeholder in the meantime so the crate has a real, running
//! entry point to build the rest of the chrome against, rather than introducing a `View`
//! trait now for the one concrete view that exists so far -- see ADR-0016's note on why that
//! split waits for `NavState`.

use gpui::{Context, Window, div, prelude::*};
use gpui_component::ActiveTheme;

/// Owns the shell's render tree. Empty today; gains `NavState` and the chrome modules as the
/// rest of the map's tickets land.
pub struct Shell;

impl Shell {
    pub fn new() -> Self {
        Self
    }
}

impl Render for Shell {
    // `cx.theme()`'s background/foreground are a placeholder only -- a plain `div` has no
    // default fill or text color, so without them the window renders blank. The fixed
    // Modernist palette (`docs/ux/desktop/README.md`'s "Design tokens") replaces this once
    // issue #146 lands.
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child("Personal Ledger -- desktop shell scaffold (see docs/ux/desktop/README.md)")
    }
}
