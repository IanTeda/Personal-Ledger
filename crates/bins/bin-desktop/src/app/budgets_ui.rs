//! The Budgets dialogs' wiring in the Dialog host: confirming them, the requests their lists answer
//! with, the palette's `budgets` verbs and the element builders for each dialog. The page itself
//! is the `BudgetsView` Entity (ADR-0037), which builds the dialogs `Shell` opens here; the pure
//! rules live in `budgets`.

use crate::budgets::{
    BudgetsDialog,
    apply::{BudgetOutcome, BudgetsApply},
};
use lib_ledger::{ArchiveBudget, DuplicateBudget, LedgerCommand, RestoreBudget, SetDefaultBudget};

use super::Shell;
use std::rc::Rc;

use crate::{
    budgets,
    categories::{self},
    chrome::rail::{self as chrome_rail},
    chrome::{dialog_apply, dialog_host::OpenDialog},
    payees::{self},
    period::Period,
    transactions::query::TransactionFilters,
    view::format,
    view::{
        accounts as accounts_view,
        budgets::{self as budgets_view, manage_dialog::ManageAction},
    },
};

use gpui::{Context, px};

/// Where the Switcher's card starts below the window's top: under the top bar, the page's top
/// padding and the title line.
pub(super) const BUDGETS_SWITCHER_TOP: gpui::Pixels = px(122.0);

impl Shell {
    /// The shared Budgets, read from their store Entity.
    pub(super) fn budgets<'a>(&self, cx: &'a gpui::App) -> &'a budgets::Budgets {
        self.budgets_store.read(cx).budgets(cx)
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
    pub(super) fn budgets_current(&self, cx: &gpui::App) -> u32 {
        self.views.budgets.read(cx).current()
    }

    /// Everything the Budgets figures are worked out from, as the Budgets page reads it.
    pub(super) fn budgets_ledger<'a>(&self, cx: &'a gpui::App) -> budgets::BudgetSources<'a> {
        self.views.budgets.read(cx).budget_sources(cx)
    }

    /// The open Category detail's figures, or `None` once its Budget or Category is gone.
    pub(super) fn budgets_detail(&self, cx: &gpui::App) -> Option<budgets::CategoryDetail> {
        let Some(BudgetsDialog::CategoryDetail(detail)) = self.dialog::<BudgetsDialog>() else {
            return None;
        };
        self.views
            .budgets
            .read(cx)
            .category_detail(detail.category_id, detail.month, cx)
    }

    /// Whether 9d offers Edit budget on `category_id`.
    pub(super) fn budgets_detail_editable(&self, category_id: u32, cx: &gpui::App) -> bool {
        self.views.budgets.read(cx).detail_editable(category_id, cx)
    }

    /// The Budget on show when it takes edits: an archived one is read-only.
    pub(super) fn budgets_editable<'a>(&self, cx: &'a gpui::App) -> Option<&'a budgets::Budget> {
        self.views.budgets.read(cx).editable(cx)
    }

    /// Opens a dialog the Budgets page built, when it built one.
    fn open_budgets_built(&mut self, dialog: Option<BudgetsDialog>) {
        if let Some(dialog) = dialog {
            self.open_budgets_dialog(dialog);
        }
    }

    /// 9e on a leaf Category, from 9d's **Edit budget**.
    pub(super) fn open_budgets_limit(&mut self, category_id: u32, month: Period, cx: &gpui::App) {
        let dialog = self
            .views
            .budgets
            .read(cx)
            .limit_dialog(category_id, month, cx);
        self.open_budgets_built(dialog);
    }

    /// 9g, from 9e's footer link.
    pub(super) fn open_budgets_stop(&mut self, category_id: u32, month: Period, cx: &gpui::App) {
        let dialog = self
            .views
            .budgets
            .read(cx)
            .stop_dialog(category_id, month, cx);
        self.open_budgets_built(dialog);
    }

    /// `budgets switch`: 11b with the Budget on show highlighted.
    pub(super) fn open_budgets_switcher(&mut self, cx: &gpui::App) {
        let dialog = self.views.budgets.read(cx).switcher_dialog(cx);
        self.open_budgets_dialog(dialog);
    }

    /// `budgets new` and the Switcher's and Manage's `+ New budget`: 11c.
    pub(super) fn open_budgets_new(&mut self, cx: &gpui::App) {
        let dialog = self.views.budgets.read(cx).new_budget_dialog(cx);
        self.open_budgets_dialog(dialog);
    }

    /// 11c's edit mode on Budget `id`. An archived Budget is read-only, so it opens nothing.
    pub(super) fn open_budgets_edit(&mut self, id: u32, cx: &gpui::App) {
        let dialog = self.views.budgets.read(cx).edit_budget_dialog(id, cx);
        self.open_budgets_built(dialog);
    }

    /// `budgets manage` and the Switcher's `Manage budgets…`: 11f.
    pub(super) fn open_budgets_manage(&mut self, cx: &gpui::App) {
        let dialog = self.views.budgets.read(cx).manage_dialog(cx);
        self.open_budgets_dialog(dialog);
    }

    /// Shows Budget `id` on the Budgets page.
    pub(super) fn switch_budget(&mut self, id: u32, cx: &mut gpui::App) {
        self.views
            .budgets
            .update(cx, |view, cx| view.switch_budget(id, cx));
    }

    /// Confirms the open Budgets dialog. The forms that write (the Budget form, Edit budget, Stop
    /// and Fill) go through the host's cycle: the command runs against the Budgets store, a
    /// refusal stays on the form, and a landed one closes the dialog. The Switcher, Manage and
    /// Category detail lists answer with a request instead, run by [`Self::apply_budgets_request`].
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
            let Some(OpenDialog::Budgets(dialog)) = self.chrome.dialog.take() else {
                return;
            };
            self.nav.exit_mode();
            self.apply_budgets_request(*dialog, cx);
            return;
        }
        let mut apply = BudgetsApply {
            ledger: self.ledger.clone(),
            current: self.budgets_current(cx),
            today: self.today,
        };
        let Some(dialog) = self.dialog_mut::<BudgetsDialog>() else {
            return;
        };
        let Some(outcome) = dialog_apply::confirm(dialog, &mut apply, cx) else {
            return;
        };
        self.close_dialog();
        if let BudgetOutcome::Created(id) = outcome {
            self.switch_budget(id, cx);
        }
    }

    /// Runs the request a list dialog answered with, on the dialog the confirm took. A Dialog
    /// that stays open afterwards (Manage's `*`/`x`, 9d's handoffs that find nothing to open) goes
    /// back into the slot first, so the same code runs as when a click made the request.
    fn apply_budgets_request(&mut self, dialog: BudgetsDialog, cx: &mut Context<'_, Self>) {
        use budgets::{DetailRequest, ManageRequest, SwitcherRequest};
        match dialog {
            BudgetsDialog::Switcher(switcher) => match switcher.request {
                SwitcherRequest::Choose => {
                    if let Some(id) = switcher.chosen() {
                        self.choose_budgets_switcher(id, cx);
                    }
                }
                SwitcherRequest::New => self.open_budgets_new(cx),
            },
            BudgetsDialog::Manage(mut manage) => match manage.request.take() {
                Some(ManageRequest::New) => self.open_budgets_new(cx),
                Some(ManageRequest::Action(id, action)) => {
                    self.open_budgets_dialog(BudgetsDialog::Manage(manage));
                    self.run_budgets_manage_action(id, action, cx);
                }
                None => {}
            },
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
    /// Budget's on-budget Accounts, following [`Self::open_payee_transactions`].
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
        self.open_transactions_filtered(
            TransactionFilters::for_budget_category(category_id, month, &account_ids),
            cx,
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

    /// `enter` or a click on a Switcher row: switches to it and closes the popover.
    pub(super) fn choose_budgets_switcher(&mut self, id: u32, cx: &mut gpui::App) {
        self.close_dialog();
        self.switch_budget(id, cx);
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
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
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
            on_cancel: plain(Shell::handle_dialog_cancel),
            on_confirm: plain(Shell::handle_dialog_confirm),
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

    /// Runs one of 11f's actions on Budget `id`. Open and Edit leave Manage budgets; Duplicate
    /// opens the copy in edit mode; the rest stay, showing a refusal when there is one. Run from
    /// the palette with Manage closed, a refusal goes to the status line instead.
    pub(super) fn run_budgets_manage_action(
        &mut self,
        id: u32,
        action: ManageAction,
        cx: &mut Context<'_, Self>,
    ) {
        let outcome = match action {
            ManageAction::Open => {
                self.choose_budgets_switcher(id, cx);
                return;
            }
            ManageAction::Edit => {
                self.open_budgets_edit(id, cx);
                return;
            }
            ManageAction::Duplicate => {
                self.apply_budgets_command(cx, DuplicateBudget(id))
                    .map(|copy| {
                        self.switch_budget(copy, cx);
                        self.open_budgets_edit(copy, cx);
                    })
            }
            ManageAction::SetDefault => self.apply_budgets_command(cx, SetDefaultBudget(id)),
            ManageAction::Archive => self.apply_budgets_command(
                cx,
                ArchiveBudget {
                    id,
                    today: self.today,
                },
            ),
            ManageAction::Restore => self.apply_budgets_command(cx, RestoreBudget(id)),
        };
        let refused = outcome.err();
        match self.chrome.dialog.as_mut() {
            Some(OpenDialog::Budgets(dialog)) if matches!(**dialog, BudgetsDialog::Manage(_)) => {
                if let BudgetsDialog::Manage(manage) = &mut **dialog {
                    // Archiving or restoring moves the row between the two sections: follow it.
                    manage.refresh(self.budgets_store.read(cx).budgets(cx), id);
                    manage.error = refused;
                }
            }
            _ => {
                if let Some(error) = refused {
                    self.chrome.status_message = Some(match error {
                        budgets::BudgetError::DefaultCannotBeArchived => {
                            crate::msg::desktop_budgets_manage_error_default()
                        }
                        other => budgets_view::limit_dialog::error_text(&other),
                    });
                }
            }
        }
    }

    /// Manage budgets (11f).
    pub(super) fn render_budgets_manage_dialog(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let Some(BudgetsDialog::Manage(manage)) = self.dialog::<BudgetsDialog>() else {
            return None;
        };
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let current = Period::of(self.today);
        let rows = budgets::switcher_ids(self.budgets(cx), "")
            .into_iter()
            .filter_map(|id| self.budgets(cx).get(id))
            .map(|budget| budgets_view::manage_dialog::ManageRow {
                id: budget.id,
                name: budget.name.clone(),
                method: budget.method,
                accounts: crate::msg::desktop_budgets_manage_accounts(
                    i64::try_from(budget.account_ids.len()).unwrap_or(i64::MAX),
                ),
                scope: crate::msg::desktop_budgets_manage_scope(
                    i64::try_from(
                        budget
                            .category_ids()
                            .filter(|id| budgets::applied(budget.chain(*id), current).is_some())
                            .count(),
                    )
                    .unwrap_or(i64::MAX),
                ),
                is_default: budget.is_default,
                archived: budget.is_archived(),
            })
            .collect();
        Some(budgets_view::manage_dialog::render(
            budgets_view::manage_dialog::ManageDialogProps {
                counts: crate::msg::desktop_budgets_manage_counts(
                    &self.budgets(cx).active().count().to_string(),
                    &self.budgets(cx).archived().count().to_string(),
                    &self
                        .budgets(cx)
                        .default_budget()
                        .map(|budget| budget.name.clone())
                        .unwrap_or_default(),
                ),
                rows,
                selected: manage.selected,
                error: manage.error.clone(),
                on_row_click: {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            if let Some(BudgetsDialog::Manage(manage)) =
                                shell.dialog_mut::<BudgetsDialog>()
                            {
                                manage.selected = index;
                            }
                            cx.notify();
                        });
                    })
                },
                on_action_click: {
                    let entity = entity.clone();
                    Rc::new(move |id, action, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.run_budgets_manage_action(id, action, cx);
                            cx.notify();
                        });
                    })
                },
                on_new: plain(Shell::handle_budgets_new_click),
                on_close: plain(Shell::handle_dialog_cancel),
            },
            cx,
        ))
    }

    pub(super) fn handle_budgets_switcher_search_click(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(BudgetsDialog::Switcher(switcher)) = self.dialog_mut::<BudgetsDialog>() {
            switcher.searching = true;
        }
        cx.notify();
    }

    pub(super) fn handle_budgets_new_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_new(cx);
        cx.notify();
    }

    pub(super) fn handle_budgets_manage_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_manage(cx);
        cx.notify();
    }

    /// A Budget's one-line summary in the Switcher: its health this month, or when it was
    /// archived.
    pub(super) fn budgets_summary(&self, budget: &budgets::Budget, cx: &gpui::App) -> String {
        if let Some(date) = budget.archived_at {
            return crate::msg::desktop_budgets_switcher_archived_on(&format::date(
                date,
                self.settings_date_style,
            ));
        }
        let health = budgets::health(budget, &self.budgets_ledger(cx), self.today);
        let left = format::amount(&health.left).1;
        if health.at_risk > 0 {
            crate::msg::desktop_budgets_switcher_health_at_risk(
                &health.over.to_string(),
                &left,
                &health.at_risk.to_string(),
            )
        } else {
            crate::msg::desktop_budgets_switcher_health(&health.over.to_string(), &left)
        }
    }

    /// The Switcher popover (11b), anchored under the page title.
    pub(super) fn render_budgets_switcher(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let Some(BudgetsDialog::Switcher(switcher)) = self.dialog::<BudgetsDialog>() else {
            return None;
        };
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let rows = budgets::switcher_ids(self.budgets(cx), switcher.query.text())
            .into_iter()
            .filter_map(|id| self.budgets(cx).get(id))
            .map(|budget| budgets_view::switcher::SwitcherRow {
                id: budget.id,
                name: budget.name.clone(),
                is_default: budget.is_default,
                is_current: budget.id == self.budgets_current(cx),
                archived: budget.is_archived(),
                summary: self.budgets_summary(budget, cx),
                method: budget.method,
            })
            .collect();
        // The page's left edge: past the primary rail and the context rail, inside its gutter.
        let rail = match self.nav.primary_rail() {
            crate::navigation::nav::RailMode::Expanded => chrome_rail::primary::WIDTH,
            crate::navigation::nav::RailMode::Collapsed => chrome_rail::primary::COLLAPSED_WIDTH,
        };
        Some(budgets_view::switcher::render(
            budgets_view::switcher::SwitcherProps {
                state: switcher,
                rows,
                left: rail + chrome_rail::context::WIDTH + px(28.0),
                top: BUDGETS_SWITCHER_TOP,
                on_search_click: plain(Shell::handle_budgets_switcher_search_click),
                on_budget_click: {
                    let entity = entity.clone();
                    Rc::new(move |id, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.choose_budgets_switcher(id, cx);
                            cx.notify();
                        });
                    })
                },
                on_new: plain(Shell::handle_budgets_new_click),
                on_manage: plain(Shell::handle_budgets_manage_click),
                on_cancel: plain(Shell::handle_dialog_cancel),
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
        let ledger = self.budgets_ledger(cx);
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let previews = budgets::FillSource::ALL
            .map(|each| budgets::fill_preview(budget, &ledger, month, each));
        let chosen = previews.iter().find(|preview| preview.source == source)?;
        let name = |category_id: &u32| {
            self.categories(cx)
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
                on_cancel: plain(Shell::handle_dialog_cancel),
                on_confirm: plain(Shell::handle_dialog_confirm),
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
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let leaf_name = |category_id: u32| {
            self.categories(cx)
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
                        self.categories(cx),
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
                    on_stop: (!form.is_pick()).then(|| plain(Shell::handle_budgets_limit_stop)),
                    on_cancel: plain(Shell::handle_dialog_cancel),
                    on_confirm: plain(Shell::handle_dialog_confirm),
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
                let on_option_click: accounts_view::select_field::OnOptionClick = {
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
                        on_field_click: plain(Shell::handle_budgets_stop_field_click),
                        on_option_click,
                        on_cancel: plain(Shell::handle_dialog_cancel),
                        on_confirm: plain(Shell::handle_dialog_confirm),
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
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let category =
            categories::path(self.categories(cx), detail.category_id).unwrap_or_default();
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
                    .and_then(|id| payees::get(self.payees_list(cx), id))
                    .map_or_else(crate::msg::desktop_budgets_detail_no_payee, |payee| {
                        payee.name.clone()
                    }),
                account: self
                    .accounts
                    .read(cx)
                    .accounts(cx)
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
                on_close: plain(Shell::handle_budgets_detail_close),
                on_open_transactions: plain(Shell::handle_budgets_detail_transactions),
                on_edit: self
                    .budgets_detail_editable(detail.category_id, cx)
                    .then(|| plain(Shell::handle_budgets_detail_edit)),
            },
            cx,
        ))
    }
}
