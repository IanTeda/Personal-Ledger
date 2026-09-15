//! `Shell` -- the desktop window's root render tree (`docs/ux/desktop/README.md`'s
//! "Component tree"), replacing `feasibility_demo::DesktopApp`'s `TabBar`-driven screen
//! cycling as the real navigation entry point (ADR-0016).
//!
//! Assembles the static chrome from `docs/ux/desktop/Shell & Navigation/README.md`'s "1a"
//! spec (issue #148) and drives it with real keyboard interaction: `Tab`/`Shift-Tab`
//! focus-zone cycling and `j`/`k`/`Down`/`Up`/`gg`/`G`/`Ctrl-d`/`Ctrl-u`/`Enter` movement,
//! scoped strictly to whichever zone (`NavState::focus`) currently has it (issue #149); the
//! `g`-prefix jump chords (`g d`, `g t`, ...) and the `Normal`/`Insert`/`Command`/`Search`
//! mode transitions (issue #150); the command palette (issue #151); the collapsed rail, its
//! `b`-key/click toggle, and its hover tooltip (issue #152). `?`'s help overlay is a separate
//! build ticket still to land on the
//! [Desktop Shell & Navigation](https://github.com/IanTeda/Personal-Ledger/issues/144) map. A
//! `View` trait mirroring `bin-tui`'s is still deliberately deferred (see ADR-0016):
//! `Dashboard` is the only real view, and `render_view` below is a plain match rather than a
//! trait object because there's still only one concrete implementor to dispatch to.
//!
//! `Shell` holds exactly one `gpui::FocusHandle` for the whole window rather than one per
//! zone: the three `FocusZone`s are our own conceptual navigation state
//! (`NavState::focus`), not `gpui`'s native focus system, which we only need once, to receive
//! keystrokes at all.

use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    Context, FocusHandle, Focusable, KeyDownEvent, Keystroke, ScrollHandle, Timer, Window, div,
    point, prelude::*, px,
};

use crate::{
    command::Command,
    nav::{FocusZone, InputMode, NavState, Noun},
    palette::Palette,
    rail::{self, context::ContextRail, primary::PrimaryRail},
    statusline::StatusLine,
    theme::{color, type_scale},
    topbar::{self, TopBar},
    view::dashboard::Dashboard,
};

/// The collapsed rail's own hover-reveal delay (`docs/ux/desktop/Shell & Navigation/README.md`'s
/// "1c" tooltip spec) -- deliberately the same 500ms `gpui`'s own built-in `.tooltip()` uses,
/// even though this tooltip is hand-rolled (row-anchored, not cursor-anchored -- see
/// `rail::primary::collapsed_tooltip`'s doc) rather than that builtin.
const TOOLTIP_REVEAL_DELAY: Duration = Duration::from_millis(500);

/// The handoff's own "Jumps" timeout: a `g` with no completing chord within this window is
/// abandoned rather than left waiting indefinitely.
const PENDING_G_TIMEOUT: Duration = Duration::from_millis(1000);

/// The `g`-prefix jump target for each bound completion key. Every noun has one today --
/// `Transactions` moved off `g t` onto `g l` to free `t` for the newer `Tags` noun.
fn jump_noun_for_key(key: &str) -> Option<Noun> {
    match key {
        "d" => Some(Noun::Dashboard),
        "l" => Some(Noun::Transactions),
        "a" => Some(Noun::Accounts),
        "c" => Some(Noun::Categories),
        "p" => Some(Noun::Payees),
        "t" => Some(Noun::Tags),
        "w" => Some(Noun::Bills),
        "b" => Some(Noun::Budgets),
        "r" => Some(Noun::Reports),
        "s" => Some(Noun::Settings),
        _ => None,
    }
}

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
    /// When this `g` was pressed -- `Some` while a completing chord (`gg`, or `g` + a jump
    /// key) is still possible. Cleared by the next keypress regardless of outcome, so an
    /// abandoned `g` never leaks into the one after it. Checked against
    /// [`PENDING_G_TIMEOUT`] rather than driven by a timer: nothing needs to happen on its
    /// own with no further keypress, so a lazily-checked timestamp is enough.
    pending_g: Option<Instant>,
    /// Replaces the status line's hint strip until the next keypress -- the `g`-prefix's own
    /// "flash the hint strip" abort message, and the command palette's "not yet built" message
    /// once it closes back to `Normal` (see [`Self::run_command`]).
    status_message: Option<String>,
    /// The command palette's own input/selection state -- `Some` only while
    /// `NavState::mode` is `InputMode::Command`, mirroring `bin-tui`'s own
    /// `Shell`'s `Option<popup::command::CommandPopup>` (`docs/ux/desktop/README.md`'s Notes).
    palette: Option<Palette>,
    /// The collapsed primary rail's row whose hover has settled past
    /// [`TOOLTIP_REVEAL_DELAY`] -- `None` while nothing's hovered, the delay hasn't elapsed
    /// yet, or the rail isn't collapsed (see `rail::primary::PrimaryRail`, which only wires
    /// hover at all in its collapsed rendering).
    collapsed_rail_tooltip: Option<Noun>,
    /// Bumped on every hover transition; a pending reveal timer checks this against the value
    /// it captured before applying, so hovering a second row (or leaving the rail entirely)
    /// before the first row's delay elapses can't reveal the wrong tooltip.
    hover_generation: u64,
}

impl Shell {
    pub fn new(nav: NavState, focus_handle: FocusHandle) -> Self {
        Self {
            nav,
            focus_handle,
            view_scroll_handle: ScrollHandle::new(),
            pending_g: None,
            status_message: None,
            palette: None,
            collapsed_rail_tooltip: None,
            hover_generation: 0,
        }
    }

    pub fn nav(&self) -> &NavState {
        &self.nav
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    /// Routes a keystroke: `Esc` first (works in any mode, clears a pending `g` before
    /// leaving the current mode), then -- while a non-`Normal` mode is active -- nothing else
    /// (mode transitions pre-empt zone/movement handling, and there's no real `Insert`/
    /// `Command`/`Search` input surface to route keys to yet, see the module doc), then a
    /// pending `g`'s own completion/abort (before anything else can claim the key, so `g a`
    /// reaches Accounts rather than bare `a`'s mode entry), then the global mode-entry keys,
    /// `Tab` cycling, arming a fresh `g`, and finally [`Movement`] dispatched to whichever
    /// zone is focused. Returns `false` for a keystroke that changed nothing (nothing to
    /// redraw).
    fn handle_key_down(&mut self, event: &KeyDownEvent) -> bool {
        let keystroke = &event.keystroke;
        let ctrl = keystroke.modifiers.control;
        let shift = keystroke.modifiers.shift;
        let key = keystroke.key.as_str();

        if key == "escape" {
            if self.pending_g.take().is_some() {
                return true;
            }
            if self.nav.mode() != InputMode::Normal {
                self.palette = None;
                self.nav.exit_mode();
                return true;
            }
            return false;
        }

        // The handoff's own precedent for hint-strip messages (`docs/ux/desktop/README.md`'s
        // "Loading and error states"): any keypress clears one, not just a timer.
        let had_status_message = self.status_message.take().is_some();

        // Popup-owned keys (mirroring `docs/ux/tui/navigation.md`'s own tier 2): while the
        // palette is open it owns every keystroke, checked before the "Normal only" gate below
        // since `Command` is the one non-`Normal` mode with a real input surface to route keys
        // to today (`Insert`/`Search` don't have one yet -- see the module doc).
        if self.nav.mode() == InputMode::Command {
            return self.handle_palette_key(keystroke) || had_status_message;
        }

        if self.nav.mode() != InputMode::Normal {
            return had_status_message;
        }

        // A pending `g` consumes the very next key unconditionally, completing or aborting
        // the chord -- checked before the mode-entry keys and `Tab` below so e.g. `g a`
        // reaches Accounts rather than the bare `a` mode-entry key, and `g` then anything
        // else never leaks into ordinary handling.
        if let Some(pending_since) = self.pending_g.take()
            && pending_since.elapsed() <= PENDING_G_TIMEOUT
        {
            if key == "g" && !ctrl && !shift {
                self.apply_movement(Movement::First);
                return true;
            }
            if !ctrl
                && !shift
                && let Some(noun) = jump_noun_for_key(key)
            {
                self.nav.set_noun(noun);
                self.reset_view_scroll();
                return true;
            }
            // The handoff: "`g` + an unbound key is a no-op: clear the pending prefix and
            // flash the hint strip." `key` itself is consumed doing nothing else -- it
            // completes (aborts) the chord rather than also being processed as its own
            // ordinary keystroke.
            self.status_message = Some(format!("g {key} is not a jump"));
            return true;
        }
        // No pending `g` (or it timed out) -- fall through and process `key` fresh.

        match key {
            ":" => {
                self.nav.enter_mode(InputMode::Command);
                self.palette = Some(Palette::new());
                return true;
            }
            "/" => {
                self.nav.enter_mode(InputMode::Search);
                return true;
            }
            // `g a` (above) jumps to Accounts instead -- the pending-`g` branch always runs
            // first and returns before this match is reached, so the two never collide.
            "a" if !ctrl && !shift => {
                self.nav.enter_mode(InputMode::Insert);
                return true;
            }
            // Mirrors the TopBar's own rail-toggle button (`Shell::handle_toggle_rail`) --
            // same action, two entry points. Clears any settled collapsed-rail tooltip: it's
            // meaningless once the rail that anchors it changes shape.
            "b" if !ctrl && !shift => {
                self.nav.toggle_primary_rail();
                self.collapsed_rail_tooltip = None;
                return true;
            }
            _ => {}
        }

        if key == "tab" {
            if shift {
                self.nav.cycle_focus_backward();
            } else {
                self.nav.cycle_focus_forward();
            }
            return true;
        }

        if key == "g" && !ctrl && !shift {
            self.pending_g = Some(Instant::now());
            return had_status_message;
        }

        let movement = match key {
            // Bare Shift-`g` (`G`), no pending prefix -- jump to last in the focused zone.
            "g" if shift => Some(Movement::Last),
            "j" | "down" => Some(Movement::Next),
            "k" | "up" => Some(Movement::Prev),
            "d" if ctrl => Some(Movement::HalfPageDown),
            "u" if ctrl => Some(Movement::HalfPageUp),
            "enter" => Some(Movement::Enter),
            _ => None,
        };

        let Some(movement) = movement else {
            return had_status_message;
        };
        self.apply_movement(movement);
        true
    }

    fn apply_movement(&mut self, movement: Movement) {
        let noun_before = self.nav.noun();
        match self.nav.focus() {
            FocusZone::PrimaryRail => self.apply_primary_rail_movement(movement),
            FocusZone::ContextRail => self.apply_context_rail_movement(movement),
            FocusZone::View => self.apply_view_movement(movement),
        }
        if self.nav.noun() != noun_before {
            self.reset_view_scroll();
        }
    }

    /// A new noun's view is a different (usually much shorter) length -- carrying over the
    /// old scroll offset could leave it scrolled past all its content, rendering blank. Every
    /// fresh noun starts scrolled to the top.
    fn reset_view_scroll(&mut self) {
        self.view_scroll_handle.set_offset(gpui::Point::default());
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

    /// Routes a keystroke while the palette is open (tier 2, "popup-owned keys" -- mirroring
    /// `docs/ux/tui/navigation.md`): `Backspace` mutates the input buffer, `Up`/`Down` move the
    /// selection, `Enter` runs the selected command (see [`Self::run_command`]), and any other
    /// unmodified, printable key is typed into the query. Everything else is swallowed here
    /// rather than falling through to the zone/movement handling below -- keeping "the popup
    /// owns every keystroke" true even for a key (e.g. `Tab`) this palette gives no meaning to.
    fn handle_palette_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(palette) = self.palette.as_mut() else {
            return false;
        };

        match keystroke.key.as_str() {
            "backspace" => {
                palette.backspace();
                true
            }
            "up" => {
                palette.move_up();
                true
            }
            "down" => {
                palette.move_down();
                true
            }
            "enter" => {
                let command = palette.selected_command();
                self.palette = None;
                self.nav.exit_mode();
                if let Some(command) = command {
                    self.run_command(command);
                }
                true
            }
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                match keystroke.key_char.as_deref() {
                    Some(text) if text.chars().count() == 1 => {
                        palette.push_char(text.chars().next().expect("checked above"));
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    /// Runs `command`'s handler if it has one, resetting the view's scroll when it lands on a
    /// different noun (matching every other navigation entry point -- `g`-jumps, rail `Enter`).
    /// A command with no handler yet (`Command::handler == None`) shows the same "not yet
    /// built" message `docs/ux/tui/navigation.md` describes for its own popup, reusing the
    /// status line's existing `status_message` slot (the "1d" spec's own COMMAND-mode status
    /// line has no message slot of its own, and the palette has already closed by the time this
    /// runs -- see the `enter` arm of [`Self::handle_palette_key`]).
    fn run_command(&mut self, command: &'static Command) {
        match command.handler {
            Some(handler) => {
                let noun_before = self.nav.noun();
                handler(&mut self.nav);
                if self.nav.noun() != noun_before {
                    self.reset_view_scroll();
                }
            }
            None => {
                self.status_message = Some(format!(":{} — not yet built", command.name));
            }
        }
    }

    /// The TopBar's own rail-toggle button (`Shell::render`'s `on_rail_toggle` closure) --
    /// same action as the `b` key, see [`Self::handle_key_down`].
    fn handle_toggle_rail(&mut self, cx: &mut Context<Self>) {
        self.nav.toggle_primary_rail();
        self.collapsed_rail_tooltip = None;
        cx.notify();
    }

    /// A collapsed primary-rail row's raw hover transition (`rail::primary::PrimaryRail`'s
    /// `on_row_hover`). Leaving a row clears any settled tooltip immediately; entering one
    /// only reveals its tooltip after [`TOOLTIP_REVEAL_DELAY`], and only if hover hasn't since
    /// moved elsewhere -- `hover_generation` is the guard: a stale timer whose captured
    /// generation no longer matches the current one simply does nothing.
    fn handle_rail_hover(&mut self, noun: Noun, hovered: bool, cx: &mut Context<Self>) {
        self.hover_generation += 1;
        if !hovered {
            self.collapsed_rail_tooltip = None;
            cx.notify();
            return;
        }

        let generation = self.hover_generation;
        cx.spawn(async move |this, cx| {
            Timer::after(TOOLTIP_REVEAL_DELAY).await;
            this.update(cx, |shell, cx| {
                if shell.hover_generation == generation {
                    shell.collapsed_rail_tooltip = Some(noun);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// A primary-rail row's click (`rail::primary::PrimaryRail`'s `on_row_click`), expanded or
    /// collapsed alike. A direct `NavState::set_noun`, not a browse-then-commit -- the same
    /// call `g`-jump (`Self::handle_key_down`'s pending-`g` arm) and the palette
    /// (`command::COMMANDS`'s `goto_*` handlers) both make, so rail click, `g`-jump and the
    /// palette land in the same state per acceptance criterion 1.
    fn handle_rail_click(&mut self, noun: Noun, cx: &mut Context<Self>) {
        let noun_before = self.nav.noun();
        self.nav.set_noun(noun);
        if self.nav.noun() != noun_before {
            self.reset_view_scroll();
        }
        cx.notify();
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
        // The "1d" spec: "The shell behind the palette drops to 30% opacity." Applied to the
        // top bar and content row only, not the status line -- that same spec separately
        // describes the status line's own COMMAND-mode content (the live query, "esc close
        // command window"), which stays meaningful precisely because it stays legible; only
        // the navigational chrome the palette visually floats over goes dim.
        let content_opacity = if self.palette.is_some() { 0.3 } else { 1.0 };

        // Both closures go through an `Entity` handle (mirroring `feasibility_demo`'s own
        // `TabBar::on_click` wiring) rather than `cx.listener`: `on_click`/`on_hover`'s own
        // signatures are `Fn(_, &mut Window, &mut App)`, with no `&mut Shell` parameter for
        // `cx.listener` to supply, and the collapsed rail's `on_row_hover` closure additionally
        // needs to close over each row's own `Noun` -- `PrimaryRail` curries that in per-row
        // from the single `Rc` given here.
        let entity = cx.entity();
        let on_rail_toggle: topbar::OnRailToggle = {
            let entity = entity.clone();
            Rc::new(move |_event, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_toggle_rail(cx));
            })
        };
        let on_row_hover: rail::primary::OnRowHover = {
            let entity = entity.clone();
            Rc::new(move |noun, hovered, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_rail_hover(noun, hovered, cx));
            })
        };
        let on_row_click: rail::primary::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |noun, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_rail_click(noun, cx));
            })
        };

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
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .opacity(content_opacity)
                    .child(TopBar::new(on_rail_toggle))
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.0))
                            .flex()
                            .child(PrimaryRail::new(
                                self.nav.primary_highlight(),
                                focus == FocusZone::PrimaryRail,
                                self.nav.primary_rail(),
                                self.collapsed_rail_tooltip,
                                on_row_hover,
                                on_row_click,
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
                    ),
            )
            .child(StatusLine::new(
                self.nav.mode(),
                self.nav.noun(),
                self.status_message.clone(),
                self.palette
                    .as_ref()
                    .map(|palette| palette.input().to_string()),
            ))
            .children(self.palette.as_ref().map(Palette::render))
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
