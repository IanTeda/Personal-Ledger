//! The **Match rules** field the Add (6b) and Edit (6c) payee dialogs share: a chip row (a neutral
//! "no rules yet" chip while empty, otherwise one accent chip per rule with a `✕` to remove it),
//! then the monospace rule input and its **+ add** button. `Shell` owns the form and the keys;
//! `enter` in the input adds a chip rather than submitting the dialog.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{dialog, payees::PayeeError, theme::color, view::accounts::add_dialog};

/// Called with the index of the chip whose `✕` was clicked.
pub type OnRemoveRule = Rc<dyn Fn(usize, &mut Window, &mut App)>;

const MONOSPACE: &str = "monospace";

pub struct RulesFieldProps<'a> {
    /// Prefixes every element id, so the Add and Edit dialogs never share one.
    pub id: &'static str,
    pub rules: &'a [String],
    pub input: &'a str,
    pub placeholder: String,
    pub focused: bool,
    pub on_input_click: dialog::OnClick,
    pub on_add_click: dialog::OnClick,
    pub on_remove_rule: OnRemoveRule,
}

pub fn render(props: RulesFieldProps<'_>, cx: &App) -> AnyElement {
    let RulesFieldProps {
        id,
        rules,
        input,
        placeholder,
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
                .child(crate::msg::desktop_payees_no_rules())
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
        .child(add_dialog::optional_label(
            crate::msg::desktop_payees_field_match_rules(),
            cx,
        ))
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
                        .child(crate::msg::desktop_payees_rule_add("+")),
                ),
        )
        .into_any_element()
}

/// `.tag-accent`, monospace 11.5px, `padding:3px 8px`, with a trailing `✕`.
fn rule_chip(
    id: &'static str,
    index: usize,
    rule: &str,
    on_remove_rule: OnRemoveRule,
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
                .cursor_pointer()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .on_click(move |_event, window, cx| on_remove_rule(index, window, cx))
                .child("\u{2715}"),
        )
        .into_any_element()
}

/// The inline error under a field, in the accent text colour.
pub fn error_line(error: &PayeeError, cx: &App) -> AnyElement {
    let text = match error {
        PayeeError::DuplicateName(name) => crate::msg::desktop_payees_error_name_taken(name),
        PayeeError::AliasTaken { alias, owner } => {
            crate::msg::desktop_payees_error_alias_taken(alias, owner)
        }
        // Not reachable from a form: blank names only disable the confirm button, and in-use and
        // not-found come from delete and a vanished Payee.
        other => other.to_string(),
    };
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::accent_text(cx))
        .child(text)
        .into_any_element()
}
