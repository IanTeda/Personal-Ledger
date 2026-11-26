//! The text caret every Desktop text box draws while it has focus: a thin bar flush against the
//! last character (or the start of an empty box), blinking. A drawn bar rather than a `│` glyph,
//! which sat a character's side bearing to the right of the text.
//!
//! The blink is one [`CaretBlink`] for the App, toggled by the Shell's blink poll
//! (`shell/router/caret_blink.rs`) only while a caret was drawn since the last toggle, so a window
//! with no text box focused never redraws for it. A key held the caret on through the next toggle,
//! so it stays solid while a person types.

use std::cell::Cell;
use std::time::Duration;

use gpui::{AnyElement, App, Div, Global, SharedString, div, prelude::*, px};

use crate::theme::color;

/// How long the caret shows, then hides, the platforms' usual half-second blink.
pub const CARET_BLINK: Duration = Duration::from_millis(530);

/// Whether the caret shows now, and whether any was drawn since the blink last toggled.
struct CaretBlink {
    visible: bool,
    /// Set by a key, so the next toggle leaves the caret showing.
    held: bool,
    /// Set by each caret drawn: a `Cell`, as a render reads the App but can't write it.
    drawn: Cell<bool>,
}

impl Global for CaretBlink {}

/// Starts the caret showing. Called once, by the Shell's blink poll.
pub(crate) fn install(cx: &mut App) {
    cx.set_global(CaretBlink {
        visible: true,
        held: false,
        drawn: Cell::new(false),
    });
}

/// Toggles the caret, returning whether the window needs redrawing: only while one is drawn, and
/// not on the toggle after a key. With none drawn the caret is left showing, ready for the next
/// field focused.
pub(crate) fn tick(cx: &mut App) -> bool {
    let Some(blink) = cx.try_global::<CaretBlink>() else {
        return false;
    };
    if !blink.drawn.get() {
        let blink = cx.global_mut::<CaretBlink>();
        blink.visible = true;
        blink.held = false;
        return false;
    }
    let blink = cx.global_mut::<CaretBlink>();
    if blink.held {
        blink.held = false;
        return false;
    }
    blink.visible = !blink.visible;
    // The redraw this asks for sets it again if a caret is still on screen.
    blink.drawn.set(false);
    true
}

/// Shows the caret through the next toggle: a key was pressed, and a caret blinking out under the
/// person's typing reads as lag.
pub(crate) fn hold(cx: &mut App) {
    if cx.has_global::<CaretBlink>() {
        let blink = cx.global_mut::<CaretBlink>();
        blink.visible = true;
        blink.held = true;
    }
}

/// The caret bar, in the text's ink, hidden on the blink's off half. Always laid out, so the text
/// beside it never shifts as it blinks.
pub(crate) fn caret(cx: &App) -> AnyElement {
    let visible = cx.try_global::<CaretBlink>().is_none_or(|blink| {
        blink.drawn.set(true);
        blink.visible
    });
    let bar = div().flex_none().w(px(1.5)).h(px(15.0));
    if visible {
        bar.bg(color::foreground(cx))
    } else {
        bar
    }
    .into_any_element()
}

/// A text box's contents: `value` with the caret after its last character while `focused`, or,
/// empty, the `placeholder` in the faint ink with the caret before it, where typing begins.
pub(crate) fn field_text(value: &str, placeholder: &str, focused: bool, cx: &App) -> Div {
    if value.is_empty() {
        with_caret(placeholder.to_string(), focused, true, cx).text_color(color::faint_text(cx))
    } else {
        with_caret(value.to_string(), focused, false, cx).text_color(color::foreground(cx))
    }
}

/// `text` on one row, cut to the box with an ellipsis, with the caret, when `focused`, before it
/// (`at_start`) or after it.
pub(crate) fn with_caret(
    text: impl Into<SharedString>,
    focused: bool,
    at_start: bool,
    cx: &App,
) -> Div {
    let text = div().min_w(px(0.0)).truncate().child(text.into());
    let row = div().flex().items_center().min_w(px(0.0));
    match (focused, at_start) {
        (false, _) => row.child(text),
        (true, true) => row.child(caret(cx)).child(text),
        (true, false) => row.child(text).child(caret(cx)),
    }
}
