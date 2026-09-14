//! `Shell` -- the desktop window's root render tree (`docs/ux/desktop/README.md`'s
//! "Component tree"), replacing `feasibility_demo::DesktopApp`'s `TabBar`-driven screen
//! cycling as the real navigation entry point (ADR-0016).
//!
//! Assembles the static chrome from `docs/ux/desktop/Shell & Navigation/README.md`'s "1a"
//! spec (issue #148): top bar / content row (primary rail, context rail, view) / status
//! line, driven by `NavState`. No interaction yet -- focus cycling, `g`-jumps, mode
//! transitions, and the command palette are separate build tickets (#149-#151) still to
//! land on the [Desktop Shell & Navigation](https://github.com/IanTeda/Personal-Ledger/issues/144)
//! map. A `View` trait mirroring `bin-tui`'s is still deliberately deferred (see ADR-0016):
//! `Dashboard` is the only real view, and `render_view` below is a plain match rather than a
//! trait object because there's still only one concrete implementor to dispatch to.

use gpui::{Context, Window, div, prelude::*};

use crate::{
    nav::{NavState, Noun},
    rail::{context::ContextRail, primary::PrimaryRail},
    statusline::StatusLine,
    theme::{color, type_scale},
    topbar::TopBar,
    view::dashboard::Dashboard,
};

/// Owns the shell's render tree and the live `NavState`.
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
            .flex_col()
            .bg(color::GROUND)
            .text_color(color::INK)
            .font_family(type_scale::FONT_FAMILY)
            .text_size(type_scale::BODY)
            .child(TopBar::new())
            .child(
                div()
                    .flex_1()
                    .min_h(gpui::px(0.0))
                    .flex()
                    .child(PrimaryRail::new(self.nav.noun()))
                    .when(self.nav.noun().has_context_entities(), |this| {
                        this.child(ContextRail::new(self.nav.noun()))
                    })
                    .child(render_view(self.nav.noun())),
            )
            .child(StatusLine::new(self.nav.mode(), self.nav.noun()))
    }
}

/// The active noun's own view interior. Only `Dashboard` is real; every other noun is a
/// placeholder until its own view lands (issue #153).
fn render_view(noun: Noun) -> gpui::AnyElement {
    match noun {
        Noun::Dashboard => Dashboard::new().into_any_element(),
        other => div()
            .flex_1()
            .min_w(gpui::px(0.0))
            .p(gpui::px(24.0))
            .text_color(color::INK_TERTIARY)
            .child(format!("{other:?} -- not yet built (see issue #153)"))
            .into_any_element(),
    }
}
