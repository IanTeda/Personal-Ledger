//! Renders the **Remove tag** dialog (`docs/ux/desktop/Tags/README.md`'s 7d) on the shared
//! `crate::dialog` chrome at the handoff's 400px. #353 settled that removing always deletes the
//! Tag, used or not, and keeps the neutral (not red) styling the handoff gives it: nothing but a
//! label is lost. A used Tag still sits behind the typed-name confirm, since the untagging can't
//! be undone; an unused one needs only **Remove tag**.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};

use crate::{
    dialog,
    tags::{RemoveTagForm, Tag},
    theme::color,
};

/// The dialog's width: the handoff's 7d.
pub const WIDTH: gpui::Pixels = px(400.0);

pub struct RemoveTagProps<'a> {
    pub tag: &'a Tag,
    pub form: &'a RemoveTagForm,
    /// Transactions carrying the Tag.
    pub transactions: usize,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

pub fn render(props: RemoveTagProps<'_>, cx: &App) -> AnyElement {
    let RemoveTagProps {
        tag,
        form,
        transactions,
        on_cancel,
        on_confirm,
    } = props;
    let mut fields = vec![warning_copy(&tag.name, transactions)];
    if transactions > 0 {
        fields.push(confirm_input_field(&tag.name, &form.confirmation_name, cx));
    }

    let card = div()
        .flex()
        .flex_col()
        .child(dialog::header(
            crate::msg::desktop_tags_remove_title(&tag.name),
            false,
            cx,
        ))
        .child(dialog::body(fields))
        .child(dialog::action_row(
            [
                dialog::cancel_button("remove-tag-cancel", on_cancel, cx).into_any_element(),
                dialog::confirm_button(
                    "remove-tag-confirm",
                    crate::msg::desktop_tags_remove_submit(),
                    form.allows(&tag.name, transactions),
                    false,
                    on_confirm,
                    cx,
                )
                .into_any_element(),
            ],
            cx,
        ));

    dialog::overlay(WIDTH, false, card, cx)
}

fn warning_copy(name: &str, transactions: usize) -> AnyElement {
    let segments = if transactions == 0 {
        crate::msg::desktop_tags_remove_unused(name)
    } else {
        crate::msg::desktop_tags_remove_used(name, i64::try_from(transactions).unwrap_or(i64::MAX))
    };
    div()
        .text_size(px(13.0))
        .child(dialog::rich_text(segments, None))
        .into_any_element()
}

fn confirm_input_field(name: &str, input_value: &str, cx: &App) -> AnyElement {
    let (text, text_color) = if input_value.is_empty() {
        (name.to_string(), color::muted(cx))
    } else {
        (input_value.to_string(), color::foreground(cx))
    };
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(crate::msg::desktop_tags_remove_confirm_label(name)),
        )
        .child(
            div()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(color::border(cx))
                .text_size(px(13.0))
                .text_color(text_color)
                .child(SharedString::from(text)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_used_warning_emphasises_the_name_and_the_transaction_count() {
        crate::locale::init_for_tests();
        let strong: Vec<String> = crate::msg::desktop_tags_remove_used("travel", 4)
            .into_iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .map(|segment| segment.text)
            .collect();
        assert_eq!(strong, ["travel", "4 transactions"]);
    }
}
