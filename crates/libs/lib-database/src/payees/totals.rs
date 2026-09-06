//! # Payee Totals
//!
//! FR.36: the sum of Transaction amounts per Payee over a date range, scoped to a single
//! Unit (aggregating every Account denominated in it) or a single Account, at a time.
//! Unlike [`crate::Categories::totals`], which iterates every active Category first (a
//! small, curated set), this is driven by the matching Transactions themselves: Payees can
//! number in the dozens or hundreds (often auto-created from typed text, see ADR-0012), so
//! only a Payee with at least one matching Transaction in range appears — no zero-sum rows.
//! An inactive Payee still appears if it has a matching Transaction: `is_active` governs
//! whether a Payee is offered for *new* Transactions, not whether its past activity counts.
//! Transactions with no Payee (`payee_id IS NULL`, FR.16's optional Payee) are excluded.
//! Shows the raw signed sum per Payee, same reasoning as Category-total: a Payee can receive
//! both outgoing and incoming Transactions (e.g. an employer), so negating would be wrong.
//! Grouped and summed in Rust via `BigDecimal`, not SQL `SUM()`/`GROUP BY`, since `Money` is
//! stored as arbitrary-precision TEXT that SQLite's own `SUM()` can't total correctly.

impl crate::Payees {
    /// The signed Transaction total for every Payee with at least one matching Transaction,
    /// within `scope` and `[start, end]` (inclusive).
    ///
    /// # Errors
    /// Returns an error if the underlying query fails.
    #[tracing::instrument(name = "Compute Payee totals: ", level = "debug", skip(pool))]
    pub async fn totals(
        scope: crate::TransactionScope,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<(lib_core::RowID, lib_core::Money)>> {
        let rows: Vec<(lib_core::RowID, lib_core::Money)> = match scope {
            crate::TransactionScope::Unit(unit_id) => sqlx::query!(
                r#"
                    SELECT t.payee_id AS "payee_id!: lib_core::RowID", t.amount AS "amount!: lib_core::Money"
                    FROM transactions t
                    JOIN accounts a ON t.account_id = a.id
                    WHERE t.payee_id IS NOT NULL
                      AND a.unit_id = ?
                      AND t.date >= ?
                      AND t.date <= ?
                "#,
                unit_id,
                start,
                end
            )
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|row| (row.payee_id, row.amount))
            .collect(),
            crate::TransactionScope::Account(account_id) => sqlx::query!(
                r#"
                    SELECT payee_id AS "payee_id!: lib_core::RowID", amount AS "amount!: lib_core::Money"
                    FROM transactions
                    WHERE payee_id IS NOT NULL
                      AND account_id = ?
                      AND date >= ?
                      AND date <= ?
                "#,
                account_id,
                start,
                end
            )
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|row| (row.payee_id, row.amount))
            .collect(),
        };

        // RowID isn't Hash, so group with a linear scan, matching this codebase's usual
        // small-collection id-lookup convention rather than pulling in a HashMap.
        let mut sums: Vec<(lib_core::RowID, bigdecimal::BigDecimal)> = Vec::new();
        for (payee_id, amount) in rows {
            match sums.iter_mut().find(|(id, _)| *id == payee_id) {
                Some((_, sum)) => *sum += &amount.0,
                None => sums.push((payee_id, amount.0)),
            }
        }

        Ok(sums
            .into_iter()
            .map(|(id, sum)| (id, lib_core::Money::from(sum)))
            .collect())
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
        crate::Categories::mock().insert(pool).await.unwrap().id
    }

    async fn seed_payee(pool: &SqlitePool, is_active: bool) -> lib_core::RowID {
        let mut payee = crate::Payees::mock();
        payee.is_active = is_active;
        payee.insert(pool).await.unwrap().id
    }

    fn date(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn totals_sums_signed_amounts_per_payee(pool: SqlitePool) {
        let category_id = seed_category(&pool).await;
        let (unit_id, account_id) = seed_unit_and_account(&pool).await;
        let payee_id = seed_payee(&pool, true).await;

        for amount in ["-20", "-30", "5"] {
            let mut transaction = crate::Transactions::mock(category_id, account_id);
            transaction.status = lib_core::TransactionStatus::Open;
            transaction.date = date(2026, 1, 15);
            transaction.amount = amount.parse().unwrap();
            transaction.payee_id = Some(payee_id);
            transaction.insert(&pool).await.unwrap();
        }

        let totals = crate::Payees::totals(
            crate::TransactionScope::Unit(unit_id),
            date(2026, 1, 1),
            date(2026, 1, 31),
            &pool,
        )
        .await
        .unwrap();

        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0], (payee_id, "-45".parse().unwrap()));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn totals_excludes_payees_with_no_matching_transaction(pool: SqlitePool) {
        let (unit_id, _) = seed_unit_and_account(&pool).await;
        seed_payee(&pool, true).await;

        let totals = crate::Payees::totals(
            crate::TransactionScope::Unit(unit_id),
            date(2026, 1, 1),
            date(2026, 1, 31),
            &pool,
        )
        .await
        .unwrap();

        assert!(
            totals.is_empty(),
            "a Payee with no matching Transaction must not appear"
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn totals_includes_an_inactive_payee_with_a_matching_transaction(pool: SqlitePool) {
        let category_id = seed_category(&pool).await;
        let (unit_id, account_id) = seed_unit_and_account(&pool).await;
        let payee_id = seed_payee(&pool, false).await;

        let mut transaction = crate::Transactions::mock(category_id, account_id);
        transaction.status = lib_core::TransactionStatus::Open;
        transaction.date = date(2026, 1, 15);
        transaction.amount = "-10".parse().unwrap();
        transaction.payee_id = Some(payee_id);
        transaction.insert(&pool).await.unwrap();

        let totals = crate::Payees::totals(
            crate::TransactionScope::Unit(unit_id),
            date(2026, 1, 1),
            date(2026, 1, 31),
            &pool,
        )
        .await
        .unwrap();

        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].0, payee_id);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn totals_excludes_transactions_with_no_payee(pool: SqlitePool) {
        let category_id = seed_category(&pool).await;
        let (unit_id, account_id) = seed_unit_and_account(&pool).await;

        let mut transaction = crate::Transactions::mock(category_id, account_id);
        transaction.status = lib_core::TransactionStatus::Open;
        transaction.date = date(2026, 1, 15);
        transaction.amount = "-10".parse().unwrap();
        transaction.payee_id = None;
        transaction.insert(&pool).await.unwrap();

        let totals = crate::Payees::totals(
            crate::TransactionScope::Unit(unit_id),
            date(2026, 1, 1),
            date(2026, 1, 31),
            &pool,
        )
        .await
        .unwrap();

        assert!(totals.is_empty());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn totals_excludes_transactions_outside_the_date_range(pool: SqlitePool) {
        let category_id = seed_category(&pool).await;
        let (unit_id, account_id) = seed_unit_and_account(&pool).await;
        let payee_id = seed_payee(&pool, true).await;

        let mut transaction = crate::Transactions::mock(category_id, account_id);
        transaction.status = lib_core::TransactionStatus::Open;
        transaction.date = date(2026, 2, 1);
        transaction.amount = "-999".parse().unwrap();
        transaction.payee_id = Some(payee_id);
        transaction.insert(&pool).await.unwrap();

        let totals = crate::Payees::totals(
            crate::TransactionScope::Unit(unit_id),
            date(2026, 1, 1),
            date(2026, 1, 31),
            &pool,
        )
        .await
        .unwrap();

        assert!(totals.is_empty());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn account_scope_excludes_other_accounts_in_the_same_unit(pool: SqlitePool) {
        let category_id = seed_category(&pool).await;
        let (unit_id, account_id) = seed_unit_and_account(&pool).await;
        let other_account = crate::Accounts::mock(unit_id).insert(&pool).await.unwrap();
        let payee_id = seed_payee(&pool, true).await;

        let mut transaction = crate::Transactions::mock(category_id, other_account.id);
        transaction.status = lib_core::TransactionStatus::Open;
        transaction.date = date(2026, 1, 15);
        transaction.amount = "-100".parse().unwrap();
        transaction.payee_id = Some(payee_id);
        transaction.insert(&pool).await.unwrap();

        let totals = crate::Payees::totals(
            crate::TransactionScope::Account(account_id),
            date(2026, 1, 1),
            date(2026, 1, 31),
            &pool,
        )
        .await
        .unwrap();

        assert!(
            totals.is_empty(),
            "the other Account's Transaction must not count"
        );
    }
}
