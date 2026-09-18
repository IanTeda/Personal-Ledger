//! The dialogs' shared dropdown (the Desktop Accounts map's "select control" decision): a closed
//! field showing the current value and a `▾`, opening a list **inline** beneath it -- pushing the
//! fields below down rather than floating over them, so nothing clips at the dialog's edge. The
//! list shows at most [`crate::select::MAX_VISIBLE_ROWS`] rows and scrolls with the highlight.
//!
//! A pure render-helper over `crate::select::SelectState`; `Shell` owns the state and the keys.
//! A read-only variant shows fixed text and takes no clicks (Institution while Type is Cash).

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{dialog, select::SelectState, theme::color};

pub type OnOptionClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;

pub struct SelectFieldProps<'a> {
    pub id: &'static str,
    pub label: SharedString,
    pub options: &'a [String],
    pub state: &'a SelectState,
    pub focused: bool,
    /// `Some` makes the field read-only, showing this text instead of the value.
    pub read_only: Option<&'a str>,
    pub on_field_click: dialog::OnClick,
    pub on_option_click: OnOptionClick,
}

pub fn render(props: SelectFieldProps<'_>) -> AnyElement {
    let SelectFieldProps {
        id,
        label,
        options,
        state,
        focused,
        read_only,
        on_field_click,
        on_option_click,
    } = props;

    let text = read_only
        .or_else(|| state.value())
        .unwrap_or("")
        .to_string();

    let mut field = div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .w_full()
        .py(px(8.0))
        .px(px(10.0))
        .border_1()
        .border_color(if focused && read_only.is_none() {
            color::ACCENT
        } else {
            color::BORDER
        })
        .text_size(px(13.0))
        .child(div().min_w(px(0.0)).truncate().child(text))
        .child(
            div()
                .flex_none()
                .text_size(px(10.0))
                .text_color(color::INK_TERTIARY)
                .child(if state.is_open() {
                    "\u{25b4}"
                } else {
                    "\u{25be}"
                }),
        );
    field = if read_only.is_some() {
        field.bg(color::CHROME).text_color(color::INK_TERTIARY)
    } else {
        field
            .bg(color::GROUND)
            .cursor_pointer()
            .on_click(move |_event, window, cx| on_field_click(window, cx))
    };

    div()
        .flex_1()
        .min_w(px(0.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .mb(px(6.0))
                .child(label),
        )
        .child(field)
        .when(state.is_open() && read_only.is_none(), |this| {
            this.child(option_list(id, options, state, on_option_click))
        })
        .into_any_element()
}

fn option_list(
    id: &'static str,
    options: &[String],
    state: &SelectState,
    on_option_click: OnOptionClick,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("{id}-list")))
        .flex()
        .flex_col()
        .bg(color::GROUND)
        .border_1()
        .border_t_0()
        .border_color(color::BORDER)
        .children(state.visible_range(options.len()).map(|index| {
            let highlighted = index == state.highlight();
            let current = state.value() == Some(options[index].as_str());
            let on_option_click = on_option_click.clone();
            div()
                .id(SharedString::from(format!("{id}-option-{index}")))
                .cursor_pointer()
                .py(px(6.0))
                .px(px(10.0))
                .text_size(px(13.0))
                .truncate()
                .when(highlighted, |this| {
                    this.bg(color::INK).text_color(color::INK_ON_DARK)
                })
                .when(current && !highlighted, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .on_click(move |_event, window, cx| on_option_click(index, window, cx))
                .child(options[index].clone())
        }))
}
