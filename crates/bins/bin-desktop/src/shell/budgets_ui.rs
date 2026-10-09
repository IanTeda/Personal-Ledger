//! The Budgets destination's wiring: keys, clicks, dialogs, the Progress, Plan and History tabs' state changes, and the element builders for its dialogs. The pure rules live in `budgets`; the chrome in `view::budgets`.

use lib_core::CategoryTypes;
use lib_toast::ToastKind;

use super::{ACCOUNTS_HALF_PAGE, Shell, key_dispatch::PENDING_G_TIMEOUT};
use std::rc::Rc;

use crate::{
    accounts::{self},
    bills::{self},
    budgets,
    categories::{self},
    chrome::dialog_host::OpenDialog,
    chrome::rail::{self as chrome_rail},
    navigation::key_router::Movement,
    navigation::nav::{FocusZone, InputMode, Noun},
    payees::{self},
    period::Period,
    transactions::query::TransactionFilters,
    view::format,
    view::{
        accounts as accounts_view,
        budgets::{self as budgets_view, manage_dialog::ManageAction},
    },
};

use gpui::{Context, Keystroke, px};

/// Where the Switcher's card starts below the window's top: under the top bar, the page's top
/// padding and the title line.
pub(super) const BUDGETS_SWITCHER_TOP: gpui::Pixels = px(122.0);

/// The file name the History export's save dialog suggests.
pub(super) const BUDGET_HISTORY_FILE: &str = "budget-history.csv";

/// The Budgets Progress tab's status-line legend (`docs/ux/desktop-mockups/14-budgets-v2/README.md`'s
/// 9a).
pub(super) fn budgets_progress_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("e", crate::msg::desktop_hint_edit_budget()),
        ("c", crate::msg::desktop_hint_budget_category()),
        ("s", crate::msg::desktop_hint_stop_budgeting()),
        ("B", crate::msg::desktop_hint_switch_budget()),
        ("[/]", crate::msg::desktop_hint_period()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The Budgets Plan tab's status-line legend in Normal mode (9b).
pub(super) fn budgets_plan_hints() -> Vec<(&'static str, String)> {
    vec![
        ("h/j/k/l", crate::msg::desktop_hint_cell()),
        ("enter", crate::msg::desktop_hint_edit()),
        ("r", crate::msg::desktop_hint_rollover()),
        ("x", crate::msg::desktop_hint_clear()),
        ("f", crate::msg::desktop_hint_fill()),
        ("c", crate::msg::desktop_hint_budget_category()),
        ("[/]", crate::msg::desktop_hint_range()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The Plan grid's legend while a cell is being typed into.
pub(super) fn budgets_plan_insert_hints() -> Vec<(&'static str, String)> {
    vec![
        ("enter", crate::msg::desktop_hint_save_cell()),
        ("esc", crate::msg::desktop_hint_cancel()),
        ("tab", crate::msg::desktop_hint_next_month()),
        ("shift+enter", crate::msg::desktop_hint_this_month_only()),
    ]
}

/// The status-line legend while the Category detail dialog (9d) is open.
pub(super) fn budgets_detail_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_close()),
        ("j/k", crate::msg::desktop_hint_row()),
        ("e", crate::msg::desktop_hint_edit_budget()),
        ("t", crate::msg::desktop_hint_transactions()),
    ]
}

/// The status-line legend while Edit budget (9e) is open.
pub(super) fn budgets_limit_hints() -> Vec<(&'static str, String)> {
    vec![
        ("tab", crate::msg::desktop_hint_next_field()),
        ("\u{2190}/\u{2192}", crate::msg::desktop_hint_choose()),
        ("enter", crate::msg::desktop_hint_save()),
        ("esc", crate::msg::desktop_hint_cancel()),
    ]
}

/// The Budgets History tab's status-line legend (9c).
pub(super) fn budgets_history_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("h/l", crate::msg::desktop_hint_month()),
        ("enter", crate::msg::desktop_hint_open()),
        ("x", crate::msg::desktop_hint_export()),
        ("[/]", crate::msg::desktop_hint_range()),
        ("tab", crate::msg::desktop_hint_switch_view()),
    ]
}

/// The status-line legend while the Switcher (11b) is open.
pub(super) fn budgets_switcher_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("/", crate::msg::desktop_hint_search()),
        ("n", crate::msg::desktop_hint_new_budget()),
        ("esc", crate::msg::desktop_hint_close()),
    ]
}

/// The status-line legend while Manage budgets (11f) is open.
pub(super) fn budgets_manage_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("n", crate::msg::desktop_hint_new_budget()),
        ("e", crate::msg::desktop_hint_edit()),
        ("d", crate::msg::desktop_hint_duplicate()),
        ("*", crate::msg::desktop_hint_set_default()),
        ("x", crate::msg::desktop_hint_archive()),
    ]
}

/// The status-line legend while Fill (9f) is open.
pub(super) fn budgets_fill_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_source()),
        ("enter", crate::msg::desktop_hint_fill()),
        ("esc", crate::msg::desktop_hint_cancel()),
    ]
}

/// The status-line legend while Stop budgeting (9g) is open.
pub(super) fn budgets_stop_hints() -> Vec<(&'static str, String)> {
    vec![
        ("\u{2191}/\u{2193}", crate::msg::desktop_hint_choose()),
        ("enter", crate::msg::desktop_hint_confirm()),
        ("esc", crate::msg::desktop_hint_cancel()),
    ]
}

impl Shell {
    /// Applies a confirmed Budgets dialog (`Enter` and the confirm button). The list dialogs
    /// answer with a request: a Dialog that stays open afterwards (Manage's `*`/`x`, 9d's
    /// handoffs that find nothing to open) goes back into the slot first, so the same code runs
    /// as when a click made the request.
    pub(super) fn apply_budgets_dialog(&mut self, dialog: budgets::BudgetsDialog) {
        use budgets::{BudgetsDialog, DetailRequest, ManageRequest, SwitcherRequest};
        match dialog {
            BudgetsDialog::Switcher(switcher) => match switcher.request {
                SwitcherRequest::Choose => {
                    if let Some(id) = switcher.chosen() {
                        self.choose_budgets_switcher(id);
                    }
                }
                SwitcherRequest::New => self.open_budgets_new(),
            },
            BudgetsDialog::Budget(form) => self.apply_budgets_form(form),
            BudgetsDialog::Manage(mut manage) => match manage.request.take() {
                Some(ManageRequest::New) => self.open_budgets_new(),
                Some(ManageRequest::Action(id, action)) => {
                    self.open_budgets_dialog(BudgetsDialog::Manage(manage));
                    self.run_budgets_manage_action(id, action);
                }
                None => {}
            },
            BudgetsDialog::CategoryDetail(mut detail) => {
                let Some(request) = detail.request.take() else {
                    return;
                };
                self.open_budgets_dialog(BudgetsDialog::CategoryDetail(detail));
                match request {
                    DetailRequest::Transactions => self.open_budgets_detail_transactions(),
                    DetailRequest::Edit => self.open_budgets_detail_limit(),
                }
            }
            BudgetsDialog::EditLimit(form) => self.apply_budgets_limit(form),
            BudgetsDialog::Fill { month, source } => self.apply_budgets_fill(month, source),
            BudgetsDialog::Stop(form) => self.apply_budgets_stop(form),
        }
    }

    pub(super) fn open_budgets_dialog(&mut self, dialog: budgets::BudgetsDialog) {
        self.open_dialog(OpenDialog::Budgets(Box::new(dialog)));
    }

    /// The Budget the Budgets surface shows and its figures for `budgets_period`, as of today.
    pub(super) fn budgets_figures(&self) -> Option<(&budgets::Budget, budgets::PeriodFigures)> {
        let budget = self.budgets.get(self.budgets_state.current)?;
        let figures = budgets::period_figures(
            budget,
            &self.budgets_ledger(),
            self.budgets_state.period,
            self.today,
        );
        Some((budget, figures))
    }

    pub(super) fn budgets_ledger(&self) -> budgets::Ledger<'_> {
        budgets::Ledger {
            categories: &self.categories,
            accounts: &self.accounts,
            transactions: &self.transactions,
            plans: &self.bill_plans,
            entries: &self.bill_entries,
        }
    }

    /// The Plan tab's grid for the range in view.
    pub(super) fn budgets_plan_data(&self) -> Option<budgets::Plan> {
        let budget = self.budgets.get(self.budgets_state.current)?;
        Some(budgets::plan(
            budget,
            &self.budgets_ledger(),
            self.budgets_state.plan_start,
            self.today,
        ))
    }

    /// The cursor held inside the grid, so a row that vanished can't leave it dangling.
    pub(super) fn budgets_plan_cursor_in(&self, plan: &budgets::Plan) -> (usize, usize) {
        (
            self.budgets_state
                .plan_cursor
                .0
                .min(plan.row_count().saturating_sub(1)),
            self.budgets_state
                .plan_cursor
                .1
                .min(budgets::PLAN_ROLLOVER_COLUMN),
        )
    }

    /// Steps the Plan range a month, inside the Budget's bounds.
    pub(super) fn shift_budgets_plan_range(&mut self, forward: bool) -> bool {
        let Some(budget) = self.budgets.get(self.budgets_state.current) else {
            return false;
        };
        let (earliest, latest) = budgets::plan_start_bounds(budget, self.today);
        let target = if forward {
            self.budgets_state.plan_start.next()
        } else {
            self.budgets_state.plan_start.prev()
        };
        if target < earliest || target > latest {
            return false;
        }
        self.budgets_state.plan_start = target;
        true
    }

    /// `enter`/`i`/a click on the cursor cell: types into an open month, or cycles Rollover.
    pub(super) fn start_budgets_plan_edit(&mut self) {
        let Some(plan) = self.budgets_plan_data() else {
            return;
        };
        let (row_index, column) = self.budgets_plan_cursor_in(&plan);
        let Some(row) = plan.row(row_index) else {
            return;
        };
        if column == budgets::PLAN_ROLLOVER_COLUMN {
            let _ = self.budgets.cycle_rollover(
                self.budgets_state.current,
                row.category_id,
                self.today,
            );
            return;
        }
        let Some(cell) = row.cells.get(column) else {
            return;
        };
        let archived = self
            .budgets
            .get(self.budgets_state.current)
            .is_none_or(budgets::Budget::is_archived);
        if cell.closed || archived {
            return;
        }
        self.budgets_state.plan_edit = Some(budgets::PlanEdit::new(
            row.category_id,
            cell.month,
            cell.amount.as_ref(),
        ));
        self.nav.enter_mode(InputMode::Insert);
    }

    /// Saves the typed cell and leaves Insert; `step` moves on to the next (1) or previous (-1)
    /// month's cell and edits it, as `tab`/`shift+tab` do. Text that isn't an amount keeps the
    /// cell open.
    pub(super) fn commit_budgets_plan_edit(&mut self, span: budgets::Span, step: i32) {
        let Some(edit) = self.budgets_state.plan_edit.take() else {
            return;
        };
        let saved = self.budgets.save_cell(
            self.budgets_state.current,
            &self.categories,
            edit.category_id,
            edit.month,
            span,
            &edit.text,
            self.today,
        );
        if saved == Err(budgets::BudgetError::InvalidAmount) {
            self.budgets_state.plan_edit = Some(edit);
            return;
        }
        self.nav.exit_mode();
        if step == 0 {
            return;
        }
        let column = self.budgets_state.plan_cursor.1;
        if step > 0 {
            if column + 1 < budgets::PLAN_MONTHS {
                self.budgets_state.plan_cursor.1 += 1;
            } else if !self.shift_budgets_plan_range(true) {
                return;
            }
        } else if column > 0 {
            self.budgets_state.plan_cursor.1 -= 1;
        } else if !self.shift_budgets_plan_range(false) {
            return;
        }
        self.start_budgets_plan_edit();
    }

    /// Writes straight to the cursor's month cell in Normal mode: `x`/`backspace` clear it (a
    /// Stop from that month) and `0` writes an explicit 0.00 onward.
    pub(super) fn write_budgets_plan_cell(&mut self, text: &str) {
        let Some(plan) = self.budgets_plan_data() else {
            return;
        };
        let (row_index, column) = self.budgets_plan_cursor_in(&plan);
        let Some(row) = plan.row(row_index) else {
            return;
        };
        let Some(cell) = row.cells.get(column).filter(|cell| !cell.closed) else {
            return;
        };
        let _ = self.budgets.save_cell(
            self.budgets_state.current,
            &self.categories,
            row.category_id,
            cell.month,
            budgets::Span::Onward,
            text,
            self.today,
        );
    }

    /// Keys typed into a Plan cell (`docs/ux/desktop-mockups/14-budgets-v2/README.md`'s 9b, Insert
    /// mode). `esc` is left to the router, which cancels the edit and leaves the mode.
    pub(super) fn handle_budgets_plan_edit_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.nav.mode() != InputMode::Insert
            || self.nav.noun() != Noun::Budgets
            || self.budgets_state.plan_edit.is_none()
        {
            return false;
        }
        let shift = keystroke.modifiers.shift;
        match keystroke.key.as_str() {
            "escape" => return false,
            "enter" => self.commit_budgets_plan_edit(
                if shift {
                    budgets::Span::MonthOnly
                } else {
                    budgets::Span::Onward
                },
                0,
            ),
            "tab" => {
                self.commit_budgets_plan_edit(budgets::Span::Onward, if shift { -1 } else { 1 })
            }
            "backspace" => {
                if let Some(edit) = self.budgets_state.plan_edit.as_mut() {
                    edit.backspace();
                }
            }
            _ => {
                if let (Some(edit), Some(text), false) = (
                    self.budgets_state.plan_edit.as_mut(),
                    keystroke.key_char.as_deref(),
                    keystroke.modifiers.control,
                ) {
                    for c in text.chars() {
                        edit.type_char(c);
                    }
                }
            }
        }
        true
    }

    /// The Plan grid's Normal-mode keys. `false` for any it doesn't take.
    pub(super) fn handle_budgets_plan_key(&mut self, key: &str) -> bool {
        match key {
            "h" | "left" => {
                self.budgets_state.plan_cursor.1 =
                    self.budgets_state.plan_cursor.1.saturating_sub(1);
            }
            "l" | "right" => {
                self.budgets_state.plan_cursor.1 =
                    (self.budgets_state.plan_cursor.1 + 1).min(budgets::PLAN_ROLLOVER_COLUMN);
            }
            "i" => self.start_budgets_plan_edit(),
            "r" => {
                if let Some(plan) = self.budgets_plan_data()
                    && let Some(row) = plan.row(self.budgets_plan_cursor_in(&plan).0)
                {
                    let _ = self.budgets.cycle_rollover(
                        self.budgets_state.current,
                        row.category_id,
                        self.today,
                    );
                }
            }
            "f" => self.open_budgets_fill(),
            "x" | "backspace" => self.write_budgets_plan_cell(""),
            "0" => self.write_budgets_plan_cell("0"),
            "[" => {
                self.shift_budgets_plan_range(false);
            }
            "]" => {
                self.shift_budgets_plan_range(true);
            }
            _ => return false,
        }
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` step the Progress rows. `enter` opens the Category detail (9d).
    pub(super) fn apply_budgets_movement(&mut self, movement: Movement) {
        if self.budgets_state.tab == budgets::BudgetsTab::Plan {
            let Some(plan) = self.budgets_plan_data() else {
                return;
            };
            let len = plan.row_count();
            let selected = self.budgets_plan_cursor_in(&plan).0;
            self.budgets_state.plan_cursor.0 = match movement {
                Movement::Next => accounts::step_selection(selected, len, 1),
                Movement::Prev => accounts::step_selection(selected, len, -1),
                Movement::First => 0,
                Movement::Last => len.saturating_sub(1),
                Movement::HalfPageDown => {
                    accounts::step_selection(selected, len, ACCOUNTS_HALF_PAGE)
                }
                Movement::HalfPageUp => {
                    accounts::step_selection(selected, len, -ACCOUNTS_HALF_PAGE)
                }
                Movement::Enter => {
                    self.budgets_state.plan_cursor.0 = selected;
                    self.start_budgets_plan_edit();
                    selected
                }
            };
            return;
        }
        if self.budgets_state.tab == budgets::BudgetsTab::History {
            let Some(history) = self.budgets_history_data() else {
                return;
            };
            let len = history.rows.len();
            let selected = self.budgets_history_cursor_in(&history).0;
            self.budgets_state.history_cursor.0 = match movement {
                Movement::Next => accounts::step_selection(selected, len, 1),
                Movement::Prev => accounts::step_selection(selected, len, -1),
                Movement::First => 0,
                Movement::Last => len.saturating_sub(1),
                Movement::HalfPageDown => {
                    accounts::step_selection(selected, len, ACCOUNTS_HALF_PAGE)
                }
                Movement::HalfPageUp => {
                    accounts::step_selection(selected, len, -ACCOUNTS_HALF_PAGE)
                }
                Movement::Enter => {
                    self.open_budgets_history_detail();
                    selected
                }
            };
            return;
        }
        let len = self
            .budgets_figures()
            .map_or(0, |(_, figures)| figures.rows.len());
        let selected = self.budgets_state.selected.min(len.saturating_sub(1));
        self.budgets_state.selected = match movement {
            Movement::Next => accounts::step_selection(selected, len, 1),
            Movement::Prev => accounts::step_selection(selected, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => accounts::step_selection(selected, len, ACCOUNTS_HALF_PAGE),
            Movement::HalfPageUp => accounts::step_selection(selected, len, -ACCOUNTS_HALF_PAGE),
            Movement::Enter => {
                self.open_budgets_detail_at(selected);
                selected
            }
        };
    }

    /// The History tab's chart and table for the range in view.
    pub(super) fn budgets_history_data(&self) -> Option<budgets::History> {
        let budget = self.budgets.get(self.budgets_state.current)?;
        let (first, last) =
            budgets::history_range(budget, self.budgets_state.history_end, self.today)?;
        Some(budgets::history(
            budget,
            &self.budgets_ledger(),
            first,
            last,
            self.today,
        ))
    }

    /// The cursor held inside the table, so a shorter range can't leave it dangling.
    pub(super) fn budgets_history_cursor_in(&self, history: &budgets::History) -> (usize, usize) {
        (
            self.budgets_state
                .history_cursor
                .0
                .min(history.rows.len().saturating_sub(1)),
            self.budgets_state
                .history_cursor
                .1
                .min(history.months.len().saturating_sub(1)),
        )
    }

    /// Steps the History range a month, inside the Budget's bounds.
    pub(super) fn shift_budgets_history_range(&mut self, forward: bool) {
        let Some(budget) = self.budgets.get(self.budgets_state.current) else {
            return;
        };
        // Step from where the range really ends, not from a stale out-of-bounds end.
        let Some((_, last)) =
            budgets::history_range(budget, self.budgets_state.history_end, self.today)
        else {
            return;
        };
        let target = if forward { last.next() } else { last.prev() };
        if let Some((_, held)) = budgets::history_range(budget, target, self.today) {
            self.budgets_state.history_end = held;
        }
    }

    /// `h`/`l` on the History tab: the month cursor, stopping at either end of the range.
    pub(super) fn step_budgets_history_month(&mut self, forward: bool) {
        let Some(history) = self.budgets_history_data() else {
            return;
        };
        let (row, column) = self.budgets_history_cursor_in(&history);
        let column = if forward {
            (column + 1).min(history.months.len().saturating_sub(1))
        } else {
            column.saturating_sub(1)
        };
        self.budgets_state.history_cursor = (row, column);
    }

    /// `enter` on a History cell: 9d for that Category and month (a parent shows its rollup).
    pub(super) fn open_budgets_history_detail(&mut self) {
        let Some(history) = self.budgets_history_data() else {
            return;
        };
        let (row, column) = self.budgets_history_cursor_in(&history);
        let (Some(row), Some(month)) = (history.rows.get(row), history.months.get(column)) else {
            return;
        };
        self.open_budgets_detail(row.category_id, month.month);
    }

    /// **Export CSV**: asks where to save through the platform's save dialog, writes the visible
    /// History table there and raises a Toast naming the path. A cancelled dialog does nothing.
    pub(super) fn export_budgets_history(&mut self, cx: &mut Context<'_, Self>) {
        let Some(history) = self.budgets_history_data() else {
            return;
        };
        let labels = budgets::HistoryCsvLabels {
            category: lib_locale::msg::column_category(),
            budget: crate::msg::desktop_budgets_column_budget(),
            spent: crate::msg::desktop_budgets_column_spent(),
            average: crate::msg::desktop_budgets_history_column_avg(),
            over: crate::msg::desktop_budgets_history_column_over(),
        };
        let csv = budgets::history_csv(
            &history,
            &self.categories,
            &labels,
            budgets_view::period_label,
        );
        let directory = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_default();
        let chosen = cx.prompt_for_new_path(&directory, Some(BUDGET_HISTORY_FILE));
        cx.spawn(async move |this, cx| {
            let outcome = match chosen.await {
                Ok(Ok(Some(path))) => std::fs::write(&path, csv)
                    .map(|()| path.display().to_string())
                    .map_err(|error| error.to_string()),
                Ok(Err(error)) => Err(error.to_string()),
                // Cancelled, or the dialog went away.
                Ok(Ok(None)) | Err(_) => return,
            };
            this.update(cx, |shell, cx| {
                match outcome {
                    Ok(path) => shell.raise_toast(
                        ToastKind::Success,
                        crate::msg::desktop_budgets_history_exported(&path),
                    ),
                    Err(error) => shell.raise_toast(
                        ToastKind::Error,
                        crate::msg::desktop_budgets_history_export_failed(&error),
                    ),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn handle_budgets_export_click(&mut self, cx: &mut Context<'_, Self>) {
        self.export_budgets_history(cx);
        cx.notify();
    }

    /// A click on a History cell moves the cursor there; a second click opens its detail.
    pub(super) fn handle_budgets_history_cell_click(
        &mut self,
        row: usize,
        column: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let was_here = self.budgets_state.history_cursor == (row, column);
        self.budgets_state.history_cursor = (row, column);
        if was_here {
            self.open_budgets_history_detail();
        }
        cx.notify();
    }

    /// Opens 9d on the Progress row at `index`, for the month being shown.
    pub(super) fn open_budgets_detail_at(&mut self, index: usize) {
        let Some(category_id) = self
            .budgets_figures()
            .and_then(|(_, figures)| figures.rows.get(index).map(|row| row.category_id))
        else {
            return;
        };
        self.open_budgets_detail(category_id, self.budgets_state.period);
    }

    /// Opens 9d, counting the Transactions it lists now so its `j`/`k` never need the ledger.
    pub(super) fn open_budgets_detail(&mut self, category_id: u32, month: Period) {
        let listed = self
            .budgets_detail_for(category_id, month)
            .map_or(0, |detail| {
                detail.lines.len().min(budgets_view::detail_dialog::LISTED)
            });
        self.open_budgets_dialog(budgets::BudgetsDialog::CategoryDetail(
            budgets::Detail::new(category_id, month, listed),
        ));
    }

    /// The open Category detail's figures, or `None` once its Budget or Category is gone.
    pub(super) fn budgets_detail(&self) -> Option<budgets::CategoryDetail> {
        let Some(budgets::BudgetsDialog::CategoryDetail(detail)) = self.budgets_dialog() else {
            return None;
        };
        self.budgets_detail_for(detail.category_id, detail.month)
    }

    pub(super) fn budgets_detail_for(
        &self,
        category_id: u32,
        month: Period,
    ) -> Option<budgets::CategoryDetail> {
        let budget = self.budgets.get(self.budgets_state.current)?;
        let ledger = budgets::Ledger {
            categories: &self.categories,
            accounts: &self.accounts,
            transactions: &self.transactions,
            plans: &self.bill_plans,
            entries: &self.bill_entries,
        };
        budgets::category_detail(budget, &ledger, category_id, month, self.today)
    }

    /// 9d's `open in Transactions →`: Transactions filtered to the Category, the month and the
    /// Budget's on-budget Accounts, following [`Self::open_payee_transactions`].
    pub(super) fn open_budgets_detail_transactions(&mut self) {
        let Some(budgets::BudgetsDialog::CategoryDetail(detail)) = self.budgets_dialog() else {
            return;
        };
        let (category_id, month) = (detail.category_id, detail.month);
        self.close_dialog();
        let account_ids = self
            .budgets
            .get(self.budgets_state.current)
            .map(|budget| budget.account_ids.clone())
            .unwrap_or_default();
        self.transactions_filters =
            TransactionFilters::for_budget_category(category_id, month, &account_ids);
        self.transactions_search.clear();
        self.transactions_filter_form = None;
        self.reset_transactions_selection();
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll();
    }

    /// The Starting / From options for the Budget on show: `count` months from the current one.
    pub(super) fn budgets_limit_options(
        &self,
        count: usize,
    ) -> Option<budgets::limit_form::LimitOptions> {
        let budget = self.budgets.get(self.budgets_state.current)?;
        Some(budgets::limit_form::LimitOptions::new(
            budget,
            &self.categories,
            Period::of(self.today),
            count,
            |month, current| {
                let label = budgets_view::period_label(month);
                if current {
                    crate::msg::desktop_budgets_limit_month_current(&label)
                } else {
                    label
                }
            },
        ))
    }

    /// The Budget on show when it takes edits: an archived one is read-only.
    pub(super) fn budgets_editable(&self) -> Option<&budgets::Budget> {
        self.budgets
            .get(self.budgets_state.current)
            .filter(|budget| !budget.is_archived())
    }

    /// Opens 9e on a leaf Category, starting in `month` when that is offered. A Category with no
    /// Budget Amount this month opens the picker with it preselected (9a's `set`); a parent only
    /// rolls up, so it opens nothing.
    pub(super) fn open_budgets_limit(&mut self, category_id: u32, month: Period) {
        let (Some(budget), Some(options)) = (
            self.budgets_editable(),
            self.budgets_limit_options(budgets::limit_form::START_MONTHS),
        ) else {
            return;
        };
        let current = Period::of(self.today);
        let form = if budgets::applied(budget.chain(category_id), current).is_some() {
            if !categories::is_leaf(&self.categories, category_id) {
                return;
            }
            budgets::limit_form::LimitForm::edit(budget, category_id, month, &options)
        } else if budgets::unbudgeted_leaves(budget, &self.categories, current)
            .contains(&category_id)
        {
            budgets::limit_form::LimitForm::pick(Some(category_id), &options)
        } else {
            return;
        };
        self.open_budgets_dialog(budgets::BudgetsDialog::EditLimit(form));
    }

    /// **+ Budget a category**: 9e with its Category picker.
    pub(super) fn open_budgets_limit_picker(&mut self) {
        if self.budgets_editable().is_none() {
            return;
        }
        let Some(options) = self.budgets_limit_options(budgets::limit_form::START_MONTHS) else {
            return;
        };
        self.open_budgets_dialog(budgets::BudgetsDialog::EditLimit(
            budgets::limit_form::LimitForm::pick(None, &options),
        ));
    }

    /// Opens 9g on a leaf Category that has a Budget Amount this month or in `month`.
    pub(super) fn open_budgets_stop(&mut self, category_id: u32, month: Period) {
        let (Some(budget), Some(options)) = (
            self.budgets_editable(),
            self.budgets_limit_options(budgets::limit_form::STOP_MONTHS),
        ) else {
            return;
        };
        let chain = budget.chain(category_id);
        let current = Period::of(self.today);
        if budgets::applied(chain, current).is_none() && budgets::applied(chain, month).is_none() {
            return;
        }
        self.open_budgets_dialog(budgets::BudgetsDialog::Stop(
            budgets::limit_form::StopForm::new(category_id, month, &options),
        ));
    }

    /// The Category under the cursor on Progress or Plan, with the month its dialog starts in.
    pub(super) fn budgets_cursor_category(&self) -> Option<(u32, Period)> {
        match self.budgets_state.tab {
            budgets::BudgetsTab::Progress => {
                let (_, figures) = self.budgets_figures()?;
                let row = figures.rows.get(self.budgets_state.selected)?;
                (!row.is_parent).then_some((row.category_id, self.budgets_state.period))
            }
            budgets::BudgetsTab::Plan => {
                let plan = self.budgets_plan_data()?;
                let (row_index, column) = self.budgets_plan_cursor_in(&plan);
                let month = plan
                    .months
                    .get(column)
                    .copied()
                    .unwrap_or_else(|| Period::of(self.today));
                Some((plan.row(row_index)?.category_id, month))
            }
            budgets::BudgetsTab::History => None,
        }
    }

    /// **Save budget** and `enter`: writes the record. A refused save reopens the dialog with the
    /// error shown.
    pub(super) fn apply_budgets_limit(&mut self, mut form: budgets::limit_form::LimitForm) {
        let Some(draft) = form.draft() else {
            return;
        };
        let saved = budgets::limit_form::save(
            &mut self.budgets,
            self.budgets_state.current,
            &self.categories,
            &draft,
            self.today,
        );
        if let Err(error) = saved {
            form.error = Some(error);
            self.open_budgets_dialog(budgets::BudgetsDialog::EditLimit(form));
        }
    }

    /// **Stop budgeting** and `enter`: writes the Stop from the chosen month. A refused one
    /// reopens the dialog with the error shown.
    pub(super) fn apply_budgets_stop(&mut self, mut form: budgets::limit_form::StopForm) {
        let Some(month) = form.month() else {
            return;
        };
        let stopped = self.budgets.stop(
            self.budgets_state.current,
            &self.categories,
            form.category_id,
            month,
            self.today,
        );
        if let Err(error) = stopped {
            form.error = Some(error);
            self.open_budgets_dialog(budgets::BudgetsDialog::Stop(form));
        }
    }

    /// 9e's footer link: swaps Edit budget for Stop budgeting on the same Category and month.
    pub(super) fn open_budgets_stop_from_limit(&mut self) {
        let Some(budgets::BudgetsDialog::EditLimit(form)) = self.budgets_dialog() else {
            return;
        };
        let (Some(category_id), Some(month)) = (form.fixed_category, form.month()) else {
            return;
        };
        self.open_budgets_stop(category_id, month);
    }

    /// The Budgets page's status-line legend: the open dialog's keys, else the tab's.
    pub(super) fn budgets_hints(&self) -> Vec<(&'static str, String)> {
        use budgets::{BudgetsDialog, BudgetsTab};
        match (self.budgets_dialog(), self.budgets_state.tab) {
            (Some(BudgetsDialog::Switcher(_)), _) => budgets_switcher_hints(),
            (Some(BudgetsDialog::EditLimit(_) | BudgetsDialog::Budget(_)), _) => {
                budgets_limit_hints()
            }
            (Some(BudgetsDialog::Stop(_)), _) => budgets_stop_hints(),
            (Some(BudgetsDialog::Manage(_)), _) => budgets_manage_hints(),
            (Some(BudgetsDialog::Fill { .. }), _) => budgets_fill_hints(),
            (Some(_), _) => budgets_detail_hints(),
            (None, BudgetsTab::Progress) => budgets_progress_hints(),
            (None, BudgetsTab::Plan) if self.budgets_state.plan_edit.is_some() => {
                budgets_plan_insert_hints()
            }
            (None, BudgetsTab::Plan) => budgets_plan_hints(),
            (None, BudgetsTab::History) => budgets_history_hints(),
        }
    }

    /// `B` or a click on the title: opens 11b with the Budget on show highlighted.
    pub(super) fn open_budgets_switcher(&mut self) {
        self.open_budgets_dialog(budgets::BudgetsDialog::Switcher(budgets::Switcher::new(
            &self.budgets,
            self.budgets_state.current,
        )));
    }

    /// Shows Budget `id` on the surface, keeping the active tab (every method built has all
    /// three) and putting each tab's cursor and range back at its start.
    pub(super) fn switch_budget(&mut self, id: u32) {
        if self.budgets.get(id).is_none() {
            return;
        }
        self.budgets_state.current = id;
        self.budgets_state.selected = 0;
        self.budgets_state.plan_cursor = (0, 0);
        self.budgets_state.plan_edit = None;
        self.budgets_state.plan_start = budgets::default_plan_start(self.today);
        self.budgets_state.history_cursor = (0, 0);
        self.budgets_state.history_end = Period::of(self.today);
        self.reset_view_scroll();
    }

    /// `enter` or a click on a Switcher row: switches to it and closes the popover.
    pub(super) fn choose_budgets_switcher(&mut self, id: u32) {
        self.close_dialog();
        self.switch_budget(id);
    }

    /// `n` on the Budgets page and the Switcher's `+ New budget`: 11c, with the Budget on show as
    /// what "Copy categories from" names.
    pub(super) fn open_budgets_new(&mut self) {
        let form = budgets::form::BudgetForm::new(
            &self.accounts,
            self.budgets.get(self.budgets_state.current),
        );
        self.open_budgets_dialog(budgets::BudgetsDialog::Budget(form));
    }

    /// 11c's edit mode on Budget `id`: rename it or change its Accounts. An archived Budget is
    /// read-only, so it opens nothing.
    pub(super) fn open_budgets_edit(&mut self, id: u32) {
        let Some(budget) = self.budgets.get(id).filter(|budget| !budget.is_archived()) else {
            return;
        };
        self.open_budgets_dialog(budgets::BudgetsDialog::Budget(
            budgets::form::BudgetForm::from_budget(budget, &self.accounts),
        ));
    }

    /// **Create budget** / **Save** and `enter`: creates the Budget and switches to it, or saves
    /// the rename and Accounts. A refused save reopens the dialog with the error shown.
    pub(super) fn apply_budgets_form(&mut self, mut form: budgets::form::BudgetForm) {
        let Some(draft) = form.draft() else {
            return;
        };
        let saved = match draft {
            budgets::form::BudgetDraft::Create(new) => {
                let ledger = budgets::Ledger {
                    categories: &self.categories,
                    accounts: &self.accounts,
                    transactions: &self.transactions,
                    plans: &self.bill_plans,
                    entries: &self.bill_entries,
                };
                self.budgets.create(&new, &ledger, self.today).map(Some)
            }
            budgets::form::BudgetDraft::Edit {
                id,
                name,
                account_ids,
            } => self
                .budgets
                .edit(id, &name, account_ids, &self.accounts)
                .map(|()| None),
        };
        match saved {
            Ok(Some(id)) => self.switch_budget(id),
            Ok(None) => {}
            Err(error) => {
                form.error = Some(error);
                self.open_budgets_dialog(budgets::BudgetsDialog::Budget(form));
            }
        }
    }

    pub(super) fn budget_form_mut(&mut self) -> Option<&mut budgets::form::BudgetForm> {
        match self.budgets_dialog_mut() {
            Some(budgets::BudgetsDialog::Budget(form)) => Some(form),
            _ => None,
        }
    }

    pub(super) fn handle_budgets_form_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog();
        cx.notify();
    }

    /// New budget (11c), or its edit mode.
    pub(super) fn render_budgets_form_dialog(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let Some(budgets::BudgetsDialog::Budget(form)) = self.budgets_dialog() else {
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
            on_cancel: plain(Shell::handle_budgets_dialog_cancel),
            on_confirm: plain(Shell::handle_budgets_form_confirm),
        };
        Some(budgets_view::budget_dialog::render(
            budgets_view::budget_dialog::BudgetDialogProps {
                form,
                options: &options,
                budget_name: named
                    .and_then(|id| self.budgets.get(id))
                    .map(|budget| budget.name.clone()),
                handlers,
            },
            cx,
        ))
    }

    /// The Switcher's `Manage budgets…`: 11f, with the Budget on show under the cursor.
    pub(super) fn open_budgets_manage(&mut self) {
        self.open_budgets_dialog(budgets::BudgetsDialog::Manage(budgets::Manage::new(
            &self.budgets,
            self.budgets_state.current,
        )));
    }

    /// Runs one of 11f's actions on Budget `id`. Open and Edit leave Manage budgets; Duplicate
    /// opens the copy in edit mode; the rest stay, showing a refusal when there is one. Run from
    /// the palette with Manage closed, a refusal goes to the status line instead.
    pub(super) fn run_budgets_manage_action(&mut self, id: u32, action: ManageAction) {
        let outcome = match action {
            ManageAction::Open => {
                self.choose_budgets_switcher(id);
                return;
            }
            ManageAction::Edit => {
                self.open_budgets_edit(id);
                return;
            }
            ManageAction::Duplicate => self.budgets.duplicate(id).map(|copy| {
                self.switch_budget(copy);
                self.open_budgets_edit(copy);
            }),
            ManageAction::SetDefault => self.budgets.set_default(id),
            ManageAction::Archive => self.budgets.archive(id, self.today),
            ManageAction::Restore => self.budgets.restore(id),
        };
        let refused = outcome.err();
        match self.dialog.as_mut() {
            Some(OpenDialog::Budgets(dialog))
                if matches!(**dialog, budgets::BudgetsDialog::Manage(_)) =>
            {
                if let budgets::BudgetsDialog::Manage(manage) = &mut **dialog {
                    // Archiving or restoring moves the row between the two sections: follow it.
                    manage.refresh(&self.budgets, id);
                    manage.error = refused;
                }
            }
            _ => {
                if let Some(error) = refused {
                    self.status_message = Some(match error {
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
        let Some(budgets::BudgetsDialog::Manage(manage)) = self.budgets_dialog() else {
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
        let rows = budgets::switcher_ids(&self.budgets, "")
            .into_iter()
            .filter_map(|id| self.budgets.get(id))
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
                    &self.budgets.active().count().to_string(),
                    &self.budgets.archived().count().to_string(),
                    &self
                        .budgets
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
                            if let Some(budgets::BudgetsDialog::Manage(manage)) =
                                shell.budgets_dialog_mut()
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
                            shell.run_budgets_manage_action(id, action);
                            cx.notify();
                        });
                    })
                },
                on_new: plain(Shell::handle_budgets_new_click),
                on_close: plain(Shell::handle_budgets_dialog_cancel),
            },
            cx,
        ))
    }

    pub(super) fn handle_budgets_title_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_switcher();
        cx.notify();
    }

    pub(super) fn handle_budgets_switcher_search_click(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(budgets::BudgetsDialog::Switcher(switcher)) = self.budgets_dialog_mut() {
            switcher.searching = true;
        }
        cx.notify();
    }

    pub(super) fn handle_budgets_new_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_new();
        cx.notify();
    }

    pub(super) fn handle_budgets_manage_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_manage();
        cx.notify();
    }

    /// A Budget's one-line summary in the Switcher: its health this month, or when it was
    /// archived.
    pub(super) fn budgets_summary(&self, budget: &budgets::Budget) -> String {
        if let Some(date) = budget.archived_at {
            return crate::msg::desktop_budgets_switcher_archived_on(&format::date(
                date,
                self.settings_date_style,
            ));
        }
        let health = budgets::health(budget, &self.budgets_ledger(), self.today);
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
        let Some(budgets::BudgetsDialog::Switcher(switcher)) = self.budgets_dialog() else {
            return None;
        };
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let rows = budgets::switcher_ids(&self.budgets, switcher.query.text())
            .into_iter()
            .filter_map(|id| self.budgets.get(id))
            .map(|budget| budgets_view::switcher::SwitcherRow {
                id: budget.id,
                name: budget.name.clone(),
                is_default: budget.is_default,
                is_current: budget.id == self.budgets_state.current,
                archived: budget.is_archived(),
                summary: self.budgets_summary(budget),
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
                            shell.choose_budgets_switcher(id);
                            cx.notify();
                        });
                    })
                },
                on_new: plain(Shell::handle_budgets_new_click),
                on_manage: plain(Shell::handle_budgets_manage_click),
                on_cancel: plain(Shell::handle_budgets_dialog_cancel),
            },
            cx,
        ))
    }

    /// The month Fill would target from the Plan cursor: the first open month at or after its
    /// column.
    pub(super) fn budgets_fill_target(&self) -> Period {
        let cursor = self.budgets_plan_data().and_then(|plan| {
            let column = self.budgets_plan_cursor_in(&plan).1;
            plan.months.get(column).copied()
        });
        budgets::fill_target(cursor, self.today)
    }

    /// **Fill … from…** and `f` on the Plan tab: opens 9f on the cursor's target month.
    pub(super) fn open_budgets_fill(&mut self) {
        if self.budgets_editable().is_none() {
            return;
        }
        self.open_budgets_dialog(budgets::BudgetsDialog::Fill {
            month: self.budgets_fill_target(),
            source: budgets::FillSource::default(),
        });
    }

    /// **Fill** and `enter`: writes the chosen source's Month-only amounts. A Fill that would
    /// change nothing stays open, as its disabled button says.
    pub(super) fn apply_budgets_fill(&mut self, month: Period, source: budgets::FillSource) {
        let ledger = budgets::Ledger {
            categories: &self.categories,
            accounts: &self.accounts,
            transactions: &self.transactions,
            plans: &self.bill_plans,
            entries: &self.bill_entries,
        };
        let wrote = self.budgets.fill(
            self.budgets_state.current,
            &ledger,
            month,
            source,
            self.today,
        );
        if !matches!(wrote, Ok(count) if count > 0) {
            self.open_budgets_dialog(budgets::BudgetsDialog::Fill { month, source });
        }
    }

    pub(super) fn set_budgets_fill_source(&mut self, chosen: budgets::FillSource) {
        if let Some(budgets::BudgetsDialog::Fill { source, .. }) = self.budgets_dialog_mut() {
            *source = chosen;
        }
    }

    pub(super) fn handle_budgets_fill_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_fill();
        cx.notify();
    }

    pub(super) fn handle_budgets_fill_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog();
        cx.notify();
    }

    /// Fill (9f), with each source's total and the chosen one's diff against the plan.
    pub(super) fn render_budgets_fill_dialog(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let Some(&budgets::BudgetsDialog::Fill { month, source }) = self.budgets_dialog() else {
            return None;
        };
        let budget = self.budgets.get(self.budgets_state.current)?;
        let ledger = self.budgets_ledger();
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
            self.categories
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
                on_cancel: plain(Shell::handle_budgets_dialog_cancel),
                on_confirm: plain(Shell::handle_budgets_fill_confirm),
            },
            cx,
        ))
    }

    /// 9d's **Edit budget**: 9e on the detail's Category and month. A parent's rollup has none.
    pub(super) fn open_budgets_detail_limit(&mut self) {
        if let Some(budgets::BudgetsDialog::CategoryDetail(detail)) = self.budgets_dialog() {
            let (category_id, month) = (detail.category_id, detail.month);
            self.open_budgets_limit(category_id, month);
        }
    }

    /// Whether 9d offers Edit budget: a leaf Expense Category in a Budget that takes edits.
    pub(super) fn budgets_detail_editable(&self, category_id: u32) -> bool {
        self.budgets_editable().is_some()
            && categories::is_leaf(&self.categories, category_id)
            && self.categories.iter().any(|category| {
                category.id == category_id && category.category_type == CategoryTypes::Expense
            })
    }

    pub(super) fn handle_budgets_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    pub(super) fn handle_budgets_limit_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog();
        cx.notify();
    }

    pub(super) fn handle_budgets_limit_stop(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_stop_from_limit();
        cx.notify();
    }

    pub(super) fn handle_budgets_stop_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog();
        cx.notify();
    }

    pub(super) fn limit_form_mut(&mut self) -> Option<&mut budgets::limit_form::LimitForm> {
        match self.budgets_dialog_mut() {
            Some(budgets::BudgetsDialog::EditLimit(form)) => Some(form),
            _ => None,
        }
    }

    pub(super) fn handle_budgets_stop_field_click(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(budgets::BudgetsDialog::Stop(form)) = self.budgets_dialog_mut() {
            form.click_select();
        }
        cx.notify();
    }

    pub(super) fn handle_budgets_detail_edit(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_detail_limit();
        cx.notify();
    }

    pub(super) fn handle_budgets_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_limit_picker();
        cx.notify();
    }

    /// A Progress row's `edit` or `set`.
    pub(super) fn handle_budgets_action_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.budgets_state.selected = index;
        if let Some((category_id, month)) = self.budgets_cursor_category() {
            self.open_budgets_limit(category_id, month);
        }
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
            self.categories
                .iter()
                .find(|category| category.id == category_id)
                .map(|category| category.name.clone())
        };
        match self.budgets_dialog()? {
            budgets::BudgetsDialog::EditLimit(form) => {
                let draft = form.draft();
                let preview = draft.as_ref().and_then(|draft| {
                    budgets::limit_form::preview(
                        &self.budgets,
                        self.budgets_state.current,
                        &self.categories,
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
                    on_cancel: plain(Shell::handle_budgets_dialog_cancel),
                    on_confirm: plain(Shell::handle_budgets_limit_confirm),
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
                        mirrors_category_field: self.budgets_state.current
                            == budgets::PERSONAL_SPENDING_ID,
                        valid: draft.is_some(),
                        handlers,
                    },
                    cx,
                ))
            }
            budgets::BudgetsDialog::Stop(form) => {
                let category = leaf_name(form.category_id)?;
                let on_option_click: accounts_view::select_field::OnOptionClick = {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            if let Some(budgets::BudgetsDialog::Stop(form)) =
                                shell.budgets_dialog_mut()
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
                            .bill_plans
                            .iter()
                            .filter(|plan| plan.category_id == form.category_id && plan.is_active)
                            .map(|plan| plan.name.clone())
                            .collect(),
                        on_field_click: plain(Shell::handle_budgets_stop_field_click),
                        on_option_click,
                        on_cancel: plain(Shell::handle_budgets_dialog_cancel),
                        on_confirm: plain(Shell::handle_budgets_stop_confirm),
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
        self.open_budgets_detail_transactions();
        cx.notify();
    }

    /// The Category detail (9d) over the Progress tab, or nothing once its figures are gone.
    pub(super) fn render_budgets_detail(
        &self,
        entity: &gpui::Entity<Self>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let detail = self.budgets_detail()?;
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let category = categories::path(&self.categories, detail.category_id).unwrap_or_default();
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
                    .and_then(|id| payees::get(&self.payees, id))
                    .map_or_else(crate::msg::desktop_budgets_detail_no_payee, |payee| {
                        payee.name.clone()
                    }),
                account: self
                    .accounts
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
                selected: match self.budgets_dialog() {
                    Some(budgets::BudgetsDialog::CategoryDetail(open)) => open.selected,
                    _ => 0,
                },
                track,
                rollover_note,
                bills,
                on_close: plain(Shell::handle_budgets_detail_close),
                on_open_transactions: plain(Shell::handle_budgets_detail_transactions),
                on_edit: self
                    .budgets_detail_editable(detail.category_id)
                    .then(|| plain(Shell::handle_budgets_detail_edit)),
            },
            cx,
        ))
    }

    /// `tab` on the Budgets page switches its tab rather than cycling focus, as on Bills.
    pub(super) fn handle_budgets_tab_key(&mut self, keystroke: &Keystroke) -> bool {
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        let modifiers = &keystroke.modifiers;
        if keystroke.key != "tab"
            || modifiers.shift
            || modifiers.control
            || modifiers.alt
            || modifiers.platform
            || pending_g_active
            || self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Budgets
            || self.nav.focus() != FocusZone::View
        {
            return false;
        }
        self.set_budgets_tab(self.budgets_state.tab.next());
        true
    }

    pub(super) fn set_budgets_tab(&mut self, tab: budgets::BudgetsTab) {
        self.budgets_state.tab = tab;
        self.budgets_state.selected = 0;
        self.status_message = None;
        self.reset_view_scroll();
    }

    /// Steps the period a calendar month. The Plan and History range navs are their own tickets'.
    pub(super) fn shift_budgets_period(&mut self, forward: bool) {
        self.budgets_state.period = if forward {
            self.budgets_state.period.next()
        } else {
            self.budgets_state.period.prev()
        };
        self.budgets_state.selected = 0;
    }

    /// `B` opens the Switcher and `n` New budget; `[`/`]` step the period and `1`/`2`/`3` pick the
    /// tab; `c` budgets a Category, and on Progress `e` edits the row's budget and `s` stops it.
    pub(super) fn handle_budgets_key(&mut self, keystroke: &Keystroke) -> bool {
        if self.nav.noun() != Noun::Budgets || self.nav.focus() != FocusZone::View {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }
        // Bare `b` stays the rail toggle, so the Switcher takes the shifted key (#400).
        if modifiers.shift {
            if keystroke.key.eq_ignore_ascii_case("b") {
                self.open_budgets_switcher();
                return true;
            }
            return false;
        }
        if let Some(tab) = keystroke
            .key
            .chars()
            .next()
            .filter(|_| keystroke.key.chars().count() == 1)
            .and_then(budgets::BudgetsTab::from_digit)
        {
            self.set_budgets_tab(tab);
            return true;
        }
        match keystroke.key.as_str() {
            "c" if self.budgets_state.tab != budgets::BudgetsTab::History => {
                self.open_budgets_limit_picker();
                return true;
            }
            "n" => {
                self.open_budgets_new();
                return true;
            }
            "e" | "s" if self.budgets_state.tab == budgets::BudgetsTab::Progress => {
                if let Some((category_id, month)) = self.budgets_cursor_category() {
                    if keystroke.key == "e" {
                        self.open_budgets_limit(category_id, month);
                    } else {
                        self.open_budgets_stop(category_id, month);
                    }
                }
                return true;
            }
            _ => {}
        }
        if self.budgets_state.tab == budgets::BudgetsTab::Plan {
            return self.handle_budgets_plan_key(keystroke.key.as_str());
        }
        let history = self.budgets_state.tab == budgets::BudgetsTab::History;
        match keystroke.key.as_str() {
            "[" if history => self.shift_budgets_history_range(false),
            "]" if history => self.shift_budgets_history_range(true),
            "h" | "left" if history => self.step_budgets_history_month(false),
            "l" | "right" if history => self.step_budgets_history_month(true),
            "x" if history => self.budgets_state.export_pending = true,
            "[" => self.shift_budgets_period(false),
            "]" => self.shift_budgets_period(true),
            _ => return false,
        }
        true
    }

    /// The KNOWN COSTS stat's link: the Bills Schedule on the period being shown.
    pub(super) fn open_budgets_schedule(&mut self) {
        self.bills_period = self.budgets_state.period;
        self.bills_all = false;
        self.bills_filters = bills::history::BillFilters::default();
        self.set_bills_tab(bills::BillsTab::Schedule);
        self.nav.set_noun(Noun::Bills);
        self.reset_view_scroll();
    }

    pub(super) fn handle_budgets_tab_click(
        &mut self,
        tab: budgets::BudgetsTab,
        cx: &mut Context<'_, Self>,
    ) {
        self.set_budgets_tab(tab);
        cx.notify();
    }

    pub(super) fn handle_budgets_period_prev(&mut self, cx: &mut Context<'_, Self>) {
        self.shift_budgets_period(false);
        cx.notify();
    }

    pub(super) fn handle_budgets_period_next(&mut self, cx: &mut Context<'_, Self>) {
        self.shift_budgets_period(true);
        cx.notify();
    }

    pub(super) fn handle_budgets_edit_plan_click(&mut self, cx: &mut Context<'_, Self>) {
        self.set_budgets_tab(budgets::BudgetsTab::Plan);
        cx.notify();
    }

    pub(super) fn handle_budgets_range_prev(&mut self, cx: &mut Context<'_, Self>) {
        if self.budgets_state.tab == budgets::BudgetsTab::History {
            self.shift_budgets_history_range(false);
        } else {
            self.shift_budgets_plan_range(false);
        }
        cx.notify();
    }

    pub(super) fn handle_budgets_range_next(&mut self, cx: &mut Context<'_, Self>) {
        if self.budgets_state.tab == budgets::BudgetsTab::History {
            self.shift_budgets_history_range(true);
        } else {
            self.shift_budgets_plan_range(true);
        }
        cx.notify();
    }

    /// A click puts the cursor on the cell; on an open month it also starts typing, and on the
    /// ROLLOVER cell it cycles the mode. A click elsewhere abandons an edit in progress.
    pub(super) fn handle_budgets_plan_cell_click(
        &mut self,
        row: usize,
        column: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if self.budgets_state.plan_edit.take().is_some() {
            self.nav.exit_mode();
        }
        self.budgets_state.plan_cursor = (row, column);
        self.start_budgets_plan_edit();
        cx.notify();
    }

    pub(super) fn handle_budgets_known_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_budgets_schedule();
        cx.notify();
    }

    /// A click selects a row; clicking the selected row opens its detail.
    pub(super) fn handle_budgets_row_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let was_selected = self.budgets_state.selected == index;
        self.budgets_state.selected = index;
        if was_selected {
            self.open_budgets_detail_at(index);
        }
        cx.notify();
    }
}
