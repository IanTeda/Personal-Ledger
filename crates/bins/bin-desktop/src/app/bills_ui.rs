//! Today's Pay Dialog in the Dialog host, which the Bills Router opens through
//! `ViewEvent::OpenDialog` and the Shell still draws and confirms until `lib-bill-payments` takes it
//! (#779). The Bills page and its other Dialogs are the Bills module's (`crate::bills`).

use super::Shell;

use std::rc::Rc;

use lib_bills::BillsPorts as _;
use lib_bills::interactor::MatchBill;

use crate::{
    bills::view::pay_dialog,
    bills::{
        self,
        pay_form::{PayAction, PayForm},
    },
    chrome::dialog_apply,
    view::format,
};

use gpui::{App, Context, Entity};

impl Shell {
    /// The Bill Plans, read through their store.
    pub(super) fn bill_plans<'a>(&self, cx: &'a App) -> &'a [bills::BillPlan] {
        self.ledger.read(cx).ledger().bill_plans()
    }

    /// The Bill Schedule entries, read through their store.
    pub(super) fn bill_entries<'a>(&self, cx: &'a App) -> &'a [bills::BillScheduleEntry] {
        self.ledger.read(cx).ledger().bill_entries()
    }

    /// Confirms the Pay Dialog through the host's cycle (`Enter` and the confirm button). Pay it
    /// directly runs `lib-ledger`'s command; Match runs `lib-bills`' Interactor on the Bills ports.
    /// A refusal stays on the form, and a landed one closes the Dialog.
    pub(super) fn confirm_pay_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let ledger = self.ledger.clone();
        let ports = self.bills.clone();
        let Some(form) = self.dialog_mut::<PayForm>() else {
            return;
        };
        let landed = match form.action() {
            Some(PayAction::Pay { .. }) => {
                dialog_apply::confirm_ledger(form, &ledger, cx).is_some()
            }
            Some(PayAction::Match(split)) => {
                bills::ports::copy_reads(&ports, ledger.read(cx).ledger());
                let matched = {
                    let mut ports = ports.borrow_mut();
                    let ports = ports.split_mut();
                    MatchBill {
                        entry: form.entry,
                        split,
                    }
                    .run(
                        ports.bills,
                        ports.transactions,
                        ports.accounts,
                        ports.categories,
                    )
                };
                bills::ports::sync(&ports, &ledger, cx);
                match matched {
                    Ok(()) => true,
                    Err(refusal) => {
                        form.error = Some(refusal);
                        false
                    }
                }
            }
            None => false,
        };
        if landed {
            self.close_dialog();
        }
    }

    pub(super) fn pay_form_mut(&mut self) -> Option<&mut PayForm> {
        self.dialog_mut::<PayForm>()
    }

    pub(super) fn handle_pay_bill_mode_click(
        &mut self,
        mode: bills::pay_form::PayMode,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.set_mode(mode);
        }
        cx.notify();
    }

    pub(super) fn handle_pay_bill_choice_click(
        &mut self,
        choice: bills::pay_form::MatchChoice,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.choose(choice);
        }
        cx.notify();
    }

    pub(super) fn handle_pay_bill_field_click(
        &mut self,
        field: bills::pay_form::PayField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.pay_form_mut() {
            form.focus(field);
        }
        cx.notify();
    }

    /// The Pay dialog (8d) over the Schedule, or nothing once its entry's Plan is gone.
    pub(super) fn render_pay_bill_dialog(
        &self,
        form: &PayForm,
        entity: &Entity<Self>,
        cx: &App,
    ) -> Option<gpui::AnyElement> {
        let plan = bills::get(self.bill_plans(cx), form.entry.plan_id)?;
        let on_mode_click: pay_dialog::OnModeClick = {
            let entity = entity.clone();
            Rc::new(move |mode, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_pay_bill_mode_click(mode, cx));
            })
        };
        let on_choice_click: pay_dialog::OnChoiceClick = {
            let entity = entity.clone();
            Rc::new(move |choice, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_pay_bill_choice_click(choice, cx)
                });
            })
        };
        let on_field_click: pay_dialog::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_pay_bill_field_click(field, cx));
            })
        };
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let account = self
            .ledger
            .read(cx)
            .ledger()
            .accounts()
            .iter()
            .find(|a| a.id == plan.account_id)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let payee = plan
            .payee_id
            .and_then(|id| lib_payees::get(self.ledger.read(cx).ledger().payees(), id))
            .map_or_else(crate::msg::desktop_bills_pay_no_payee, |p| p.name.clone());
        let category =
            lib_categories::path(self.categories(cx), plan.category_id).unwrap_or_default();
        Some(pay_dialog::render(
            pay_dialog::PayDialogProps {
                plan_name: &plan.name,
                due: lib_locale::format::format_month_day(form.entry.due),
                form,
                candidates: self.pay_bill_candidate_rows(form, cx),
                planned: format!("{} {}", format::amount(&plan.planned_amount).1, plan.unit),
                chips: [category, payee, account],
                amount_invalid: form.amount_invalid(),
                date_error: form.date_error(),
                error: form
                    .error
                    .as_ref()
                    .map(|_| crate::msg::desktop_bills_pay_error_gone()),
                valid: form.action().is_some(),
                handlers: pay_dialog::PayDialogHandlers {
                    on_mode_click,
                    on_choice_click,
                    on_field_click,
                    on_cancel: plain(Shell::handle_dialog_cancel),
                    on_confirm: plain(Shell::handle_dialog_confirm),
                },
            },
            cx,
        ))
    }

    /// The Pay dialog's Match candidates, worded for its radio list.
    pub(super) fn pay_bill_candidate_rows(
        &self,
        form: &PayForm,
        cx: &App,
    ) -> Vec<pay_dialog::CandidateRow> {
        form.candidates
            .iter()
            .filter_map(|split_ref| {
                let transaction = self
                    .transactions(cx)
                    .iter()
                    .find(|t| t.id == split_ref.transaction_id)?;
                let split = transaction.splits.get(split_ref.split_index)?;
                let payee = split
                    .payee_id
                    .and_then(|id| lib_payees::get(self.ledger.read(cx).ledger().payees(), id))
                    .map(|p| p.name.clone())
                    .or_else(|| transaction.description.clone())
                    .unwrap_or_default();
                let account = self
                    .ledger
                    .read(cx)
                    .ledger()
                    .accounts()
                    .iter()
                    .find(|a| a.id == transaction.account_id)
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                Some(pay_dialog::CandidateRow {
                    summary: format!(
                        "{payee} \u{b7} {} \u{b7} {}",
                        lib_locale::format::format_month_day(transaction.date),
                        format::signed_amount(&split.amount).1,
                    ),
                    account,
                })
            })
            .collect()
    }

    /// Today's Pay Dialog, if it is the one open.
    pub(super) fn render_bills_dialog(
        &self,
        entity: &Entity<Self>,
        cx: &App,
    ) -> Option<gpui::AnyElement> {
        let form = self.dialog::<PayForm>()?;
        self.render_pay_bill_dialog(form, entity, cx)
    }
}
