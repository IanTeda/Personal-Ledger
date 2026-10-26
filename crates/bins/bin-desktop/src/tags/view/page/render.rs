//! Draws the Tags page (`docs/ux/desktop-mockups/16-settings/README.md`'s 2j) from the Presenter's
//! view model: a kicker (A–Z) with **+ Add tag** over one bordered list of colour swatch and name,
//! with `edit · remove` per row. A likely duplicate carries its warning badge and a direct
//! **merge**; the merge flow is 7e's. Usage lives in Transactions and Reports, so there are no
//! transaction, total or last-used columns.
//!
//! Every click is an event sent to the Presenter, so the keyboard (`n`/`e`/`x`/`m`) and the mouse
//! reach the same handlers.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Render, Rgba, SharedString, Window, div,
    prelude::*, px,
};
use lib_locale::{Segment, format::upper};
use lib_tags::presenter::{TagLine, TagsPageEvent, TagsPageViewModel};

use super::TagsPage;
use crate::{
    form::swatch::{SWATCH_SIZE, swatch},
    theme::color,
};

/// A click, run with its `Window` and `App`.
type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// A click that sends `event` to the page.
fn on(entity: &Entity<TagsPage>, event: TagsPageEvent) -> OnClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| {
        entity.update(cx, |page, cx| page.send(event.clone(), cx));
    })
}

/// The heading's meta: the count, then the duplicates clause with its number bold in the accent
/// text colour and no "merge them" link -- each flagged row has its own **merge**.
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
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .text_color(color::accent_text(cx))
                            .child(text)
                            .into_any_element(),
                        _ => div().child(text).into_any_element(),
                    }
                }),
        )
        .into_any_element()
}

impl Render for TagsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let view = self.view_model();
        let focused = self.page_focused;
        let last = view.rows.len().saturating_sub(1);
        let cx: &App = cx;
        div()
            .flex()
            .flex_col()
            .child(kicker_row(&view, &entity, cx))
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
                        .children(view.rows.iter().enumerate().map(|(position, tag)| {
                            row(
                                &view,
                                &entity,
                                tag,
                                position == last,
                                view.selected == Some(tag.id),
                                focused,
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

/// The page's `+ Add tag` button, which opens the Add dialog as `n` does.
fn kicker_row(view: &TagsPageViewModel, entity: &Entity<TagsPage>, cx: &App) -> impl IntoElement {
    let hover = color::muted(cx);
    let on_add = on(entity, TagsPageEvent::New);
    div()
        .flex()
        .items_center()
        .justify_between()
        .mb(px(10.0))
        .child(
            div()
                .font_weight(FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(upper(&view.kicker)),
        )
        // `btn btn-primary` at `height:32px`.
        .child(
            div()
                .debug_selector(|| "settings-tags-add".to_string())
                .id("settings-tags-add")
                .cursor_pointer()
                .flex_none()
                .py(px(8.0))
                .px(px(14.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(FontWeight::EXTRA_BOLD)
                .whitespace_nowrap()
                .hover(move |style| style.bg(hover))
                .on_click(move |_event, window, cx| on_add(window, cx))
                .child(view.add_button.clone()),
        )
}

/// `padding:6px 14px; background:#eae9e9`: TAG and ACTIONS only.
fn table_header(view: &TagsPageViewModel, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .px(px(14.0))
        .py(px(6.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .font_weight(FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        .child(div().flex_1().child(upper(&view.column_tag)))
        .child(div().child(upper(&view.column_actions)))
}

/// The selected row is inverted while the page has focus (`background:#201e1d; color:#f3f2f2`)
/// and only tinted while the index does. A likely duplicate takes the accent tint.
fn row(
    view: &TagsPageViewModel,
    entity: &Entity<TagsPage>,
    tag: &TagLine,
    last: bool,
    selected: bool,
    focused: bool,
    cx: &App,
) -> impl IntoElement {
    let id = tag.id;
    let flagged = tag.duplicate_of.is_some();
    let inverted = selected && focused;
    let hover = color::hover(cx);
    let (secondary, flag, button_border): (Rgba, Rgba, Rgba) = if inverted {
        (
            color::selection_muted(cx),
            color::selection_accent_text(cx),
            color::selection_muted(cx),
        )
    } else {
        (color::muted(cx), color::accent_text(cx), color::border(cx))
    };
    let on_row = {
        let entity = entity.clone();
        move |_event: &gpui::ClickEvent, _window: &mut Window, cx: &mut App| {
            entity.update(cx, |page, cx| page.click(TagsPageEvent::Select(id), cx));
        }
    };

    div()
        .debug_selector(move || format!("settings-tags-row-{id}"))
        .id(SharedString::from(format!("settings-tags-row-{id}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(14.0))
        .py(px(5.0))
        .text_size(px(12.5))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(!selected && flagged, |this| this.bg(color::accent_tint(cx)))
        .when(selected && !focused, |this| this.bg(color::chrome(cx)))
        .when(inverted, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        // An inactive Tag stays listed, dimmed and tagged, rather than hidden.
        .when(!tag.is_active, |this| this.opacity(0.55))
        .on_click(on_row)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(swatch(tag.color.as_ref(), SWATCH_SIZE))
                .child(
                    div()
                        .truncate()
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .child(tag.name.clone()),
                )
                .when(!tag.is_active, |this| {
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
                })
                .children(tag.duplicate_of.clone().map(|text| {
                    div()
                        .flex_none()
                        .ml(px(4.0))
                        .px(px(6.0))
                        .py(px(1.0))
                        .border_1()
                        .border_color(color::accent(cx))
                        .text_size(px(10.0))
                        .text_color(flag)
                        .child(text)
                })),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .when(flagged, |this| {
                    this.child(row_action_button(
                        SharedString::from(format!("settings-tags-merge-{id}")),
                        view.row_merge.clone(),
                        button_border,
                        on(entity, TagsPageEvent::MergeRow(id)),
                        cx,
                    ))
                })
                .child(row_action_button(
                    SharedString::from(format!("settings-tags-edit-{id}")),
                    view.row_edit.clone(),
                    button_border,
                    on(entity, TagsPageEvent::EditRow(id)),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("settings-tags-remove-{id}")),
                    view.row_remove.clone(),
                    button_border,
                    on(entity, TagsPageEvent::RemoveRow(id)),
                    cx,
                )),
        )
}

/// `padding:2px 6px; font-size:11px; border:1px solid`. Stops the click reaching the row.
fn row_action_button(
    id: SharedString,
    label: String,
    border: Rgba,
    on_click: OnClick,
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
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
}
