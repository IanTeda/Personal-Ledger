//! # Budget Progress
//!
//! Computes how a Budget is tracking against its current period: spend-so-far against its
//! limit, and how far through the period today is — the data the TUI's progress-bar-vs-
//! "today"-line visualization renders (see the closed TUI feasibility map's prototype,
//! carried forward to CC-TUI-010).
//!
//! "Spend" sums every Transaction's signed amount in this Budget's Category and Unit within
//! the current period, then negates the sum: a net outflow (the common case, since Category
//! Transactions are usually negative-signed expenses) reports as a positive spend figure,
//! while a refund/credit in the same Category correctly reduces it — rather than being
//! ignored outright. Summed in Rust via `BigDecimal`, not SQL `SUM()`, since `Money` is
//! stored as arbitrary-precision TEXT that SQLite's own `SUM()` can't total correctly.

use lib_core as domain;

/// How a Budget is tracking against its current period.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetProgress {
    /// The negated sum of signed Transaction amounts in this Budget's Category/Unit within
    /// the current period — positive for a net outflow, the common case.
    pub spend: lib_core::Money,

    /// This Budget's limit amount, carried through for convenience.
    pub limit_amount: lib_core::Money,

    /// The current period's first day (inclusive).
    pub period_start: chrono::NaiveDate,

    /// The current period's last day (inclusive).
    pub period_end: chrono::NaiveDate,

    /// How far through the current period today is, from `0.0` (the first day) to `1.0`
    /// (the last day) — the "today" line's position on the progress bar.
    pub today_fraction: f64,
}

impl BudgetProgress {
    /// Spend as a fraction of the limit — the progress bar's fill, not clamped to `1.0` so a
    /// caller can tell an over-budget Budget apart from one merely at its limit.
    pub fn spend_fraction(&self) -> f64 {
        use bigdecimal::ToPrimitive;

        if self.limit_amount.0 == 0 {
            return 0.0;
        }
        let fraction = &self.spend.0 / &self.limit_amount.0;
        fraction.to_f64().unwrap_or(0.0)
    }

    /// Whether spend has outpaced how far through the period today is — "ahead of pace" in
    /// the bad sense: spending faster than the period is elapsing.
    pub fn is_ahead_of_pace(&self) -> bool {
        self.spend_fraction() > self.today_fraction
    }
}

impl crate::Budgets {
    /// Compute this Budget's progress against its current calendar period.
    ///
    /// # Errors
    /// Returns an error if the underlying query fails.
    #[tracing::instrument(
        name = "Compute Budget progress: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id),
    )]
    pub async fn current_progress(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<BudgetProgress> {
        let today = chrono::Utc::now().date_naive();
        let (period_start, period_end) = self.period.current_bounds(today);

        let amounts = sqlx::query_scalar!(
            r#"
                SELECT t.amount AS "amount!: domain::Money"
                FROM transactions t
                JOIN accounts a ON t.account_id = a.id
                WHERE t.category_id = ?
                  AND a.unit_id = ?
                  AND t.date >= ?
                  AND t.date <= ?
            "#,
            self.category_id,
            self.unit_id,
            period_start,
            period_end
        )
        .fetch_all(pool)
        .await?;

        let signed_sum = amounts
            .iter()
            .fold(bigdecimal::BigDecimal::from(0), |acc, amount| {
                acc + &amount.0
            });
        let spend = lib_core::Money::from(-signed_sum);

        Ok(BudgetProgress {
            spend,
            limit_amount: self.limit_amount.clone(),
            period_start,
            period_end,
            today_fraction: self.period.progress_fraction(today),
        })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_expense_category(pool: &SqlitePool) -> lib_core::RowID {
        let mut category = crate::Categories::mock();
        category.category_type = lib_core::CategoryTypes::Expense;
        category.insert(pool).await.unwrap().id
    }

    async fn seed_account(pool: &SqlitePool, unit_id: lib_core::RowID) -> lib_core::RowID {
        crate::Accounts::mock(unit_id)
            .insert(pool)
            .await
            .unwrap()
            .id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn current_progress_sums_negated_spend_in_the_current_period(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let unit = crate::Units::mock().insert(&pool).await.unwrap();
        let account_id = seed_account(&pool, unit.id).await;

        let mut budget = crate::Budgets::mock(category_id, unit.id);
        budget.limit_amount = "100".parse::<lib_core::Money>().unwrap();
        budget.period = lib_core::BudgetPeriod::Monthly;
        let budget = budget.insert(&pool).await.unwrap();

        let today = chrono::Utc::now().date_naive();
        for amount in ["-20", "-30"] {
            let mut transaction = crate::Transactions::mock(category_id, account_id);
            transaction.status = lib_core::TransactionStatus::Open;
            transaction.date = today;
            transaction.amount = amount.parse().unwrap();
            transaction.insert(&pool).await.unwrap();
        }

        let progress = budget.current_progress(&pool).await.unwrap();

        assert_eq!(progress.spend, "50".parse::<lib_core::Money>().unwrap());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn current_progress_ignores_transactions_outside_the_period(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let unit = crate::Units::mock().insert(&pool).await.unwrap();
        let account_id = seed_account(&pool, unit.id).await;

        let mut budget = crate::Budgets::mock(category_id, unit.id);
        budget.period = lib_core::BudgetPeriod::Monthly;
        let budget = budget.insert(&pool).await.unwrap();

        let mut transaction = crate::Transactions::mock(category_id, account_id);
        transaction.status = lib_core::TransactionStatus::Open;
        transaction.date = chrono::Utc::now().date_naive() - chrono::Duration::days(400);
        transaction.amount = "-999".parse().unwrap();
        transaction.insert(&pool).await.unwrap();

        let progress = budget.current_progress(&pool).await.unwrap();

        assert_eq!(progress.spend, "0".parse::<lib_core::Money>().unwrap());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn current_progress_ignores_a_different_category(pool: SqlitePool) {
        let category_id = seed_expense_category(&pool).await;
        let other_category_id = seed_expense_category(&pool).await;
        let unit = crate::Units::mock().insert(&pool).await.unwrap();
        let account_id = seed_account(&pool, unit.id).await;

        let budget = crate::Budgets::mock(category_id, unit.id)
            .insert(&pool)
            .await
            .unwrap();

        let mut transaction = crate::Transactions::mock(other_category_id, account_id);
        transaction.status = lib_core::TransactionStatus::Open;
        transaction.date = chrono::Utc::now().date_naive();
        transaction.amount = "-50".parse().unwrap();
        transaction.insert(&pool).await.unwrap();

        let progress = budget.current_progress(&pool).await.unwrap();

        assert_eq!(progress.spend, "0".parse::<lib_core::Money>().unwrap());
    }

    #[test]
    fn spend_fraction_computes_the_ratio() {
        let progress = super::BudgetProgress {
            spend: "25".parse::<lib_core::Money>().unwrap(),
            limit_amount: "100".parse::<lib_core::Money>().unwrap(),
            period_start: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            period_end: chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            today_fraction: 0.5,
        };
        assert!((progress.spend_fraction() - 0.25).abs() < f64::EPSILON);
        assert!(!progress.is_ahead_of_pace());
    }

    #[test]
    fn is_ahead_of_pace_when_spend_fraction_exceeds_today_fraction() {
        let progress = super::BudgetProgress {
            spend: "80".parse::<lib_core::Money>().unwrap(),
            limit_amount: "100".parse::<lib_core::Money>().unwrap(),
            period_start: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            period_end: chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            today_fraction: 0.3,
        };
        assert!(progress.is_ahead_of_pace());
    }
}
