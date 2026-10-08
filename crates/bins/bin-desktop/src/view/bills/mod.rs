//! The Bills page (`docs/ux/desktop/12-bills/README.md`'s 8a–8f): a header row (title, the active
//! tab's meta line, **+ Add bill plan**), the Schedule / Planner tab row with the Schedule tab's
//! period nav at its right, a 2px rule, then the active tab's body.
//!
//! The Schedule (8a, `schedule`, with its `filters`) and Planner (8b, `planner`) tabs are built;
//! 8f's History tab was folded into the Schedule (#381). Every action is a callback into `Shell`,
//! so the keyboard and the mouse reach the same handlers.

pub mod filters;
pub mod pay_dialog;
pub mod plan_dialog;
pub mod planner;
pub mod schedule;
pub mod skip_dialog;

use std::rc::Rc;

use gpui::{AnyElement, App, ScrollHandle, SharedString, Window, div, prelude::*, px};

use crate::{bills::BillsTab, navigation::nav::Noun, period::Period, theme::color};

pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;
/// Called with a row's position in the active tab's order (`bills::schedule_rows` or
/// `bills::planner_order`).
pub type OnRowClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub type OnTabClick = Rc<dyn Fn(BillsTab, &mut Window, &mut App)>;

pub struct BillsPageProps<'a> {
    pub tab: BillsTab,
    pub period: Period,
    /// The Schedule shows All rather than `period`.
    pub all: bool,
    pub schedule: schedule::ScheduleProps<'a>,
    pub planner: planner::PlannerProps<'a>,
    pub on_add_click: OnPlainClick,
    pub on_tab_click: OnTabClick,
    pub on_period_prev: OnPlainClick,
    pub on_period_next: OnPlainClick,
    pub on_all_click: OnPlainClick,
}

pub fn render(
    focused: bool,
    scroll_handle: &ScrollHandle,
    props: BillsPageProps<'_>,
    cx: &App,
) -> AnyElement {
    let meta = match props.tab {
        BillsTab::Schedule => Some(schedule::meta_line(&props.schedule, cx).into_any_element()),
        BillsTab::Planner => Some(planner::meta_line(&props.planner, cx).into_any_element()),
    };
    let body = match props.tab {
        BillsTab::Schedule => schedule::render(&props.schedule, cx),
        BillsTab::Planner => planner::render(&props.planner, cx),
    };
    div()
        .id("bills")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
        .px(px(28.0))
        .py(px(22.0))
        .child(page_header(meta, props.on_add_click.clone(), cx))
        .child(tab_row(&props, cx))
        .child(
            div()
                .h(px(2.0))
                .flex_none()
                .bg(color::structural_rule(cx))
                .mb(px(24.0)),
        )
        .child(body)
        .into_any_element()
}

fn page_header(meta: Option<AnyElement>, on_add_click: OnPlainClick, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_end()
        .justify_between()
        .gap(px(16.0))
        .mb(px(16.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(28.0))
                        .text_color(color::foreground(cx))
                        .child(Noun::Bills.label()),
                )
                .children(meta),
        )
        .child(add_button(on_add_click, cx))
}

fn add_button(on_click: OnPlainClick, cx: &App) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .id("bills-add")
        .debug_selector(|| "bills-add".to_string())
        .cursor_pointer()
        .flex_none()
        .py(px(10.0))
        .px(px(16.0))
        .bg(color::foreground(cx))
        .text_color(color::selection_text(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(crate::msg::desktop_bills_add_button("+"))
}

fn tab_label(tab: BillsTab) -> String {
    match tab {
        BillsTab::Schedule => crate::msg::desktop_bills_tab_schedule(),
        BillsTab::Planner => crate::msg::desktop_bills_tab_planner(),
    }
}

/// The handoff's tab switcher: the active tab a dark filled cell, the rest plain text; the period
/// nav at the right on the Schedule tab only.
fn tab_row(props: &BillsPageProps<'_>, cx: &App) -> impl IntoElement {
    let tabs = [BillsTab::Schedule, BillsTab::Planner];
    div()
        .flex()
        .items_center()
        .children(tabs.into_iter().map(|tab| {
            let active = tab == props.tab;
            let on_click = props.on_tab_click.clone();
            let hover = color::hover(cx);
            div()
                .id(SharedString::from(format!("bills-tab-{tab:?}")))
                .debug_selector(move || format!("bills-tab-{tab:?}"))
                .cursor_pointer()
                .py(px(8.0))
                .px(px(16.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(active, |this| {
                    this.bg(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                })
                .when(!active, |this| {
                    this.text_color(color::muted(cx))
                        .hover(move |style| style.bg(hover))
                })
                .on_click(move |_event, window, cx| on_click(tab, window, cx))
                .child(tab_label(tab))
        }))
        .child(div().flex_1())
        .when(props.tab == BillsTab::Schedule, |this| {
            this.child(period_nav(props, cx))
        })
}

/// `‹ Sep 2026 ›  All`: the arrows step the Schedule a calendar month (from All, back to the month
/// last viewed); `All` toggles every row ever generated.
fn period_nav(props: &BillsPageProps<'_>, cx: &App) -> impl IntoElement {
    let arrow = |id: &'static str, glyph: &'static str, on_click: OnPlainClick| {
        let hover = color::foreground(cx);
        div()
            .id(id)
            .debug_selector(move || id.to_string())
            .cursor_pointer()
            .px(px(8.0))
            .text_color(color::muted(cx))
            .hover(move |style| style.text_color(hover))
            .on_click(move |_event, window, cx| on_click(window, cx))
            .child(glyph)
    };
    let on_all_click = props.on_all_click.clone();
    let hover = color::hover(cx);
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .text_size(px(12.0))
        .child(arrow(
            "bills-period-prev",
            "\u{2039}",
            props.on_period_prev.clone(),
        ))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(if props.all {
                    color::muted(cx)
                } else {
                    color::foreground(cx)
                })
                .whitespace_nowrap()
                .child(period_label(props.period)),
        )
        .child(arrow(
            "bills-period-next",
            "\u{203a}",
            props.on_period_next.clone(),
        ))
        .child(
            div()
                .id("bills-period-all")
                .debug_selector(|| "bills-period-all".to_string())
                .cursor_pointer()
                .ml(px(8.0))
                .py(px(4.0))
                .px(px(10.0))
                .border_1()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(props.all, |this| {
                    this.bg(color::selection_background(cx))
                        .border_color(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                })
                .when(!props.all, |this| {
                    this.border_color(color::border(cx))
                        .text_color(color::muted(cx))
                        .hover(move |style| style.bg(hover))
                })
                .on_click(move |_event, window, cx| on_all_click(window, cx))
                .child(crate::msg::desktop_bills_period_all()),
        )
}

/// A period as the Locale writes a month and year (`Sept 2026`).
pub fn period_label(period: Period) -> String {
    lib_locale::format::format_year_month(period.year, period.month)
}
