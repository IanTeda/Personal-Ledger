//! The Bills page (`docs/ux/desktop-mockups/12-bills/README.md`'s 8a–8f): a header row (title, the active
//! tab's meta line, **+ Add bill plan**), the Schedule / Planner tab row with the Schedule tab's
//! period nav at its right, a 2px rule, then the active tab's body.
//!
//! The Schedule (8a, `schedule`, with its `filters`) and Planner (8b, `planner`) tabs are built;
//! 8f's History tab was folded into the Schedule (#381). [`state::BillsView`] renders the page
//! itself (ADR-0037): every action is a call back into the Bills Entity, so the keyboard and the
//! mouse reach the same methods.

pub mod filters;
pub(crate) mod hints;
pub mod pay_dialog;
pub mod plan_dialog;
pub mod planner;
pub mod schedule;
pub mod skip_dialog;
pub mod snapshot;
pub mod state;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, ScrollHandle, SharedString, Window, div, prelude::*, px};

use crate::{bills, bills::BillsTab, navigation::nav::Noun, period::Period, theme::color};
use state::BillsView;

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

pub fn render(scroll_handle: &ScrollHandle, props: BillsPageProps<'_>, cx: &App) -> AnyElement {
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

impl Render for BillsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let plain = |handler: fn(&mut BillsView, &mut Context<'_, BillsView>)| {
            let entity = entity.clone();
            let on_click: OnPlainClick = Rc::new(move |_window, cx| entity.update(cx, handler));
            on_click
        };
        let indexed = |handler: fn(&mut BillsView, usize, &mut Context<'_, BillsView>)| {
            let entity = entity.clone();
            let on_click: OnRowClick = Rc::new(move |index, _window, cx| {
                entity.update(cx, |view, cx| handler(view, index, cx));
            });
            on_click
        };
        let on_tab_click: OnTabClick = {
            let entity = entity.clone();
            Rc::new(move |tab, _window, cx| {
                entity.update(cx, |view, cx| view.set_tab(tab, cx));
            })
        };
        let on_field_click: filters::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |view, cx| view.click_filter_field(field, cx));
            })
        };
        let on_option_click: filters::OnOptionClick = {
            let entity = entity.clone();
            Rc::new(move |field, index, _window, cx| {
                entity.update(cx, |view, cx| view.click_filter_option(field, index, cx));
            })
        };

        let cx: &App = cx;
        let store = self.sources.bills.read(cx);
        let (plans, entries) = (store.plans(cx), store.entries(cx));
        let transactions = self.sources.transactions.read(cx).transactions(cx);
        let accounts = self.sources.accounts.read(cx).accounts(cx);
        let units = self.sources.ledger.read(cx).ledger().units();
        let base_unit = units
            .iter()
            .find(|unit| unit.is_base)
            .map(|unit| unit.code.as_str());
        let state = &self.state;
        let unfiltered = self.unfiltered_rows(cx);
        let rows = state.filters.apply(&unfiltered, plans);
        let summary = bills::period_summary(&rows, plans, entries, transactions, |id| {
            schedule::unit_code(units, id).to_string()
        });
        let planner_plans = bills::planner_order(plans);
        let selects = filters::FilterField::ORDER
            .into_iter()
            .map(|field| {
                let focused = state
                    .filter_focus
                    .as_ref()
                    .filter(|(focused, _)| *focused == field);
                filters::FilterSelect {
                    field,
                    options: self.filter_options(field, cx).0,
                    state: focused.map_or_else(
                        || self.filter_select_state(field, cx),
                        |(_, select)| select.clone(),
                    ),
                    focused: focused.is_some(),
                }
            })
            .collect();
        let stats = state
            .filters
            .plan_id
            .and_then(|id| bills::get(plans, id))
            .map(|plan| {
                let stats =
                    bills::history::plan_stats(plan, plans, entries, transactions, self.today);
                (plan, stats)
            });
        let props = BillsPageProps {
            tab: state.tab,
            period: state.period,
            all: state.all,
            schedule: schedule::ScheduleProps {
                rows: &rows,
                total: unfiltered.len(),
                all: state.all,
                filters: filters::FilterProps {
                    filters: &state.filters,
                    selects,
                    stats,
                    base_unit,
                    units,
                    on_chip_click: indexed(BillsView::toggle_filter_chip),
                    on_field_click,
                    on_option_click,
                },
                summary: &summary,
                plans,
                entries,
                accounts,
                transactions,
                base_unit,
                units,
                glyphs: self.glyphs,
                selected: (!rows.is_empty()).then(|| state.selected.min(rows.len() - 1)),
                on_row_click: indexed(BillsView::select),
                on_pay_click: indexed(BillsView::click_pay),
                on_skip_click: indexed(BillsView::click_skip),
                on_view_transaction_click: indexed(BillsView::click_view_transaction),
            },
            planner: planner::PlannerProps {
                plans: &planner_plans,
                inactive: bills::inactive_count(plans),
                categories: self.sources.categories.read(cx).categories(cx),
                accounts,
                base_unit,
                units,
                selected: (!planner_plans.is_empty())
                    .then(|| state.selected.min(planner_plans.len() - 1)),
                on_row_click: indexed(BillsView::select),
                on_edit_click: indexed(BillsView::click_edit_plan),
            },
            on_add_click: plain(BillsView::open_add_plan),
            on_tab_click,
            on_period_prev: plain(|view, cx| view.shift_period(false, cx)),
            on_period_next: plain(|view, cx| view.shift_period(true, cx)),
            on_all_click: plain(BillsView::toggle_all),
        };
        render(&self.scroll, props, cx)
    }
}
