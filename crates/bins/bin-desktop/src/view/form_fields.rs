//! Form widgets shared by the Desktop's dialogs and Settings pages: a field label, a clickable text
//! field and a segmented control. Each is a pure render helper; the owning View or Shell holds the
//! state and handles the clicks, so these stay reusable across Documents, Inventory, Units and
//! Institutions without depending on any one of them.

use std::rc::Rc;

use gpui::{App, SharedString, Window, div, prelude::*, px};

use crate::{dialog, theme::color};

/// A segmented control row's own click handler, generic over which enum it selects.
pub(crate) type OnSegmentClick<T> = Rc<dyn Fn(T, &mut Window, &mut App)>;

/// A field label: `display:block; font-weight:800; font-size:12px; margin-bottom:6px`
/// (`docs/ux/desktop-mockups/16-settings/README.md`'s Components table) -- shared by every section that
/// lays out `Field label` + `Input/select` pairs.
pub(crate) fn field_label(label: impl Into<SharedString>) -> impl IntoElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .mb(px(6.0))
        .child(label.into())
}

/// One Code/Name field: `field_label` above a clickable value box mirroring
/// `super::field_value`'s own border/padding, plus the focused-state border and caret this
/// crate's first real text input needs (see the module doc). `label`/`placeholder` take
/// non-`'static` strings too -- the Delete unit dialog's own confirm field (issue #186) needs a
/// row-specific label ("Type btc to confirm") and placeholder (the code itself), neither known
/// until render time.
pub(crate) fn text_field(
    id: &'static str,
    label: impl Into<SharedString>,
    value: &str,
    placeholder: &str,
    focused: bool,
    on_click: dialog::OnClick,
    cx: &App,
) -> impl IntoElement {
    let caret = if focused { "\u{2502}" } else { "" };
    let (text, text_color) = if value.is_empty() {
        (
            SharedString::from(format!("{placeholder}{caret}")),
            color::faint_text(cx),
        )
    } else {
        (
            SharedString::from(format!("{value}{caret}")),
            color::foreground(cx),
        )
    };

    div().child(field_label(label)).child(
        div()
            .id(id)
            .debug_selector(move || id.to_string())
            .cursor_pointer()
            .w_full()
            .py(px(8.0))
            .px(px(10.0))
            .border_1()
            .border_color(if focused {
                color::accent(cx)
            } else {
                color::border(cx)
            })
            .text_size(px(13.0))
            .text_color(text_color)
            .on_click(move |_event, window, cx| on_click(window, cx))
            .child(text),
    )
}

/// `.seg`/`.seg-opt`: a bordered, radius-0 pill row, each option separated by a 1px rule, the
/// selected option taking `background: var(--color-accent); color: var(--color-bg)` -- the
/// segmented control's own named exception to the shell's "accent never a background" rule (see
/// `theme::color::accent`'s own doc). Only usable for `&'static [T]` option lists --
/// `super::add_institution_dialog`'s own Default unit field needs a runtime `&[UnitRow]`
/// instead, so it builds its own bespoke row rather than reusing this one (see that module's
/// own doc).
pub(crate) fn segmented_control<T: Copy + PartialEq + 'static, L: Into<SharedString>>(
    id_prefix: &'static str,
    options: &'static [T],
    current: T,
    label: fn(T) -> L,
    on_click: OnSegmentClick<T>,
    cx: &App,
) -> impl IntoElement {
    segmented_control_sized(id_prefix, options, current, label, on_click, None, cx)
}

/// The shared option width of the Display page's uniform segmented controls.
pub(crate) const UNIFORM_OPTION_WIDTH: f32 = 88.0;

/// [`segmented_control`] with every option the same fixed width (its label centred), for short
/// parallel labels that read better as a uniform row.
pub(crate) fn segmented_control_sized<T: Copy + PartialEq + 'static, L: Into<SharedString>>(
    id_prefix: &'static str,
    options: &'static [T],
    current: T,
    label: fn(T) -> L,
    on_click: OnSegmentClick<T>,
    option_width: Option<gpui::Pixels>,
    cx: &App,
) -> impl IntoElement {
    // A block-level flex row would stretch to the column and leave an empty trailing "button" of
    // bar, so a uniform control is exactly as wide as its options (plus the 1px border each side).
    let bar_width = option_width.map(|width| width * options.len() as f32 + px(2.0));
    div()
        .flex()
        .when_some(bar_width, |this, width| this.w(width))
        .border_1()
        .border_color(color::divider(cx))
        .children(options.iter().enumerate().map(|(index, &option)| {
            let selected = option == current;
            let on_click = on_click.clone();
            div()
                .id(SharedString::from(format!("{id_prefix}-{index}")))
                .debug_selector(move || format!("{id_prefix}-{index}"))
                .cursor_pointer()
                .py(px(7.0))
                .px(px(12.0))
                .text_size(px(13.0))
                .when_some(option_width, |this, width| {
                    this.w(width).flex().justify_center()
                })
                .when(index > 0, |this| {
                    this.border_l(px(1.0)).border_color(color::divider(cx))
                })
                .when(selected, |this| {
                    this.bg(color::accent(cx)).text_color(color::background(cx))
                })
                .on_click(move |_event, window, cx| on_click(option, window, cx))
                .child(label(option).into())
        }))
}
