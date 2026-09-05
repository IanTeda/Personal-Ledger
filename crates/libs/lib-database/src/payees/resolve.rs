//! # Payee Resolution
//!
//! The single entry point Transaction entry goes through to turn typed Payee text into a
//! `payee_id`: reuse an exact (case-insensitive) name match, fall back to a Payee Alias
//! match, or auto-create a new canonical Payee (see `CONTEXT.md`'s Payee entry,
//! [ADR-0012](../../../../../docs/adr/0012-payee-entity-with-rename-aliases.md)).

impl crate::Payees {
    /// Resolve `name` to a Payee: an existing exact (case-insensitive) name match wins
    /// first, then a Payee Alias match (surfacing its current canonical Payee), and failing
    /// both, a new Payee is created with `name` verbatim.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::Validation`] if `name` is empty after trimming, or an
    /// error if the underlying queries fail.
    #[tracing::instrument(name = "Resolve or create Payee: ", level = "debug", skip(pool))]
    pub async fn resolve_or_create(
        name: &str,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Self> {
        let name = name.trim();
        if name.is_empty() {
            return Err(crate::DatabaseError::Validation(
                "Payee name must not be empty".to_string(),
            ));
        }

        if let Some(payee) = Self::find_by_name(name, pool).await? {
            return Ok(payee);
        }

        for alias in crate::PayeeAliases::find_all(pool).await? {
            let is_match = match regex::Regex::new(&alias.pattern) {
                Ok(regex) => regex.is_match(name),
                Err(err) => {
                    tracing::warn!(
                        "Skipping unparsable Payee Alias pattern {}: {err}",
                        alias.id
                    );
                    false
                }
            };
            if is_match && let Some(payee) = Self::find_by_id(alias.payee_id, pool).await? {
                return Ok(payee);
            }
        }

        crate::payees::PayeesBuilder::new()
            .with_name(name)
            .build()?
            .insert(pool)
            .await
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    #[sqlx::test(migrations = "migrations/client")]
    async fn resolve_or_create_reuses_an_exact_case_insensitive_match(pool: SqlitePool) {
        let mut payee = crate::Payees::mock();
        payee.name = "Woolworths".to_string();
        let payee = payee.insert(&pool).await.unwrap();

        let resolved = crate::Payees::resolve_or_create("woolworths", &pool)
            .await
            .unwrap();

        assert_eq!(resolved.id, payee.id);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn resolve_or_create_resolves_a_renamed_payee_via_its_alias(pool: SqlitePool) {
        let mut payee = crate::Payees::mock();
        payee.name = "Kmart".to_string();
        let payee = payee.insert(&pool).await.unwrap();
        let renamed = crate::Payees::rename(payee.id, "Kmart AU", &pool)
            .await
            .unwrap();

        let resolved = crate::Payees::resolve_or_create("kmart", &pool)
            .await
            .unwrap();

        assert_eq!(resolved.id, renamed.id);
        assert_eq!(resolved.name, "Kmart AU");
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn resolve_or_create_creates_a_new_payee_when_nothing_matches(pool: SqlitePool) {
        let resolved = crate::Payees::resolve_or_create("Brand New Payee", &pool)
            .await
            .unwrap();

        assert_eq!(resolved.name, "Brand New Payee");
        assert!(
            crate::Payees::find_by_id(resolved.id, &pool)
                .await
                .unwrap()
                .is_some()
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn resolve_or_create_rejects_a_blank_name(pool: SqlitePool) {
        let result = crate::Payees::resolve_or_create("   ", &pool).await;
        assert!(matches!(result, Err(crate::DatabaseError::Validation(_))));
    }
}
