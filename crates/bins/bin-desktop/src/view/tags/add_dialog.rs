//! Renders the **Add tag** dialog (`docs/ux/desktop-mockups/19-tags/README.md`'s 7b) on the shared
//! `crate::dialog` chrome: Name with its live inline error, the shared Colour field, and a callout
//! on how names are matched. The handoff's callout offered to reuse a case-insensitive match; #354
//! settled on refusing any name equal once case, spaces and punctuation are ignored, so the callout
//! says that instead and there is no reuse button. `Shell` owns the live form (`tags::form::TagForm`)
//! and every keystroke while it is open.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use super::colour_field::{self, ColourFieldProps, OnPick, error_line};
use crate::{
    dialog,
    tags::{
        TagError,
        form::{TagField, TagForm},
    },
    theme::color,
    view::accounts::add_dialog::{label, text_field},
};

pub type OnFieldClick = Rc<dyn Fn(TagField, &mut Window, &mut App)>;

pub struct TagDialogHandlers {
    pub on_field_click: OnFieldClick,
    pub on_pick: OnPick,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub fn render(
    form: &TagForm,
    name_error: Option<TagError>,
    valid: bool,
    handlers: TagDialogHandlers,
    cx: &App,
) -> AnyElement {
    let TagDialogHandlers {
        on_field_click,
        on_pick,
        on_cancel,
        on_confirm,
    } = handlers;
    let click = |field: TagField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_tags_add_title(),
            false,
            cx,
        ))
        .child(dialog::body([
            div()
                .child(text_field(
                    "add-tag-name",
                    label(lib_locale::msg::column_name()),
                    form.name.text(),
                    &crate::msg::desktop_tags_name_placeholder(),
                    form.focused == TagField::Name,
                    click(TagField::Name),
                    cx,
                ))
                .children(name_error.map(|error| error_line(name_error_text(&error), cx)))
                .into_any_element(),
            colour_field::render(
                ColourFieldProps {
                    id: "add-tag",
                    form,
                    on_pick,
                    on_hex_click: click(TagField::Hex),
                },
                cx,
            ),
            callout(crate::msg::desktop_tags_add_callout(), cx),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("add-tag-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "add-tag-confirm",
                    crate::msg::desktop_tags_add_submit(),
                    valid,
                    false,
                    on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(dialog::WIDTH, false, card, cx)
}

/// A name error as the dialog words it.
pub fn name_error_text(error: &TagError) -> String {
    match error {
        TagError::DuplicateName(name) => crate::msg::desktop_tags_error_name_taken(name),
        TagError::NoLetterOrDigit => crate::msg::desktop_tags_error_no_letter_or_digit(),
        // Not reachable from a form: a blank name only disables the confirm button, and the rest
        // come from merge and a vanished Tag.
        other => other.to_string(),
    }
}

/// The handoff's dark-bordered note.
pub fn callout(text: String, cx: &App) -> AnyElement {
    div()
        .p(px(10.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::foreground(cx))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}
