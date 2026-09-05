//! # Transactions Query Operations

use lib_core as domain;

impl crate::Transactions {
    /// Find a Transaction by its id.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find Transaction by id: ", level = "debug", skip(pool))]
    pub async fn find_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Option<Self>> {
        let transaction = sqlx::query_as!(
            crate::Transactions,
            r#"
                SELECT
                    id            AS "id!: domain::RowID",
                    date          AS "date!: chrono::NaiveDate",
                    amount        AS "amount!: domain::Money",
                    category_id   AS "category_id!: domain::RowID",
                    account_id    AS "account_id!: domain::RowID",
                    payee,
                    description,
                    status        AS "status!: domain::TransactionStatus",
                    is_flagged,
                    updated_on    AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM transactions
                WHERE id = ?
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(transaction)
    }

    /// List every Transaction, most recent value date first.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find all Transactions: ", level = "debug", skip(pool))]
    pub async fn find_all(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Vec<Self>> {
        let transactions = sqlx::query_as!(
            crate::Transactions,
            r#"
                SELECT
                    id            AS "id!: domain::RowID",
                    date          AS "date!: chrono::NaiveDate",
                    amount        AS "amount!: domain::Money",
                    category_id   AS "category_id!: domain::RowID",
                    account_id    AS "account_id!: domain::RowID",
                    payee,
                    description,
                    status        AS "status!: domain::TransactionStatus",
                    is_flagged,
                    updated_on    AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM transactions
                ORDER BY date DESC, id DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(transactions)
    }

    /// List Transactions with pagination, alongside the total (unpaginated) row count.
    ///
    /// # Errors
    /// Returns an error if either query fails.
    #[tracing::instrument(name = "Find Transactions with pagination: ", level = "debug", skip(pool))]
    pub async fn find_all_with_pagination(
        offset: i64,
        limit: i64,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i64)> {
        let transactions = sqlx::query_as!(
            crate::Transactions,
            r#"
                SELECT
                    id            AS "id!: domain::RowID",
                    date          AS "date!: chrono::NaiveDate",
                    amount        AS "amount!: domain::Money",
                    category_id   AS "category_id!: domain::RowID",
                    account_id    AS "account_id!: domain::RowID",
                    payee,
                    description,
                    status        AS "status!: domain::TransactionStatus",
                    is_flagged,
                    updated_on    AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM transactions
                ORDER BY date DESC, id DESC
                LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let total_count =
            sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!: i64" FROM transactions"#)
                .fetch_one(pool)
                .await?;

        Ok((transactions, total_count))
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    async fn seed_category_and_account(pool: &SqlitePool) -> (lib_core::RowID, lib_core::RowID) {
        let category = crate::Categories::mock().insert(pool).await.unwrap();
        let unit = crate::Units::mock().insert(pool).await.unwrap();
        let account = crate::Accounts::mock(unit.id).insert(pool).await.unwrap();
        (category.id, account.id)
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_finds_an_existing_transaction(pool: SqlitePool) {
        let (category_id, account_id) = seed_category_and_account(&pool).await;
        let transaction = crate::Transactions::mock(category_id, account_id)
            .insert(&pool)
            .await
            .unwrap();

        let found = crate::Transactions::find_by_id(transaction.id, &pool)
            .await
            .unwrap();

        assert_eq!(found.map(|t| t.id), Some(transaction.id));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_by_id_returns_none_when_missing(pool: SqlitePool) {
        let found = crate::Transactions::find_by_id(lib_core::RowID::new(), &pool)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_with_pagination_returns_a_page_and_the_total_count(pool: SqlitePool) {
        let (category_id, account_id) = seed_category_and_account(&pool).await;
        for _ in 0..5 {
            crate::Transactions::mock(category_id, account_id)
                .insert(&pool)
                .await
                .unwrap();
        }

        let (page, total) = crate::Transactions::find_all_with_pagination(0, 2, &pool)
            .await
            .unwrap();

        assert_eq!(page.len(), 2);
        assert_eq!(total, 5);
    }
}
