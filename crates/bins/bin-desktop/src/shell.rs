//! `Shell` -- the desktop window's root render tree (`docs/ux/desktop/README.md`'s
//! "Component tree"), replacing `feasibility_demo::DesktopApp`'s `TabBar`-driven screen
//! cycling as the real navigation entry point (ADR-0016).
//!
//! Assembles the static chrome from `docs/ux/desktop/Shell & Navigation/README.md`'s "1a"
//! spec (issue #148) and drives it with real keyboard interaction (issue #149): `Tab`/
//! `Shift-Tab` focus-zone cycling and `j`/`k`/`Down`/`Up`/`gg`/`G`/`Ctrl-d`/`Ctrl-u`/`Enter`
//! movement, scoped strictly to whichever zone (`NavState::focus`) currently has it. `g`-jump
//! chords, mode transitions (`:`/`/`/`a`/`?`/`Esc`), and the command palette are separate
//! build tickets (#150/#151) still to land on the
//! [Desktop Shell & Navigation](https://github.com/IanTeda/Personal-Ledger/issues/144) map. A
//! `View` trait mirroring `bin-tui`'s is still deliberately deferred (see ADR-0016):
//! `Dashboard` is the only real view, and `render_view` below is a plain match rather than a
//! trait object because there's still only one concrete implementor to dispatch to.
//!
//! `Shell` holds exactly one `gpui::FocusHandle` for the whole window rather than one per
//! zone: the three `FocusZone`s are our own conceptual navigation state
//! (`NavState::focus`), not `gpui`'s native focus system, which we only need once, to receive
//! keystrokes at all.

use gpui::{
    Context, FocusHandle, Focusable, KeyDownEvent, ScrollHandle, Window, div, point, prelude::*, px,
};

use crate::{
    nav::{FocusZone, NavState, Noun},
    rail::{self, context::ContextRail, primary::PrimaryRail},
    statusline::StatusLine,
    theme::{color, type_scale},
    topbar::TopBar,
    view::dashboard::Dashboard,
};

/// A single semantic movement, parsed once from a keystroke and then dispatched against
/// whichever zone is focused -- the same physical keys mean different things per zone, but
/// the keys-to-intent mapping itself doesn't vary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Movement {
    Next,
    Prev,
    First,
    Last,
    HalfPageDown,
    HalfPageUp,
    Enter,
}

/// The primary rail has a fixed 10-row list, not a real "page" of variable-height content --
/// half of that is a reasonable stand-in for `Ctrl-d`/`Ctrl-u` there.
const PRIMARY_RAIL_HALF_PAGE: usize = 5;

/// A single `j`/`k`/`Down`/`Up` step in the `View` zone's own scroll, in logical pixels --
/// there's no literal "row height" for a dashboard of charts and figures, so this is a plain
/// reading-sized increment, not a computed value.
const VIEW_LINE_STEP: f32 = 40.0;

/// Owns the shell's render tree and the live `NavState`.
pub struct Shell {
    nav: NavState,
    focus_handle: FocusHandle,
    /// Drives the `View` zone's own scroll when it's focused (`Movement::*`) -- rails don't
    /// scroll at all yet (their content always fits; overflow is a future ticket's problem).
    view_scroll_handle: ScrollHandle,
    /// `true` after a lone `g` keypress with no completing chord yet -- the leader half of
    /// `gg` ("jump to first"), this ticket's own narrow slice of the handoff's `g`-prefix
    /// grammar. Cleared by the very next key regardless of whether it completed `gg`, so an
    /// abandoned `g` never leaks into the next keypress. Ticket #150 replaces this with the
    /// full `g`-jump grammar (`g d`, `g t`, ...) and its 1000ms timeout -- this field only
    /// needs to recognise `gg` for the "Movement" rules this ticket covers.
    pending_g: bool,
}

impl Shell {
    pub fn new(nav: NavState, focus_handle: FocusHandle) -> Self {
        Self {
            nav,
            focus_handle,
            view_scroll_handle: ScrollHandle::new(),
            pending_g: false,
        }
    }

    pub fn nav(&self) -> &NavState {
        &self.nav
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    /// Routes a keystroke to focus-zone cycling or, via [`Movement`], to whichever zone is
    /// currently focused. Returns `false` for a keystroke this ticket doesn't handle (nothing
    /// to redraw).
    fn handle_key_down(&mut self, event: &KeyDownEvent) -> bool {
        let keystroke = &event.keystroke;
        let ctrl = keystroke.modifiers.control;
        let shift = keystroke.modifiers.shift;
        let key = keystroke.key.as_str();

        if key == "tab" {
            if shift {
                self.nav.cycle_focus_backward();
            } else {
                self.nav.cycle_focus_forward();
            }
            return true;
        }

        let was_pending_g = std::mem::take(&mut self.pending_g);
        let movement = if was_pending_g && key == "g" && !ctrl && !shift {
            Some(Movement::First)
        } else {
            match key {
                "g" if !ctrl && !shift => {
                    self.pending_g = true;
                    return false;
                }
                // `Keystroke::key` is always the lowercase base character -- Shift-g (`G`)
                // arrives as `key == "g"`, `modifiers.shift == true`, not `key == "G"`.
                "g" if shift => Some(Movement::Last),
                "j" | "down" => Some(Movement::Next),
                "k" | "up" => Some(Movement::Prev),
                "d" if ctrl => Some(Movement::HalfPageDown),
                "u" if ctrl => Some(Movement::HalfPageUp),
                "enter" => Some(Movement::Enter),
                _ => None,
            }
        };

        let Some(movement) = movement else {
            return false;
        };

        let noun_before = self.nav.noun();
        match self.nav.focus() {
            FocusZone::PrimaryRail => self.apply_primary_rail_movement(movement),
            FocusZone::ContextRail => self.apply_context_rail_movement(movement),
            FocusZone::View => self.apply_view_movement(movement),
        }
        if self.nav.noun() != noun_before {
            // A new noun's view is a different (usually much shorter) length -- carrying over
            // the old scroll offset could leave it scrolled past all its content, rendering
            // blank. Every fresh noun starts scrolled to the top.
            self.view_scroll_handle.set_offset(gpui::Point::default());
        }
        true
    }

    fn apply_primary_rail_movement(&mut self, movement: Movement) {
        match movement {
            Movement::Next => self.nav.move_primary_highlight_next(),
            Movement::Prev => self.nav.move_primary_highlight_prev(),
            Movement::First => self.nav.move_primary_highlight_first(),
            Movement::Last => self.nav.move_primary_highlight_last(),
            Movement::HalfPageDown => {
                for _ in 0..PRIMARY_RAIL_HALF_PAGE {
                    self.nav.move_primary_highlight_next();
                }
            }
            Movement::HalfPageUp => {
                for _ in 0..PRIMARY_RAIL_HALF_PAGE {
                    self.nav.move_primary_highlight_prev();
                }
            }
            Movement::Enter => self.nav.commit_primary_highlight(),
        }
    }

    fn apply_context_rail_movement(&mut self, movement: Movement) {
        let count = rail::context::entity_count(self.nav.noun());
        // Every noun besides Dashboard has a placeholder context rail with nothing in it yet
        // (see `rail::context::entity_count`'s own doc) -- movement is a no-op there, not an
        // out-of-bounds index.
        if count == 0 {
            return;
        }
        let half_page = (count / 2).max(1);
        let current = self.nav.context().unwrap_or(0);
        let next = match movement {
            Movement::Next => (current + 1).min(count - 1),
            Movement::Prev => current.saturating_sub(1),
            Movement::First => 0,
            Movement::Last => count - 1,
            Movement::HalfPageDown => (current + half_page).min(count - 1),
            Movement::HalfPageUp => current.saturating_sub(half_page),
            // The handoff's own "Movement" bullet: `Enter` "selects the entity and leaves
            // focus where it is" -- but movement here already updates `context` directly
            // (rule 2 guarantees that's side-effect-free), so there's nothing left for
            // `Enter` to additionally commit until a real per-entity detail view exists
            // (out of scope for this map, per issue #144).
            Movement::Enter => current,
        };
        self.nav.set_context(Some(next));
    }

    fn apply_view_movement(&mut self, movement: Movement) {
        let offset = self.view_scroll_handle.offset();
        let max_height = f32::from(self.view_scroll_handle.max_offset().height);
        let viewport_height = f32::from(self.view_scroll_handle.bounds().size.height);
        let current_y = f32::from(offset.y);

        let new_y = match movement {
            Movement::Next => current_y - VIEW_LINE_STEP,
            Movement::Prev => current_y + VIEW_LINE_STEP,
            Movement::First => 0.0,
            Movement::Last => -max_height,
            Movement::HalfPageDown => current_y - viewport_height / 2.0,
            Movement::HalfPageUp => current_y + viewport_height / 2.0,
            // No action is defined for `Enter` in the View zone by the handoff's "Movement"
            // bullet (only the primary and context rails are named) -- a no-op until one is.
            Movement::Enter => current_y,
        };
        let clamped_y = new_y.clamp(-max_height, 0.0);
        self.view_scroll_handle
            .set_offset(point(offset.x, px(clamped_y)));
    }
}

impl Focusable for Shell {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.nav.focus();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(color::GROUND)
            .text_color(color::INK)
            .font_family(type_scale::FONT_FAMILY)
            .text_size(type_scale::BODY)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                if this.handle_key_down(event) {
                    cx.notify();
                }
            }))
            .child(TopBar::new())
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .child(PrimaryRail::new(
                        self.nav.primary_highlight(),
                        focus == FocusZone::PrimaryRail,
                    ))
                    .when(self.nav.noun().has_context_entities(), |this| {
                        this.child(ContextRail::new(
                            self.nav.noun(),
                            self.nav.context(),
                            focus == FocusZone::ContextRail,
                        ))
                    })
                    .child(render_view(
                        self.nav.noun(),
                        focus == FocusZone::View,
                        &self.view_scroll_handle,
                    )),
            )
            .child(StatusLine::new(self.nav.mode(), self.nav.noun()))
    }
}

/// The active noun's own view interior. Only `Dashboard` is real; every other noun is a
/// placeholder until its own view lands (issue #153). Scrollable and focus-bordered
/// regardless of which noun is active, since both are properties of the `View` zone itself,
/// not of any one noun's content.
fn render_view(noun: Noun, focused: bool, scroll_handle: &ScrollHandle) -> gpui::AnyElement {
    let content = match noun {
        Noun::Dashboard => Dashboard::new().into_any_element(),
        other => div()
            .p(px(24.0))
            .text_color(color::INK_TERTIARY)
            .child(format!("{other:?} -- not yet built (see issue #153)"))
            .into_any_element(),
    };

    div()
        .id("view")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::INK)
        })
        .child(content)
        .into_any_element()
}
