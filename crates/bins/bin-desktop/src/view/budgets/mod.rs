//! The Budgets page (`docs/ux/desktop-mockups/14-budgets-v2/`): the shared header (the Budget's name as the
//! switcher's title with its method tag, the meta line, the primary action), the tab strip with
//! its legend and the `‹ month ›` period nav, a 2px rule, then the active tab's body.
//!
//! The Progress (9a, `progress`), Plan (9b, `plan`) and History (9c, `history`) tabs, with the
//! Category detail (9d), Edit budget (9e), Fill (9f) and Stop budgeting (9g) dialogs and the
//! Switcher popover (11b, `switcher`) and the New budget (11c, `budget_dialog`) and Manage budgets (11f,
//! `manage_dialog`) dialogs. [`state::BudgetsView`] renders the page itself (ADR-0037): every
//! action is a call back into the Budgets Entity, so the keyboard and the mouse reach the same
//! methods. The dialogs are still drawn by `Root`'s Dialog host.

pub mod budget_dialog;
pub mod detail_dialog;
pub mod fill_dialog;
pub(crate) mod hints;
pub mod history;
pub mod limit_dialog;
pub mod manage_dialog;
pub mod plan;
pub mod progress;
pub mod snapshot;
pub mod state;
pub mod stop_dialog;
pub mod switcher;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, ScrollHandle, SharedString, Window, div, prelude::*, px};
use lib_locale::format::upper;

use crate::{
    budgets::{BudgetsTab, Method, PeriodFigures},
    navigation::nav::Noun,
    period::Period,
    theme::color,
    view::placeholder,
};
use lib_categories::Category;
use state::BudgetsView;

pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;
/// Called with a row's position in the Progress table.
pub type OnRowClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub type OnTabClick = Rc<dyn Fn(BudgetsTab, &mut Window, &mut App)>;

pub struct BudgetsPageProps<'a> {
    pub name: &'a str,
    pub method: Method,
    pub tab: BudgetsTab,
    pub period: Period,
    pub figures: &'a PeriodFigures,
    /// The Plan tab's grid, built only while that tab shows.
    pub plan: Option<plan::PlanProps<'a>>,
    /// The History tab's chart and table, built only while that tab shows.
    pub history: Option<history::HistoryProps<'a>>,
    /// The History tab's `Export CSV`.
    pub on_export_click: OnPlainClick,
    pub on_range_prev: OnPlainClick,
    pub on_range_next: OnPlainClick,
    pub categories: &'a [Category],
    /// Index into `figures.rows` of the selected row.
    pub selected: Option<usize>,
    pub on_tab_click: OnTabClick,
    /// The title is the switcher: a click opens 11b.
    pub on_title_click: OnPlainClick,
    pub on_period_prev: OnPlainClick,
    pub on_period_next: OnPlainClick,
    pub on_edit_plan_click: OnPlainClick,
    /// The Plan tab's `+ Budget a category`.
    pub on_add_click: OnPlainClick,
    /// The Plan tab's `Fill October from…`, worded for the month Fill would target.
    pub fill_label: String,
    pub on_fill_click: OnPlainClick,
    /// A Progress row's `edit` or `set` action.
    pub on_action_click: OnRowClick,
    pub on_row_click: OnRowClick,
    /// The KNOWN COSTS stat's link to the Bills Schedule.
    pub on_known_click: OnPlainClick,
}

pub fn render(scroll_handle: &ScrollHandle, props: BudgetsPageProps<'_>, cx: &App) -> AnyElement {
    let body = match props.tab {
        BudgetsTab::Progress => progress::render(&props, cx),
        BudgetsTab::Plan => match &props.plan {
            Some(plan_props) => plan::render(plan_props, cx),
            None => div().into_any_element(),
        },
        BudgetsTab::History => match &props.history {
            Some(history_props) => history::render(history_props, cx),
            None => div()
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_budgets_history_empty())
                .into_any_element(),
        },
    };
    div()
        .id("budgets")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .px(px(28.0))
        .py(px(22.0))
        .child(page_header(&props, cx))
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

pub(super) fn method_label(method: Method) -> String {
    match method {
        Method::Limits => crate::msg::desktop_budgets_method_limits(),
    }
}

/// The title as the switcher (name, chevron, method tag) with the meta line below it, and the
/// primary action at the right: `Edit plan`, or `+ Budget a category` on the Plan tab itself.
fn page_header(props: &BudgetsPageProps<'_>, cx: &App) -> impl IntoElement {
    let (on_action, action_label) = match props.tab {
        BudgetsTab::Plan => (
            props.on_add_click.clone(),
            crate::msg::desktop_budgets_plan_add(),
        ),
        BudgetsTab::History => (
            props.on_export_click.clone(),
            crate::msg::desktop_budgets_history_export(),
        ),
        BudgetsTab::Progress => (
            props.on_edit_plan_click.clone(),
            crate::msg::desktop_budgets_edit_plan(),
        ),
    };
    let hover = color::muted(cx);
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
                        .id("budgets-title")
                        .debug_selector(|| "budgets-title".to_string())
                        .cursor_pointer()
                        .on_click({
                            let on_title = props.on_title_click.clone();
                            move |_event, window, cx| on_title(window, cx)
                        })
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .text_size(px(28.0))
                                .text_color(color::foreground(cx))
                                .child(props.name.to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(color::muted(cx))
                                .child("\u{25be}"),
                        )
                        .child(method_tag(props.method, cx)),
                )
                .child(match props.tab {
                    BudgetsTab::Plan => div()
                        .text_size(px(12.0))
                        .text_color(color::muted(cx))
                        .child(crate::msg::desktop_budgets_plan_meta())
                        .into_any_element(),
                    BudgetsTab::History => div()
                        .text_size(px(12.0))
                        .text_color(color::muted(cx))
                        .child(crate::msg::desktop_budgets_history_meta())
                        .into_any_element(),
                    BudgetsTab::Progress => progress::meta_line(props, cx).into_any_element(),
                }),
        )
        .when(props.tab == BudgetsTab::Plan, |this| {
            let on_fill = props.on_fill_click.clone();
            let hover = color::hover(cx);
            this.child(div().flex_1()).child(
                div()
                    .id("budgets-fill")
                    .debug_selector(|| "budgets-fill".to_string())
                    .cursor_pointer()
                    .flex_none()
                    .py(px(9.0))
                    .px(px(16.0))
                    .border_1()
                    .border_color(color::border(cx))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .whitespace_nowrap()
                    .hover(move |style| style.bg(hover))
                    .on_click(move |_event, window, cx| on_fill(window, cx))
                    .child(props.fill_label.clone()),
            )
        })
        .child(
            div()
                .id("budgets-primary-action")
                .debug_selector(|| "budgets-primary-action".to_string())
                .cursor_pointer()
                .flex_none()
                .py(px(10.0))
                .px(px(16.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .whitespace_nowrap()
                .hover(move |style| style.bg(hover))
                .on_click(move |_event, window, cx| on_action(window, cx))
                .child(action_label),
        )
}

/// The method's solid tag: 10px, extra bold, tracked out.
fn method_tag(method: Method, cx: &App) -> impl IntoElement {
    div()
        .py(px(4.0))
        .px(px(7.0))
        .bg(color::foreground(cx))
        .text_color(color::selection_text(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .child(upper(&method_label(method)))
}

fn tab_label(tab: BudgetsTab) -> String {
    match tab {
        BudgetsTab::Progress => crate::msg::desktop_budgets_tab_progress(),
        BudgetsTab::Plan => crate::msg::desktop_budgets_tab_plan(),
        BudgetsTab::History => crate::msg::desktop_budgets_tab_history(),
    }
}

/// The active tab a dark filled cell, the rest plain text; the Progress legend and the period nav
/// at the right.
fn tab_row(props: &BudgetsPageProps<'_>, cx: &App) -> impl IntoElement {
    let tabs = [BudgetsTab::Progress, BudgetsTab::Plan, BudgetsTab::History];
    div()
        .flex()
        .items_center()
        .children(tabs.into_iter().map(|tab| {
            let active = tab == props.tab;
            let on_click = props.on_tab_click.clone();
            let hover = color::hover(cx);
            div()
                .id(SharedString::from(format!("budgets-tab-{tab:?}")))
                .debug_selector(move || format!("budgets-tab-{tab:?}"))
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
        .when(props.tab == BudgetsTab::Progress, |this| {
            this.child(progress::legend(cx))
        })
        .child(div().flex_1())
        .child(match (props.tab, &props.plan, &props.history) {
            (BudgetsTab::Plan, Some(plan_props), _) => period_nav(
                plan::range_label(plan_props.plan),
                props.on_range_prev.clone(),
                props.on_range_next.clone(),
                cx,
            ),
            (BudgetsTab::History, _, Some(history_props)) => period_nav(
                history::range_label(history_props.history),
                props.on_range_prev.clone(),
                props.on_range_next.clone(),
                cx,
            ),
            _ => period_nav(
                period_label(props.period),
                props.on_period_prev.clone(),
                props.on_period_next.clone(),
                cx,
            ),
        })
}

/// `‹ September 2026 ›`, or the Plan's `‹ Jul – Dec 2026 ›`.
fn period_nav(label: String, on_prev: OnPlainClick, on_next: OnPlainClick, cx: &App) -> AnyElement {
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
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .text_size(px(12.0))
        .child(arrow("budgets-period-prev", "\u{2039}", on_prev))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::foreground(cx))
                .whitespace_nowrap()
                .child(label),
        )
        .child(arrow("budgets-period-next", "\u{203a}", on_next))
        .into_any_element()
}

/// A period as the Locale writes a month and year (`September 2026`).
pub fn period_label(period: Period) -> String {
    lib_locale::format::format_year_month(period.year, period.month)
}

impl Render for BudgetsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let entity = cx.entity();
        let plain = |handler: fn(&mut BudgetsView, &mut Context<'_, BudgetsView>)| {
            let entity = entity.clone();
            let on_click: OnPlainClick = Rc::new(move |_window, cx| entity.update(cx, handler));
            on_click
        };
        let indexed = |handler: fn(&mut BudgetsView, usize, &mut Context<'_, BudgetsView>)| {
            let entity = entity.clone();
            let on_click: OnRowClick = Rc::new(move |index, _window, cx| {
                entity.update(cx, |view, cx| handler(view, index, cx));
            });
            on_click
        };
        let cell = |handler: fn(&mut BudgetsView, usize, usize, &mut Context<'_, BudgetsView>)| {
            let entity = entity.clone();
            let on_click: plan::OnCellClick = Rc::new(move |row, column, _window, cx| {
                entity.update(cx, |view, cx| handler(view, row, column, cx));
            });
            on_click
        };
        let on_tab_click: OnTabClick = {
            let entity = entity.clone();
            Rc::new(move |tab, _window, cx| entity.update(cx, |view, cx| view.set_tab(tab, cx)))
        };

        let cx: &App = cx;
        let Some((budget, figures)) = self.figures(cx) else {
            // The Budget on show has gone.
            return div()
                .id("budgets")
                .flex_1()
                .h_full()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(placeholder::render(Noun::Budgets, cx))
                .into_any_element();
        };
        let state = &self.state;
        let plan = (state.tab == BudgetsTab::Plan)
            .then(|| self.plan_data(cx))
            .flatten();
        let history = (state.tab == BudgetsTab::History)
            .then(|| self.history_data(cx))
            .flatten();
        let props = BudgetsPageProps {
            name: &budget.name,
            method: budget.method,
            tab: state.tab,
            period: state.period,
            figures: &figures,
            plan: plan.as_ref().map(|plan| plan::PlanProps {
                plan,
                current: Period::of(self.today),
                cursor: self.plan_cursor_in(plan),
                edit: state.plan_edit.as_ref(),
                on_cell_click: cell(BudgetsView::click_plan_cell),
            }),
            history: history.as_ref().map(|history| history::HistoryProps {
                history,
                categories: self.categories(cx),
                cursor: self.history_cursor_in(history),
                on_cell_click: cell(BudgetsView::click_history_cell),
            }),
            on_export_click: plain(BudgetsView::export_history),
            on_range_prev: plain(|view, cx| view.shift_range(false, cx)),
            on_range_next: plain(|view, cx| view.shift_range(true, cx)),
            categories: self.categories(cx),
            selected: (!figures.rows.is_empty())
                .then(|| state.selected.min(figures.rows.len() - 1)),
            on_tab_click,
            on_title_click: plain(BudgetsView::click_title),
            on_period_prev: plain(|view, cx| view.shift_period(false, cx)),
            on_period_next: plain(|view, cx| view.shift_period(true, cx)),
            on_edit_plan_click: plain(|view, cx| view.set_tab(BudgetsTab::Plan, cx)),
            on_add_click: plain(BudgetsView::click_add),
            fill_label: crate::msg::desktop_budgets_fill_button(&lib_locale::format::format_month(
                self.fill_target(cx).month,
            )),
            on_fill_click: plain(BudgetsView::click_fill),
            on_action_click: indexed(BudgetsView::click_action),
            on_row_click: indexed(BudgetsView::click_row),
            on_known_click: plain(BudgetsView::click_known),
        };
        render(&self.scroll, props, cx)
    }
}
