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

use crate::{
    nav::NavState,
    theme::{color, type_scale},
};

/// Owns the shell's render tree and the live `NavState`. The chrome modules that actually
/// read `nav` (the rails, the status line) are the rest of the map's tickets; today it's
/// carried through unused by rendering but already reachable for restart persistence (see
/// `crate::main`'s `on_app_quit` hook).
pub struct Shell {
    nav: NavState,
}

impl Shell {
    pub fn new(nav: NavState) -> Self {
        Self { nav }
    }

    pub fn nav(&self) -> &NavState {
        &self.nav
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(color::GROUND)
            .text_color(color::INK)
            .font_family(type_scale::FONT_FAMILY)
            .text_size(type_scale::BODY)
            .child("Personal Ledger -- desktop shell scaffold (see docs/ux/desktop/README.md)")
    }
}
