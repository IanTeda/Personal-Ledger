//! Save cell: saves the text typed into a Plan cell.

use chrono::NaiveDate;
use lib_core::{BudgetId, CategoryId, Period};

use super::support::{logged, write_budget};
use crate::{BudgetCategories, BudgetRepository, Result, Span};

/// Saves the text typed into a Plan cell; returns whether anything changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCell {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub span: Span,
    pub text: String,
    pub today: NaiveDate,
}

impl SaveCell {
    pub fn run(
        self,
        categories: &dyn BudgetCategories,
        repo: &mut dyn BudgetRepository,
    ) -> Result<bool> {
        logged(
            "save cell",
            write_budget(repo, self.id, |budget| {
                budget.save_cell(
                    categories,
                    self.category_id,
                    self.month,
                    self.span,
                    &self.text,
                    self.today,
                )
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn saving_a_cell_writes_an_amount_clears_on_empty_text_and_refuses_bad_text() {
        let mut repo = repo();
        let save = |text: &str, span| SaveCell {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            month: current().next(),
            span,
            text: text.into(),
            today: today(),
        };

        assert_eq!(
            save("612.505", Span::MonthOnly).run(&reads(), &mut repo),
            Ok(true)
        );
        assert_eq!(
            amount_in(&repo, PERSONAL_SPENDING_ID, current().next()),
            Some(cents_money(61_251)),
            "rounded half up to cents"
        );
        assert_eq!(save("", Span::MonthOnly).run(&reads(), &mut repo), Ok(true));
        assert_eq!(
            save("", Span::MonthOnly).run(&reads(), &mut repo),
            Ok(false)
        );
        let before = repo.clone();
        assert_eq!(
            save("lots", Span::Onward).run(&reads(), &mut repo),
            Err(Error::InvalidAmount)
        );
        assert_eq!(repo, before);
    }
}
