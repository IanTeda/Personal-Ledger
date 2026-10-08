//! Shared chrome for a floating modal dialog (`docs/ux/desktop/16-settings/README.md`'s "Dialog"
//! component table): the full-viewport dimmer + centred bordered card, its header (with a
//! destructive variant), body, action row, and Cancel/Confirm buttons.
//!
//! Extracted by studying `crate::navigation::explorer::FileExplorer` and `crate::palette::Palette`'s own
//! hand-rolled floating overlays (issue #174's own ticket body) -- **neither is retrofitted to
//! use this module**. Both already match a different, already-shipped spec of their own (the
//! Shell & Navigation README's "1d"/"1e" component tables: a fixed top offset rather than
//! full-viewport centring, and -- in the file explorer's case -- header padding that's
//! genuinely `16px 20px`, not this module's `18px 20px`). Retrofitting either risks regressing
//! #165/#167's already-shipped, already-tested behaviour for no real gain, so this module exists
//! purely for the four Settings dialogs still to come (issues #184-#187) to build on.
//!
//! **A pure render-helper, not a stateful dialog owner.** Like `Palette`/`FileExplorer`, each
//! consuming dialog keeps owning its own open/close state as `Option<T>` on `Shell` and its own
//! esc/enter key handling -- this module only draws the chrome. `Shell` has no *real* `gpui`
//! focus-trapping for any floating overlay today (see `shell.rs`'s own module doc: one
//! `FocusHandle` for the whole window); "the dialog traps focus" is achieved by routing every
//! keystroke through `Shell::handle_key_down` while the owning `Option<T>` is `Some`, and that
//! pattern carries over unchanged for whichever ownership/`InputMode` shape each dialog ticket
//! picks. Deciding that shape is deliberately left to those tickets: unlike the file explorer
//! (opened via a `:`-palette command, so reusing `InputMode::Command` while it's open was a
//! natural fit), a Settings dialog opens from a plain button click inside the Settings view, with
//! no palette dispatch to piggyback on. Issues #184/#185 picked `InputMode::Dialog`
//! (`Shell::handle_dialog_key`), a new mode alongside `Command`/`Search`.
//!
//! Dialog state and the keyboard handling Dialogs share now live in `crate::dialog_host`; this
//! module still only draws.

use std::rc::Rc;

use gpui::{AnyElement, App, BoxShadow, SharedString, Window, div, point, prelude::*, px};

use crate::theme::color;

/// Fixed width: `docs/ux/desktop/16-settings/README.md`'s Dialog component table -- every dialog
/// this map builds except the Add institution dialog (issue #187), whose own raw markup sets
/// `width:460px` instead. [`overlay`] takes `width` as a parameter rather than hardcoding this
/// constant so that one dialog's own deviation doesn't need a special case.
pub const WIDTH: gpui::Pixels = px(420.0);

pub type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// Wraps `card` in the full-viewport dimmer + centred frame (`docs/ux/desktop/16-settings/README.md`'s
/// "Dimmer"/"Dialog" rows: `position:absolute; inset:0; ...; align-items:center;
/// justify-content:center`), then the bordered card itself -- `destructive` swaps the border to
/// `negative` (the "Destructive dialog" row). `width` is almost always [`WIDTH`]; see that
/// constant's own doc for the one exception.
pub fn overlay(
    width: gpui::Pixels,
    destructive: bool,
    card: impl IntoElement,
    cx: &App,
) -> AnyElement {
    div()
        .id("dialog-overlay")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(color::scrim(cx))
        .child(
            div()
                .w(width)
                .flex()
                .flex_col()
                .bg(color::background(cx))
                .border(px(2.0))
                .border_color(if destructive {
                    color::negative(cx)
                } else {
                    color::foreground(cx)
                })
                .shadow(vec![BoxShadow {
                    color: color::dialog_shadow(cx).into(),
                    offset: point(px(0.0), px(16.0)),
                    blur_radius: px(48.0),
                    spread_radius: px(0.0),
                }])
                .child(card),
        )
        .into_any_element()
}

/// The header row: `padding:18px 20px; border-bottom:2px solid rgba(32,30,29,.30)`, title
/// `800 16px`. `destructive` swaps the rule to `negative` and the title to its text shade
/// (`docs/ux/desktop/16-settings/README.md`'s "Destructive header" row, e.g. "Delete unit — btc").
pub fn header(title: impl Into<SharedString>, destructive: bool, cx: &App) -> impl IntoElement {
    div()
        .px(px(20.0))
        .py(px(18.0))
        .border_b(px(2.0))
        .border_color(if destructive {
            color::negative(cx)
        } else {
            color::border(cx)
        })
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(16.0))
        .text_color(if destructive {
            color::negative_text(cx)
        } else {
            color::foreground(cx)
        })
        .child(title.into())
}

/// The body wrapper: `padding:20px`, `fields` laid out in a column with `gap:16px`.
pub fn body(fields: impl IntoIterator<Item = AnyElement>) -> impl IntoElement {
    div()
        .p(px(20.0))
        .flex()
        .flex_col()
        .gap(px(16.0))
        .children(fields)
}

/// The action row: `padding:16px 20px; border-top:1px solid #d7d3d3; justify-content:flex-end;
/// gap:10px`. `buttons` are typically [`cancel_button`]/[`confirm_button`], in the order they
/// should read left to right (Cancel first, per every "Dialog lifecycle" row in the README).
pub fn action_row(buttons: impl IntoIterator<Item = AnyElement>, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .justify_end()
        .gap(px(10.0))
        .px(px(20.0))
        .py(px(16.0))
        .border_t(px(1.0))
        .border_color(color::hairline(cx))
        .children(buttons)
}

/// The Cancel button: `padding:8px 16px; border:1px solid rgba(32,30,29,.30);
/// background:transparent; font-weight:800`. `id` must be unique within the dialog it's used in
/// (`gpui`'s own element-identity requirement for a clickable node).
pub fn cancel_button(id: impl Into<SharedString>, on_click: OnClick, cx: &App) -> impl IntoElement {
    let id = id.into();
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .py(px(8.0))
        .px(px(16.0))
        .border_1()
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::foreground(cx))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(lib_locale::msg::dialog_cancel())
}

/// A rich Message as one wrapping run of text, its `<strong>` spans bolded (and in `strong_color`
/// when given). A `div` per Segment can't break inside a Segment, so long copy with an argument or
/// a bold span in it overflowed the card instead of wrapping.
pub fn rich_text(
    segments: Vec<lib_locale::Segment>,
    strong_color: Option<gpui::Hsla>,
) -> gpui::StyledText {
    let mut text = String::new();
    let mut highlights = Vec::new();
    for segment in segments {
        let start = text.len();
        text.push_str(&segment.text);
        if segment.tag.as_deref() == Some("strong") {
            highlights.push((
                start..text.len(),
                gpui::HighlightStyle {
                    font_weight: Some(gpui::FontWeight::EXTRA_BOLD),
                    color: strong_color,
                    ..Default::default()
                },
            ));
        }
    }
    gpui::StyledText::new(text).with_highlights(highlights)
}

/// The Info panel: `padding:10-12px; background:#eae9e9; border-left:2px solid #ec3013;
/// font-size:11.5px` (`docs/ux/desktop/16-settings/README.md`'s Components table) -- the Edit unit
/// dialog's own usage notice (issue #185) and the Delete unit dialog's own reference panel
/// (issue #186) share this exact style, so it's extracted here rather than built twice.
pub fn info_panel(content: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    div()
        .p(px(10.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::accent(cx))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(content.into())
}

/// The Confirm button: `padding:8px 16px; background:#201e1d; color:#f3f2f2; border:none;
/// font-weight:800`. `destructive` swaps the fill to `negative` (the "Destructive confirm" row).
/// Disabled (45% opacity, matching the shell's own "Disabled" state convention, and no click
/// handler at all) while `enabled` is `false` -- the Delete-unit dialog's own typed-confirmation
/// gate (issue #186) is exactly this parameter.
pub fn confirm_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    enabled: bool,
    destructive: bool,
    on_click: OnClick,
    cx: &App,
) -> impl IntoElement {
    let id = id.into();
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .when(enabled, |this| this.cursor_pointer())
        .when(!enabled, |this| this.opacity(0.45))
        .py(px(8.0))
        .px(px(16.0))
        .bg(if destructive {
            color::negative(cx)
        } else {
            color::foreground(cx)
        })
        .text_color(color::background(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .when(enabled, |this| {
            this.on_click(move |_event, window, cx| on_click(window, cx))
        })
        .child(label.into())
}
