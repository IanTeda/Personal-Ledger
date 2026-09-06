//! # Category Totals
//!
//! FR.35: the sum of Transaction amounts per Category over a date range, scoped to a single
//! Unit (aggregating every Account denominated in it) or a single Account, at a time. Shows
//! the raw signed sum per Category — unlike `Budgets::current_progress`'s Expense-only
//! negation, a Category here can be Income/Asset/Liability/Equity/Expense, so negating would
//! incorrectly invert an Income-type Category's total. Summed in Rust via `BigDecimal`, not
//! SQL `SUM()`, since `Money` is stored as arbitrary-precision TEXT that SQLite's own
//! `SUM()` can't total correctly — the same approach `Budgets::current_progress` and
//! `Accounts::balance` already use.

impl crate::Categories {
    /// The signed Transaction total for every active Category, within `scope` and
    /// `[start, end]` (inclusive) — every active Category appears, even ones summing to
    /// zero for this range/scope.
    ///
    /// # Errors
    /// Returns an error if the underlying queries fail.
    #[tracing::instrument(name = "Compute Category totals: ", level = "debug", skip(pool))]
    pub async fn totals(
        scope: crate::TransactionScope,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<(lib_core::RowID, lib_core::Money)>> {
        let categories = Self::find_all_active(pool).await?;

        let mut totals = Vec::with_capacity(categories.len());
        for category in categories {
            let amounts = match scope {
                crate::TransactionScope::Unit(unit_id) => {
                    sqlx::query_scalar!(
                        r#"
                            SELECT t.amount AS "amount!: lib_core::Money"
                            FROM transactions t
                            JOIN accounts a ON t.account_id = a.id
                            WHERE t.category_id = ?
                              AND a.unit_id = ?
                              AND t.date >= ?
                              AND t.date <= ?
                        "#,
                        category.id,
                        unit_id,
                        start,
                        end
                    )
                    .fetch_all(pool)
                    .await?
                }
                crate::TransactionScope::Account(account_id) => {
                    sqlx::query_scalar!(
                        r#"
                            SELECT amount AS "amount!: lib_core::Money"
                            FROM transactions
                            WHERE category_id = ?
                              AND account_id = ?
                              AND date >= ?
                              AND date <= ?
                        "#,
                        category.id,
                        account_id,
                        start,
                        end
                    )
                    .fetch_all(pool)
                    .await?
                }
            };

            let sum = amounts
                .iter()
                .fold(bigdecimal::BigDecimal::from(0), |acc, amount| {
                    acc + &amount.0
                });
            totals.push((category.id, lib_core::Money::from(sum)));
        }

        Ok(totals)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_unit_and_account(pool: &SqlitePool) -> (lib_core::RowID, lib_core::RowID) {
        let unit = crate::Units::mock().insert(pool).await.unwrap();
        let account = crate::Accounts::mock(unit.id).insert(pool).await.unwrap();
        (unit.id, account.id)
    }

    async fn seed_category(pool: &SqlitePool) -> lib_core::RowID {
        // `Categories::mock()` picks a random is_active, which may be false and would then
        // be silently excluded by `totals()`'s `find_all_active` -- pin it to true so this
        // test exercises the real "always active" seeded state rather than depending on luck.
        let mut category = crate::Categories::mock();
        category.is_active = true;
        category.insert(pool).await.unwrap().id
    }

    fn date(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn totals_includes_every_active_category_even_at_zero(pool: SqlitePool) {
        let category_id = seed_category(&pool).await;
        let (unit_id, _) = seed_unit_and_account(&pool).await;

        let totals = crate::Categories::totals(
            crate::TransactionScope::Unit(unit_id),
            date(2026, 1, 1),
            date(2026, 1, 31),
            &pool,
        )
        .await
        .unwrap();

        assert!(
            totals
                .iter()
                .any(|(id, total)| { *id == category_id && *total == "0".parse().unwrap() })
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn totals_sums_signed_amounts_without_negating(pool: SqlitePool) {
        let category_id = seed_category(&pool).await;
        let (unit_id, account_id) = seed_unit_and_account(&pool).await;

        for amount in ["-20", "-30", "5"] {
            let mut transaction = crate::Transactions::mock(category_id, account_id);
            transaction.status = lib_core::TransactionStatus::Open;
            transaction.date = date(2026, 1, 15);
            transaction.amount = amount.parse().unwrap();
            transaction.insert(&pool).await.unwrap();
        }

        let totals = crate::Categories::totals(
            crate::TransactionScope::Unit(unit_id),
            date(2026, 1, 1),
            date(2026, 1, 31),
            &pool,
        )
        .await
        .unwrap();

        let total = totals
            .iter()
            .find(|(id, _)| *id == category_id)
            .unwrap()
            .1
            .clone();
        assert_eq!(total, "-45".parse().unwrap());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn totals_excludes_transactions_outside_the_date_range(pool: SqlitePool) {
        let category_id = seed_category(&pool).await;
        let (unit_id, account_id) = seed_unit_and_account(&pool).await;

        let mut transaction = crate::Transactions::mock(category_id, account_id);
        transaction.status = lib_core::TransactionStatus::Open;
        transaction.date = date(2026, 2, 1);
        transaction.amount = "-999".parse().unwrap();
        transaction.insert(&pool).await.unwrap();

        let totals = crate::Categories::totals(
            crate::TransactionScope::Unit(unit_id),
            date(2026, 1, 1),
            date(2026, 1, 31),
            &pool,
        )
        .await
        .unwrap();

        let total = totals
            .iter()
            .find(|(id, _)| *id == category_id)
            .unwrap()
            .1
            .clone();
        assert_eq!(total, "0".parse().unwrap());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn account_scope_excludes_other_accounts_in_the_same_unit(pool: SqlitePool) {
        let category_id = seed_category(&pool).await;
        let (unit_id, account_id) = seed_unit_and_account(&pool).await;
        let other_account = crate::Accounts::mock(unit_id).insert(&pool).await.unwrap();

        let mut transaction = crate::Transactions::mock(category_id, other_account.id);
        transaction.status = lib_core::TransactionStatus::Open;
        transaction.date = date(2026, 1, 15);
        transaction.amount = "-100".parse().unwrap();
        transaction.insert(&pool).await.unwrap();

        let totals = crate::Categories::totals(
            crate::TransactionScope::Account(account_id),
            date(2026, 1, 1),
            date(2026, 1, 31),
            &pool,
        )
        .await
        .unwrap();

        let total = totals
            .iter()
            .find(|(id, _)| *id == category_id)
            .unwrap()
            .1
            .clone();
        assert_eq!(
            total,
            "0".parse().unwrap(),
            "the other Account's Transaction must not count"
        );
    }
}
