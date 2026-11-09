//! `impl BudgetsRouter` for the Desktop: where the Budgets page's Presenter asks to go. Every
//! Dialog (the Switcher, Manage budgets, the Budget form, Edit budget, Stop budgeting, Fill and the
//! Category detail) opens as a `ModuleDialog` in the Dialog host, holding its Presenter. All of it goes
//! through the page's gpui context, which is why the Router is built for each `handle` call (`PageView::with_router`). A
//! Notice goes onto the bin's queue (`crate::notices`), which the input's `send` drains.
//!
//! The History export is the Router's: the platform's save dialog, the write and the Toast that
//! reports it (#811 Q15).

use gpui::{AppContext as _, Context, Entity};
use lib_budgets::presenter::{
    BudgetFormPresenter, BudgetsPageEvent, BudgetsPagePresenter, DetailPresenter, FillPresenter,
    LimitFormPresenter, ManagePresenter, StopPresenter, SwitcherPresenter,
};
use lib_budgets::{BudgetsRouter, ExportHistory, SharedPorts};
use lib_core::{BudgetId, CategoryId, Clock, DateStyle, Period};
use lib_toast::{Notice, Notify, ToastKind};
use lib_transactions::query::TransactionFilters;
use lib_viper::dialog::DialogKeys;

use super::view::page::BudgetsPage;
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialog};
use crate::shell::view::module_page::ModulePage;
use crate::view::event::{Reveal, ViewEvent};

/// The file name the History export's save dialog suggests.
const BUDGET_HISTORY_FILE: &str = "budget-history.csv";

/// What the Budgets page's Router is built from for each call: the ports its Dialogs open on, the
/// day they open on and the Dialog keys their hints show.
pub struct BudgetsRoutes {
    pub ports: SharedPorts,
    pub clock: Clock,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}

pub(crate) struct PageRouter<'a, 'b> {
    pub routes: &'a BudgetsRoutes,
    /// The date style the Switcher writes its dates in: the Presenter's.
    pub date_style: Option<DateStyle>,
    pub cx: &'a mut Context<'b, BudgetsPage>,
}

impl PageRouter<'_, '_> {
    /// Hosts `presenter`'s Dialog, which hands its outcome back to the page. A Dialog with nothing
    /// to open on (`None`) opens nothing.
    fn open<P>(&mut self, presenter: Option<P>) -> bool
    where
        P: DialogView<Outcome = lib_budgets::BudgetsOutcome>,
    {
        let Some(presenter) = presenter else {
            return false;
        };
        let landed = BudgetsPage::landing(self.cx);
        ModuleDialog::open(self.cx, presenter, landed);
        true
    }

    fn ports(&self) -> &SharedPorts {
        &self.routes.ports
    }

    fn clock(&self) -> Clock {
        self.routes.clock.clone()
    }

    fn keys(&self) -> DialogKeys {
        self.routes.dialog_keys.get()
    }
}

impl BudgetsRouter for PageRouter<'_, '_> {
    fn open_switcher(&mut self, current: BudgetId) -> bool {
        let presenter = SwitcherPresenter::new(
            self.ports().clone(),
            current,
            self.clock(),
            self.date_style,
            self.keys(),
        );
        self.open(Some(presenter))
    }

    fn open_manage(&mut self, current: BudgetId) -> bool {
        let presenter =
            ManagePresenter::new(self.ports().clone(), current, self.clock(), self.keys());
        self.open(Some(presenter))
    }

    fn open_new_budget(&mut self, current: BudgetId) -> bool {
        let presenter =
            BudgetFormPresenter::new_budget(self.ports(), self.clock(), current, self.keys());
        self.open(Some(presenter))
    }

    fn open_edit_budget(&mut self, id: BudgetId) -> bool {
        let presenter = BudgetFormPresenter::edit(self.ports(), self.clock(), id, self.keys());
        self.open(presenter)
    }

    fn open_limit(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        let presenter = LimitFormPresenter::edit(
            self.ports(),
            self.clock(),
            budget,
            category,
            month,
            self.keys(),
        );
        self.open(presenter)
    }

    fn open_limit_picker(&mut self, budget: BudgetId) -> bool {
        let presenter = LimitFormPresenter::picker(self.ports(), self.clock(), budget, self.keys());
        self.open(presenter)
    }

    fn open_stop(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        let presenter = StopPresenter::new(
            self.ports(),
            self.clock(),
            budget,
            category,
            month,
            self.keys(),
        );
        self.open(presenter)
    }

    fn open_fill(&mut self, budget: BudgetId, month: Period) -> bool {
        let presenter = FillPresenter::new(self.ports(), self.clock(), budget, month, self.keys());
        self.open(presenter)
    }

    fn open_detail(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        let presenter = DetailPresenter::new(
            self.ports(),
            self.clock(),
            budget,
            category,
            month,
            self.keys(),
        );
        self.open(presenter)
    }

    /// Asks where to save through the platform's save dialog, writes the CSV there and raises a
    /// Toast naming the path. A cancelled dialog does nothing.
    fn export_history(&mut self, export: ExportHistory) {
        let directory = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_default();
        let chosen = self
            .cx
            .prompt_for_new_path(&directory, Some(BUDGET_HISTORY_FILE));
        self.cx
            .spawn(async move |this, cx| {
                let outcome = match chosen.await {
                    Ok(Ok(Some(path))) => std::fs::write(&path, export.csv)
                        .map(|()| path.display().to_string())
                        .map_err(|error| error.to_string()),
                    Ok(Err(error)) => Err(error.to_string()),
                    // Cancelled, or the dialog went away.
                    Ok(Ok(None)) | Err(_) => return,
                };
                let (kind, text) = match outcome {
                    Ok(path) => (
                        ToastKind::Success,
                        crate::msg::desktop_budgets_history_exported(&path),
                    ),
                    Err(error) => (
                        ToastKind::Error,
                        crate::msg::desktop_budgets_history_export_failed(&error),
                    ),
                };
                this.update(cx, |_, cx| cx.emit(ViewEvent::RaiseToast { kind, text }))
                    .ok();
            })
            .detach();
    }

    fn show_bills_period(&mut self, period: Period) {
        self.cx.emit(ViewEvent::Reveal(Reveal::BillsPeriod(period)));
    }

    /// Transactions filtered to the Category, the month and the Budget's on-budget Accounts, through
    /// `transactions::router::show_filtered`.
    fn show_transactions(&mut self, budget: BudgetId, category: CategoryId, month: Period) {
        let account_ids = self
            .ports()
            .borrow()
            .budgets()
            .budget(budget)
            .map(|budget| budget.account_ids)
            .unwrap_or_default();
        crate::transactions::router::show_filtered(
            self.cx,
            TransactionFilters::for_budget_category(category, month, &account_ids),
        );
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Budgets module's entry point: its page, built on `ports`, which the Shell hosts as the
/// Budgets destination. `clock` reads the calendar day the figures are worked out on.
pub fn budgets_page(
    ports: SharedPorts,
    announcer: &Entity<Announcer>,
    keymap: lib_viper::keymap::Keymap<BudgetsPageEvent>,
    clock: Clock,
    dialog_keys: crate::dialog_keys::LiveDialogKeys,
    cx: &mut gpui::App,
) -> Entity<BudgetsPage> {
    let presenter = BudgetsPagePresenter::new(ports.clone(), clock.clone(), keymap);
    let routes = BudgetsRoutes {
        ports,
        clock,
        dialog_keys,
    };
    cx.new(|cx| ModulePage::new(presenter, routes, announcer, cx))
}
