//! The Dashboard page: `lib-dashboard`'s Presenter in a gpui Entity, which the Shell hosts as the
//! Dashboard destination (ADR-0032, ADR-0037). It draws the Presenter's view model (`content.rs`),
//! turns a Needs Attention row's click into an event through [`PageClicks`], scrolls itself and
//! reaches `Root` only through [`ViewEvent`]. It has no keys of its own beyond scrolling, so most
//! of the View interface is a no-op. Each click is one input (`crate::input::send`), whose commit
//! writes nothing, since the Dashboard writes nothing.

use crate::announcer::Domain;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, Subscription, Window};
use gpui::{div, prelude::*, px};
use lib_core::{Clock, DateStyle};
use lib_dashboard::{DashboardEvent, DashboardPresenter, DashboardReads, DashboardViewModel};
use lib_viper::display::StatusGlyphs;

use super::content;
use crate::announcer::{Announcer, LedgerChanged};
use crate::dashboard::router::PageRouter;
use crate::navigation::movement::Movement;
use crate::shell::view::page_clicks::{PageClicks, PresenterPage};
use crate::shell::view::statusline::PageStatus;
use crate::view::event::{Reveal, ViewEvent};
use crate::view::scroll_by_movement;

impl EventEmitter<ViewEvent> for DashboardPage {}

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

/// The Dashboard Entity.
pub struct DashboardPage {
    presenter: DashboardPresenter,
    /// The page's own scroll, reset as it becomes the destination.
    scroll: ScrollHandle,
    _ledger_changed: Subscription,
}

impl DashboardPage {
    /// The page over `reads`, dated by `clock`. It redraws on a change to any domain its figures
    /// read, as the Ledger announces it, as the Budgets page does (#952): the Budget figures read
    /// the Accounts and Payees as well as the Bills, Transactions, Categories and Units.
    pub fn new(
        reads: std::rc::Rc<dyn DashboardReads>,
        announcer: &Entity<Announcer>,
        clock: Clock,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let _ledger_changed = cx.subscribe(announcer, |this, _, event: &LedgerChanged, cx| {
            let reads = [
                Domain::Bills,
                Domain::Budgets,
                Domain::Transactions,
                Domain::Categories,
                Domain::Units,
                Domain::Accounts,
                Domain::Payees,
            ];
            if reads.iter().any(|domain| event.what.contains(domain)) {
                // Not `send`: the announcement is raised inside the input that wrote, whose one commit
                // is still to come, so a second commit here would count as another input.
                this.presenter
                    .handle(DashboardEvent::LedgerChanged, &mut PageRouter { cx });
                cx.notify();
            }
        });
        Self {
            presenter: DashboardPresenter::new(reads, clock),
            scroll: ScrollHandle::new(),
            _ledger_changed,
        }
    }

    /// A click on the page, or the Shell telling it something: `event` as one input, committed once
    /// it is handled.
    fn send(&mut self, event: DashboardEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| {
            self.presenter.handle(event, &mut PageRouter { cx });
            cx.notify();
        });
    }

    /// The page as drawn.
    pub fn view_model(&self) -> DashboardViewModel {
        self.presenter.view_model()
    }

    /// Follows a change to the Display Preferences, handed to the Presenter as one input.
    pub fn set_display(
        &mut self,
        date_style: Option<DateStyle>,
        glyphs: StatusGlyphs,
        cx: &mut Context<'_, Self>,
    ) {
        self.send(DashboardEvent::DisplayChanged { date_style, glyphs }, cx);
    }

    /// No key is the Dashboard's ahead of the Shell's fallback.
    pub fn key_ahead(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// No key is the Dashboard's once every other step has left it.
    pub fn handle_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page has no list, so movement scrolls it.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        scroll_by_movement(&self.scroll, movement);
        cx.notify();
    }

    /// Nothing is open within the page, so `Esc` is `Root`'s.
    pub fn escape(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// `/` enters Search mode although the page has no box to search: the mode, its badge and
    /// the `Esc` out of it are unchanged from before the Dashboard moved here, and a Search with
    /// nothing to type into takes no keys.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page has just become the destination: it starts scrolled to the top.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// No other View hands a record to the Dashboard.
    pub fn reveal(&mut self, _reveal: Reveal, _cx: &mut Context<'_, Self>) {}

    /// The Dashboard words no hints of its own.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        None
    }

    /// The Needs Attention rows for a test, in the order they draw.
    #[doc(hidden)]
    pub fn bills_snapshot(&self) -> Vec<DashboardBillSnapshot> {
        self.view_model()
            .bills
            .into_iter()
            .map(|row| DashboardBillSnapshot {
                selector: bill_selector(row.id),
                plan: row.plan,
            })
            .collect()
    }
}

/// Reached by clicks only: a Needs Attention row's.
impl PresenterPage<DashboardEvent> for DashboardPage {
    fn send(&mut self, event: DashboardEvent, cx: &mut Context<'_, Self>) {
        Self::send(self, event, cx);
    }
}

impl Render for DashboardPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let clicks = PageClicks::new(cx);
        let view = self.view_model();
        let cx: &App = cx;
        div()
            .id("view")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(content::render(&view, &clicks, cx))
    }
}
