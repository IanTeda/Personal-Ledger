//! # Payees Insert Operations

impl crate::Payees {
    /// Insert this Payee into the database.
    ///
    /// # Errors
    /// Returns an error if the underlying INSERT or read-back SELECT fails — including a
    /// unique-constraint violation if a Payee with the same name (case-insensitively)
    /// already exists.
    #[tracing::instrument(
        name = "Insert new Payee into database: ",
        level = "debug",
        skip(self, pool),
        fields(id = %self.id, name = %self.name),
    )]
    pub async fn insert(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Self> {
        let insert_result = sqlx::query!(
            r#"
                INSERT INTO payees (id, name, is_active, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?)
            "#,
            self.id,
            self.name,
            self.is_active,
            self.created_on,
            self.updated_on
        )
        .execute(pool)
        .await;

        match insert_result {
            Ok(result) => {
                if result.rows_affected() != 1 {
                    tracing::warn!(
                        "INSERT operation affected {} rows instead of 1 for payee: {}",
                        result.rows_affected(),
                        self.name
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert payee {}: {}", self.name, e);
                return Err(e.into());
            }
        }

        Self::find_by_id(self.id, pool).await?.ok_or_else(|| {
            crate::DatabaseError::NotFound(format!("Payee {} not found after insert", self.id))
        })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_persists_and_reads_back_a_payee(pool: SqlitePool) {
        let payee = crate::Payees::mock();

        let inserted = payee.insert(&pool).await.unwrap();

        assert_eq!(inserted.id, payee.id);
        assert_eq!(inserted.name, payee.name);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn insert_rejects_a_duplicate_name_case_insensitively(pool: SqlitePool) {
        let payee = crate::Payees::mock().insert(&pool).await.unwrap();

        let mut duplicate = crate::Payees::mock();
        duplicate.name = payee.name.to_uppercase();

        let result = duplicate.insert(&pool).await;

        assert!(
            result.is_err(),
            "the case-insensitive unique constraint on name should reject this"
        );
    }
}
