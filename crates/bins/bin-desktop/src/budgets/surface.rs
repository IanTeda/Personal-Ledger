//! The Budgets Dialogs still on the legacy host (#955): the Category detail's state and the open
//! Dialog. The Switcher and Manage budgets are `lib-budgets` Presenters (`view::dialog`).

use crate::{
    form::field::TextField,
    period::Period,
    shell::view::dialog_host::{Dialog, DialogKey, DialogOutcome},
};
use lib_core::CategoryId;

use super::form::BudgetForm;
use super::limit_form::{LimitForm, StopForm};
use lib_budgets::FillSource;
use lib_list_cursor::Step;

/// What a key on 9d asks `Root` to do once it confirms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailRequest {
    /// `t` or `enter`: hand off to Transactions.
    Transactions,
    /// `e`: Edit budget.
    Edit,
}

/// 9d for a Category (a leaf or a parent's rollup) and month: the Transaction row cursor, over
/// `listed` rows counted as it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    pub category_id: CategoryId,
    pub month: Period,
    pub selected: usize,
    pub listed: usize,
    pub request: Option<DetailRequest>,
}

impl Detail {
    pub fn new(category_id: CategoryId, month: Period, listed: usize) -> Self {
        Self {
            category_id,
            month,
            selected: 0,
            listed,
            request: None,
        }
    }
}

impl Dialog for Detail {
    /// `j`/`k` move the cursor, `t` or `enter` hand off to Transactions and `e` opens Edit budget.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match key {
            DialogKey::Down | DialogKey::Char('j') => {
                self.selected = lib_list_cursor::step(self.selected, self.listed, Step::Next);
            }
            DialogKey::Up | DialogKey::Char('k') => {
                self.selected = lib_list_cursor::step(self.selected, self.listed, Step::Prev);
            }
            DialogKey::Enter | DialogKey::Char('t') => {
                self.request = Some(DetailRequest::Transactions);
                return Some(DialogOutcome::Confirm);
            }
            DialogKey::Char('e') => {
                self.request = Some(DetailRequest::Edit);
                return Some(DialogOutcome::Confirm);
            }
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn is_valid(&self) -> bool {
        self.request.is_some()
    }
}

/// The open Budgets dialog. Each screen's ticket adds its form to its variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetsDialog {
    /// 11c creating a Budget, or its edit mode (the form names the Budget).
    Budget(BudgetForm),
    /// 9d.
    CategoryDetail(Detail),
    /// 9e: editing a Category's amount, or picking an unbudgeted Category to budget.
    EditLimit(LimitForm),
    /// 9f, filling this month from the chosen source.
    Fill { month: Period, source: FillSource },
    /// 9g for a Category.
    Stop(StopForm),
}

impl BudgetsDialog {
    fn inner(&self) -> Option<&dyn Dialog> {
        match self {
            Self::Budget(dialog) => Some(dialog),
            Self::CategoryDetail(dialog) => Some(dialog),
            Self::EditLimit(dialog) => Some(dialog),
            Self::Stop(dialog) => Some(dialog),
            Self::Fill { .. } => None,
        }
    }

    fn inner_mut(&mut self) -> Option<&mut dyn Dialog> {
        match self {
            Self::Budget(dialog) => Some(dialog),
            Self::CategoryDetail(dialog) => Some(dialog),
            Self::EditLimit(dialog) => Some(dialog),
            Self::Stop(dialog) => Some(dialog),
            Self::Fill { .. } => None,
        }
    }
}

impl Dialog for BudgetsDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        if let Self::Fill { source, .. } = self {
            // `j`/`k` pick the source; `enter` fills.
            let all = FillSource::ALL;
            let at = all.iter().position(|each| each == source).unwrap_or(0);
            match key {
                DialogKey::Down | DialogKey::Char('j') => {
                    *source = all[(at + 1).min(all.len() - 1)]
                }
                DialogKey::Up | DialogKey::Char('k') => *source = all[at.saturating_sub(1)],
                _ => return None,
            }
            return Some(DialogOutcome::Handled);
        }
        self.inner_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.inner_mut()?.focused_text()
    }

    fn cycle_field(&mut self) {
        if let Some(dialog) = self.inner_mut() {
            dialog.cycle_field();
        }
    }

    fn is_valid(&self) -> bool {
        self.inner().is_none_or(Dialog::is_valid)
    }

    fn close_open_select(&mut self) -> bool {
        self.inner_mut().is_some_and(Dialog::close_open_select)
    }

    fn hints(&self) -> Option<Vec<lib_shell::Hint>> {
        use crate::view::budgets::hints;
        Some(crate::shell::view::statusline::owned_hints(match self {
            Self::Budget(_) | Self::EditLimit(_) => hints::budgets_limit_hints(),
            Self::Stop(_) => hints::budgets_stop_hints(),
            Self::Fill { .. } => hints::budgets_fill_hints(),
            Self::CategoryDetail(_) => hints::budgets_detail_hints(),
        }))
    }
}
