//! `impl BudgetsRouter` for the Desktop: where the Budgets page's Presenter asks to go. Every
//! Dialog (the Switcher, Manage budgets, the Budget form, Edit budget, Stop budgeting, Fill and the
//! Category detail) opens as an Entity in the Dialog host, holding its Presenter. All of it goes
//! through the page's gpui context, which is why the Router is built for each `handle` call. A
//! Notice goes onto the bin's queue (`crate::notices`), which the input's `send` drains.
//!
//! The History export is the Router's: the platform's save dialog, the write and the Toast that
//! reports it (#811 Q15).

use gpui::{AppContext as _, Context, Entity};
use lib_budgets::presenter::{
    BudgetFormPresenter, BudgetsPageEvent, DetailPresenter, FillPresenter, LimitFormPresenter,
    ManagePresenter, StopPresenter, SwitcherPresenter,
};
use lib_budgets::{BudgetRepository as _, BudgetsRouter, ExportHistory};
use lib_core::{BudgetId, CategoryId, Clock, DateStyle, Period};
use lib_toast::{Notice, Notify, ToastKind};
use lib_transactions::query::TransactionFilters;

use super::ports::DesktopPorts;
use super::view::dialog::ListDialog;
use super::view::form_dialog::{Form, FormDialog};
use super::view::page::BudgetsPage;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;
use crate::view::event::{Reveal, ViewEvent};

/// The file name the History export's save dialog suggests.
const BUDGET_HISTORY_FILE: &str = "budget-history.csv";

pub(super) struct PageRouter<'a, 'b> {
    pub ports: DesktopPorts,
    pub ledger: Entity<LedgerStore>,
    /// The day the page's Dialogs are opened on, and the date style the Switcher writes in.
    pub clock: Clock,
    pub date_style: Option<DateStyle>,
    pub cx: &'a mut Context<'b, BudgetsPage>,
}

impl PageRouter<'_, '_> {
    /// Hosts the Dialog `form` opens, which hands its outcome back to the page.
    fn host(&mut self, form: Form) -> bool {
        let dialog = FormDialog::new(form, &self.ledger, self.landed());
        host_dialog(self.cx, |_| dialog);
        true
    }

    /// What a closed Dialog hands back: its outcome, to
    /// [`BudgetsPage::dialog_landed`].
    fn landed(&self) -> super::view::dialog::Landed {
        let page = self.cx.weak_entity();
        Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        })
    }
}

impl BudgetsRouter for PageRouter<'_, '_> {
    fn open_switcher(&mut self, current: BudgetId) -> bool {
        let presenter = SwitcherPresenter::new(
            self.ports.presenter(),
            current,
            self.clock.clone(),
            self.date_style,
        );
        let dialog = ListDialog::switcher(presenter, &self.ledger, self.landed());
        host_dialog(self.cx, |_| dialog);
        true
    }

    fn open_manage(&mut self, current: BudgetId) -> bool {
        let presenter = ManagePresenter::new(self.ports.presenter(), current, self.clock.clone());
        let dialog = ListDialog::manage(presenter, &self.ledger, self.landed());
        host_dialog(self.cx, |_| dialog);
        true
    }

    fn open_new_budget(&mut self, current: BudgetId) -> bool {
        let presenter =
            BudgetFormPresenter::new_budget(&self.ports.presenter(), self.clock.clone(), current);
        self.host(Form::Budget(presenter, 0))
    }

    fn open_edit_budget(&mut self, id: BudgetId) -> bool {
        match BudgetFormPresenter::edit(&self.ports.presenter(), self.clock.clone(), id) {
            Some(presenter) => self.host(Form::Budget(presenter, 0)),
            None => false,
        }
    }

    fn open_limit(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        match LimitFormPresenter::edit(
            &self.ports.presenter(),
            self.clock.clone(),
            budget,
            category,
            month,
        ) {
            Some(presenter) => self.host(Form::Limit(presenter, 0)),
            None => false,
        }
    }

    fn open_limit_picker(&mut self, budget: BudgetId) -> bool {
        match LimitFormPresenter::picker(&self.ports.presenter(), self.clock.clone(), budget) {
            Some(presenter) => self.host(Form::Limit(presenter, 0)),
            None => false,
        }
    }

    fn open_stop(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        match StopPresenter::new(
            &self.ports.presenter(),
            self.clock.clone(),
            budget,
            category,
            month,
        ) {
            Some(presenter) => self.host(Form::Stop(presenter, 0)),
            None => false,
        }
    }

    fn open_fill(&mut self, budget: BudgetId, month: Period) -> bool {
        match FillPresenter::new(&self.ports.presenter(), self.clock.clone(), budget, month) {
            Some(presenter) => self.host(Form::Fill(presenter)),
            None => false,
        }
    }

    fn open_detail(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        match DetailPresenter::new(
            &self.ports.presenter(),
            self.clock.clone(),
            budget,
            category,
            month,
        ) {
            Some(presenter) => self.host(Form::Detail(presenter)),
            None => false,
        }
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
            .ports
            .shared
            .borrow()
            .budgets
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
    ports: DesktopPorts,
    ledger: Entity<LedgerStore>,
    keymap: lib_viper::keymap::Keymap<BudgetsPageEvent>,
    clock: Clock,
    cx: &mut gpui::App,
) -> Entity<BudgetsPage> {
    cx.new(|cx| BudgetsPage::new(ports, ledger, keymap, clock, cx))
}
