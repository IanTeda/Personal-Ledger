//! # Balance Checks CSV Import
//!
//! FR.33: import Balance Checks for one Account from a CSV file with `date`/`balance`
//! columns (matched case-insensitively by header name, not position), one Balance Check per
//! row. The whole import is atomic (NFR.1) — a single DB transaction, committed only once
//! every row has parsed and inserted successfully; any malformed row aborts the entire
//! import rather than partially applying it. Only date/balance are read; Transactions,
//! Categories, and Payees are never imported (see `docs/product-requirements.md`,
//! Constraints).

impl crate::BalanceChecks {
    /// Import Balance Checks for `account_id` from `reader`'s CSV content.
    ///
    /// A header row is required, with `date` and `balance` columns present (matched
    /// case-insensitively, any order, extra columns ignored). An empty file (header only, or
    /// zero rows) succeeds as a no-op, returning an empty `Vec`.
    ///
    /// # Errors
    /// Returns [`crate::DatabaseError::CsvImport`] if the header is missing `date`/`balance`,
    /// or if any row has the wrong column count, an unparseable date, or an unparseable
    /// amount — the whole import is aborted, nothing is inserted. Also returns an error if
    /// `account_id` doesn't reference a real Account (SQLite's `foreign_keys` pragma) or if
    /// the underlying transaction fails.
    #[tracing::instrument(
        name = "Import Balance Checks from CSV: ",
        level = "debug",
        skip(reader, pool),
        fields(account_id = %account_id),
    )]
    pub async fn import_csv<R: std::io::Read>(
        account_id: lib_core::RowID,
        reader: R,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::Result<Vec<Self>> {
        let mut csv_reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(reader);

        let headers = csv_reader
            .headers()
            .map_err(|err| crate::Error::CsvImport(err.to_string()))?
            .clone();
        let date_index = headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case("date"))
            .ok_or_else(|| {
                crate::Error::CsvImport("missing required \"date\" column".to_string())
            })?;
        let balance_index = headers
            .iter()
            .position(|h| h.eq_ignore_ascii_case("balance"))
            .ok_or_else(|| {
                crate::Error::CsvImport("missing required \"balance\" column".to_string())
            })?;

        let now = chrono::Utc::now();
        let mut balance_checks = Vec::new();
        for (row_number, record) in csv_reader.records().enumerate() {
            // Row 1 is the first data row, right after the header.
            let row_number = row_number + 1;
            let record = record
                .map_err(|err| crate::Error::CsvImport(format!("row {row_number}: {err}")))?;

            let date_field = record.get(date_index).ok_or_else(|| {
                crate::Error::CsvImport(format!("row {row_number}: missing date field"))
            })?;
            let date: chrono::NaiveDate = date_field.trim().parse().map_err(|_| {
                crate::Error::CsvImport(format!(
                    "row {row_number}: invalid date {date_field:?} (expected YYYY-MM-DD)"
                ))
            })?;

            let balance_field = record.get(balance_index).ok_or_else(|| {
                crate::Error::CsvImport(format!("row {row_number}: missing balance field"))
            })?;
            let asserted_balance: lib_core::Money = balance_field.trim().parse().map_err(|_| {
                crate::Error::CsvImport(format!(
                    "row {row_number}: invalid balance amount {balance_field:?}"
                ))
            })?;

            balance_checks.push(Self {
                id: lib_core::RowID::new(),
                account_id,
                date,
                asserted_balance,
                created_on: now,
                updated_on: now,
            });
        }

        if balance_checks.is_empty() {
            return Ok(balance_checks);
        }

        let mut tx = pool.begin().await?;
        for balance_check in &balance_checks {
            sqlx::query!(
                r#"
                    INSERT INTO balance_checks (id, account_id, date, asserted_balance, created_on, updated_on)
                    VALUES (?, ?, ?, ?, ?, ?)
                "#,
                balance_check.id,
                balance_check.account_id,
                balance_check.date,
                balance_check.asserted_balance,
                balance_check.created_on,
                balance_check.updated_on
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        Ok(balance_checks)
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use std::io::Cursor;

    async fn seed_account(pool: &SqlitePool) -> lib_core::RowID {
        let unit = crate::Units::mock().insert(pool).await.unwrap();
        crate::Accounts::mock(unit.id)
            .insert(pool)
            .await
            .unwrap()
            .id
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn imports_one_balance_check_per_row(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        let csv = "date,balance\n2026-01-01,100.00\n2026-02-01,150.50\n";

        let imported = crate::BalanceChecks::import_csv(account_id, Cursor::new(csv), &pool)
            .await
            .unwrap();

        assert_eq!(imported.len(), 2);
        let (_, total) = crate::BalanceChecks::find_all_with_pagination(0, 10, &pool)
            .await
            .unwrap();
        assert_eq!(total, 2);
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn matches_columns_by_header_name_regardless_of_order(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        let csv = "balance,date\n-42.00,2026-03-15\n";

        let imported = crate::BalanceChecks::import_csv(account_id, Cursor::new(csv), &pool)
            .await
            .unwrap();

        assert_eq!(imported.len(), 1);
        assert_eq!(
            imported[0].date,
            chrono::NaiveDate::from_ymd_opt(2026, 3, 15).unwrap()
        );
        assert_eq!(imported[0].asserted_balance, "-42.00".parse().unwrap());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn rejects_a_missing_date_column(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        let csv = "balance\n100.00\n";

        let result = crate::BalanceChecks::import_csv(account_id, Cursor::new(csv), &pool).await;

        assert!(matches!(result, Err(crate::Error::CsvImport(_))));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn aborts_the_whole_import_on_a_malformed_row(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        let csv = "date,balance\n2026-01-01,100.00\nnot-a-date,50.00\n";

        let result = crate::BalanceChecks::import_csv(account_id, Cursor::new(csv), &pool).await;

        assert!(matches!(result, Err(crate::Error::CsvImport(_))));
        let (_, total) = crate::BalanceChecks::find_all_with_pagination(0, 10, &pool)
            .await
            .unwrap();
        assert_eq!(
            total, 0,
            "the valid first row must not have been committed either"
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn rejects_an_unparseable_balance(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        let csv = "date,balance\n2026-01-01,not-a-number\n";

        let result = crate::BalanceChecks::import_csv(account_id, Cursor::new(csv), &pool).await;

        assert!(matches!(result, Err(crate::Error::CsvImport(_))));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn an_empty_file_succeeds_as_a_no_op(pool: SqlitePool) {
        let account_id = seed_account(&pool).await;
        let csv = "date,balance\n";

        let imported = crate::BalanceChecks::import_csv(account_id, Cursor::new(csv), &pool)
            .await
            .unwrap();

        assert!(imported.is_empty());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn rejects_an_unknown_account_id(pool: SqlitePool) {
        let csv = "date,balance\n2026-01-01,100.00\n";

        let result =
            crate::BalanceChecks::import_csv(lib_core::RowID::new(), Cursor::new(csv), &pool).await;

        assert!(
            result.is_err(),
            "foreign_keys pragma should reject the unknown account_id"
        );
    }
}
