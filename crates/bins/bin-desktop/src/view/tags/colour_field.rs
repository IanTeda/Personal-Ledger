//! The Add and Edit tag dialogs' **Colour** field (#352): a "none" chip, the handoff's six preset
//! swatches (`26×26`, `gap 8px`, the pick outlined `2px`) and a free hex box that takes any other
//! colour. `tags::TagForm` keeps the hex box as the one source of the colour, so this only draws
//! it and reports clicks.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use super::swatch;
use crate::{
    dialog,
    tags::{self, TagField, TagForm},
    theme::color,
    view::accounts::add_dialog::optional_label,
};

const SWATCH_SIZE: gpui::Pixels = px(26.0);
/// The hex box's preview swatch.
const PREVIEW_SIZE: gpui::Pixels = px(12.0);

/// Called with a pick: 0 for "none", `1..=6` for a preset.
pub type OnPick = Rc<dyn Fn(usize, &mut Window, &mut App)>;

pub struct ColourFieldProps<'a> {
    /// The dialog's id prefix, so each element id is unique.
    pub id: &'static str,
    pub form: &'a TagForm,
    pub on_pick: OnPick,
    pub on_hex_click: dialog::OnClick,
}

pub fn render(props: ColourFieldProps<'_>, cx: &App) -> AnyElement {
    let ColourFieldProps {
        id,
        form,
        on_pick,
        on_hex_click,
    } = props;
    let picked = form.picked();
    let row_focused = form.focused == TagField::Swatches;
    // The pick is outlined in ink; while the row has focus, in the accent, as a focused input is.
    let outline = if row_focused {
        color::accent(cx)
    } else {
        color::foreground(cx)
    };
    let pick = |index: usize, content: gpui::Div| {
        let on_pick = on_pick.clone();
        content
            .id(SharedString::from(format!("{id}-swatch-{index}")))
            .cursor_pointer()
            .flex_none()
            .h(SWATCH_SIZE)
            .when(picked == Some(index), |this| {
                this.border(px(2.0)).border_color(outline)
            })
            .on_click(move |_event, window, cx| on_pick(index, window, cx))
    };
    let none = pick(
        0,
        div()
            .px(px(6.0))
            .flex()
            .items_center()
            .border_1()
            .border_color(color::border(cx))
            .text_size(px(11.0))
            .text_color(color::muted(cx))
            .child(crate::msg::desktop_tags_colour_none()),
    );
    let presets = tags::swatches()
        .into_iter()
        .zip(1..)
        .map(|(preset, index)| {
            pick(
                index,
                div().w(SWATCH_SIZE).bg(super::swatch_colour(&preset)),
            )
        });

    div()
        .child(optional_label(crate::msg::desktop_tags_field_colour(), cx))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(none)
                .children(presets)
                .child(hex_box(id, form, on_hex_click, cx)),
        )
        .children(
            form.hex_invalid()
                .then(|| error_line(crate::msg::desktop_tags_error_hex(), cx)),
        )
        .into_any_element()
}

/// The free hex box: its own colour previewed ahead of the text once it parses.
fn hex_box(
    id: &'static str,
    form: &TagForm,
    on_click: dialog::OnClick,
    cx: &App,
) -> impl IntoElement {
    let focused = form.focused == TagField::Hex;
    let caret = if focused { "\u{2502}" } else { "" };
    let (text, text_color) = if form.hex.text().is_empty() {
        (
            format!("{}{caret}", crate::msg::desktop_tags_hex_placeholder()),
            color::faint_text(cx),
        )
    } else {
        (format!("{}{caret}", form.hex.text()), color::foreground(cx))
    };
    let preview = form.colour().ok().flatten();
    div()
        .id(SharedString::from(format!("{id}-hex")))
        .cursor_pointer()
        .flex_1()
        .min_w(px(0.0))
        .h(SWATCH_SIZE)
        .px(px(8.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .border_1()
        .border_color(if focused {
            color::accent(cx)
        } else {
            color::border(cx)
        })
        .text_size(px(12.0))
        .text_color(text_color)
        .on_click(move |_event, window, cx| on_click(window, cx))
        .when_some(preview, |this, colour| {
            this.child(swatch(Some(&colour), PREVIEW_SIZE))
        })
        .child(text)
}

/// A field's inline error, under it in the accent text shade.
pub fn error_line(text: impl Into<SharedString>, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::accent_text(cx))
        .child(text.into())
        .into_any_element()
}
