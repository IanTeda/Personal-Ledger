//! Renders the **Switcher** popover (`docs/ux/desktop-mockups/14-budgets-v2/README.md`'s 11b): a 380px card
//! anchored under the page title over a dimmed page, listing the active Budgets with their health
//! line and method tag, then the archived ones dimmed, with `+ New budget` and `Manage budgets…`
//! in the footer. `Shell` owns the state (`budgets::Switcher`) and every keystroke.

use std::rc::Rc;

use gpui::{AnyElement, App, BoxShadow, Pixels, SharedString, Window, div, point, prelude::*, px};
use lib_locale::format::upper;

use crate::{
    budgets::{Method, Switcher},
    dialog,
    theme::color,
};

/// The card's width: the handoff's 11b.
const WIDTH: Pixels = px(380.0);

/// Called with the clicked Budget's id.
pub type OnBudgetClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;

/// One Budget, already worded.
pub struct SwitcherRow {
    pub id: u32,
    pub name: String,
    pub is_default: bool,
    /// The Budget the page is showing: its row carries the check.
    pub is_current: bool,
    pub archived: bool,
    /// The health summary, or "archived <date>".
    pub summary: String,
    pub method: Method,
}

pub struct SwitcherProps<'a> {
    pub state: &'a Switcher,
    /// The rows matching the search, active first.
    pub rows: Vec<SwitcherRow>,
    /// The card's top-left corner in window coordinates: under the page title.
    pub left: Pixels,
    pub top: Pixels,
    pub on_search_click: dialog::OnClick,
    pub on_budget_click: OnBudgetClick,
    pub on_new: dialog::OnClick,
    pub on_manage: dialog::OnClick,
    /// A click outside the card.
    pub on_cancel: dialog::OnClick,
}

fn row(
    index: usize,
    row: SwitcherRow,
    selected: bool,
    on_click: &OnBudgetClick,
    cx: &App,
) -> AnyElement {
    let on_click = on_click.clone();
    let id = row.id;
    let (primary, secondary) = if selected {
        (color::selection_text(cx), color::selection_muted(cx))
    } else {
        (color::foreground(cx), color::muted(cx))
    };
    let hover = color::hover(cx);
    div()
        .id(SharedString::from(format!("budgets-switcher-{index}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(14.0))
        .py(px(9.0))
        .text_color(primary)
        .when(row.archived && !selected, |this| this.opacity(0.55))
        .when(selected, |this| this.bg(color::selection_background(cx)))
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        .on_click(move |_event, window, cx| on_click(id, window, cx))
        .child(
            div()
                .w(px(12.0))
                .flex_none()
                .child(if row.is_current { "\u{2713}" } else { "" }),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(8.0))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .truncate()
                                .child(row.name),
                        )
                        .when(row.is_default, |this| {
                            this.child(
                                div()
                                    .flex_none()
                                    .text_size(px(10.0))
                                    .text_color(secondary)
                                    .child(crate::msg::desktop_budgets_switcher_default()),
                            )
                        }),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(secondary)
                        .truncate()
                        .child(row.summary),
                ),
        )
        .child(
            div()
                .flex_none()
                .py(px(3.0))
                .px(px(6.0))
                .border_1()
                .border_color(secondary)
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(upper(&super::method_label(row.method))),
        )
        .into_any_element()
}

fn footer_button(
    id: &'static str,
    label: String,
    divided: bool,
    on_click: dialog::OnClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .id(id)
        .cursor_pointer()
        .flex_1()
        .py(px(10.0))
        .px(px(14.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.5))
        .when(divided, |this| {
            this.border_l(px(1.0)).border_color(color::border(cx))
        })
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(label)
}

pub fn render(props: SwitcherProps<'_>, cx: &App) -> AnyElement {
    let SwitcherProps {
        state,
        rows,
        left,
        top,
        on_search_click,
        on_budget_click,
        on_new,
        on_manage,
        on_cancel,
    } = props;

    let caret = if state.searching { "\u{2502}" } else { "" };
    let (search_text, search_colour) = if state.query.text().is_empty() {
        (
            format!("{}{caret}", crate::msg::desktop_budgets_switcher_search()),
            color::faint_text(cx),
        )
    } else {
        (
            format!("{}{caret}", state.query.text()),
            color::foreground(cx),
        )
    };
    let search = div().px(px(14.0)).py(px(12.0)).child(
        div()
            .id("budgets-switcher-search")
            .cursor_pointer()
            .w_full()
            .py(px(8.0))
            .px(px(10.0))
            .border_1()
            .border_color(if state.searching {
                color::accent(cx)
            } else {
                color::border(cx)
            })
            .text_size(px(13.0))
            .text_color(search_colour)
            .on_click(move |_event, window, cx| on_search_click(window, cx))
            .child(search_text),
    );

    let selected = state.selected.min(rows.len().saturating_sub(1));
    let first_archived = rows.iter().position(|row| row.archived);
    let mut list: Vec<AnyElement> = Vec::new();
    if rows.is_empty() {
        list.push(
            div()
                .px(px(14.0))
                .py(px(10.0))
                .text_size(px(12.5))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_budgets_switcher_none())
                .into_any_element(),
        );
    }
    for (index, each) in rows.into_iter().enumerate() {
        if first_archived == Some(index) {
            list.push(
                div()
                    .px(px(14.0))
                    .pt(px(10.0))
                    .pb(px(4.0))
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_color(color::muted(cx))
                    .child(upper(&crate::msg::desktop_budgets_switcher_archived()))
                    .into_any_element(),
            );
        }
        list.push(row(index, each, index == selected, &on_budget_click, cx));
    }

    let card = div()
        .id("budgets-switcher")
        .absolute()
        .left(left)
        .top(top)
        .w(WIDTH)
        .flex()
        .flex_col()
        .bg(color::background(cx))
        .border(px(2.0))
        .border_color(color::foreground(cx))
        .shadow(vec![BoxShadow {
            color: color::dialog_shadow(cx).into(),
            offset: point(px(6.0), px(6.0)),
            blur_radius: px(0.0),
            spread_radius: px(0.0),
        }])
        // A click inside the card must not reach the cancelling layer beneath it.
        .on_click(|_event, _window, cx| cx.stop_propagation())
        .child(search)
        .child(div().flex().flex_col().pb(px(6.0)).children(list))
        .child(
            div()
                .flex()
                .border_t(px(1.0))
                .border_color(color::border(cx))
                .child(footer_button(
                    "budgets-switcher-new",
                    crate::msg::desktop_budgets_switcher_new(),
                    false,
                    on_new,
                    cx,
                ))
                .child(footer_button(
                    "budgets-switcher-manage",
                    crate::msg::desktop_budgets_switcher_manage(),
                    true,
                    on_manage,
                    cx,
                )),
        );

    div()
        .id("budgets-switcher-layer")
        .absolute()
        .inset_0()
        .bg(color::scrim(cx))
        .on_click(move |_event, window, cx| on_cancel(window, cx))
        .child(card)
        .into_any_element()
}
