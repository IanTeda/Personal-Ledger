//! `impl BudgetsRouter` for the Desktop: where the Budgets page's Presenter asks to go. The
//! Switcher and Manage budgets open as Entities in the Dialog host; the Budget form, Edit budget,
//! Stop budgeting, Fill and the Category detail are still legacy Dialogs (#955), built by
//! [`Legacy`] and opened through `Root`'s host. All of it goes through the page's gpui context,
//! which is why the Router is built for each `handle` call. A Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains.
//!
//! The History export is the Router's: the platform's save dialog, the write and the Toast that
//! reports it (#811 Q15).

use gpui::{AppContext as _, Context, Entity};
use lib_budgets::presenter::{BudgetsPageEvent, ManagePresenter, SwitcherPresenter};
use lib_budgets::{BudgetsOutcome, BudgetsRouter, ExportHistory};
use lib_core::{BudgetId, CategoryId, Clock, DateStyle, Period};
use lib_toast::{Notice, Notify, ToastKind};

use super::BudgetsDialog;
use super::legacy::Legacy;
use super::ports::DesktopPorts;
use super::view::dialog::ListDialog;
use super::view::page::BudgetsPage;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;
use crate::shell::view::dialog_host::OpenDialog;
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
    fn legacy(&self) -> Legacy<'_> {
        Legacy {
            ports: &self.ports,
            today: self.clock.today(),
        }
    }

    /// Opens `dialog` in `Root`'s legacy host, when there is one.
    fn open(&mut self, dialog: Option<BudgetsDialog>) -> bool {
        let Some(dialog) = dialog else {
            return false;
        };
        self.cx
            .emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Budgets(
                Box::new(dialog),
            ))));
        true
    }

    /// What a closed Switcher or Manage budgets hands back: its outcome, to
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
        let dialog = self.legacy().new_budget(current);
        self.open(Some(dialog))
    }

    fn open_edit_budget(&mut self, id: BudgetId) -> bool {
        let dialog = self.legacy().edit_budget(id);
        self.open(dialog)
    }

    fn open_limit(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        let dialog = self.legacy().limit(budget, category, month);
        self.open(dialog)
    }

    fn open_limit_picker(&mut self, budget: BudgetId) -> bool {
        let dialog = self.legacy().limit_picker(budget);
        self.open(dialog)
    }

    fn open_stop(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        let dialog = self.legacy().stop(budget, category, month);
        self.open(dialog)
    }

    fn open_fill(&mut self, budget: BudgetId, month: Period) -> bool {
        let dialog = self.legacy().fill(budget, month);
        self.open(dialog)
    }

    fn open_detail(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool {
        let dialog = self.legacy().detail(budget, category, month);
        self.open(Some(dialog))
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

    /// The page closes no Dialog: a Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<BudgetsOutcome>) {}
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
