//! The Dashboard page's View: `lib-dashboard`'s Presenter's [`PageView`], which the generic
//! `ModulePage` hosts as the Dashboard destination (ADR-0056). It draws the view model
//! (`content.rs`) and keeps the page's scroll, which the keymap's movements scroll in
//! [`PageView::after`]; the keymap, the Search and every event are the Presenter's. Each click is
//! one input, whose commit writes nothing, since the Dashboard writes nothing.

use gpui::{AnyElement, Context, ScrollHandle, Window, div, prelude::*, px};
use lib_core::Clock;
use lib_dashboard::{DashboardEvent, DashboardPresenter, DashboardReads, DashboardViewModel};
use lib_database::Domain;
use lib_list_cursor::Step;
use lib_shell::{Movement, ShellCommand};

use super::content;
use crate::announcer::Announcer;
use crate::dashboard::router::PageRouter;
use crate::shell::view::module_page::{ModulePage, PageDraw, PageView};
use crate::shell::view::statusline::PageStatus;
use crate::view::event::ViewEvent;
use crate::view::scroll_by_movement;

/// The Dashboard page: its Presenter in the generic page.
pub type DashboardPage = ModulePage<DashboardPresenter>;

/// One Needs Attention row as a test reads it: its Plan and the `debug_selector` of its target.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardBillSnapshot {
    pub plan: String,
    pub selector: String,
}

/// The `debug_selector` a Needs Attention row is clicked by.
pub(super) fn bill_selector(id: lib_dashboard::DashboardEntryId) -> String {
    format!("dashboard-bill-{}-{}", id.plan_id, id.due)
}

/// The Needs Attention rows of `view` for a test, in the order they draw.
#[doc(hidden)]
pub fn bills_snapshot(view: DashboardViewModel) -> Vec<DashboardBillSnapshot> {
    view.bills
        .into_iter()
        .map(|row| DashboardBillSnapshot {
            selector: bill_selector(row.id),
            plan: row.plan,
        })
        .collect()
}

/// The page's own scroll, back at the top as it becomes the destination.
#[derive(Default)]
pub struct DashboardWidgets {
    scroll: ScrollHandle,
}

impl PageView for DashboardPresenter {
    type Setup = ();
    type Widgets = DashboardWidgets;
    /// The scroll a movement asks for, applied once the event is handled.
    type Before = Option<Step>;

    /// The Bills and Budget figures, and the Accounts, Categories, Payees, Transactions and Units
    /// they read.
    const REPAINTS: &'static [Domain] = &[
        Domain::Bills,
        Domain::Budgets,
        Domain::Transactions,
        Domain::Categories,
        Domain::Units,
        Domain::Accounts,
        Domain::Payees,
    ];

    fn with_router<R>(
        presenter: &mut Self,
        _setup: &(),
        cx: &mut Context<'_, DashboardPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut PageRouter { cx })
    }

    fn before(&mut self, event: &DashboardEvent, _widgets: &DashboardWidgets) -> Option<Step> {
        match event {
            DashboardEvent::Scroll(step) => Some(*step),
            _ => None,
        }
    }

    /// The page has no list, so a movement scrolls it.
    fn after(
        &self,
        scroll: Option<Step>,
        _setup: &Self::Setup,
        widgets: &mut DashboardWidgets,
    ) -> Option<ViewEvent> {
        if let Some(movement) = scroll.and_then(|step| Movement::of(ShellCommand::Move(step))) {
            scroll_by_movement(&widgets.scroll, movement);
        }
        None
    }

    /// A Search left open when the page was left doesn't come back with it.
    fn entered() -> Option<DashboardEvent> {
        Some(DashboardEvent::EndSearch)
    }

    /// The kept view model is stale once a domain it reads changes.
    fn ledger_changed() -> Option<DashboardEvent> {
        Some(DashboardEvent::LedgerChanged)
    }

    fn render(
        view: &DashboardViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, DashboardPage>,
    ) -> AnyElement {
        div()
            .id("view")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&page.widgets.scroll)
            .child(content::render(view, page.clicks, cx))
            .into_any_element()
    }

    /// The Dashboard words no hints of its own.
    fn status(_view: &DashboardViewModel) -> PageStatus {
        PageStatus {
            hints: Vec::new(),
            right: String::new(),
        }
    }

    /// `Esc` ends the Search, which has nothing else open.
    fn escape(view: &DashboardViewModel) -> Option<DashboardEvent> {
        view.searching.then_some(DashboardEvent::EndSearch)
    }
}

/// The Dashboard module's entry point: its page over `reads`, dated by `clock`, its keys under
/// `[keybindings]`'s defaults until `Root` pushes the ones in effect.
pub fn dashboard_page(
    reads: std::rc::Rc<dyn DashboardReads>,
    announcer: &gpui::Entity<Announcer>,
    clock: Clock,
    cx: &mut gpui::App,
) -> gpui::Entity<DashboardPage> {
    let keymap = lib_dashboard::keymap::dashboard_keymap(&lib_config::KeyBindingConfig::default());
    let presenter = DashboardPresenter::new(reads, clock, keymap);
    cx.new(|cx| ModulePage::new(presenter, (), announcer, cx))
}
