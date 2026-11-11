//! The Budgets page's View: the `lib-budgets` page Presenter's [`PageView`], which the generic
//! `ModulePage` hosts as the Budgets destination (ADR-0056,
//! `docs/ux/desktop-mockups/14-budgets-v2/`). It names the page's Router, the domains it repaints
//! on and its widget state (the page's scroll), and draws the view model (`progress.rs`,
//! `plan.rs`, `history.rs`); the keymap for the tab shown and every event are the Presenter's.
//!
//! Typing into a Plan cell is the page's Insert mode, which the Presenter reports as it starts an
//! edit and drops as it ends one; the page turns that into the Shell's mode around the handle,
//! and hands the Presenter every key while it lasts.

use gpui::{AnyElement, App, Context, ScrollHandle, SharedString, Window, div, prelude::*, px};
use lib_budgets::presenter::{
    BudgetsPageEvent, BudgetsPagePresenter, BudgetsPageViewModel, BudgetsTab, HeaderView,
    HistoryEvent, PlanEvent, ProgressEvent, TabBody,
};
use lib_core::BudgetId;
use lib_database::Domain;
use lib_locale::format::upper;

use super::{history, plan, progress};
use crate::budgets::router::{BudgetsRoutes, PageRouter};
use crate::navigation::nav::Noun;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView};
use crate::shell::view::page_clicks::PageClicks;
use crate::shell::view::statusline::PageStatus;
use crate::theme::color;
use crate::view::event::ViewEvent;
use crate::view::help::HelpGroup;
use crate::view::placeholder;

/// The Budgets page: its Presenter in the generic page.
pub type BudgetsPage = ModulePage<BudgetsPagePresenter>;

/// The page's own scroll, back at the top as it becomes the destination, as its tab changes and
/// as another Budget is shown.
#[derive(Default)]
pub struct BudgetsWidgets {
    scroll: ScrollHandle,
}

/// What [`PageView::before`] notes: the tab and Budget shown, and whether the event shows a tab,
/// which a click on a tab clears the status line after as the key that switches tabs does.
#[derive(Default)]
pub struct BudgetsBefore {
    tab: BudgetsTab,
    budget: Option<BudgetId>,
    shows_tab: bool,
}

impl PageView for BudgetsPagePresenter {
    type Setup = BudgetsRoutes;
    type Widgets = BudgetsWidgets;
    type Before = BudgetsBefore;

    /// The figures read every domain Budgets counts.
    const REPAINTS: &'static [Domain] = &[
        Domain::Accounts,
        Domain::Bills,
        Domain::Budgets,
        Domain::Categories,
        Domain::Payees,
        Domain::Transactions,
        Domain::Units,
    ];

    /// The Router opens the Switcher in the date style the Presenter holds.
    fn with_router<R>(
        presenter: &mut Self,
        setup: &BudgetsRoutes,
        cx: &mut Context<'_, BudgetsPage>,
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

    fn before(&mut self, event: &BudgetsPageEvent, _widgets: &BudgetsWidgets) -> BudgetsBefore {
        BudgetsBefore {
            tab: self.tab(),
            budget: Some(self.current()),
            shows_tab: matches!(event, BudgetsPageEvent::ShowTab(_)),
        }
    }

    /// Another tab or Budget starts the page at the top; a tab shown clears the status line.
    fn after(
        &self,
        before: BudgetsBefore,
        _setup: &Self::Setup,
        widgets: &mut BudgetsWidgets,
    ) -> Option<ViewEvent> {
        if self.tab() != before.tab || Some(self.current()) != before.budget {
            widgets.scroll.set_offset(gpui::Point::default());
        }
        before.shows_tab.then_some(ViewEvent::SetStatus(None))
    }

    /// An edit left open when the page was left doesn't come back with it.
    fn entered() -> Option<BudgetsPageEvent> {
        Some(BudgetsPageEvent::Entered)
    }

    fn render(
        view: &BudgetsPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, BudgetsPage>,
    ) -> AnyElement {
        render(view, page.clicks, page.widgets, cx)
    }

    /// The tab shown's hints, or the Plan cell editor's, and its right-hand text.
    fn status(view: &BudgetsPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.status.clone().into(),
        }
    }

    /// `Esc` drops the Plan cell being typed into. Nothing else on the page holds an `Esc` of its
    /// own.
    fn escape(view: &BudgetsPageViewModel) -> Option<BudgetsPageEvent> {
        view.editing
            .then_some(BudgetsPageEvent::Plan(PlanEvent::Abandon))
    }

    /// The `?` cheat-sheet's Budgets group: the hints the status line shows for the tab shown.
    fn help(view: &BudgetsPageViewModel) -> Option<HelpGroup> {
        Some(HelpGroup {
            heading: crate::msg::desktop_help_budgets_label(),
            keys: view
                .hints
                .iter()
                .map(|hint| (hint.action.clone(), hint.chord.clone()))
                .collect(),
        })
    }
}

fn render(
    view: &BudgetsPageViewModel,
    clicks: &PageClicks<BudgetsPage>,
    widgets: &mut BudgetsWidgets,
    cx: &App,
) -> AnyElement {
    let page = div()
        .id("budgets")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(&widgets.scroll);
    let Some(header) = view.header.as_ref() else {
        // The Budget on show has gone.
        return page
            .child(placeholder::render(Noun::Budgets, cx))
            .into_any_element();
    };
    let body = match &view.body {
        TabBody::Progress(progress) => progress::render(progress, clicks, cx),
        TabBody::Plan(plan) => plan::render(plan, clicks, cx),
        TabBody::History(history) => history::render(history, clicks, cx),
        TabBody::Gone => div().into_any_element(),
    };
    page.px(px(28.0))
        .py(px(22.0))
        .child(page_header(header, view.tab, clicks, cx))
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

/// The title as the Switcher (name, chevron, method tag) with the meta line below it, the Plan's
/// Fill button, and the primary action at the right.
fn page_header(
    header: &HeaderView,
    tab: BudgetsTab,
    clicks: &PageClicks<BudgetsPage>,
    cx: &App,
) -> impl IntoElement {
    let hover = color::muted(cx);
    let on_title = clicks.send(BudgetsPageEvent::Switch);
    let on_action = clicks.send(header.action_event.clone());
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
                        .on_click(on_title)
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .text_size(px(28.0))
                                .text_color(color::foreground(cx))
                                .child(header.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(color::muted(cx))
                                .child("\u{25be}"),
                        )
                        .child(method_tag(&header.method, cx)),
                )
                .child(meta_line(&header.meta, cx)),
        )
        .when_some(header.fill.clone(), |this, fill| {
            let on_fill = clicks.send(BudgetsPageEvent::Plan(PlanEvent::Fill));
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
                    .on_click(on_fill)
                    .child(fill),
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
                .on_click(on_action)
                .child(header.action.clone()),
        )
        // The tab is part of what a click on the action means, so a test can tell them apart.
        .debug_selector(move || format!("budgets-header-{tab:?}"))
}

/// The method's solid tag: 10px, extra bold, tracked out.
pub(super) fn method_tag(method: &str, cx: &App) -> impl IntoElement {
    div()
        .py(px(4.0))
        .px(px(7.0))
        .bg(color::foreground(cx))
        .text_color(color::selection_text(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .child(upper(method))
}

/// The meta line's clauses, separated by a middle dot; a `strong` run is the accent.
fn meta_line(clauses: &[Vec<lib_locale::Segment>], cx: &App) -> impl IntoElement {
    let accent = color::accent_text(cx);
    let clause = |segments: &Vec<lib_locale::Segment>| {
        div()
            .flex()
            .children(segments.iter().map(move |segment| {
                match segment.tag.as_deref() {
                    Some("strong") => div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(accent)
                        .child(segment.text.clone())
                        .into_any_element(),
                    _ => div().child(segment.text.clone()).into_any_element(),
                }
            }))
            .into_any_element()
    };
    div()
        .flex()
        .flex_wrap()
        .gap(px(6.0))
        .text_size(px(12.0))
        .text_color(color::muted(cx))
        .children(clauses.iter().enumerate().flat_map(|(index, segments)| {
            let separator = (index > 0).then(|| div().child("\u{b7}").into_any_element());
            separator
                .into_iter()
                .chain(std::iter::once(clause(segments)))
        }))
}

/// The active tab a dark filled cell, the rest plain text; the Progress legend and the period nav
/// at the right.
fn tab_row(
    view: &BudgetsPageViewModel,
    clicks: &PageClicks<BudgetsPage>,
    cx: &App,
) -> impl IntoElement {
    let (previous, next) = match view.tab {
        BudgetsTab::Progress => (
            BudgetsPageEvent::Progress(ProgressEvent::PreviousPeriod),
            BudgetsPageEvent::Progress(ProgressEvent::NextPeriod),
        ),
        BudgetsTab::Plan => (
            BudgetsPageEvent::Plan(PlanEvent::PreviousRange),
            BudgetsPageEvent::Plan(PlanEvent::NextRange),
        ),
        BudgetsTab::History => (
            BudgetsPageEvent::History(HistoryEvent::PreviousRange),
            BudgetsPageEvent::History(HistoryEvent::NextRange),
        ),
    };
    div()
        .flex()
        .items_center()
        .children(view.tabs.iter().map(|tab| {
            let id = tab.tab;
            // A click isn't a key, so the Shell keeps the status line through it: the tab clears
            // a message about the tab it leaves, as the key that switches tabs does.
            let on_click = clicks.send(BudgetsPageEvent::ShowTab(id));
            let hover = color::hover(cx);
            div()
                .id(SharedString::from(format!("budgets-tab-{id:?}")))
                .debug_selector(move || format!("budgets-tab-{id:?}"))
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
        .when_some(
            match &view.body {
                TabBody::Progress(progress) => Some(progress.legend.clone()),
                _ => None,
            },
            |this, legend| this.child(progress::legend(&legend, cx)),
        )
        .child(div().flex_1())
        .child(period_nav(
            view.nav.clone(),
            clicks.send(previous),
            clicks.send(next),
            cx,
        ))
}

/// A period arrow's click, boxed so the two arrows share one builder.
type ArrowClick = Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App)>;

/// `‹ Sep 2026 ›`, or the Plan's `‹ Jul – Dec 2026 ›`.
fn period_nav(
    label: String,
    on_prev: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_next: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let arrow = |id: &'static str, glyph: &'static str, on_click: ArrowClick| {
        let hover = color::foreground(cx);
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
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .text_size(px(12.0))
        .child(arrow("budgets-period-prev", "\u{2039}", Box::new(on_prev)))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::foreground(cx))
                .whitespace_nowrap()
                .child(label),
        )
        .child(arrow("budgets-period-next", "\u{203a}", Box::new(on_next)))
        .into_any_element()
}
