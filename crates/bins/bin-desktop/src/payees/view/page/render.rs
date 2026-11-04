//! Draws the Payees page's view model (`docs/ux/desktop-mockups/16-settings/README.md`, to the 2j
//! pattern): a kicker (A–Z) with **+ Add payee** over one bordered list of name, Default category
//! and match rules, with `edit · delete` per row. Every click sends the Presenter the event its key
//! does; a row click only selects.

use gpui::{AnyElement, App, ClickEvent, Context, Rgba, SharedString, Window, div, prelude::*, px};
use lib_locale::Segment;
use lib_locale::format::upper;
use lib_payees::presenter::{PayeeLine, PayeesPageEvent, PayeesPageViewModel};

use super::PayeesPage;
use crate::{shell::view::page_clicks::PageClicks, theme::color};

/// The NAME column's floor, so the fixed columns never squeeze it to nothing.
const NAME_MIN_WIDTH: gpui::Pixels = px(140.0);
const CATEGORY_WIDTH: gpui::Pixels = px(150.0);
const RULES_WIDTH: gpui::Pixels = px(90.0);

/// The heading's meta: the count, then the without-default clause with its number bold in the
/// accent text colour.
pub(super) fn scope_note(scope: &[Segment], cx: &App) -> AnyElement {
    div()
        .flex()
        .items_baseline()
        .gap(px(4.0))
        .children(
            scope
                .iter()
                .filter(|segment| !segment.text.trim().is_empty())
                .map(|segment| {
                    let text = segment.text.trim().to_string();
                    match segment.tag.as_deref() {
                        Some("strong") => div()
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .text_color(color::accent_text(cx))
                            .child(text)
                            .into_any_element(),
                        _ => div().child(text).into_any_element(),
                    }
                }),
        )
        .into_any_element()
}

impl Render for PayeesPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let clicks = PageClicks::new(cx);
        let view = self.view_model(cx);
        let focused = self.page_focused;
        let last = view.rows.len().saturating_sub(1);
        div()
            .flex()
            .flex_col()
            .child(kicker_row(&view, &clicks, cx))
            .children(view.empty.clone().map(|empty| {
                div()
                    .text_color(color::muted(cx))
                    .child(empty)
                    .into_any_element()
            }))
            .when(!view.rows.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .border_1()
                        .border_color(color::border(cx))
                        .mb(px(18.0))
                        .child(table_header(&view, cx))
                        .children(view.rows.iter().enumerate().map(|(position, line)| {
                            row(
                                line,
                                &view,
                                position == last,
                                view.selected == Some(line.id),
                                focused,
                                &clicks,
                                cx,
                            )
                        })),
                )
            })
            .child(
                div()
                    .max_w(px(620.0))
                    .text_size(px(11.5))
                    .text_color(color::muted(cx))
                    .child(view.footnote.clone()),
            )
    }
}

fn kicker_row(
    view: &PayeesPageViewModel,
    clicks: &PageClicks<PayeesPage>,
    cx: &App,
) -> impl IntoElement {
    let on_add = clicks.send(PayeesPageEvent::New);
    let hover = color::muted(cx);
    div()
        .flex()
        .items_center()
        .justify_between()
        .mb(px(10.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(upper(&view.kicker)),
        )
        // `btn btn-primary` at `height:32px`.
        .child(
            div()
                .debug_selector(|| "settings-payees-add".to_string())
                .id("settings-payees-add")
                .cursor_pointer()
                .flex_none()
                .py(px(8.0))
                .px(px(14.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .whitespace_nowrap()
                .hover(move |style| style.bg(hover))
                .on_click(on_add)
                .child(view.add_button.clone()),
        )
}

/// `padding:6px 14px; background:#eae9e9`: NAME, DEFAULT CATEGORY, MATCH RULES and ACTIONS.
fn table_header(view: &PayeesPageViewModel, cx: &App) -> impl IntoElement {
    let columns = &view.columns;
    div()
        .flex()
        .px(px(14.0))
        .py(px(6.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        .child(
            div()
                .flex_1()
                .min_w(NAME_MIN_WIDTH)
                .child(upper(&columns.name)),
        )
        .child(
            div()
                .w(CATEGORY_WIDTH)
                .flex_none()
                .child(upper(&columns.default_category)),
        )
        .child(
            div()
                .w(RULES_WIDTH)
                .flex_none()
                .child(upper(&columns.match_rules)),
        )
        .child(div().child(upper(&columns.actions)))
}

/// The selected row is inverted while the page has focus (`background:#201e1d; color:#f3f2f2`)
/// and only tinted while the index does.
fn row(
    line: &PayeeLine,
    view: &PayeesPageViewModel,
    last: bool,
    selected: bool,
    focused: bool,
    clicks: &PageClicks<PayeesPage>,
    cx: &App,
) -> impl IntoElement {
    let id = line.id;
    let inverted = selected && focused;
    let hover = color::hover(cx);
    let (secondary, faint, missing, button_border): (Rgba, Rgba, Rgba, Rgba) = if inverted {
        (
            color::selection_muted(cx),
            color::selection_muted(cx),
            color::selection_accent_text(cx),
            color::selection_muted(cx),
        )
    } else {
        (
            color::muted(cx),
            color::faint_text(cx),
            color::accent_text(cx),
            color::border(cx),
        )
    };
    let on_row_click = clicks.row(PayeesPageEvent::Select(id));

    div()
        .debug_selector(move || format!("settings-payees-row-{id}"))
        .id(SharedString::from(format!("settings-payees-row-{id}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(14.0))
        .py(px(5.0))
        .text_size(px(12.5))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected && !focused, |this| this.bg(color::chrome(cx)))
        .when(inverted, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        // An inactive Payee stays listed, dimmed and tagged, rather than hidden.
        .when(!line.is_active, |this| this.opacity(0.55))
        .on_click(on_row_click)
        .child(
            div()
                .flex_1()
                .min_w(NAME_MIN_WIDTH)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .truncate()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child(line.name.clone()),
                )
                .when(!line.is_active, |this| {
                    this.child(
                        div()
                            .flex_none()
                            .px(px(6.0))
                            .py(px(1.0))
                            .border_1()
                            .border_color(button_border)
                            .text_size(px(10.0))
                            .text_color(secondary)
                            .child(view.inactive.clone()),
                    )
                }),
        )
        .child(div().w(CATEGORY_WIDTH).flex_none().truncate().map(|this| {
            match &line.default_category {
                Some(category) => this.text_color(secondary).child(category.clone()),
                None => this
                    .text_color(missing)
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .child(view.no_default_category.clone()),
            }
        }))
        .child(
            div()
                .w(RULES_WIDTH)
                .flex_none()
                .text_color(faint)
                .child(line.rules.clone()),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(row_action_button(
                    SharedString::from(format!("settings-payees-edit-{id}")),
                    view.row_edit.clone(),
                    button_border,
                    clicks.send(PayeesPageEvent::EditRow(id)),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("settings-payees-delete-{id}")),
                    view.row_delete.clone(),
                    button_border,
                    clicks.send(PayeesPageEvent::DeleteRow(id)),
                    cx,
                )),
        )
}

/// `padding:2px 6px; font-size:11px; border:1px solid`. Stops the click reaching the row.
fn row_action_button(
    id: SharedString,
    label: String,
    border: Rgba,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .py(px(2.0))
        .px(px(6.0))
        .border_1()
        .border_color(border)
        .text_size(px(11.0))
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |event: &ClickEvent, window, cx| {
            cx.stop_propagation();
            on_click(event, window, cx);
        })
        .child(label)
}
