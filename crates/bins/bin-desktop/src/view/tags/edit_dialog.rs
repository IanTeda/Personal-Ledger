//! Renders the **Edit tag** dialog (`docs/ux/desktop/handoff_tags/README.md`'s 7c) on the shared
//! `crate::dialog` chrome, reusing the Add dialog's Name field, Colour field and callout. #353 adds
//! the Active checkbox the handoff lacks: removing a Tag always deletes it, so Edit is where a Tag
//! is deactivated and reactivated. Splits refer to a Tag by id, so the callout's count is every
//! Transaction a rename or recolour reaches.

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*, px};

use super::{
    add_dialog::{OnFieldClick, TagDialogHandlers, callout, name_error_text},
    colour_field::{self, ColourFieldProps, error_line},
};
use crate::{
    dialog,
    tags::{TagError, TagField, TagForm},
    theme::color,
    view::accounts::add_dialog::{label, text_field},
};

pub struct EditTagProps<'a> {
    /// The Tag's name before this edit, for the title.
    pub original_name: &'a str,
    pub form: &'a TagForm,
    pub name_error: Option<TagError>,
    pub valid: bool,
    /// Transactions carrying the Tag, for the callout.
    pub transactions: usize,
    pub on_toggle_active: dialog::OnClick,
}

pub fn render(props: EditTagProps<'_>, handlers: TagDialogHandlers, cx: &App) -> AnyElement {
    let TagDialogHandlers {
        on_field_click,
        on_pick,
        on_cancel,
        on_confirm,
    } = handlers;
    let form = props.form;
    let click = |field: TagField| -> dialog::OnClick {
        let on_field_click = on_field_click.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| on_field_click(field, window, cx))
    };
    let count = i64::try_from(props.transactions).unwrap_or(i64::MAX);
    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_tags_edit_title(props.original_name),
            false,
            cx,
        ))
        .child(dialog::body([
            div()
                .child(text_field(
                    "edit-tag-name",
                    label(lib_locale::msg::column_name()),
                    &form.name,
                    &crate::msg::desktop_tags_name_placeholder(),
                    form.focused == TagField::Name,
                    click(TagField::Name),
                    cx,
                ))
                .children(
                    props
                        .name_error
                        .map(|error| error_line(name_error_text(&error), cx)),
                )
                .into_any_element(),
            colour_field::render(
                ColourFieldProps {
                    id: "edit-tag",
                    form,
                    on_pick,
                    on_hex_click: click(TagField::Hex),
                },
                cx,
            ),
            active_checkbox(form, props.on_toggle_active, &on_field_click, cx),
            callout(crate::msg::desktop_tags_edit_callout(count), cx),
        ]))
        .child(dialog::action_row(
            [
                dialog::cancel_button("edit-tag-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "edit-tag-confirm",
                    crate::msg::desktop_tags_edit_submit(),
                    props.valid,
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

/// The Active checkbox, drawn like the Import footer's: a 12px box, filled with a tick when on,
/// underlined while it has keyboard focus.
fn active_checkbox(
    form: &TagForm,
    on_toggle: dialog::OnClick,
    on_field_click: &OnFieldClick,
    cx: &App,
) -> AnyElement {
    let focused = form.focused == TagField::Active;
    let on_field_click = on_field_click.clone();
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .id("edit-tag-active")
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(8.0))
                .on_click(move |_event, window, cx| {
                    on_field_click(TagField::Active, window, cx);
                    on_toggle(window, cx);
                })
                .child(
                    div()
                        .size(px(12.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_1()
                        .border_color(color::foreground(cx))
                        .when(form.is_active, |this| {
                            this.bg(color::foreground(cx))
                                .text_color(color::background(cx))
                                .text_size(px(9.0))
                                .child("\u{2713}")
                        }),
                )
                .child(
                    div()
                        .text_color(color::foreground(cx))
                        .when(focused, |this| this.underline())
                        .child(crate::msg::desktop_tags_field_active()),
                ),
        )
        .child(
            div()
                .pl(px(20.0))
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_tags_active_hint()),
        )
        .into_any_element()
}
