//! The legacy Budgets Dialogs' wiring in the Dialog host (#955): confirming them, the requests the
//! Category detail answers with, and the element builders for each. The page, the Switcher and
//! Manage budgets run on `lib-budgets`' Presenters (`crate::budgets::view`); the Dialogs here are
//! built by [`Legacy`] over the page's ports.

use crate::budgets::{
    BudgetsDialog,
    apply::{BudgetOutcome, OnBudget},
    legacy::Legacy,
};
use lib_budgets::BudgetReads;
use lib_core::{BudgetId, CategoryId};
use lib_ledger::LedgerCommand;

use super::Root;
use lib_shell::NavInput;
use lib_transactions::query::TransactionFilters;
use std::rc::Rc;

use crate::{
    budgets,
    period::Period,
    shell::view::{dialog_apply, dialog_host::OpenDialog},
    view::budgets as budgets_view,
    view::format,
};
use lib_categories::{self};

use gpui::Context;

impl Root {
    /// The shared Budgets, read from their store Entity.
    pub(super) fn budgets<'a>(&self, cx: &'a gpui::App) -> &'a budgets::Budgets {
        self.ledger.read(cx).ledger().budgets()
    }

    /// Runs a Budgets command on the Ledger, which announces the change to every reader.
    pub(super) fn apply_budgets_command<C>(
        &self,
        cx: &mut gpui::App,
        command: C,
    ) -> Result<C::Outcome, budgets::BudgetError>
    where
        C: LedgerCommand<Refusal = budgets::BudgetError>,
    {
        self.ledger.update(cx, |store, cx| store.apply(cx, command))
    }

    /// The Budget the Budgets page shows.
    pub(super) fn budgets_current(&self, cx: &gpui::App) -> BudgetId {
        self.views.budgets.read(cx).current()
    }

    /// The legacy Budgets Dialogs' builders, over the page's ports and day.
    pub(super) fn budgets_legacy<'a>(&self, cx: &'a gpui::App) -> Legacy<'a> {
        let page = self.views.budgets.read(cx);
        Legacy {
            ports: page.ports(),
            today: page.today(),
        }
    }

    /// Everything the Budgets figures are worked out from, as the Budgets page reads it.
    pub(super) fn budgets_reads(&self, cx: &gpui::App) -> Rc<lib_database::StoredBudgetReads> {
        self.views.budgets.read(cx).ports().reads()
    }

    /// The open Category detail's figures, or `None` once its Budget or Category is gone.
    pub(super) fn budgets_detail(&self, cx: &gpui::App) -> Option<budgets::CategoryDetail> {
        let Some(BudgetsDialog::CategoryDetail(detail)) = self.dialog::<BudgetsDialog>() else {
            return None;
        };
        self.budgets_legacy(cx).category_detail(
            self.budgets_current(cx),
            detail.category_id,
            detail.month,
        )
    }

    /// Whether 9d offers Edit budget on `category_id`.
    pub(super) fn budgets_detail_editable(&self, category_id: CategoryId, cx: &gpui::App) -> bool {
        self.budgets_legacy(cx)
            .detail_editable(self.budgets_current(cx), category_id)
    }

    /// Opens a Dialog `Legacy` built, when it built one.
    fn open_budgets_built(&mut self, dialog: Option<BudgetsDialog>) {
        if let Some(dialog) = dialog {
            self.open_budgets_dialog(dialog);
        }
    }

    /// 9e on a leaf Category, from 9d's **Edit budget**.
    pub(super) fn open_budgets_limit(
        &mut self,
        category_id: CategoryId,
        month: Period,
        cx: &gpui::App,
    ) {
        let dialog = self
            .budgets_legacy(cx)
            .limit(self.budgets_current(cx), category_id, month);
        self.open_budgets_built(dialog);
    }

    /// 9g, from 9e's footer link.
    pub(super) fn open_budgets_stop(
        &mut self,
        category_id: CategoryId,
        month: Period,
        cx: &gpui::App,
    ) {
        let dialog = self
            .budgets_legacy(cx)
            .stop(self.budgets_current(cx), category_id, month);
        self.open_budgets_built(dialog);
    }

    /// Shows Budget `id` on the Budgets page.
    pub(super) fn switch_budget(&mut self, id: BudgetId, cx: &mut gpui::App) {
        self.views
            .budgets
            .update(cx, |page, cx| page.show_budget(id, cx));
    }

    /// Confirms the open Budgets dialog. The forms that write (the Budget form, Edit budget, Stop
    /// and Fill) go through the host's cycle: the command runs against the Budgets store, a
    /// refusal stays on the form, and a landed one closes the dialog. The Category detail answers
    /// with a request instead, run by [`Self::apply_budgets_request`].
    pub(super) fn confirm_budgets_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let writes = matches!(
            self.dialog::<BudgetsDialog>(),
            Some(
                BudgetsDialog::Budget(_)
                    | BudgetsDialog::EditLimit(_)
                    | BudgetsDialog::Stop(_)
                    | BudgetsDialog::Fill { .. }
            )
        );
        if !writes {
            let Some(OpenDialog::Budgets(dialog)) = self.state.dialog.take() else {
                return;
            };
            self.navigate(NavInput::ExitMode);
            self.apply_budgets_request(*dialog, cx);
            return;
        }
        let ledger = self.ledger.clone();
        let (budget, today) = (self.budgets_current(cx), self.today);
        let Some(dialog) = self.dialog_mut::<BudgetsDialog>() else {
            return;
        };
        let Some(landed) =
            dialog_apply::confirm_ledger_with(dialog, &ledger, cx, |command| OnBudget {
                command,
                budget,
                today,
            })
        else {
            return;
        };
        self.close_dialog();
        if let BudgetOutcome::Created(id) = landed.outcome {
            self.switch_budget(id, cx);
        }
    }

    /// Runs the request the Category detail answered with, on the dialog the confirm took. A
    /// Dialog that stays open afterwards (9d's handoffs that find nothing to open) goes back into
    /// the slot first, so the same code runs as when a click made the request.
    fn apply_budgets_request(&mut self, dialog: BudgetsDialog, cx: &mut Context<'_, Self>) {
        use budgets::DetailRequest;
        match dialog {
            BudgetsDialog::CategoryDetail(mut detail) => {
                let Some(request) = detail.request.take() else {
                    return;
                };
                self.open_budgets_dialog(BudgetsDialog::CategoryDetail(detail));
                match request {
                    DetailRequest::Transactions => self.open_budgets_detail_transactions(cx),
                    DetailRequest::Edit => self.open_budgets_detail_limit(cx),
                }
            }
            // The write forms confirm through the host's cycle.
            BudgetsDialog::Budget(_)
            | BudgetsDialog::EditLimit(_)
            | BudgetsDialog::Fill { .. }
            | BudgetsDialog::Stop(_) => {}
        }
    }

    pub(super) fn open_budgets_dialog(&mut self, dialog: BudgetsDialog) {
        self.open_dialog(OpenDialog::Budgets(Box::new(dialog)));
    }

    /// 9d's `open in Transactions →`: Transactions filtered to the Category, the month and the
    /// Budget's on-budget Accounts, through `transactions::router::show_filtered`.
    pub(super) fn open_budgets_detail_transactions(&mut self, cx: &mut gpui::App) {
        let Some(BudgetsDialog::CategoryDetail(detail)) = self.dialog::<BudgetsDialog>() else {
            return;
        };
        let (category_id, month) = (detail.category_id, detail.month);
        self.close_dialog();
        let account_ids = self
            .budgets(cx)
            .get(self.budgets_current(cx))
            .map(|budget| budget.account_ids.clone())
            .unwrap_or_default();
        crate::transactions::router::show_filtered(
            cx,
            TransactionFilters::for_budget_category(category_id, month, &account_ids),
        );
    }

    /// 9e's footer link: swaps Edit budget for Stop budgeting on the same Category and month.
    pub(super) fn open_budgets_stop_from_limit(&mut self, cx: &gpui::App) {
        let Some(BudgetsDialog::EditLimit(form)) = self.dialog::<BudgetsDialog>() else {
            return;
        };
        let (Some(category_id), Some(month)) = (form.fixed_category, form.month()) else {
            return;
        };
        self.open_budgets_stop(category_id, month, cx);
    }

    pub(super) fn budget_form_mut(&mut self) -> Option<&mut budgets::form::BudgetForm> {
        match self.dialog_mut::<BudgetsDialog>() {
            Some(BudgetsDialog::Budget(form)) => Some(form),
            _ => None,
        }
    }

    /// New budget (11c), or its edit mode.
    pub(super) fn render_budgets_form_dialog(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let Some(BudgetsDialog::Budget(form)) = self.dialog::<BudgetsDialog>() else {
            return None;
        };
        let plain = |handler: fn(&mut Root, &mut Context<'_, Root>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let options = form.options();
        let named = form
            .editing
            .or_else(|| form.copy_source.as_ref().map(|(id, _)| *id));
        let handlers = budgets_view::budget_dialog::BudgetDialogHandlers {
            on_field_click: {
                let entity = entity.clone();
                Rc::new(move |field, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        if let Some(form) = shell.budget_form_mut() {
                            if field == budgets::form::BudgetField::Unit {
                                form.click_unit();
                            } else {
                                form.focus(field);
                            }
                        }
                        cx.notify();
                    });
                })
            },
            on_unit_option_click: {
                let entity = entity.clone();
                Rc::new(move |index, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        if let Some(form) = shell.budget_form_mut() {
                            form.choose_unit(index);
                        }
                        cx.notify();
                    });
                })
            },
            on_account_click: {
                let entity = entity.clone();
                Rc::new(move |id, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        if let Some(form) = shell.budget_form_mut() {
                            form.toggle_account(id);
                        }
                        cx.notify();
                    });
                })
            },
            on_start_click: {
                let entity = entity.clone();
                Rc::new(move |choice, _window, cx| {
                    entity.update(cx, |shell, cx| {
                        if let Some(form) = shell.budget_form_mut() {
                            form.set_start_from(choice);
                        }
                        cx.notify();
                    });
                })
            },
            on_cancel: plain(Root::handle_dialog_cancel),
            on_confirm: plain(Root::handle_dialog_confirm),
        };
        Some(budgets_view::budget_dialog::render(
            budgets_view::budget_dialog::BudgetDialogProps {
                form,
                options: &options,
                budget_name: named
                    .and_then(|id| self.budgets(cx).get(id))
                    .map(|budget| budget.name.clone()),
                handlers,
            },
            cx,
        ))
    }

    pub(super) fn set_budgets_fill_source(&mut self, chosen: budgets::FillSource) {
        if let Some(BudgetsDialog::Fill { source, .. }) = self.dialog_mut::<BudgetsDialog>() {
            *source = chosen;
        }
    }

    /// Fill (9f), with each source's total and the chosen one's diff against the plan.
    pub(super) fn render_budgets_fill_dialog(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let Some(&BudgetsDialog::Fill { month, source }) = self.dialog::<BudgetsDialog>() else {
            return None;
        };
        let budget = self.budgets(cx).get(self.budgets_current(cx))?;
        let reads = self.budgets_reads(cx);
        let ledger: &dyn BudgetReads = &*reads;
        let plain = |handler: fn(&mut Root, &mut Context<'_, Root>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let previews =
            budgets::FillSource::ALL.map(|each| budgets::fill_preview(budget, ledger, month, each));
        let chosen = previews.iter().find(|preview| preview.source == source)?;
        let categories = self.budgets_legacy(cx).categories();
        let name = |category_id: &CategoryId| {
            categories
                .iter()
                .find(|category| category.id == *category_id)
                .map(|category| category.name.clone())
                .unwrap_or_default()
        };
        let sources = previews
            .iter()
            .map(|preview| {
                let (title, detail) = match preview.source {
                    budgets::FillSource::PreviousMonth => (
                        crate::msg::desktop_budgets_fill_source_previous(
                            &lib_locale::format::format_month(month.prev().month),
                        ),
                        crate::msg::desktop_budgets_fill_source_previous_detail(),
                    ),
                    budgets::FillSource::Average => (
                        crate::msg::desktop_budgets_fill_source_average(),
                        crate::msg::desktop_budgets_fill_source_average_detail(
                            &lib_locale::format::format_month(month.prev().prev().prev().month),
                            &lib_locale::format::format_month(month.prev().month),
                        ),
                    ),
                };
                budgets_view::fill_dialog::SourceRow {
                    source: preview.source,
                    title,
                    detail,
                    total: format::amount(&preview.total).1,
                }
            })
            .collect();
        Some(budgets_view::fill_dialog::render(
            budgets_view::fill_dialog::FillDialogProps {
                month: budgets_view::period_label(month),
                sources,
                preview: chosen,
                change_names: chosen
                    .changes
                    .iter()
                    .map(|line| name(&line.category_id))
                    .collect(),
                kept_names: chosen.kept.iter().map(name).collect(),
                on_source_click: {
                    let entity = entity.clone();
                    Rc::new(move |source, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.set_budgets_fill_source(source);
                            cx.notify();
                        });
                    })
                },
                on_cancel: plain(Root::handle_dialog_cancel),
                on_confirm: plain(Root::handle_dialog_confirm),
            },
            cx,
        ))
    }

    /// 9d's **Edit budget**: 9e on the detail's Category and month. A parent's rollup has none.
    pub(super) fn open_budgets_detail_limit(&mut self, cx: &gpui::App) {
        if let Some(BudgetsDialog::CategoryDetail(detail)) = self.dialog::<BudgetsDialog>() {
            let (category_id, month) = (detail.category_id, detail.month);
            self.open_budgets_limit(category_id, month, cx);
        }
    }

    pub(super) fn handle_budgets_limit_stop(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_stop_from_limit(cx);
        cx.notify();
    }

    pub(super) fn limit_form_mut(&mut self) -> Option<&mut budgets::limit_form::LimitForm> {
        match self.dialog_mut::<BudgetsDialog>() {
            Some(BudgetsDialog::EditLimit(form)) => Some(form),
            _ => None,
        }
    }

    pub(super) fn handle_budgets_stop_field_click(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(BudgetsDialog::Stop(form)) = self.dialog_mut::<BudgetsDialog>() {
            form.click_select();
        }
        cx.notify();
    }

    pub(super) fn handle_budgets_detail_edit(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_detail_limit(cx);
        cx.notify();
    }

    /// Edit budget (9e) or Stop budgeting (9g), whichever is open.
    pub(super) fn render_budgets_limit_dialog(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let plain = |handler: fn(&mut Root, &mut Context<'_, Root>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let categories = self.budgets_legacy(cx).categories();
        let leaf_name = |category_id: CategoryId| {
            categories
                .iter()
                .find(|category| category.id == category_id)
                .map(|category| category.name.clone())
        };
        match self.dialog::<BudgetsDialog>()? {
            BudgetsDialog::EditLimit(form) => {
                let draft = form.draft();
                let preview = draft.as_ref().and_then(|draft| {
                    budgets::limit_form::preview(
                        self.budgets(cx),
                        self.budgets_current(cx),
                        &categories,
                        draft,
                        self.today,
                    )
                });
                let handlers = budgets_view::limit_dialog::LimitDialogHandlers {
                    on_field_click: {
                        let entity = entity.clone();
                        Rc::new(move |field, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                if let Some(form) = shell.limit_form_mut() {
                                    match field {
                                        budgets::limit_form::LimitField::Category
                                        | budgets::limit_form::LimitField::Starting => {
                                            form.click_select(field);
                                        }
                                        _ => form.focus(field),
                                    }
                                }
                                cx.notify();
                            });
                        })
                    },
                    on_option_click: {
                        let entity = entity.clone();
                        Rc::new(move |field, index, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                if let Some(form) = shell.limit_form_mut() {
                                    form.choose(field, index);
                                }
                                cx.notify();
                            });
                        })
                    },
                    on_span_click: {
                        let entity = entity.clone();
                        Rc::new(move |span, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                if let Some(form) = shell.limit_form_mut() {
                                    form.set_span(span);
                                }
                                cx.notify();
                            });
                        })
                    },
                    on_rollover_click: {
                        let entity = entity.clone();
                        Rc::new(move |rollover, _window, cx| {
                            entity.update(cx, |shell, cx| {
                                if let Some(form) = shell.limit_form_mut() {
                                    form.set_rollover(rollover);
                                }
                                cx.notify();
                            });
                        })
                    },
                    on_stop: (!form.is_pick()).then(|| plain(Root::handle_budgets_limit_stop)),
                    on_cancel: plain(Root::handle_dialog_cancel),
                    on_confirm: plain(Root::handle_dialog_confirm),
                };
                Some(budgets_view::limit_dialog::render(
                    budgets_view::limit_dialog::LimitDialogProps {
                        category: form.fixed_category.and_then(leaf_name),
                        form,
                        options: &form.options,
                        preview: preview.as_ref(),
                        unchanged_month: preview
                            .as_ref()
                            .and_then(|preview| preview.unchanged.as_ref())
                            .map(|(month, _)| budgets_view::period_label(*month)),
                        month: preview
                            .as_ref()
                            .map(|preview| budgets_view::period_label(preview.month))
                            .unwrap_or_default(),
                        mirrors_category_field: self.budgets_current(cx)
                            == budgets::PERSONAL_SPENDING_ID,
                        valid: draft.is_some(),
                        handlers,
                    },
                    cx,
                ))
            }
            BudgetsDialog::Stop(form) => {
                let category = leaf_name(form.category_id)?;
                let on_option_click: crate::form::select_field::OnOptionClick = {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            if let Some(BudgetsDialog::Stop(form)) =
                                shell.dialog_mut::<BudgetsDialog>()
                            {
                                form.choose(index);
                            }
                            cx.notify();
                        });
                    })
                };
                Some(budgets_view::stop_dialog::render(
                    budgets_view::stop_dialog::StopDialogProps {
                        category: &category,
                        form,
                        options: &form.options,
                        bill_plans: self
                            .bill_plans(cx)
                            .iter()
                            .filter(|plan| plan.category_id == form.category_id && plan.is_active)
                            .map(|plan| plan.name.clone())
                            .collect(),
                        on_field_click: plain(Root::handle_budgets_stop_field_click),
                        on_option_click,
                        on_cancel: plain(Root::handle_dialog_cancel),
                        on_confirm: plain(Root::handle_dialog_confirm),
                    },
                    cx,
                ))
            }
            _ => None,
        }
    }

    pub(super) fn handle_budgets_detail_close(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    pub(super) fn handle_budgets_detail_transactions(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_detail_transactions(cx);
        cx.notify();
    }

    /// The Category detail (9d) over the Progress tab, or nothing once its figures are gone.
    pub(super) fn render_budgets_detail(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let detail = self.budgets_detail(cx)?;
        let plain = |handler: fn(&mut Root, &mut Context<'_, Root>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let reads = self.budgets_reads(cx);
        let reads: &dyn BudgetReads = &*reads;
        let category =
            lib_categories::path(&self.budgets_legacy(cx).categories(), detail.category_id)
                .unwrap_or_default();
        let accounts = reads.budget_accounts();
        let name = category
            .rsplit(" / ")
            .next()
            .unwrap_or(&category)
            .to_string();
        let listed = budgets_view::detail_dialog::LISTED;
        let lines: Vec<budgets_view::detail_dialog::LineRow> = detail
            .lines
            .iter()
            .take(listed)
            .map(|line| budgets_view::detail_dialog::LineRow {
                date: lib_locale::format::format_month_day(line.date),
                payee: line
                    .payee_id
                    .and_then(|id| reads.payee_name(id))
                    .unwrap_or_else(crate::msg::desktop_budgets_detail_no_payee),
                account: accounts
                    .iter()
                    .find(|account| account.id == line.account_id)
                    .map(|account| account.name.clone())
                    .unwrap_or_default(),
                amount: format::amount(&line.amount).1,
            })
            .collect();
        let hidden = detail
            .lines
            .iter()
            .skip(listed)
            .fold(bigdecimal::BigDecimal::from(0), |sum, line| {
                sum + line.amount.0.clone()
            });
        let more = (detail.lines.len() > listed).then(|| {
            crate::msg::desktop_budgets_detail_more(
                &(detail.lines.len() - listed).to_string(),
                &format::amount(&lib_core::Money(hidden)).1,
            )
        });
        let track = detail.track.as_ref().map_or_else(
            crate::msg::desktop_budgets_detail_track_none,
            |track| {
                crate::msg::desktop_budgets_detail_track(
                    &track.over_months.to_string(),
                    i64::from(track.months),
                    &format::amount(&track.average).1,
                )
            },
        );
        let next = budgets_view::period_label(detail.month.next());
        let rollover_note = detail
            .figures
            .over
            .then_some(detail.rollover)
            .flatten()
            .map(|rollover| match rollover {
                budgets::Rollover::None => crate::msg::desktop_budgets_detail_rollover_off(&next),
                budgets::Rollover::CarryUnspent => {
                    crate::msg::desktop_budgets_detail_rollover_unspent(&next)
                }
                budgets::Rollover::CarryBoth => {
                    crate::msg::desktop_budgets_detail_rollover_both(&next)
                }
            });
        let bills = match i64::try_from(detail.bill_plans).unwrap_or(i64::MAX) {
            0 => crate::msg::desktop_budgets_detail_no_bills(),
            count => crate::msg::desktop_budgets_detail_bills(count),
        };
        Some(budgets_view::detail_dialog::render(
            budgets_view::detail_dialog::DetailDialogProps {
                title: crate::msg::desktop_budgets_detail_title(
                    &name,
                    &budgets_view::period_label(detail.month),
                ),
                detail: &detail,
                lines,
                more,
                selected: match self.dialog::<BudgetsDialog>() {
                    Some(BudgetsDialog::CategoryDetail(open)) => open.selected,
                    _ => 0,
                },
                track,
                rollover_note,
                bills,
                on_close: plain(Root::handle_budgets_detail_close),
                on_open_transactions: plain(Root::handle_budgets_detail_transactions),
                on_edit: self
                    .budgets_detail_editable(detail.category_id, cx)
                    .then(|| plain(Root::handle_budgets_detail_edit)),
            },
            cx,
        ))
    }
}
