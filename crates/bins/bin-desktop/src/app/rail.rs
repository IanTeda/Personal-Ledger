//! The primary rail's click, hover and toggle handlers of `Shell`. An `impl Shell` block, not the
//! rail itself (`chrome::rail`).

use gpui::{Context, Timer};
use lib_shell::NavInput;

use super::{Shell, TOOLTIP_REVEAL_DELAY};
use crate::{
    chrome::{rail as chrome_rail, window_size::WindowSize},
    navigation::nav::Noun,
};

impl Shell {
    /// The TopBar's own rail-toggle button (`Shell::render`'s `on_rail_toggle` closure) --
    /// same action as the `b` key, see [`Self::handle_key_down`].
    pub(super) fn handle_toggle_rail(
        &mut self,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
    ) {
        self.toggle_rail(window_size, cx);
        cx.notify();
    }

    /// `b`, the TopBar's toggle and the status line's hint: flips the rail drawn at this Window
    /// Size until the window crosses a Breakpoint. Clears any settled collapsed-rail tooltip, which
    /// is meaningless once the rail that anchors it changes shape.
    pub(super) fn toggle_rail(&mut self, window_size: WindowSize, cx: &gpui::App) {
        self.chrome.rail_override = Some(chrome_rail::toggled(
            window_size,
            self.start_sidebar_minimised(cx),
            self.chrome.rail_override,
        ));
        self.chrome.collapsed_rail_tooltip = None;
    }

    /// A collapsed primary-rail row's raw hover transition (`chrome::rail::primary::PrimaryRail`'s
    /// `on_row_hover`). Leaving a row clears any settled tooltip immediately; entering one
    /// only reveals its tooltip after [`TOOLTIP_REVEAL_DELAY`], and only if hover hasn't since
    /// moved elsewhere -- `hover_generation` is the guard: a stale timer whose captured
    /// generation no longer matches the current one simply does nothing.
    pub(super) fn handle_rail_hover(
        &mut self,
        noun: Noun,
        hovered: bool,
        cx: &mut Context<'_, Self>,
    ) {
        self.chrome.hover_generation += 1;
        if !hovered {
            self.chrome.collapsed_rail_tooltip = None;
            cx.notify();
            return;
        }

        let generation = self.chrome.hover_generation;
        cx.spawn(async move |this, cx| {
            Timer::after(TOOLTIP_REVEAL_DELAY).await;
            this.update(cx, |shell, cx| {
                if shell.chrome.hover_generation == generation {
                    shell.chrome.collapsed_rail_tooltip = Some(noun);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// A primary-rail row's click (`chrome::rail::primary::PrimaryRail`'s `on_row_click`), expanded or
    /// collapsed alike. A direct `NavState::set_noun`, not a browse-then-commit -- the same
    /// call `g`-jump (`Self::handle_key_down`'s pending-`g` arm) and the palette
    /// (`Self::run_command`'s own `CommandEffect::Navigate` arm) both make, so rail click,
    /// `g`-jump and the palette land in the same state per acceptance criterion 1.
    pub(super) fn handle_rail_click(&mut self, noun: Noun, cx: &mut Context<'_, Self>) {
        let noun_before = self.nav().noun();
        self.navigate(NavInput::Show(noun));
        if self.nav().noun() != noun_before {
            self.reset_view_scroll(cx);
        }
        cx.notify();
    }
}
