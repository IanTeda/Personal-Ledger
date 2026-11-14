//! `impl BudgetsRouter` for the Desktop: where the Budgets page's Presenter asks to go. Every
//! Dialog (the Switcher, Manage budgets, the Budget form, Edit budget, Stop budgeting, Fill and the
//! Category detail) opens as a `ModuleDialog` in the Dialog host, holding its Presenter. All of it
//! goes through the generic [`PageRouter`]'s gpui context, which is why the Router is built for
//! each `handle` call (`PageView::with_router`); the page lends it the Presenter's date style. It
//! queues a Notice for the input's `send` to drain.
//!
//! The History export is the Router's: the platform's save dialog, the write and the Toast that
//! reports it (#811 Q15).

use gpui::{AppContext as _, Entity};
use lib_budgets::presenter::{
    BudgetFormPresenter, BudgetsPagePresenter, DetailPresenter, FillPresenter, LimitFormPresenter,
    ManagePresenter, StopPresenter, SwitcherPresenter,
};
use lib_budgets::{BudgetsRouter, ExportHistory, SharedPorts};
use lib_core::{BudgetId, CategoryId, Clock, DateStyle, Period};
use lib_toast::ToastKind;
use lib_transactions::query::TransactionFilters;
use lib_viper::dialog::DialogKeys;

use super::BudgetsSetup;
use super::view::page::BudgetsPage;
use crate::announcer::Announcer;
use crate::shell::view::dialog_entity::DialogView;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;
use crate::view::event::{Reveal, ViewEvent};

/// The file name the History export's save dialog suggests.
const BUDGET_HISTORY_FILE: &str = "budget-history.csv";

/// The Budgets page's Router, lent the date style the Switcher writes its dates in: the
/// Presenter's.
pub(crate) type BudgetsPageRouter<'a, 'b> =
    PageRouter<'a, 'b, BudgetsPagePresenter, Option<DateStyle>>;

impl BudgetsPageRouter<'_, '_> {
    /// Hosts `presenter`'s Dialog, which hands its outcome back to the page. A Dialog with nothing
    /// to open on (`None`) opens nothing.
    fn open_found<P>(&mut self, presenter: Option<P>) -> bool
    where
        P: DialogView<Outcome = lib_budgets::BudgetsOutcome>,
    {
        let Some(presenter) = presenter else {
            return false;
        };
        self.open(presenter);
        true
    }

    fn ports(&self) -> &SharedPorts {
        &self.setup.ports
    }

    fn clock(&self) -> Clock {
        self.setup.clock.clone()
    }

    fn keys(&self) -> DialogKeys {
        self.setup.dialog_keys.get()
    }
}

impl BudgetsRouter for BudgetsPageRouter<'_, '_> {
    fn open_switcher(&mut self, current: BudgetId) -> bool {
        let presenter = SwitcherPresenter::new(
            self.ports().clone(),
            current,
            self.clock(),
            self.lent,
            self.keys(),
        );
        self.open_found(Some(presenter))
    }

    fn open_manage(&mut self, current: BudgetId) -> bool {
        let presenter =
            ManagePresenter::new(self.ports().clone(), current, self.clock(), self.keys());
        self.open_found(Some(presenter))
    }

    fn open_new_budget(&mut self, current: BudgetId) -> bool {
        let presenter =
            BudgetFormPresenter::new_budget(self.ports(), self.clock(), current, self.keys());
        self.open_found(Some(presenter))
    }

    fn open_edit_budget(&mut self, id: BudgetId) -> bool {
        let presenter = BudgetFormPresenter::edit(self.ports(), self.clock(), id, self.keys());
        self.open_found(presenter)
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
        self.open_found(presenter)
    }

    fn open_limit_picker(&mut self, budget: BudgetId) -> bool {
        let presenter = LimitFormPresenter::picker(self.ports(), self.clock(), budget, self.keys());
        self.open_found(presenter)
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
        self.open_found(presenter)
    }

    fn open_fill(&mut self, budget: BudgetId, month: Period) -> bool {
        let presenter = FillPresenter::new(self.ports(), self.clock(), budget, month, self.keys());
        self.open_found(presenter)
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
        self.open_found(presenter)
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

/// The Budgets module's entry point: its page, `presenter` routed over `setup`, which the Shell
/// hosts as the Budgets destination.
pub fn budgets_page(
    presenter: BudgetsPagePresenter,
    setup: BudgetsSetup,
    announcer: &Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<BudgetsPage> {
    cx.new(|cx| ModulePage::new(presenter, setup, announcer, cx))
}
