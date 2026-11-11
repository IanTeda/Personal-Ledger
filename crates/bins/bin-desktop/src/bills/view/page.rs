//! The Bills page's View: the `lib-bills` page Presenter's [`PageView`], which the generic
//! `ModulePage` hosts as the Bills destination (ADR-0056, `docs/ux/desktop-mockups/12-bills/`). It
//! names the page's Router, the domains it repaints on and its widget state (the page's scroll and
//! the open scope list's window), and draws the view model (`schedule.rs`, `filters.rs`,
//! `planner.rs`); the keymap, the scope selects' open lists and every event are the Presenter's.

use crate::navigation::nav::NounLabel;

use gpui::{AnyElement, App, Context, ScrollHandle, SharedString, Window, div, prelude::*, px};
use lib_bills::presenter::{
    BillsPageEvent, BillsPagePresenter, BillsPageViewModel, BillsTab, ScheduleEvent,
};
use lib_database::Domain;

use super::{planner, schedule};
use crate::bills::router::{BillsRoutes, PageRouter};
use crate::navigation::nav::Noun;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView};
use crate::shell::view::page_clicks::PageClicks;
use crate::view::help::HelpGroup;
use crate::{
    shell::view::statusline::PageStatus,
    theme::color,
    view::event::{Reveal, ViewEvent},
};

/// The Bills page: its Presenter in the generic page.
pub type BillsPage = ModulePage<BillsPagePresenter>;

/// The widget state the page keeps beside its Presenter: its scroll, back at the top as it becomes
/// the destination and as its tab changes, and the open scope list's window.
#[derive(Default)]
pub struct BillsWidgets {
    scroll: ScrollHandle,
    filter_window: usize,
}

/// What [`PageView::before`] notes: the tab shown, and whether the event shows a tab, which a
/// click on a tab clears the status line after as the key that switches tabs does.
#[derive(Default)]
pub struct BillsBefore {
    tab: BillsTab,
    shows_tab: bool,
}

impl PageView for BillsPagePresenter {
    type Setup = BillsRoutes;
    type Widgets = BillsWidgets;
    type Before = BillsBefore;

    /// A Bills write, another module's included (Account delete's Unmatch), and the Accounts,
    /// Categories, Payees, Transactions and Units the rows name, which the ports read from their
    /// stores on each ask.
    const REPAINTS: &'static [Domain] = &[
        Domain::Bills,
        Domain::Accounts,
        Domain::Categories,
        Domain::Payees,
        Domain::Transactions,
        Domain::Units,
    ];

    /// The Router opens the Plan form and Pay in the date style the Presenter holds.
    fn with_router<R>(
        presenter: &mut Self,
        setup: &BillsRoutes,
        cx: &mut Context<'_, BillsPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        let date_style = presenter.date_style();
        handle(
            presenter,
            &mut PageRouter {
                routes: setup,
                date_style,
                cx,
            },
        )
    }

    fn before(&mut self, event: &BillsPageEvent, _widgets: &BillsWidgets) -> BillsBefore {
        BillsBefore {
            tab: self.tab(),
            shows_tab: matches!(event, BillsPageEvent::ShowTab(_)),
        }
    }

    /// A tab change starts the page at the top; a tab shown clears the status line.
    fn after(
        &self,
        before: BillsBefore,
        _setup: &Self::Setup,
        widgets: &mut BillsWidgets,
    ) -> Option<ViewEvent> {
        if self.tab() != before.tab {
            widgets.scroll.set_offset(gpui::Point::default());
        }
        before.shows_tab.then_some(ViewEvent::SetStatus(None))
    }

    /// Generates a month the Desktop has crossed into since the page was last shown.
    fn entered() -> Option<BillsPageEvent> {
        Some(BillsPageEvent::Entered)
    }

    fn render(
        view: &BillsPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, BillsPage>,
    ) -> AnyElement {
        render(view, page.clicks, page.widgets, cx)
    }

    /// The tab shown's hints and right-hand text.
    fn status(view: &BillsPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.status.clone().into(),
        }
    }

    /// Shows a record handed off from another View.
    fn reveal(reveal: Reveal) -> Option<BillsPageEvent> {
        match reveal {
            Reveal::BillEntry(id) => Some(BillsPageEvent::RevealEntry(id)),
            Reveal::BillPlan(id) => Some(BillsPageEvent::RevealPlan(id)),
            Reveal::BillsPeriod(period) => Some(BillsPageEvent::RevealPeriod(period)),
            Reveal::Account(_) | Reveal::Payee(_) => None,
        }
    }

    /// The scope selects: the first `Esc` closes an open list, the next leaves the filter row.
    fn escape(view: &BillsPageViewModel) -> Option<BillsPageEvent> {
        (view.tab == BillsTab::Schedule && view.schedule.filters.focused.is_some())
            .then_some(BillsPageEvent::Schedule(ScheduleEvent::LeaveFilters))
    }

    /// The `?` cheat-sheet's Bills group: the hints the status line shows for the tab shown.
    fn help(view: &BillsPageViewModel) -> Option<HelpGroup> {
        Some(HelpGroup {
            heading: crate::msg::desktop_help_bills_label(),
            keys: view
                .hints
                .iter()
                .map(|hint| (hint.action.clone(), hint.chord.clone()))
                .collect(),
        })
    }
}

fn render(
    view: &BillsPageViewModel,
    clicks: &PageClicks<BillsPage>,
    widgets: &mut BillsWidgets,
    cx: &App,
) -> AnyElement {
    let (meta, body) = match view.tab {
        BillsTab::Schedule => (
            schedule::meta_line(&view.schedule.meta, cx),
            schedule::render(&view.schedule, &mut widgets.filter_window, clicks, cx),
        ),
        BillsTab::Planner => (
            planner::meta_line(&view.planner, cx),
            planner::render(&view.planner, clicks, cx),
        ),
    };
    div()
        .id("bills")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(&widgets.scroll)
        .px(px(28.0))
        .py(px(22.0))
        .child(page_header(
            meta,
            &view.add_button,
            clicks.send(BillsPageEvent::New),
            cx,
        ))
        .child(tab_row(view, clicks, cx))
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

fn page_header(
    meta: AnyElement,
    add_button: &str,
    on_add_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
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
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(28.0))
                        .text_color(color::foreground(cx))
                        .child(Noun::Bills.label()),
                )
                .child(meta),
        )
        .child(
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
                .on_click(on_add_click)
                .child(add_button.to_string()),
        )
}

/// The handoff's tab switcher: the active tab a dark filled cell, the rest plain text; the period
/// nav at the right on the Schedule tab only.
fn tab_row(
    view: &BillsPageViewModel,
    clicks: &PageClicks<BillsPage>,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .children(view.tabs.iter().map(|tab| {
            let id = tab.tab;
            // A click isn't a key, so the Shell keeps the status line through it: the tab clears
            // a message about the tab it leaves, as the key that switches tabs does.
            let on_click = clicks.send(BillsPageEvent::ShowTab(id));
            let hover = color::hover(cx);
            div()
                .id(SharedString::from(format!("bills-tab-{id:?}")))
                .debug_selector(move || format!("bills-tab-{id:?}"))
                .cursor_pointer()
                .py(px(8.0))
                .px(px(16.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(tab.active, |this| {
                    this.bg(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                })
                .when(!tab.active, |this| {
                    this.text_color(color::muted(cx))
                        .hover(move |style| style.bg(hover))
                })
                .on_click(on_click)
                .child(tab.label.clone())
        }))
        .child(div().flex_1())
        .when(view.tab == BillsTab::Schedule, |this| {
            this.child(period_nav(view, clicks, cx))
        })
}

/// `‹ Sep 2026 ›  All`: the arrows step the Schedule a calendar month (from All, back to the month
/// last viewed); `All` toggles every row ever generated.
fn period_nav(
    view: &BillsPageViewModel,
    clicks: &PageClicks<BillsPage>,
    cx: &App,
) -> impl IntoElement {
    let schedule = &view.schedule;
    let arrow = |id: &'static str, glyph: &'static str, event: ScheduleEvent| {
        let hover = color::foreground(cx);
        let on_click = clicks.send(BillsPageEvent::Schedule(event));
        div()
            .id(id)
            .debug_selector(move || id.to_string())
            .cursor_pointer()
            .px(px(8.0))
            .text_color(color::muted(cx))
            .hover(move |style| style.text_color(hover))
            .on_click(on_click)
            .child(glyph)
    };
    let on_all_click = clicks.send(BillsPageEvent::Schedule(ScheduleEvent::ToggleAll));
    let hover = color::hover(cx);
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .text_size(px(12.0))
        .child(arrow(
            "bills-period-prev",
            "\u{2039}",
            ScheduleEvent::PrevPeriod,
        ))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(if schedule.all {
                    color::muted(cx)
                } else {
                    color::foreground(cx)
                })
                .whitespace_nowrap()
                .child(schedule.period_label.clone()),
        )
        .child(arrow(
            "bills-period-next",
            "\u{203a}",
            ScheduleEvent::NextPeriod,
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
                .when(schedule.all, |this| {
                    this.bg(color::selection_background(cx))
                        .border_color(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                })
                .when(!schedule.all, |this| {
                    this.border_color(color::border(cx))
                        .text_color(color::muted(cx))
                        .hover(move |style| style.bg(hover))
                })
                .on_click(on_all_click)
                .child(schedule.all_label.clone()),
        )
}
