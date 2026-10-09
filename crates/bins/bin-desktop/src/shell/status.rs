//! The status line's hint clicks and page status of `Shell`. An `impl Shell` block for the
//! concern, not the status line itself (`chrome::statusline`).

use gpui::Context;

use super::{Shell, document_types_ui, inventory_ui};
use crate::view::{
    accounts::hints::accounts_hints,
    bills::hints::{
        bills_planner_hints, bills_schedule_hints, pay_bill_dialog_hints, skip_bill_dialog_hints,
    },
    import::import_hints,
    settings::hints::{
        confirm_dialog_hints, merge_tags_dialog_hints, payee_dialog_hints,
        settings_categories_hints, settings_documents_hints, settings_inventory_hints,
        settings_payees_hints, settings_tags_hints, tag_dialog_hints,
    },
    transactions::hints::{filter_hints, transactions_hints},
};
use crate::{
    bills, budgets,
    chrome::statusline::{HintAction, PageStatus},
    import,
    navigation::active_view::ActiveView,
    navigation::nav::{FocusZone, InputMode},
    payees, tags,
    view::format,
    view::{
        bills as bills_view, budgets as budgets_view,
        settings::{self as settings_view, inventory as inventory_view},
    },
};

impl Shell {
    /// A click on the status line's hint strip: the same action as the matching `Normal`-mode key,
    /// and ignored in any other mode (where those keys aren't live either).
    pub(super) fn handle_hint_click(&mut self, action: HintAction, cx: &mut Context<'_, Self>) {
        if self.nav.mode() != InputMode::Normal {
            return;
        }
        self.chrome.status_message = None;
        self.pending_g = None;
        match action {
            HintAction::Command => self.open_palette(),
            HintAction::Search => self.nav.enter_mode(InputMode::Search),
            HintAction::Help => self.nav.enter_mode(InputMode::Help),
            HintAction::ToggleRail => {
                self.nav.toggle_primary_rail();
                self.chrome.collapsed_rail_tooltip = None;
            }
        }
        cx.notify();
    }

    /// The status line's per-page hints and right-hand text for the View on show, or `None` when
    /// it has none. The Budgets arguments are the figures `Shell::render` already built for the
    /// page, so the status line reads the same numbers.
    pub(super) fn page_status(
        &self,
        budgets_figures: Option<&(&budgets::Budget, budgets::PeriodFigures)>,
        budgets_plan: Option<&budgets::Plan>,
        budgets_history: Option<&budgets::History>,
    ) -> Option<PageStatus> {
        match self.active_view() {
            ActiveView::Documents => self.documents_page_status(),
            ActiveView::Settings(_) if self.accounts_page_has_focus() => Some(PageStatus {
                hints: accounts_hints(),
                right: crate::msg::desktop_accounts_count(
                    i64::try_from(self.accounts.len()).unwrap_or(i64::MAX),
                ),
            }),
            ActiveView::Settings(_) if self.settings_tags_page_has_focus() => Some(PageStatus {
                hints: match self.tags_dialog() {
                    Some(tags::form::TagsDialog::Add(_)) => tag_dialog_hints(false),
                    Some(tags::form::TagsDialog::Edit(..)) => tag_dialog_hints(true),
                    Some(tags::form::TagsDialog::Remove(..)) => confirm_dialog_hints(),
                    Some(tags::form::TagsDialog::Merge(_)) => merge_tags_dialog_hints(),
                    None => settings_tags_hints(),
                },
                right: settings_view::tags::scope_text(
                    &self.tags,
                    &tags::duplicate_groups(&self.tags, &self.transactions),
                ),
            }),
            ActiveView::Settings(_) if self.settings_payees_page_has_focus() => Some(PageStatus {
                hints: match self.payees_dialog() {
                    Some(payees::form::PayeesDialog::Delete(..)) => confirm_dialog_hints(),
                    Some(_) => payee_dialog_hints(),
                    None => settings_payees_hints(),
                },
                right: settings_view::payees::scope_text(&self.payees),
            }),
            ActiveView::Settings(_) if self.settings_documents_page_has_focus() => {
                Some(PageStatus {
                    hints: self
                        .document_types_dialog()
                        .map_or_else(settings_documents_hints, document_types_ui::dialog_hints),
                    right: settings_view::documents::scope_text(&self.document_types),
                })
            }
            ActiveView::Settings(_) if self.settings_inventory_page_has_focus() => {
                Some(PageStatus {
                    hints: self.inventory_dialog().map_or_else(
                        || settings_inventory_hints(self.settings_inventory_selected_row()),
                        inventory_ui::dialog_hints,
                    ),
                    right: inventory_view::scope_text(&self.inventory),
                })
            }
            ActiveView::Settings(_) if self.settings_categories_page_has_focus() => {
                Some(PageStatus {
                    hints: settings_categories_hints(),
                    right: settings_view::categories::scope_note(&self.categories),
                })
            }
            ActiveView::Settings(_) if self.nav.focus() == FocusZone::View => Some(PageStatus {
                hints: self.settings_hints(),
                right: self.settings_selected_section.scope_note(),
            }),
            ActiveView::Bills
                if matches!(self.bills_dialog(), Some(bills::BillsDialog::Pay(_))) =>
            {
                Some(PageStatus {
                    hints: pay_bill_dialog_hints(),
                    right: crate::msg::desktop_bills_status_plans(
                        i64::try_from(self.bill_plans.len()).unwrap_or(i64::MAX),
                    ),
                })
            }
            ActiveView::Bills
                if matches!(self.bills_dialog(), Some(bills::BillsDialog::Skip(_))) =>
            {
                Some(PageStatus {
                    hints: skip_bill_dialog_hints(),
                    right: crate::msg::desktop_bills_status_plans(
                        i64::try_from(self.bill_plans.len()).unwrap_or(i64::MAX),
                    ),
                })
            }
            ActiveView::Bills
                if matches!(
                    self.bills_dialog(),
                    Some(bills::BillsDialog::Add(_) | bills::BillsDialog::Edit(..))
                ) =>
            {
                Some(PageStatus {
                    hints: payee_dialog_hints(),
                    right: crate::msg::desktop_bills_status_plans(
                        i64::try_from(self.bill_plans.len()).unwrap_or(i64::MAX),
                    ),
                })
            }
            ActiveView::Bills if self.bills_tab == bills::BillsTab::Schedule => Some(PageStatus {
                hints: bills_schedule_hints(),
                right: crate::msg::desktop_bills_status_period(
                    &if self.bills_all {
                        crate::msg::desktop_bills_period_all()
                    } else {
                        bills_view::period_label(self.bills_period)
                    },
                    &self.bills_schedule_rows().len().to_string(),
                    i64::try_from(self.bills_unfiltered_rows().len()).unwrap_or(i64::MAX),
                ),
            }),
            ActiveView::Bills if self.bills_tab == bills::BillsTab::Planner => Some(PageStatus {
                hints: bills_planner_hints(),
                right: crate::msg::desktop_bills_status_plans(
                    i64::try_from(self.bill_plans.len()).unwrap_or(i64::MAX),
                ),
            }),
            ActiveView::Budgets if self.budgets_state.tab == budgets::BudgetsTab::Progress => {
                Some(PageStatus {
                    hints: self.budgets_hints(),
                    right: crate::msg::desktop_budgets_status_period(
                        &budgets_view::period_label(self.budgets_state.period),
                        budgets_figures.map_or(0, |(_, figures)| {
                            i64::try_from(figures.rows.len()).unwrap_or(i64::MAX)
                        }),
                    ),
                })
            }
            ActiveView::Budgets if self.budgets_state.tab == budgets::BudgetsTab::Plan => {
                Some(PageStatus {
                    hints: self.budgets_hints(),
                    right: budgets_plan.map_or_else(String::new, |plan| {
                        crate::msg::desktop_budgets_status_plan(
                            &budgets_view::plan::range_label(plan),
                            i64::try_from(plan.row_count()).unwrap_or(i64::MAX),
                        )
                    }),
                })
            }
            ActiveView::Budgets => Some(PageStatus {
                hints: self.budgets_hints(),
                right: budgets_history
                    .as_ref()
                    .map_or_else(String::new, |history| {
                        crate::msg::desktop_budgets_status_history(
                            i64::try_from(history.leaves_ever_budgeted).unwrap_or(i64::MAX),
                            &history.leaves_shown.to_string(),
                        )
                    }),
            }),
            ActiveView::Import => {
                let pending = self
                    .import
                    .as_ref()
                    .map_or(0, |state| import::summary(&state.rows).needs_review);
                Some(PageStatus {
                    hints: import_hints(),
                    right: if pending == 0 {
                        crate::msg::desktop_import_status_ready()
                    } else {
                        crate::msg::desktop_import_status_pending(
                            i64::try_from(pending).unwrap_or(i64::MAX),
                        )
                    },
                })
            }
            ActiveView::Transactions => Some(PageStatus {
                hints: if self.nav.mode() == InputMode::Filter {
                    filter_hints()
                } else {
                    transactions_hints()
                },
                right: format::status_legend(self.settings_status_glyphs),
            }),
            ActiveView::Bills
            | ActiveView::Settings(_)
            | ActiveView::Dashboard
            | ActiveView::Placeholder(_) => None,
        }
    }
}
