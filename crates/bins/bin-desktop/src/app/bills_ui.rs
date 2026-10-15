//! The Bills Dialogs in the Dialog host: Add and Edit bill plan, Pay and Skip, which the Bills
//! View opens through `ViewEvent::OpenDialog` (ADR-0037). The page itself is the `BillsView`
//! Entity (`view::bills`); the pure rules live in `bills`.

use super::Shell;
use crate::bills::BillsDialog;

use std::rc::Rc;

use crate::{
    bills::{
        self,
        apply::{BillOutcome, BillsApply},
        pay_form::PayForm,
    },
    categories,
    chrome::dialog_apply,
    payees,
    view::bills as bills_view,
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

    /// The form behind the open Add or Edit bill plan dialog, if that is what's open.
    pub(super) fn bill_plan_form_mut(&mut self) -> Option<&mut bills::form::BillPlanForm> {
        self.dialog_mut::<BillsDialog>()
            .and_then(BillsDialog::plan_form_mut)
    }

    /// Confirms the open Bills dialog through the host's cycle (`Enter` and the confirm button):
    /// the form's command runs against the Ledger, a refusal stays on the form, and a landed
    /// one closes the dialog. **Add bill plan** / **Save** select the Plan on the Planner tab; a
    /// refused Skip closes with the reason in the status line.
    pub(super) fn confirm_bills_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let mut apply = BillsApply {
            ledger: self.ledger.clone(),
            today: self.today,
        };
        let Some(dialog) = self.dialog_mut::<BillsDialog>() else {
            return;
        };
        let Some(outcome) = dialog_apply::confirm(dialog, &mut apply, cx) else {
            return;
        };
        self.close_dialog();
        match outcome {
            BillOutcome::PlanSaved(id) => {
                self.views
                    .bills
                    .update(cx, |view, cx| view.show_saved_plan(id, cx));
            }
            BillOutcome::Paid | BillOutcome::Skipped => {}
            BillOutcome::SkipRefused => {
                self.chrome.status_message = Some(crate::msg::desktop_bills_skip_error_gone());
            }
        }
    }

    pub(super) fn handle_bill_plan_field_click(
        &mut self,
        field: bills::form::BillPlanField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.bill_plan_form_mut() {
            match field {
                bills::form::BillPlanField::Category
                | bills::form::BillPlanField::Unit
                | bills::form::BillPlanField::Account
                | bills::form::BillPlanField::Payee
                | bills::form::BillPlanField::Recurrence => form.click_select(field),
                bills::form::BillPlanField::Active => {
                    form.focus(field);
                    form.toggle();
                }
                _ => form.focus(field),
            }
        }
        cx.notify();
    }

    pub(super) fn handle_bill_plan_option_click(
        &mut self,
        field: bills::form::BillPlanField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.bill_plan_form_mut() {
            form.choose(field, index);
        }
        cx.notify();
    }

    pub(super) fn handle_bill_plan_amount_kind_click(
        &mut self,
        kind: bills::AmountKind,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.bill_plan_form_mut() {
            form.set_amount_kind(kind);
        }
        cx.notify();
    }

    pub(super) fn pay_form_mut(&mut self) -> Option<&mut PayForm> {
        match self.dialog_mut::<BillsDialog>() {
            Some(BillsDialog::Pay(form)) => Some(form),
            _ => None,
        }
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
        let on_mode_click: bills_view::pay_dialog::OnModeClick = {
            let entity = entity.clone();
            Rc::new(move |mode, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_pay_bill_mode_click(mode, cx));
            })
        };
        let on_choice_click: bills_view::pay_dialog::OnChoiceClick = {
            let entity = entity.clone();
            Rc::new(move |choice, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_pay_bill_choice_click(choice, cx)
                });
            })
        };
        let on_field_click: bills_view::pay_dialog::OnFieldClick = {
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
            .accounts
            .read(cx)
            .accounts(cx)
            .iter()
            .find(|a| a.id == plan.account_id)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let payee = plan
            .payee_id
            .and_then(|id| payees::get(self.payees_list(cx), id))
            .map_or_else(crate::msg::desktop_bills_pay_no_payee, |p| p.name.clone());
        let category = categories::path(self.categories(cx), plan.category_id).unwrap_or_default();
        Some(bills_view::pay_dialog::render(
            bills_view::pay_dialog::PayDialogProps {
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
                handlers: bills_view::pay_dialog::PayDialogHandlers {
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
    ) -> Vec<bills_view::pay_dialog::CandidateRow> {
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
                    .and_then(|id| payees::get(self.payees_list(cx), id))
                    .map(|p| p.name.clone())
                    .or_else(|| transaction.description.clone())
                    .unwrap_or_default();
                let account = self
                    .accounts
                    .read(cx)
                    .accounts(cx)
                    .iter()
                    .find(|a| a.id == transaction.account_id)
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                Some(bills_view::pay_dialog::CandidateRow {
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

    /// The Skip dialog (8e) over the Schedule, or nothing once its entry's Plan is gone.
    pub(super) fn render_skip_bill_dialog(
        &self,
        entry: bills::EntryId,
        entity: &Entity<Self>,
        cx: &App,
    ) -> Option<gpui::AnyElement> {
        let plan = bills::get(self.bill_plans(cx), entry.plan_id)?;
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        Some(bills_view::skip_dialog::render(
            bills_view::skip_dialog::SkipDialogProps {
                plan_name: &plan.name,
                due: lib_locale::format::format_month_day(entry.due),
                one_shot: plan.recurrence == bills::Recurrence::OneShot,
                on_cancel: plain(Shell::handle_dialog_cancel),
                on_confirm: plain(Shell::handle_dialog_confirm),
            },
            cx,
        ))
    }

    /// The open Bills dialog in the Dialog host: Add or Edit bill plan, Pay or Skip.
    pub(super) fn render_bills_dialog(
        &self,
        entity: &Entity<Self>,
        cx: &App,
    ) -> Option<gpui::AnyElement> {
        self.dialog::<BillsDialog>().and_then(|dialog| {
            let (editing, form) = match dialog {
                BillsDialog::Add(form) => (None, form),
                BillsDialog::Edit(id, form) => (
                    Some(bills::get(self.bill_plans(cx), *id)?.name.as_str()),
                    form,
                ),
                BillsDialog::Pay(form) => {
                    return self.render_pay_bill_dialog(form, entity, cx);
                }
                BillsDialog::Skip(entry) => {
                    return self.render_skip_bill_dialog(*entry, entity, cx);
                }
            };
            let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
                let entity = entity.clone();
                let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                    entity.update(cx, handler);
                });
                on_click
            };
            let on_field_click: bills_view::plan_dialog::OnFieldClick = {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_bill_plan_field_click(field, cx)
                    });
                })
            };
            let on_option_click: bills_view::plan_dialog::OnOptionClick = {
                let entity = entity.clone();
                Rc::new(move |field, index, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_bill_plan_option_click(field, index, cx);
                    });
                })
            };
            let on_amount_kind_click: bills_view::plan_dialog::OnAmountKindClick = {
                let entity = entity.clone();
                Rc::new(move |kind, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        shell.handle_bill_plan_amount_kind_click(kind, cx);
                    });
                })
            };
            Some(bills_view::plan_dialog::render(
                bills_view::plan_dialog::PlanDialogProps {
                    editing,
                    form,
                    options: &form.options,
                    errors: form.errors(),
                    valid: form.draft().is_some(),
                    handlers: bills_view::plan_dialog::PlanDialogHandlers {
                        on_field_click,
                        on_option_click,
                        on_amount_kind_click,
                        on_cancel: plain(Shell::handle_dialog_cancel),
                        on_confirm: plain(Shell::handle_dialog_confirm),
                    },
                },
                cx,
            ))
        })
    }
}
