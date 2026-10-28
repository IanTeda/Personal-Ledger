//! Draws the Institutions page's view model (`docs/ux/desktop-mockups/16-settings/README.md`'s
//! "2a resting state"): a table (INSTITUTION / PRODUCTS) with per-row edit/delete buttons and
//! a "+ Add institution" button below it, the same shape as the Units page. Every click sends the
//! Presenter the event its key does; a row click only selects.

use std::rc::Rc;

use gpui::{
    AnyElement, App, BoxShadow, Context, Entity, MouseButton, Pixels, Point, Rgba, SharedString,
    Window, div, point, prelude::*, px,
};
use lib_institutions::presenter::{
    InstitutionLine, InstitutionsPageEvent, InstitutionsPageViewModel, MenuEntry, RowMenu,
};
use lib_locale::format::upper;

use super::InstitutionsPage;
use crate::theme::color;

type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// The context menu's width (`width:190px`), as Units' is.
const MENU_WIDTH: f32 = 190.0;
const PRODUCTS_WIDTH: Pixels = px(140.0);
const ACTIONS_WIDTH: Pixels = px(120.0);

impl Render for InstitutionsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let view = self.view_model(cx);
        let focused = self.page_focused;
        let menu_at = self.menu_at;
        let menu = view.menu.clone();
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .border_1()
                    .border_color(color::border(cx))
                    .child(table_header(&view, cx))
                    .children(view.rows.iter().enumerate().map(|(index, line)| {
                        row(
                            line,
                            &view,
                            index,
                            view.selected == Some(line.id),
                            focused,
                            &entity,
                            cx,
                        )
                    })),
            )
            .child(add_button(
                &view,
                on_event(&entity, InstitutionsPageEvent::New),
                cx,
            ))
            .children(menu.map(|menu| row_menu(menu, menu_at, &entity, cx)))
    }
}

/// A click handler that sends the page `event`, as the key that does the same.
fn on_event(entity: &Entity<InstitutionsPage>, event: InstitutionsPageEvent) -> OnPlainClick {
    let entity = entity.clone();
    Rc::new(move |_window, cx| {
        let event = event.clone();
        entity.update(cx, |page, cx| page.send(event, cx));
    })
}

fn table_header(view: &InstitutionsPageViewModel, cx: &App) -> impl IntoElement {
    let columns = &view.columns;
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(12.0))
        .bg(color::chrome(cx))
        .border_b(px(1.0))
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        .child(div().flex_1().child(upper(&columns.name)))
        .child(div().w(PRODUCTS_WIDTH).child(upper(&columns.products)))
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .text_align(gpui::TextAlign::Right)
                .child(upper(&columns.actions)),
        )
}

/// The selected row is inverted while the page has focus (`background:#201e1d; color:#f3f2f2`)
/// and only tinted while the index does.
fn row(
    line: &InstitutionLine,
    view: &InstitutionsPageViewModel,
    index: usize,
    selected: bool,
    focused: bool,
    entity: &Entity<InstitutionsPage>,
    cx: &App,
) -> impl IntoElement {
    let id = line.id;
    let last = index + 1 == view.rows.len();
    let inverted = selected && focused;
    let hover = color::hover(cx);
    let (faint, button_border): (Rgba, Rgba) = if inverted {
        (color::selection_muted(cx), color::selection_muted(cx))
    } else {
        (color::faint_text(cx), color::border(cx))
    };
    let on_row_click = {
        let entity = entity.clone();
        move |cx: &mut App| {
            entity.update(cx, |page, cx| {
                page.click(InstitutionsPageEvent::Select(id), cx)
            });
        }
    };
    div()
        .debug_selector(move || format!("settings-institutions-row-{index}"))
        .id(SharedString::from(format!(
            "settings-institutions-row-{index}"
        )))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(12.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected && !focused, |this| this.bg(color::chrome(cx)))
        .when(inverted, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        .on_click(move |_event, _window, cx| on_row_click(cx))
        .on_mouse_down(MouseButton::Right, {
            let entity = entity.clone();
            move |event, _window, cx| {
                cx.stop_propagation();
                let position = event.position;
                entity.update(cx, |page, cx| page.open_menu(id, position, cx));
            }
        })
        .child(
            div()
                .flex_1()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(line.name.clone()),
        )
        .child(
            div()
                .w(PRODUCTS_WIDTH)
                .text_color(faint)
                .child(line.products.clone()),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex()
                .justify_end()
                .gap(px(10.0))
                .child(row_action_button(
                    SharedString::from(format!("institution-edit-{index}")),
                    view.row_edit.clone(),
                    button_border,
                    on_event(entity, InstitutionsPageEvent::EditRow(id)),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("institution-delete-{index}")),
                    view.row_delete.clone(),
                    button_border,
                    on_event(entity, InstitutionsPageEvent::DeleteRow(id)),
                    cx,
                )),
        )
}

/// A row action button: `padding:4px 10px; border:1px solid rgba(32,30,29,.30);
/// background:transparent; font-size:11px`. Stops the click reaching the row.
fn row_action_button(
    id: SharedString,
    label: String,
    border: Rgba,
    on_click: OnPlainClick,
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
        .py(px(4.0))
        .px(px(10.0))
        .border_1()
        .border_color(border)
        .text_size(px(11.0))
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
}

/// The "+ Add institution" button: `padding:10px 16px; border:1px solid rgba(32,30,29,.30);
/// background:#eae9e9; font-weight:800; width:fit-content; margin-top:16px`.
fn add_button(
    view: &InstitutionsPageViewModel,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    // `align_self: flex-start` (no direct `Styled` builder for it, unlike the container-level
    // `items_start`/etc.) -- without it, this button stretches to the full width of its column
    // parent instead of shrinking to its own content, unlike `width:fit-content` in the mockup.
    let mut button = div()
        .debug_selector(|| "settings-add-institution".to_string())
        .id("settings-add-institution")
        .cursor_pointer()
        .mt(px(16.0))
        .py(px(10.0))
        .px(px(16.0))
        .bg(color::chrome(cx))
        .border_1()
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(view.add_button.clone());
    button.style().align_self = Some(gpui::AlignItems::FlexStart);
    button
}

/// The row's context menu at the pointer, drawn as Units' is: `190px`, `border:2px solid`, the
/// dialog shadow, the destructive entry last in red below a 1px rule. A press anywhere outside it
/// closes it.
fn row_menu(
    menu: RowMenu,
    at: Point<Pixels>,
    entity: &Entity<InstitutionsPage>,
    cx: &App,
) -> AnyElement {
    let hover = color::hover(cx);
    let items = menu.entries.into_iter().map(
        |MenuEntry {
             action,
             label,
             destructive,
         }| {
            let on_click = on_event(entity, InstitutionsPageEvent::Menu(action));
            let id = SharedString::from(format!("settings-institutions-menu-{}", action.slug()));
            let item = div()
                .debug_selector({
                    let id = id.clone();
                    move || id.to_string()
                })
                .id(id)
                .cursor_pointer()
                .py(px(7.0))
                .px(px(12.0))
                .text_size(px(12.5))
                .text_color(if destructive {
                    color::accent_text(cx)
                } else {
                    color::foreground(cx)
                })
                .hover(move |style| style.bg(hover).font_weight(gpui::FontWeight::EXTRA_BOLD))
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(label);
            if destructive {
                div()
                    .flex()
                    .flex_col()
                    .child(div().h(px(1.0)).my(px(4.0)).bg(color::hairline(cx)))
                    .child(item)
                    .into_any_element()
            } else {
                item.into_any_element()
            }
        },
    );
    let dismiss = on_event(entity, InstitutionsPageEvent::CloseMenu);
    gpui::deferred(
        gpui::anchored().position(at).snap_to_window().child(
            div()
                .debug_selector(|| "settings-institutions-menu".to_string())
                .id("settings-institutions-menu")
                .occlude()
                .w(px(MENU_WIDTH))
                .flex()
                .flex_col()
                .py(px(4.0))
                .bg(color::background(cx))
                .border_2()
                .border_color(color::foreground(cx))
                .shadow(vec![BoxShadow {
                    color: color::dialog_shadow(cx).into(),
                    offset: point(px(0.0), px(16.0)),
                    blur_radius: px(48.0),
                    spread_radius: px(0.0),
                }])
                .font_weight(gpui::FontWeight::NORMAL)
                .on_mouse_down_out(move |_event, window, cx| dismiss(window, cx))
                .children(items),
        ),
    )
    .with_priority(1)
    .into_any_element()
}
