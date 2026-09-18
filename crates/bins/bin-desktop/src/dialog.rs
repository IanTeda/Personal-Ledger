//! Shared chrome for a floating modal dialog (`docs/ux/desktop/Settings/README.md`'s "Dialog"
//! component table): the full-viewport dimmer + centred bordered card, its header (with a
//! destructive variant), body, action row, and Cancel/Confirm buttons.
//!
//! Extracted by studying `crate::explorer::FileExplorer` and `crate::palette::Palette`'s own
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

use std::rc::Rc;

use gpui::{AnyElement, App, BoxShadow, SharedString, Window, div, point, prelude::*, px};

use crate::theme::color;

/// Fixed width: `docs/ux/desktop/Settings/README.md`'s Dialog component table.
pub const WIDTH: gpui::Pixels = px(420.0);

pub type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// Wraps `card` in the full-viewport dimmer + centred frame (`docs/ux/desktop/Settings/README.md`'s
/// "Dimmer"/"Dialog" rows: `position:absolute; inset:0; ...; align-items:center;
/// justify-content:center`), then the bordered card itself -- `destructive` swaps the border to
/// `#ec3013` (the "Destructive dialog" row).
pub fn overlay(destructive: bool, card: impl IntoElement) -> AnyElement {
    div()
        .id("dialog-overlay")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(color::DIMMER)
        .child(
            div()
                .w(WIDTH)
                .flex()
                .flex_col()
                .bg(color::GROUND)
                .border(px(2.0))
                .border_color(if destructive {
                    color::ACCENT
                } else {
                    color::INK
                })
                .shadow(vec![BoxShadow {
                    color: color::DIALOG_SHADOW.into(),
                    offset: point(px(0.0), px(16.0)),
                    blur_radius: px(48.0),
                    spread_radius: px(0.0),
                }])
                .child(card),
        )
        .into_any_element()
}

/// The header row: `padding:18px 20px; border-bottom:2px solid rgba(32,30,29,.30)`, title
/// `800 16px`. `destructive` swaps the rule to `#ec3013` and the title to `#ae1800`
/// (`docs/ux/desktop/Settings/README.md`'s "Destructive header" row, e.g. "Delete unit — btc").
pub fn header(title: impl Into<SharedString>, destructive: bool) -> impl IntoElement {
    div()
        .px(px(20.0))
        .py(px(18.0))
        .border_b(px(2.0))
        .border_color(if destructive {
            color::ACCENT
        } else {
            color::BORDER
        })
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(16.0))
        .text_color(if destructive {
            color::ACCENT_TEXT
        } else {
            color::INK
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
pub fn action_row(buttons: impl IntoIterator<Item = AnyElement>) -> impl IntoElement {
    div()
        .flex()
        .justify_end()
        .gap(px(10.0))
        .px(px(20.0))
        .py(px(16.0))
        .border_t(px(1.0))
        .border_color(color::HAIRLINE)
        .children(buttons)
}

/// The Cancel button: `padding:8px 16px; border:1px solid rgba(32,30,29,.30);
/// background:transparent; font-weight:800`. `id` must be unique within the dialog it's used in
/// (`gpui`'s own element-identity requirement for a clickable node).
pub fn cancel_button(id: impl Into<SharedString>, on_click: OnClick) -> impl IntoElement {
    div()
        .id(id.into())
        .cursor_pointer()
        .py(px(8.0))
        .px(px(16.0))
        .border_1()
        .border_color(color::BORDER)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::INK)
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child("Cancel")
}

/// The Info panel: `padding:10-12px; background:#eae9e9; border-left:2px solid #ec3013;
/// font-size:11.5px` (`docs/ux/desktop/Settings/README.md`'s Components table) -- the Edit unit
/// dialog's own usage notice (issue #185) and the Delete unit dialog's own reference panel
/// (issue #186) share this exact style, so it's extracted here rather than built twice.
pub fn info_panel(content: impl Into<SharedString>) -> impl IntoElement {
    div()
        .p(px(10.0))
        .bg(color::CHROME)
        .border_l(px(2.0))
        .border_color(color::ACCENT)
        .text_size(px(11.5))
        .text_color(color::INK_SECONDARY)
        .child(content.into())
}

/// The Confirm button: `padding:8px 16px; background:#201e1d; color:#f3f2f2; border:none;
/// font-weight:800`. `destructive` swaps the fill to `#ec3013` (the "Destructive confirm" row).
/// Disabled (45% opacity, matching the shell's own "Disabled" state convention, and no click
/// handler at all) while `enabled` is `false` -- the Delete-unit dialog's own typed-confirmation
/// gate (issue #186) is exactly this parameter.
pub fn confirm_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    enabled: bool,
    destructive: bool,
    on_click: OnClick,
) -> impl IntoElement {
    div()
        .id(id.into())
        .when(enabled, |this| this.cursor_pointer())
        .when(!enabled, |this| this.opacity(0.45))
        .py(px(8.0))
        .px(px(16.0))
        .bg(if destructive {
            color::ACCENT
        } else {
            color::INK
        })
        .text_color(color::INK_ON_DARK)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .when(enabled, |this| {
            this.on_click(move |_event, window, cx| on_click(window, cx))
        })
        .child(label.into())
}
