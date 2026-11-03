//! The Category Limit chain's edits: each writes one Category's chain on one Budget, checked
//! against the chain rules in the crate docs (an open month, a leaf Expense Category, no negative
//! amount, never an archived Budget).

use bigdecimal::{RoundingMode, Signed};
use chrono::NaiveDate;
use lib_core::{CategoryId, Money, Period};

use crate::plan::{FillSource, fill_preview};
use crate::protocols::{BudgetCategories, BudgetCategory, BudgetReads};
use crate::{
    Budget, Error, Limit, LimitRecord, Result, Rollover, applied, inherited_rollover, put, tree,
};

/// Whether a Budget Amount runs onward or for its month alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    Onward,
    MonthOnly,
}

/// The Category Limit chain rules, on the Budget they edit so the Budgets set and the Interactors
/// share them. Each refuses an archived Budget; a refusal can leave the chain half-edited, so a
/// caller applies a rule to a copy and keeps it only when it lands.
impl Budget {
    fn ensure_editable(&self) -> Result<()> {
        if self.is_archived() {
            Err(Error::Archived)
        } else {
            Ok(())
        }
    }

    /// Writes an Onward or Month-only Budget Amount starting in `month`, replacing whatever record
    /// starts there. `rollover` of `None` inherits the record it follows.
    #[expect(
        clippy::too_many_arguments,
        reason = "one call per Plan-grid or Edit budget save"
    )]
    pub fn set_amount(
        &mut self,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        month: Period,
        span: Span,
        amount: Money,
        rollover: Option<Rollover>,
        today: NaiveDate,
    ) -> Result<()> {
        check_leaf(&categories.budget_categories(), category_id)?;
        check_open_month(month, today)?;
        if amount.0.is_negative() {
            return Err(Error::NegativeAmount);
        }
        self.ensure_editable()?;
        let chain = self.limits.entry(category_id).or_default();
        let rollover = rollover.unwrap_or_else(|| inherited_rollover(chain, month));
        let limit = match span {
            Span::Onward => Limit::Onward { amount, rollover },
            Span::MonthOnly => Limit::MonthOnly { amount, rollover },
        };
        put(chain, LimitRecord { month, limit });
        Ok(())
    }

    /// Writes a Stop from `month`. A Category with no amount there and no record starting there
    /// has nothing to stop, so nothing is written.
    pub fn stop(
        &mut self,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        month: Period,
        today: NaiveDate,
    ) -> Result<()> {
        check_leaf(&categories.budget_categories(), category_id)?;
        check_open_month(month, today)?;
        self.ensure_editable()?;
        let chain = self.limits.entry(category_id).or_default();
        let has_record = chain.iter().any(|record| record.month == month);
        if has_record || applied(chain, month).is_some() {
            put(
                chain,
                LimitRecord {
                    month,
                    limit: Limit::Stop,
                },
            );
        }
        if chain.is_empty() {
            self.limits.remove(&category_id);
        }
        Ok(())
    }

    /// Stops a Category that gained a child: its current month's record becomes a Stop and later
    /// records go, so closed months keep their amounts and the parent rolls up its leaves from
    /// then on. An archived Budget is left alone, and a Category not budgeted this month only loses
    /// its later records. Returns whether the chain changed.
    pub fn stop_new_parent(&mut self, category_id: CategoryId, today: NaiveDate) -> bool {
        let current = Period::of(today);
        if self.is_archived() {
            return false;
        }
        let Some(chain) = self.limits.get_mut(&category_id) else {
            return false;
        };
        let before = chain.clone();
        let was_budgeted = applied(chain, current).is_some();
        // An unbudgeted chain keeps its record starting now (a Stop already there).
        chain.retain(|record| record.month < current || (!was_budgeted && record.month == current));
        if was_budgeted {
            put(
                chain,
                LimitRecord {
                    month: current,
                    limit: Limit::Stop,
                },
            );
        }
        if chain.is_empty() {
            self.limits.remove(&category_id);
            return true;
        }
        *chain != before
    }

    /// Removes the Month-only record starting in `month`, so the month falls back to the carried
    /// Onward value. Returns whether one was removed.
    pub fn clear_month_only(
        &mut self,
        category_id: CategoryId,
        month: Period,
        today: NaiveDate,
    ) -> Result<bool> {
        check_open_month(month, today)?;
        self.ensure_editable()?;
        let Some(chain) = self.limits.get_mut(&category_id) else {
            return Ok(false);
        };
        let before = chain.len();
        chain.retain(|r| !(r.month == month && matches!(r.limit, Limit::MonthOnly { .. })));
        let removed = chain.len() != before;
        if chain.is_empty() {
            self.limits.remove(&category_id);
        }
        Ok(removed)
    }

    /// The Plan grid's `r`: cycles the current month's Rollover, writing an Onward record from the
    /// current month with the same amount.
    pub fn cycle_rollover(
        &mut self,
        category_id: CategoryId,
        today: NaiveDate,
    ) -> Result<Rollover> {
        let current = Period::of(today);
        self.ensure_editable()?;
        let chain = self.limits.entry(category_id).or_default();
        let Some(found) = applied(chain, current) else {
            return Err(Error::NotBudgeted);
        };
        let next = found.rollover.next();
        put(
            chain,
            LimitRecord {
                month: current,
                limit: Limit::Onward {
                    amount: found.amount,
                    rollover: next,
                },
            },
        );
        Ok(next)
    }

    /// Saves what was typed into a Plan cell. An amount writes an Onward or Month-only record
    /// (`0` is a real 0.00). Empty text clears: Onward writes a Stop from `month`, Month-only
    /// removes that month's own Month-only record. Returns whether anything changed.
    pub fn save_cell(
        &mut self,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        month: Period,
        span: Span,
        text: &str,
        today: NaiveDate,
    ) -> Result<bool> {
        let text = text.trim();
        if text.is_empty() {
            return match span {
                Span::Onward => {
                    let before = self.clone();
                    self.stop(categories, category_id, month, today)?;
                    Ok(*self != before)
                }
                Span::MonthOnly => self.clear_month_only(category_id, month, today),
            };
        }
        let amount: Money = text.parse().map_err(|_| Error::InvalidAmount)?;
        let amount = Money(amount.0.with_scale_round(2, RoundingMode::HalfUp));
        let chain = self.chain(category_id);
        let unchanged = applied(chain, month).is_some_and(|found| found.amount == amount)
            && !chain
                .iter()
                .any(|r| r.month == month && matches!(r.limit, Limit::MonthOnly { .. }));
        if span == Span::Onward && unchanged {
            return Ok(false);
        }
        self.set_amount(categories, category_id, month, span, amount, None, today)?;
        Ok(true)
    }

    /// Fill (9f): writes `source`'s Month-only amounts into `target`, as [`fill_preview`] lists
    /// them. Returns how many Categories it wrote.
    pub fn fill(
        &mut self,
        reads: &dyn BudgetReads,
        target: Period,
        source: FillSource,
        today: NaiveDate,
    ) -> Result<usize> {
        check_open_month(target, today)?;
        self.ensure_editable()?;
        let preview = fill_preview(self, reads, target, source);
        for line in &preview.changes {
            let chain = self.limits.entry(line.category_id).or_default();
            let rollover = inherited_rollover(chain, target);
            put(
                chain,
                LimitRecord {
                    month: target,
                    limit: Limit::MonthOnly {
                        amount: line.after.clone(),
                        rollover,
                    },
                },
            );
        }
        Ok(preview.changes.len())
    }

    /// Categories 5c's write: a changed value is an Onward from the current month, clearing is a
    /// Stop from it, and an unchanged value writes nothing. Returns whether it wrote.
    pub fn set_monthly_limit(
        &mut self,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        amount: Option<Money>,
        today: NaiveDate,
    ) -> Result<bool> {
        let current = Period::of(today);
        let existing = applied(self.chain(category_id), current).map(|found| found.amount);
        if existing == amount {
            return Ok(false);
        }
        match amount {
            Some(amount) => self.set_amount(
                categories,
                category_id,
                current,
                Span::Onward,
                amount,
                None,
                today,
            )?,
            None => self.stop(categories, category_id, current, today)?,
        }
        Ok(true)
    }
}

pub fn check_leaf(categories: &[BudgetCategory], category_id: CategoryId) -> Result<()> {
    if tree::is_expense(categories, category_id) && tree::is_leaf(categories, category_id) {
        Ok(())
    } else {
        Err(Error::NotAnExpenseLeaf)
    }
}

fn check_open_month(month: Period, today: NaiveDate) -> Result<()> {
    if month < Period::of(today) {
        Err(Error::ClosedMonth)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn saving_replaces_the_record_that_starts_in_that_month() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::Onward,
            "300.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::MonthOnly,
            "320.00",
            None,
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(world.category("Dining")).to_vec())
            .unwrap_or_default();
        assert_eq!(chain.len(), 2);
        assert_eq!(amount_in(&chain, sep().next()), Some(money("320.00")));
        assert_eq!(
            amount_in(&chain, sep().next().next()),
            Some(money("250.00"))
        );
    }

    #[test]
    fn a_new_record_inherits_the_rollover_of_the_one_it_follows() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            Some(Rollover::CarryBoth),
        );
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::Onward,
            "300.00",
            None,
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(world.category("Dining")).to_vec())
            .unwrap_or_default();
        assert_eq!(
            applied(&chain, sep().next()).map(|a| a.rollover),
            Some(Rollover::CarryBoth)
        );
        // With nothing before it, it starts at None.
        set(
            &mut budgets,
            &world,
            id,
            "Water",
            sep(),
            Span::Onward,
            "60.00",
            None,
        );
        let water = budgets
            .get(id)
            .map(|b| b.chain(world.category("Water")).to_vec())
            .unwrap_or_default();
        assert_eq!(
            applied(&water, sep()).map(|a| a.rollover),
            Some(Rollover::None)
        );
    }

    #[test]
    fn only_leaf_expense_categories_hold_amounts() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        for name in ["Food", "Salary"] {
            let refused = budgets.set_amount(
                id,
                world.ledger(),
                world.category(name),
                sep(),
                Span::Onward,
                money("10.00"),
                None,
                today(),
            );
            assert_eq!(refused, Err(Error::NotAnExpenseLeaf), "{name}");
        }
    }

    #[test]
    fn closed_months_are_read_only_and_amounts_cant_be_negative() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        let dining = world.category("Dining");
        let closed = budgets.set_amount(
            id,
            world.ledger(),
            dining,
            sep().prev(),
            Span::Onward,
            money("1.00"),
            None,
            today(),
        );
        assert_eq!(closed, Err(Error::ClosedMonth));
        let negative = budgets.set_amount(
            id,
            world.ledger(),
            dining,
            sep(),
            Span::Onward,
            money("-1.00"),
            None,
            today(),
        );
        assert_eq!(negative, Err(Error::NegativeAmount));
        assert_eq!(
            budgets.stop(id, world.ledger(), dining, sep().prev(), today()),
            Err(Error::ClosedMonth)
        );
    }

    #[test]
    fn stopping_ends_the_amount_from_that_month_and_keeps_the_past() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        assert_eq!(
            budgets.stop(
                id,
                world.ledger(),
                world.category("Dining"),
                sep().next(),
                today()
            ),
            Ok(())
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(world.category("Dining")).to_vec())
            .unwrap_or_default();
        assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
        assert_eq!(amount_in(&chain, sep().next()), None);
    }

    #[test]
    fn stopping_a_category_with_no_amount_writes_nothing() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        assert_eq!(
            budgets.stop(id, world.ledger(), world.category("Dining"), sep(), today()),
            Ok(())
        );
        assert_eq!(budgets.get(id).map(|b| b.category_ids().count()), Some(0));
    }

    #[test]
    fn clearing_a_month_only_record_restores_the_carried_onward_value() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        let dining = world.category("Dining");
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::MonthOnly,
            "400.00",
            None,
        );
        assert_eq!(
            budgets.clear_month_only(id, dining, sep().next(), today()),
            Ok(true)
        );
        // The Onward record itself is not a Month-only, so nothing more comes off.
        assert_eq!(
            budgets.clear_month_only(id, dining, sep(), today()),
            Ok(false)
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(dining).to_vec())
            .unwrap_or_default();
        assert_eq!(amount_in(&chain, sep().next()), Some(money("250.00")));
    }

    #[test]
    fn cycling_the_rollover_writes_an_onward_from_the_current_month() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        let dining = world.category("Dining");
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::MonthOnly,
            "250.00",
            None,
        );
        assert_eq!(
            budgets.cycle_rollover(id, dining, today()),
            Ok(Rollover::CarryUnspent)
        );
        assert_eq!(
            budgets.cycle_rollover(id, dining, today()),
            Ok(Rollover::CarryBoth)
        );
        assert_eq!(
            budgets.cycle_rollover(id, dining, today()),
            Ok(Rollover::None)
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(dining).to_vec())
            .unwrap_or_default();
        assert_eq!(chain.len(), 1);
        assert!(matches!(chain[0].limit, Limit::Onward { .. }));
        assert_eq!(
            budgets.cycle_rollover(id, world.category("Water"), today()),
            Err(Error::NotBudgeted)
        );
    }

    #[test]
    fn the_5c_write_is_onward_on_change_stop_on_clear_and_nothing_when_unchanged() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        let dining = world.category("Dining");
        let write = |budgets: &mut Budgets, amount: Option<&str>| {
            budgets.set_monthly_limit(id, world.ledger(), dining, amount.map(money), today())
        };
        assert_eq!(write(&mut budgets, None), Ok(false));
        assert_eq!(write(&mut budgets, Some("250.00")), Ok(true));
        assert_eq!(write(&mut budgets, Some("250")), Ok(false));
        assert_eq!(write(&mut budgets, Some("300.00")), Ok(true));
        assert_eq!(
            budgets.monthly_limit(id, world.ledger(), dining, today()),
            Some(money("300.00"))
        );
        assert_eq!(write(&mut budgets, None), Ok(true));
        assert_eq!(
            budgets.monthly_limit(id, world.ledger(), dining, today()),
            None
        );
    }

    #[test]
    fn saving_a_cell_onward_runs_until_the_next_record() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep(),
                Span::Onward,
                "250"
            ),
            Ok(true)
        );
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().next(),
                Span::Onward,
                "300"
            ),
            Ok(true)
        );
        let chain = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
        assert_eq!(
            amount_in(&chain, sep().next().next()),
            Some(money("300.00"))
        );
    }

    #[test]
    fn saving_a_cell_month_only_leaves_the_next_month_on_the_onward_value() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250",
        )
        .ok();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::MonthOnly,
            "400",
        )
        .ok();
        let chain = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(amount_in(&chain, sep().next()), Some(money("400.00")));
        assert_eq!(
            amount_in(&chain, sep().next().next()),
            Some(money("250.00"))
        );
    }

    #[test]
    fn typing_zero_writes_a_real_zero_and_not_unbudgeted() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(&mut budgets, &world, id, "Dining", sep(), Span::Onward, "0").ok();
        let chain = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(amount_in(&chain, sep()), Some(money("0.00")));
    }

    #[test]
    fn clearing_a_cell_onward_writes_a_stop_from_that_month() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250",
        )
        .ok();
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().next(),
                Span::Onward,
                ""
            ),
            Ok(true)
        );
        let chain = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
        assert_eq!(amount_in(&chain, sep().next()), None);
        assert_eq!(amount_in(&chain, sep().next().next()), None);
    }

    #[test]
    fn clearing_a_cell_month_only_removes_that_months_own_month_only_record() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250",
        )
        .ok();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::MonthOnly,
            "400",
        )
        .ok();
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().next(),
                Span::MonthOnly,
                ""
            ),
            Ok(true)
        );
        let chain = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(amount_in(&chain, sep().next()), Some(money("250.00")));
        // With no Month-only record there, a second clear has nothing to remove.
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().next(),
                Span::MonthOnly,
                ""
            ),
            Ok(false)
        );
    }

    #[test]
    fn saving_an_unchanged_amount_onward_writes_nothing() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250",
        )
        .ok();
        let before = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().next(),
                Span::Onward,
                "250.00"
            ),
            Ok(false)
        );
        assert_eq!(chain_of(&budgets, &world, id, "Dining"), before);
    }

    #[test]
    fn a_closed_month_or_a_bad_amount_is_refused() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().prev(),
                Span::Onward,
                "250"
            ),
            Err(Error::ClosedMonth)
        );
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep(),
                Span::Onward,
                "12x"
            ),
            Err(Error::InvalidAmount)
        );
    }
}
