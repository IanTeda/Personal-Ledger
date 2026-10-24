//! The **Match rules** field the Add (6b) and Edit (6c) payee Dialogs share: a chip row (a neutral
//! "no rules yet" chip while empty, otherwise one accent chip per rule with a `✕` to remove it),
//! then the monospace rule input and its **+ add** button.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};

use crate::shell::view::dialog_entity::OnChoose;
use crate::{dialog, form::dialog_fields, theme::color};

const MONOSPACE: &str = "monospace";

pub(super) struct RulesFieldProps<'a> {
    /// Prefixes every element id, so the Add and Edit Dialogs never share one.
    pub id: &'static str,
    pub label: String,
    pub rules: &'a [String],
    pub no_rules: String,
    pub input: &'a str,
    pub placeholder: String,
    pub add: String,
    pub focused: bool,
    pub on_input_click: dialog::OnClick,
    pub on_add_click: dialog::OnClick,
    /// Called with the index of the chip whose `✕` was clicked.
    pub on_remove_rule: OnChoose<usize>,
}

pub(super) fn render(props: RulesFieldProps<'_>, cx: &App) -> AnyElement {
    let RulesFieldProps {
        id,
        label,
        rules,
        no_rules,
        input,
        placeholder,
        add,
        focused,
        on_input_click,
        on_add_click,
        on_remove_rule,
    } = props;

    let chips = if rules.is_empty() {
        vec![
            div()
                .px(px(8.0))
                .py(px(3.0))
                .bg(color::chrome(cx))
                .border_1()
                .border_color(color::border(cx))
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(no_rules)
                .into_any_element(),
        ]
    } else {
        rules
            .iter()
            .enumerate()
            .map(|(index, rule)| rule_chip(id, index, rule, on_remove_rule.clone(), cx))
            .collect()
    };

    let caret = if focused { "\u{2502}" } else { "" };
    let (text, text_color) = if input.is_empty() {
        (format!("{placeholder}{caret}"), color::faint_text(cx))
    } else {
        (format!("{input}{caret}"), color::foreground(cx))
    };
    let hover = color::hover(cx);

    div()
        .child(dialog_fields::optional_label(label, cx))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(6.0))
                .mb(px(8.0))
                .children(chips),
        )
        .child(
            div()
                .flex()
                .gap(px(6.0))
                .child(
                    div()
                        .id(SharedString::from(format!("{id}-rule-input")))
                        .debug_selector(move || format!("{id}-rule-input"))
                        .cursor_pointer()
                        .flex_1()
                        .min_w(px(0.0))
                        .py(px(8.0))
                        .px(px(10.0))
                        .border_1()
                        .border_color(if focused {
                            color::accent(cx)
                        } else {
                            color::border(cx)
                        })
                        .font_family(MONOSPACE)
                        .text_size(px(12.5))
                        .text_color(text_color)
                        .truncate()
                        .on_click(move |_event, window, cx| on_input_click(window, cx))
                        .child(text),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("{id}-rule-add")))
                        .debug_selector(move || format!("{id}-rule-add"))
                        .cursor_pointer()
                        .flex_none()
                        .py(px(8.0))
                        .px(px(12.0))
                        .border_1()
                        .border_color(color::border(cx))
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .hover(move |style| style.bg(hover))
                        .on_click(move |_event, window, cx| on_add_click(window, cx))
                        .child(add),
                ),
        )
        .into_any_element()
}

/// `.tag-accent`, monospace 11.5px, `padding:3px 8px`, with a trailing `✕`.
fn rule_chip(
    id: &'static str,
    index: usize,
    rule: &str,
    on_remove_rule: OnChoose<usize>,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(8.0))
        .py(px(3.0))
        .bg(color::accent(cx))
        .text_color(color::selection_text(cx))
        .font_family(MONOSPACE)
        .text_size(px(11.5))
        .child(rule.to_string())
        .child(
            div()
                .id(SharedString::from(format!("{id}-rule-remove-{index}")))
                .debug_selector(move || format!("{id}-rule-remove-{index}"))
                .cursor_pointer()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .on_click(move |_event, window, cx| on_remove_rule(index, window, cx))
                .child("\u{2715}"),
        )
        .into_any_element()
}

/// The inline error under a field, in the accent text colour.
pub(super) fn error_line(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::accent_text(cx))
        .child(text)
        .into_any_element()
}
