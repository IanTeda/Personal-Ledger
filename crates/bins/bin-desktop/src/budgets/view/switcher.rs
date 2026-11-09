//! Draws the **Switcher** popover (`docs/ux/desktop-mockups/14-budgets-v2/README.md`'s 11b) from
//! its view model: a 380px card anchored under the page title over a dimmed page, listing the
//! active Budgets with their health line and method tag, then the archived ones dimmed, with
//! `+ New budget` and `Manage budgets…` in the footer.

use gpui::{AnyElement, App, BoxShadow, Pixels, SharedString, div, point, prelude::*, px};
use lib_budgets::presenter::{SwitcherEvent, SwitcherPresenter, SwitcherRow, SwitcherViewModel};
use lib_locale::format::upper;
use lib_shell::Hint;

use crate::dialog::OnClick;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialogClicks};
use crate::theme::color;

/// The Switcher card's top below the window's: under the top bar, the page's top padding and
/// the title line.
const SWITCHER_TOP: Pixels = px(122.0);

/// The open Switcher's clicks.
type Clicks = ModuleDialogClicks<SwitcherPresenter>;

/// Where the Switcher card starts: the page's left edge, which the Shell sets as it lays the
/// rails out, so the card sits under the page title.
#[derive(Default)]
pub struct SwitcherWidgets {
    pub left: Pixels,
}

/// The `lib-budgets` Switcher Presenter's [`DialogView`], hosted in a `ModuleDialog` (ADR-0056).
impl DialogView for SwitcherPresenter {
    type Widgets = SwitcherWidgets;

    fn kind(_view: &SwitcherViewModel) -> &'static str {
        "Switcher"
    }

    fn render(
        view: &SwitcherViewModel,
        widgets: &mut SwitcherWidgets,
        clicks: &Clicks,
        cx: &App,
    ) -> AnyElement {
        render(view, widgets.left, clicks, cx)
    }

    /// The Budgets listed, the highlighted one and the search.
    fn readback(view: &SwitcherViewModel) -> Vec<(&'static str, String)> {
        let selected = view
            .selected
            .and_then(|index| view.rows.get(index))
            .map(|row| row.name.clone())
            .unwrap_or_default();
        let names = view
            .rows
            .iter()
            .map(|row| row.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        vec![
            ("Budgets", names),
            ("Selected", selected),
            ("Search", view.search.clone()),
        ]
    }

    fn hints(view: &SwitcherViewModel) -> Vec<Hint> {
        view.hints.clone()
    }
}

/// The card's width: the handoff's 11b.
const WIDTH: Pixels = px(380.0);

fn row(
    index: usize,
    row: &SwitcherRow,
    selected: bool,
    default_label: &str,
    on_click: OnClick,
    cx: &App,
) -> AnyElement {
    let (primary, secondary) = if selected {
        (color::selection_text(cx), color::selection_muted(cx))
    } else {
        (color::foreground(cx), color::muted(cx))
    };
    let hover = color::hover(cx);
    div()
        .id(SharedString::from(format!("budgets-switcher-{index}")))
        .debug_selector(move || format!("budgets-switcher-{index}"))
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
        .on_click(move |_event, window, cx| on_click(window, cx))
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
                                .child(row.name.clone()),
                        )
                        .when(row.is_default, |this| {
                            this.child(
                                div()
                                    .flex_none()
                                    .text_size(px(10.0))
                                    .text_color(secondary)
                                    .child(default_label.to_string()),
                            )
                        }),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(secondary)
                        .truncate()
                        .child(row.summary.clone()),
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
                .child(upper(&row.method)),
        )
        .into_any_element()
}

fn footer_button(
    id: &'static str,
    label: String,
    divided: bool,
    on_click: OnClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .id(id)
        .debug_selector(move || id.to_string())
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

fn render(view: &SwitcherViewModel, left: Pixels, clicks: &Clicks, cx: &App) -> AnyElement {
    let on_search = clicks.send(SwitcherEvent::Search);
    let on_cancel = clicks.send(SwitcherEvent::Cancel);
    let on_new = clicks.send(SwitcherEvent::New);
    let on_manage = clicks.send(SwitcherEvent::Manage);
    let app: &App = cx;

    let caret = if view.searching { "\u{2502}" } else { "" };
    let search_colour = if view.search_empty {
        color::faint_text(app)
    } else {
        color::foreground(app)
    };
    let search = div().px(px(14.0)).py(px(12.0)).child(
        div()
            .id("budgets-switcher-search")
            .debug_selector(|| "budgets-switcher-search".to_string())
            .cursor_pointer()
            .w_full()
            .py(px(8.0))
            .px(px(10.0))
            .border_1()
            .border_color(if view.searching {
                color::accent(app)
            } else {
                color::border(app)
            })
            .text_size(px(13.0))
            .text_color(search_colour)
            .on_click(move |_event, window, cx| on_search(window, cx))
            .child(format!("{}{caret}", view.search)),
    );

    let mut list: Vec<AnyElement> = Vec::new();
    if let Some(none) = &view.none {
        list.push(
            div()
                .px(px(14.0))
                .py(px(10.0))
                .text_size(px(12.5))
                .text_color(color::muted(app))
                .child(none.clone())
                .into_any_element(),
        );
    }
    for (index, each) in view.rows.iter().enumerate() {
        if view.archived_from == Some(index) {
            list.push(
                div()
                    .px(px(14.0))
                    .pt(px(10.0))
                    .pb(px(4.0))
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_color(color::muted(app))
                    .child(upper(&view.archived_label))
                    .into_any_element(),
            );
        }
        let on_click = clicks.send(SwitcherEvent::ChooseBudget(each.id));
        list.push(row(
            index,
            each,
            view.selected == Some(index),
            &view.default_label,
            on_click,
            app,
        ));
    }

    let card = div()
        .id("budgets-switcher")
        .debug_selector(|| "budgets-switcher".to_string())
        .absolute()
        .left(left)
        .top(SWITCHER_TOP)
        .w(WIDTH)
        .flex()
        .flex_col()
        .bg(color::background(app))
        .border(px(2.0))
        .border_color(color::foreground(app))
        .shadow(vec![BoxShadow {
            color: color::dialog_shadow(app).into(),
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
                .border_color(color::border(app))
                .child(footer_button(
                    "budgets-switcher-new",
                    view.new_label.clone(),
                    false,
                    on_new,
                    app,
                ))
                .child(footer_button(
                    "budgets-switcher-manage",
                    view.manage_label.clone(),
                    true,
                    on_manage,
                    app,
                )),
        );

    div()
        .id("budgets-switcher-layer")
        .debug_selector(|| "budgets-switcher-layer".to_string())
        .absolute()
        .inset_0()
        .occlude()
        .bg(color::scrim(app))
        .on_click(move |_event, window, cx| on_cancel(window, cx))
        .child(card)
        .into_any_element()
}
