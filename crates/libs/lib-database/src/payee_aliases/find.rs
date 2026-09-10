//! # Payee Aliases Query Operations

use lib_core as domain;

impl crate::PayeeAliases {
    /// List every Payee Alias — the source list [`crate::Payees::resolve_or_create`] tests
    /// typed Payee text against.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find all Payee Aliases: ", level = "debug", skip(pool))]
    pub async fn find_all(pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::Result<Vec<Self>> {
        let aliases = sqlx::query_as!(
            crate::PayeeAliases,
            r#"
                SELECT
                    id        AS "id!: domain::RowID",
                    payee_id  AS "payee_id!: domain::RowID",
                    pattern
                FROM payee_aliases
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(aliases)
    }

    /// List every Payee Alias recorded for one Payee (its rename history).
    ///
    /// # Errors
    /// Returns an error if the query fails.
    #[tracing::instrument(name = "Find Payee Aliases for a Payee: ", level = "debug", skip(pool))]
    pub async fn find_all_for_payee(
        payee_id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<Vec<Self>> {
        let aliases = sqlx::query_as!(
            crate::PayeeAliases,
            r#"
                SELECT
                    id        AS "id!: domain::RowID",
                    payee_id  AS "payee_id!: domain::RowID",
                    pattern
                FROM payee_aliases
                WHERE payee_id = ?
            "#,
            payee_id
        )
        .fetch_all(pool)
        .await?;

        Ok(aliases)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_returns_every_alias_across_every_payee(pool: SqlitePool) {
        let mut a = crate::Payees::mock();
        a.name = "Kmart".to_string();
        let a = a.insert(&pool).await.unwrap();
        crate::Payees::rename(a.id, "Kmart AU", &pool)
            .await
            .unwrap();

        let mut b = crate::Payees::mock();
        b.name = "JB".to_string();
        let b = b.insert(&pool).await.unwrap();
        crate::Payees::rename(b.id, "JB Hi-Fi", &pool)
            .await
            .unwrap();

        let aliases = crate::PayeeAliases::find_all(&pool).await.unwrap();

        assert_eq!(aliases.len(), 2);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn find_all_for_payee_scopes_to_one_payee(pool: SqlitePool) {
        let mut payee = crate::Payees::mock();
        payee.name = "Kmart".to_string();
        let payee = payee.insert(&pool).await.unwrap();
        crate::Payees::rename(payee.id, "Kmart AU", &pool)
            .await
            .unwrap();
        crate::Payees::rename(payee.id, "Kmart Australia", &pool)
            .await
            .unwrap();

        let other = crate::Payees::mock().insert(&pool).await.unwrap();

        let aliases = crate::PayeeAliases::find_all_for_payee(payee.id, &pool)
            .await
            .unwrap();

        assert_eq!(aliases.len(), 2);
        assert!(aliases.iter().all(|a| a.payee_id == payee.id));
        assert!(
            crate::PayeeAliases::find_all_for_payee(other.id, &pool)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
