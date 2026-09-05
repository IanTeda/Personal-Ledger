//! # Payees Update Operations
//!
//! Renaming is not a plain field update: it must also preserve the prior name as a
//! [`crate::PayeeAliases`] row so a suggestion typed against the old name still resolves to
//! this Payee (see `CONTEXT.md`'s Payee Alias entry,
//! [ADR-0012](../../../../../docs/adr/0012-payee-entity-with-rename-aliases.md)). There is
//! deliberately no plain "set name" method — every name change goes through [`Self::rename`]
//! so an alias is never skipped.

use lib_core as domain;

impl crate::Payees {
    /// Rename a Payee, preserving its prior name as a Payee Alias in the same transaction.
    /// A no-op (no alias created) if `new_name` is unchanged, case-insensitively.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Payee exists with this `id`, or an
    /// error if the underlying queries fail (including a unique-constraint violation if
    /// another Payee already has `new_name`).
    #[tracing::instrument(name = "Rename Payee: ", level = "debug", skip(pool))]
    pub async fn rename(
        id: domain::RowID,
        new_name: &str,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Self> {
        let current = Self::find_by_id(id, pool)
            .await?
            .ok_or_else(|| crate::DatabaseError::NotFound(format!("Payee {id} not found")))?;

        if current.name.eq_ignore_ascii_case(new_name) {
            return Ok(current);
        }

        let mut tx = pool.begin().await?;

        sqlx::query!(r#"UPDATE payees SET name = ? WHERE id = ?"#, new_name, id)
            .execute(&mut *tx)
            .await?;

        let alias_id = domain::RowID::new();
        let pattern = format!("(?i)^{}$", regex::escape(&current.name));
        sqlx::query!(
            r#"INSERT INTO payee_aliases (id, payee_id, pattern) VALUES (?, ?, ?)"#,
            alias_id,
            id,
            pattern
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Self::find_by_id(id, pool).await?.ok_or_else(|| {
            crate::DatabaseError::NotFound(format!("Payee {id} not found after rename"))
        })
    }

    /// Activate or deactivate a Payee by id.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::NotFound`] if no Payee exists with this `id`, or an
    /// error if the underlying query fails.
    #[tracing::instrument(name = "Set Payee active flag: ", level = "debug", skip(pool))]
    pub async fn set_active(
        id: domain::RowID,
        is_active: bool,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Self> {
        let result = sqlx::query!(
            r#"UPDATE payees SET is_active = ? WHERE id = ?"#,
            is_active,
            id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(crate::DatabaseError::NotFound(format!(
                "Payee {id} not found"
            )));
        }

        Self::find_by_id(id, pool).await?.ok_or_else(|| {
            crate::DatabaseError::NotFound(format!("Payee {id} not found after update"))
        })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn rename_changes_the_name_and_records_an_alias(pool: SqlitePool) {
        let mut payee = crate::Payees::mock();
        payee.name = "Kmart".to_string();
        let payee = payee.insert(&pool).await.unwrap();

        let renamed = crate::Payees::rename(payee.id, "Kmart AU", &pool)
            .await
            .unwrap();

        assert_eq!(renamed.name, "Kmart AU");

        let aliases = crate::PayeeAliases::find_all_for_payee(payee.id, &pool)
            .await
            .unwrap();
        assert_eq!(aliases.len(), 1);
        assert!(aliases[0].pattern.contains("Kmart"));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn rename_to_the_same_name_case_insensitively_is_a_no_op(pool: SqlitePool) {
        let mut payee = crate::Payees::mock();
        payee.name = "Kmart".to_string();
        let payee = payee.insert(&pool).await.unwrap();

        crate::Payees::rename(payee.id, "KMART", &pool)
            .await
            .unwrap();

        let aliases = crate::PayeeAliases::find_all_for_payee(payee.id, &pool)
            .await
            .unwrap();
        assert!(
            aliases.is_empty(),
            "no alias should be created for a no-op rename"
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn rename_errors_when_payee_missing(pool: SqlitePool) {
        let result = crate::Payees::rename(lib_core::RowID::new(), "New Name", &pool).await;
        assert!(matches!(result, Err(crate::DatabaseError::NotFound(_))));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn rename_rejects_colliding_with_another_payees_name(pool: SqlitePool) {
        let mut existing = crate::Payees::mock();
        existing.name = "Woolworths".to_string();
        existing.insert(&pool).await.unwrap();

        let mut other = crate::Payees::mock();
        other.name = "Coles".to_string();
        let other = other.insert(&pool).await.unwrap();

        let result = crate::Payees::rename(other.id, "woolworths", &pool).await;

        assert!(result.is_err());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn set_active_toggles_the_flag(pool: SqlitePool) {
        let payee = crate::Payees::mock().insert(&pool).await.unwrap();
        assert!(payee.is_active);

        let deactivated = crate::Payees::set_active(payee.id, false, &pool)
            .await
            .unwrap();
        assert!(!deactivated.is_active);
    }
}
