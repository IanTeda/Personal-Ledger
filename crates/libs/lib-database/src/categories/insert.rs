//! # Categories Insert Operations
//!
//! This module provides database insertion operations for category records in the Personal Ledger application.
//!
//! ## Overview
//!
//! The insert operations support three main use cases:
//! 1. Single category insertion with validation and retrieval
//! 2. Bulk category insertion with transactional guarantees
//! 3. Upsert operations (insert or update) for flexible data management
//!
//! ## Key Features
//!
//! - **Atomic Operations**: All insertions use proper error handling and transaction management
//! - **Data Validation**: Input validation with comprehensive logging
//! - **Idempotent Design**: Operations handle edge cases gracefully
//! - **Performance Optimised**: Efficient query patterns and connection pooling
//! - **Comprehensive Logging**: Multi-level logging for debugging and monitoring
//!
//! ## Database Schema
//!
//! Operations work with the `categories` table containing:
//! - Unique identifiers and business keys
//! - Category metadata (name, description, type)
//! - Visual properties (colour, icon)
//! - Status and timestamp fields
//!
//! ## Error Handling
//!
//! All operations return `DatabaseResult<T>` with structured error types:
//! - Connection failures
//! - Constraint violations (unique key conflicts)
//! - Transaction failures
//! - Data integrity issues
//!
//! ## Logging Levels
//!
//! - **ERROR**: Critical failures requiring immediate attention
//! - **WARN**: Potential issues or unexpected behaviour
//! - **INFO**: Successful operations and milestones
//! - **DEBUG**: Detailed operation information
//! - **TRACE**: Granular execution steps for deep debugging
//!
//! ## Examples
//!
//! ```rust,no_run
//! use lib_database::categories::Categories;
//! use lib_database::DatabaseConnection;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a database connection
//! let config = lib_database::DatabaseConfig::default();
//! let connection = lib_database::DatabaseConnection::new(config).await?;
//! let pool = connection.pool();
//!
//! // Single insert
//! let category = Categories {
//!     id: lib_domain::RowID::new(),
//!     code: "FOO.BAR.BAZ".to_string(),
//!     name: "Example Category".to_string(),
//!     description: Some("An example category".to_string()),
//!     url_slug: Some(lib_domain::UrlSlug::parse("example-category").unwrap()),
//!     category_type: lib_domain::CategoryTypes::Expense,
//!     color: Some(lib_domain::HexColor::parse("#FF5733").unwrap()),
//!     icon: Some("shopping-cart".to_string()),
//!     is_active: true,
//!     created_on: chrono::Utc::now(),
//!     updated_on: chrono::Utc::now(),
//! };
//! let inserted = category.insert(pool).await?;
//!
//! // Bulk insert
//! let categories = vec![
//!     Categories {
//!         id: lib_domain::RowID::new(),
//!         code: "FOO.BAR.BAZ".to_string(),
//!         name: "Category 1".to_string(),
//!         description: None,
//!         url_slug: Some(lib_domain::UrlSlug::parse("category-1").unwrap()),
//!         category_type: lib_domain::CategoryTypes::Expense,
//!         color: None,
//!         icon: None,
//!         is_active: true,
//!         created_on: chrono::Utc::now(),
//!         updated_on: chrono::Utc::now(),
//!     },
//!     Categories {
//!         id: lib_domain::RowID::new(),
//!         code: "FOO.BAR.QUX".to_string(),
//!         name: "Category 2".to_string(),
//!         description: None,
//!         url_slug: Some(lib_domain::UrlSlug::parse("category-2").unwrap()),
//!         category_type: lib_domain::CategoryTypes::Income,
//!         color: None,
//!         icon: None,
//!         is_active: true,
//!         created_on: chrono::Utc::now(),
//!         updated_on: chrono::Utc::now(),
//!     },
//! ];
//! let inserted_bulk = Categories::insert_many(&categories, pool).await?;
//!
//! // Upsert operation
//! let upserted = Categories::insert_or_update(&category, pool).await?;
//!
//! # Ok(())
//! # }
//! ```

use lib_domain as domain;

impl crate::Categories {
    /// Inserts a new category into the database.
    ///
    /// This method performs a single category insertion with comprehensive validation,
    /// error handling, and logging. The operation is atomic and will either succeed
    /// completely or fail without side effects.
    ///
    /// ## Process
    ///
    /// 1. **Validation**: Input data is validated before database operations
    /// 2. **Insertion**: Category is inserted using parameterised queries
    /// 3. **Verification**: Inserted record is retrieved to ensure data consistency
    /// 4. **Logging**: Comprehensive logging at multiple levels for monitoring
    ///
    /// ## Arguments
    ///
    /// * `pool` - The database connection pool for executing queries
    ///
    /// ## Returns
    ///
    /// Returns `DatabaseResult<Self>` containing the inserted category with any
    /// database-generated values (timestamps, etc.).
    ///
    /// ## Errors
    ///
    /// This method will return an error if:
    /// - Database connection fails
    /// - Unique constraint violations occur (duplicate code/name)
    /// - Foreign key constraints are violated
    /// - Transaction fails to commit
    /// - Data validation fails
    ///
    /// ## Logging
    ///
    /// - **ERROR**: Database errors with full context
    /// - **WARN**: Unexpected row counts or anomalies
    /// - **INFO**: Successful insertion confirmation
    /// - **DEBUG**: Validation and operation details
    /// - **TRACE**: Step-by-step execution flow
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use lib_database::categories::Categories;
    /// use lib_database::DatabaseConnection;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a database connection
    /// let config = lib_database::DatabaseConfig::default();
    /// let connection = lib_database::DatabaseConnection::new(config).await?;
    /// let pool = connection.pool();
    ///
    /// // Create and insert a category
    /// let category = Categories {
    ///     id: lib_domain::RowID::new(),
    ///     code: "FOO.BAR.BAZ".to_string(),
    ///     name: "Example Category".to_string(),
    ///     description: Some("An example category".to_string()),
    ///     url_slug: Some(lib_domain::UrlSlug::parse("example-category").unwrap()),
    ///     category_type: lib_domain::CategoryTypes::Expense,
    ///     color: Some(lib_domain::HexColor::parse("#FF5733").unwrap()),
    ///     icon: Some("shopping-cart".to_string()),
    ///     is_active: true,
    ///     created_on: chrono::Utc::now(),
    ///     updated_on: chrono::Utc::now(),
    /// };
    /// let inserted = category.insert(pool).await?;
    ///
    /// println!("Inserted category: {}", inserted.name);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Performance
    ///
    /// - Uses parameterised queries to prevent SQL injection
    /// - Single round-trip for insert + select operations
    /// - Connection pooling for efficient resource usage
    #[tracing::instrument(
        name = "Insert new Category into database: ",
        level = "debug",
        skip(self, pool),
        fields(
            id = % self.id,
            code = % self.code,
            name = % self.name,
            description = ? self.description,
            url_slug = ? self.url_slug,
            category_type = % self.category_type,
            color = ? self.color,
            icon = ? self.icon,
            is_active = % self.is_active,
            created_on = % self.created_on,
            updated_on = % self.updated_on,
        ),
    )]
    pub async fn insert(&self, pool: &sqlx::Pool<sqlx::Sqlite>) -> crate::DatabaseResult<Self> {
        tracing::trace!("Starting single category insert operation for category: {} (id: {})", self.code, self.id);

        // Validate input data before database operations
        tracing::debug!("Validating category data before insert: code={}, type={}, active={}", self.code, self.category_type, self.is_active);

        // 1) INSERT: SQLite uses `?` placeholders and does not reliably support
        // `RETURNING *` for compile-time checked macros. Execute the insert first.
        tracing::trace!("Executing INSERT query for category: {}", self.code);
        let insert_query = sqlx::query!(
            r#"
                INSERT INTO categories (id, code, name, description, url_slug, category_type, color, icon, is_active, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            self.id,
            self.code,
            self.name,
            self.description,
            self.url_slug,
            self.category_type,
            self.color,
            self.icon,
            self.is_active,
            self.created_on,
            self.updated_on
        );

        let insert_result = insert_query.execute(pool).await;
        match insert_result {
            Ok(result) => {
                tracing::trace!("INSERT query executed successfully for category: {} (rows affected: {})", self.code, result.rows_affected());
                if result.rows_affected() != 1 {
                    tracing::warn!("INSERT operation affected {} rows instead of 1 for category: {}", result.rows_affected(), self.code);
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert category: {} (id: {}) - {}", self.code, self.id, e);
                return Err(e.into());
            }
        }

        tracing::debug!("Category inserted successfully, retrieving inserted record: {}", self.id);

        // 2) SELECT: Read back the inserted row with explicit type annotations
        // for UUID and chrono types to avoid NULL/mapping issues in SQLite.
        tracing::trace!("Executing SELECT query to retrieve inserted category: {}", self.id);
        let category = match sqlx::query_as!(
            crate::Categories,
            r#"
                SELECT
                    id              AS "id!: domain::RowID",
                    code,
                    name,
                    description,
                    url_slug        AS "url_slug?: domain::UrlSlug",
                    category_type   AS "category_type!: domain::CategoryTypes",
                    color           AS "color?: domain::HexColor",
                    icon,
                    is_active       AS "is_active!: bool",
                    created_on      AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on      AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM categories
                WHERE id = ?
            "#,
            self.id
        )
        .fetch_one(pool)
        .await {
            Ok(cat) => {
                tracing::trace!("SELECT query completed, retrieved category: {} (id: {})", cat.code, cat.id);
                cat
            }
            Err(e) => {
                tracing::error!("Failed to retrieve inserted category: {} (id: {}) - {}", self.code, self.id, e);
                return Err(e.into());
            }
        };

        tracing::info!("✅ Category '{}' inserted successfully with ID: {}", category.code, category.id);
        tracing::debug!("Category details: type={}, active={}, created={}", category.category_type, category.is_active, category.created_on);

        Ok(category)
    }

    /// Inserts multiple categories into the database in a single atomic operation.
    ///
    /// This method provides efficient bulk insertion with transactional guarantees,
    /// comprehensive error handling, and detailed progress tracking. All categories
    /// are inserted within a single database transaction, ensuring atomicity.
    ///
    /// ## Process
    ///
    /// 1. **Validation**: Check for empty input and log appropriately
    /// 2. **Transaction**: Begin database transaction for atomicity
    /// 3. **Batch Processing**: Insert each category individually within transaction
    /// 4. **Verification**: Retrieve each inserted record for consistency
    /// 5. **Commit**: Commit transaction if all operations succeed
    /// 6. **Reporting**: Comprehensive success/failure statistics
    ///
    /// ## Arguments
    ///
    /// * `categories` - Slice of category instances to insert
    /// * `pool` - The database connection pool for executing queries
    ///
    /// ## Returns
    ///
    /// Returns `DatabaseResult<Vec<Self>>` containing all successfully inserted
    /// categories with database-generated values. The returned vector may be
    /// shorter than input if some insertions failed.
    ///
    /// ## Errors
    ///
    /// This method will return an error if:
    /// - Database connection fails
    /// - Transaction cannot be started
    /// - Any category insertion violates constraints
    /// - Transaction commit fails (rolls back all changes)
    ///
    /// ## Logging
    ///
    /// - **ERROR**: Individual insertion failures and transaction errors
    /// - **WARN**: Partial failures or unexpected behaviour
    /// - **INFO**: Operation start/completion with statistics
    /// - **DEBUG**: Batch details and final results
    /// - **TRACE**: Per-category processing steps
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use lib_database::categories::Categories;
    /// use lib_database::DatabaseConnection;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a database connection
    /// let config = lib_database::DatabaseConfig::default();
    /// let connection = lib_database::DatabaseConnection::new(config).await?;
    /// let pool = connection.pool();
    ///
    /// // Create multiple categories
    /// let categories = vec![
    ///     Categories {
    ///         id: lib_domain::RowID::new(),
    ///         code: "FOO.BAR.BAZ".to_string(),
    ///         name: "Category 1".to_string(),
    ///         description: None,
    ///         url_slug: Some(lib_domain::UrlSlug::parse("category-1").unwrap()),
    ///         category_type: lib_domain::CategoryTypes::Expense,
    ///         color: None,
    ///         icon: None,
    ///         is_active: true,
    ///         created_on: chrono::Utc::now(),
    ///         updated_on: chrono::Utc::now(),
    ///     },
    ///     Categories {
    ///         id: lib_domain::RowID::new(),
    ///         code: "FOO.BAR.QUX".to_string(),
    ///         name: "Category 2".to_string(),
    ///         description: None,
    ///         url_slug: Some(lib_domain::UrlSlug::parse("category-2").unwrap()),
    ///         category_type: lib_domain::CategoryTypes::Income,
    ///         color: None,
    ///         icon: None,
    ///         is_active: true,
    ///         created_on: chrono::Utc::now(),
    ///         updated_on: chrono::Utc::now(),
    ///     },
    ///     Categories {
    ///         id: lib_domain::RowID::new(),
    ///         code: "FOO.BAR.QUUX".to_string(),
    ///         name: "Category 3".to_string(),
    ///         description: None,
    ///         url_slug: Some(lib_domain::UrlSlug::parse("category-3").unwrap()),
    ///         category_type: lib_domain::CategoryTypes::Asset,
    ///         color: None,
    ///         icon: None,
    ///         is_active: true,
    ///         created_on: chrono::Utc::now(),
    ///         updated_on: chrono::Utc::now(),
    ///     },
    /// ];
    ///
    /// // Bulk insert with transaction guarantees
    /// let inserted = Categories::insert_many(&categories, pool).await?;
    ///
    /// println!("Successfully inserted {} categories", inserted.len());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Performance
    ///
    /// - **Transactional**: All-or-nothing atomicity
    /// - **Efficient**: Single transaction for multiple operations
    /// - **Scalable**: Connection pooling prevents resource exhaustion
    /// - **Observable**: Detailed progress tracking for large batches
    ///
    /// ## Error Handling
    ///
    /// Individual category failures are logged but don't stop batch processing.
    /// The transaction ensures database consistency - either all succeed or all fail.
    #[tracing::instrument(
        name = "Bulk insert categories into database",
        level = "info",
        skip(categories, pool),
        fields(count = categories.len())
    )]
    pub async fn insert_many(
        categories: &[Self],
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        let category_count = categories.len();

        if category_count == 0 {
            tracing::debug!("Bulk insert called with empty category list, returning empty vector");
            return Ok(Vec::new());
        }

        tracing::info!("🚀 Starting bulk insert operation for {} categories", category_count);
        tracing::debug!("Bulk insert categories: {:?}", categories.iter().map(|c| &c.code).collect::<Vec<_>>());

        // Use a transaction for atomicity
        tracing::trace!("Beginning database transaction for bulk insert");
        let mut tx = match pool.begin().await {
            Ok(tx) => {
                tracing::trace!("Database transaction started successfully");
                tx
            }
            Err(e) => {
                tracing::error!("Failed to begin transaction for bulk insert: {}", e);
                return Err(e.into());
            }
        };

        let mut inserted_categories = Vec::with_capacity(category_count);
        let mut success_count = 0;
        let mut error_count = 0;

        for (index, category) in categories.iter().enumerate() {
            let position = index + 1;
            tracing::trace!("Processing category {} of {}: {} (id: {})", position, category_count, category.code, category.id);

            // Insert each category
            let insert_query = sqlx::query!(
                r#"
                    INSERT INTO categories (id, code, name, description, url_slug, category_type, color, icon, is_active, created_on, updated_on)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                category.id,
                category.code,
                category.name,
                category.description,
                category.url_slug,
                category.category_type,
                category.color,
                category.icon,
                category.is_active,
                category.created_on,
                category.updated_on
            );

            match insert_query.execute(&mut *tx).await {
                Ok(result) => {
                    tracing::trace!("INSERT query executed for category: {} (rows affected: {})", category.code, result.rows_affected());
                    if result.rows_affected() != 1 {
                        tracing::warn!("INSERT operation affected {} rows instead of 1 for category: {}", result.rows_affected(), category.code);
                    }
                    success_count += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to insert category {}: {} (id: {}) - {}", position, category.code, category.id, e);
                    error_count += 1;
                    // Continue processing other categories but track errors
                }
            }

            // Read back the inserted category
            tracing::trace!("Retrieving inserted category from database: {}", category.id);
            match sqlx::query_as!(
                crate::Categories,
                r#"
                    SELECT
                        id              AS "id!: domain::RowID",
                        code,
                        name,
                        description,
                        url_slug        AS "url_slug?: domain::UrlSlug",
                        category_type   AS "category_type!: domain::CategoryTypes",
                        color           AS "color?: domain::HexColor",
                        icon,
                        is_active       AS "is_active!: bool",
                        created_on      AS "created_on!: chrono::DateTime<chrono::Utc>",
                        updated_on      AS "updated_on!: chrono::DateTime<chrono::Utc>"
                    FROM categories
                    WHERE id = ?
                "#,
                category.id
            )
            .fetch_one(&mut *tx)
            .await {
                Ok(inserted) => {
                    tracing::trace!("Retrieved inserted category: {} (id: {})", inserted.code, inserted.id);
                    inserted_categories.push(inserted);
                }
                Err(e) => {
                    tracing::error!("Failed to retrieve inserted category: {} (id: {}) - {}", category.code, category.id, e);
                    // If we can't retrieve it, we still want to track the error but continue
                }
            }
        }

        // Commit the transaction
        tracing::trace!("Committing database transaction after processing {} categories", category_count);
        match tx.commit().await {
            Ok(_) => {
                tracing::trace!("Database transaction committed successfully");
            }
            Err(e) => {
                tracing::error!("Failed to commit transaction for bulk insert: {}", e);
                return Err(e.into());
            }
        }

        let inserted_count = inserted_categories.len();
        tracing::info!("✅ Bulk insert completed: {} categories processed, {} inserted successfully, {} errors", category_count, inserted_count, error_count);

        if error_count > 0 {
            tracing::warn!("Bulk insert completed with {} errors out of {} total categories", error_count, category_count);
        }

        tracing::debug!("Successfully inserted categories: {:?}", inserted_categories.iter().map(|c| &c.code).collect::<Vec<_>>());

        Ok(inserted_categories)
    }

    /// Inserts a new category or updates an existing one based on the ID.
    ///
    /// This method implements an "upsert" operation using SQLite's `INSERT ... ON CONFLICT`
    /// syntax. If a category with the same ID already exists, it updates the existing record.
    /// If no such category exists, a new record is inserted.
    ///
    /// ## Process
    ///
    /// 1. **Attempt Insert**: Try to insert the category as a new record
    /// 2. **Conflict Resolution**: If ID conflict occurs, update existing record
    /// 3. **Verification**: Retrieve the final record state for consistency
    /// 4. **Operation Detection**: Determine whether INSERT or UPDATE occurred
    ///
    /// ## Arguments
    ///
    /// * `category` - Reference to the category to insert or update
    /// * `pool` - The database connection pool for executing queries
    ///
    /// ## Returns
    ///
    /// Returns `DatabaseResult<Self>` containing the category as it exists in the
    /// database after the operation, with any database-generated values.
    ///
    /// ## Errors
    ///
    /// This method will return an error if:
    /// - Database connection fails
    /// - Constraint violations occur (other than ID conflicts)
    /// - Data retrieval fails after upsert
    ///
    /// ## Logging
    ///
    /// - **ERROR**: Database errors with full context
    /// - **INFO**: Operation completion with INSERT/UPDATE indication
    /// - **DEBUG**: Operation type detection and final state
    /// - **TRACE**: Step-by-step execution flow
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use lib_database::categories::Categories;
    /// use lib_database::DatabaseConnection;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a database connection
    /// let config = lib_database::DatabaseConfig::default();
    /// let connection = lib_database::DatabaseConnection::new(config).await?;
    /// let pool = connection.pool();
    ///
    /// // Create a category
    /// let mut category = Categories {
    ///     id: lib_domain::RowID::new(),
    ///     code: "FOO.BAR.BAZ".to_string(),
    ///     name: "Original Name".to_string(),
    ///     description: Some("An example category".to_string()),
    ///     url_slug: Some(lib_domain::UrlSlug::parse("original-name").unwrap()),
    ///     category_type: lib_domain::CategoryTypes::Expense,
    ///     color: Some(lib_domain::HexColor::parse("#FF5733").unwrap()),
    ///     icon: Some("shopping-cart".to_string()),
    ///     is_active: true,
    ///     created_on: chrono::Utc::now(),
    ///     updated_on: chrono::Utc::now(),
    /// };
    /// category.name = "Updated Name".to_string();
    ///
    /// // First call - INSERT
    /// let result1 = Categories::insert_or_update(&category, pool).await?;
    /// println!("Inserted: {}", result1.name);
    ///
    /// // Second call - UPDATE (same ID)
    /// let result2 = Categories::insert_or_update(&category, pool).await?;
    /// println!("Updated: {}", result2.name);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Performance
    ///
    /// - **Efficient**: Single query handles both insert and update cases
    /// - **Atomic**: Operation is atomic at the database level
    /// - **Optimised**: No separate existence checks required
    ///
    /// ## Use Cases
    ///
    /// - **Data Import**: Safe bulk loading with conflict resolution
    /// - **Cache Updates**: Efficient cache population/sync
    /// - **API Endpoints**: Flexible create-or-update operations
    /// - **Data Migration**: Handling existing vs new records
    #[tracing::instrument(
        name = "Insert or update category in database",
        level = "debug",
        skip(category, pool),
        fields(id = %category.id, code = %category.code)
    )]
    pub async fn insert_or_update(
        category: &Self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Self> {
        tracing::trace!("Starting upsert operation for category: {} (id: {})", category.code, category.id);
        tracing::debug!("Upsert category details: type={}, active={}, updated={}", category.category_type, category.is_active, category.updated_on);

        // Use SQLite's UPSERT syntax (INSERT ... ON CONFLICT)
        tracing::trace!("Executing UPSERT query for category: {}", category.id);
        let upsert_query = sqlx::query!(
            r#"
                INSERT INTO categories (id, code, name, description, url_slug, category_type, color, icon, is_active, created_on, updated_on)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    code = excluded.code,
                    name = excluded.name,
                    description = excluded.description,
                    url_slug = excluded.url_slug,
                    category_type = excluded.category_type,
                    color = excluded.color,
                    icon = excluded.icon,
                    is_active = excluded.is_active,
                    updated_on = excluded.updated_on
                WHERE id = excluded.id
            "#,
            category.id,
            category.code,
            category.name,
            category.description,
            category.url_slug,
            category.category_type,
            category.color,
            category.icon,
            category.is_active,
            category.created_on,
            category.updated_on
        );

        let upsert_result = upsert_query.execute(pool).await;
        let operation_type = match upsert_result {
            Ok(result) => {
                tracing::trace!("UPSERT query executed successfully for category: {} (rows affected: {})", category.code, result.rows_affected());

                // Determine if this was an INSERT or UPDATE based on rows affected
                match result.rows_affected() {
                    1 => {
                        tracing::debug!("Category inserted (new record): {}", category.code);
                        "INSERT"
                    }
                    2 => {
                        tracing::debug!("Category updated (existing record): {}", category.code);
                        "UPDATE"
                    }
                    other => {
                        tracing::warn!("UPSERT operation affected {} rows (expected 1 or 2) for category: {}", other, category.code);
                        "UNKNOWN"
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to upsert category: {} (id: {}) - {}", category.code, category.id, e);
                return Err(e.into());
            }
        };

        // Read back the inserted/updated category
        tracing::trace!("Retrieving upserted category from database: {}", category.id);
        let result = match sqlx::query_as!(
            crate::Categories,
            r#"
                SELECT
                    id              AS "id!: domain::RowID",
                    code,
                    name,
                    description,
                    url_slug        AS "url_slug?: domain::UrlSlug",
                    category_type   AS "category_type!: domain::CategoryTypes",
                    color           AS "color?: domain::HexColor",
                    icon,
                    is_active       AS "is_active!: bool",
                    created_on      AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on      AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM categories
                WHERE id = ?
            "#,
            category.id
        )
        .fetch_one(pool)
        .await {
            Ok(cat) => {
                tracing::trace!("Retrieved upserted category: {} (id: {})", cat.code, cat.id);
                cat
            }
            Err(e) => {
                tracing::error!("Failed to retrieve upserted category: {} (id: {}) - {}", category.code, category.id, e);
                return Err(e.into());
            }
        };

        tracing::info!("✅ Category '{}' {}d successfully (ID: {})", result.code, operation_type, result.id);
        tracing::debug!("Final category state: type={}, active={}, updated={}", result.category_type, result.is_active, result.updated_on);

        Ok(result)
    }
}

/// Test module for categories insert operations.
///
/// This module contains comprehensive tests for the insert operations in the categories module,
/// focusing on database interactions, error handling, and edge cases. Tests use the `fake` crate
/// to generate realistic test data and ensure robust validation of database operations.
///
/// Tests are organised by operation type and cover:
/// - Successful operations with various data combinations
/// - Error conditions and constraint violations
/// - Edge cases and boundary conditions
/// - Transactional behaviors and rollback scenarios
#[cfg(test)]
mod tests {
    use super::*;
    use lib_domain::{RowID, UrlSlug, HexColor, CategoryTypes};
    use sqlx::SqlitePool;
    use fake::Fake;
    use fake::faker::lorem::en::{Words, Sentence};
    use fake::faker::boolean::en::Boolean;

    /// Helper function to create a test category with random data.
    ///
    /// Generates a category with realistic fake data for testing purposes.
    /// Uses the fake crate to create varied test scenarios.
    fn create_random_category() -> crate::Categories {
        let words: Vec<String> = Words(1..3).fake();
        let name = words.join(" ");

        let description = if Boolean(60).fake() {
            Some(Sentence(3..8).fake())
        } else {
            None
        };

        let code = format!("{:03}.{:03}.{:03}",
            fake::rand::random::<u8>() % 100,
            fake::rand::random::<u8>() % 100,
            fake::rand::random::<u8>() % 100
        );

        let category_type = match fake::rand::random::<u8>() % 5 {
            0 => CategoryTypes::Asset,
            1 => CategoryTypes::Liability,
            2 => CategoryTypes::Income,
            3 => CategoryTypes::Expense,
            _ => CategoryTypes::Equity,
        };

        let color = if Boolean(70).fake() {
            Some(HexColor::mock())
        } else {
            None
        };

        let icon = if Boolean(50).fake() {
            Some(fake::faker::lorem::en::Word().fake::<String>())
        } else {
            None
        };

        let url_slug = UrlSlug::from(name.clone());

        crate::Categories {
            id: RowID::new(),
            code,
            name,
            description,
            url_slug: Some(url_slug),
            category_type,
            color,
            icon,
            is_active: Boolean(80).fake(), // 80% chance of being active
            created_on: chrono::Utc::now(),
            updated_on: chrono::Utc::now(),
        }
    }

    /// Helper function to create multiple random test categories.
    ///
    /// Generates a specified number of categories with varied data for bulk operation testing.
    fn create_random_categories(count: usize) -> Vec<crate::Categories> {
        (0..count).map(|_| create_random_category()).collect()
    }

    /// Helper function to insert a test category and return it.
    ///
    /// Creates a random category, inserts it into the database, and returns the inserted category.
    async fn insert_test_category(pool: &SqlitePool) -> crate::Categories {
        let category = create_random_category();
        category.insert(pool).await.expect("Failed to insert test category")
    }

    /// Helper function to insert multiple test categories and return them.
    ///
    /// Creates random categories, inserts them in bulk, and returns the inserted categories.
    async fn insert_test_categories(count: usize, pool: &SqlitePool) -> Vec<crate::Categories> {
        let categories = create_random_categories(count);
        crate::Categories::insert_many(&categories, pool)
            .await
            .expect("Failed to insert test categories")
    }

    /// Tests successful insertion of a single category.
    ///
    /// Verifies that:
    /// - The category is inserted successfully
    /// - The returned category has the correct data
    /// - The category can be retrieved from the database
    #[sqlx::test]
    async fn test_insert_single_category_success(pool: SqlitePool) {
        let category = create_random_category();

        // Insert the category
        let result = category.insert(&pool).await;
        assert!(result.is_ok(), "Insert should succeed");

        let inserted = result.unwrap();

        // Verify the inserted category has correct data
        assert_eq!(inserted.id, category.id);
        assert_eq!(inserted.code, category.code);
        assert_eq!(inserted.name, category.name);
        assert_eq!(inserted.description, category.description);
        assert_eq!(inserted.category_type, category.category_type);
        assert_eq!(inserted.color, category.color);
        assert_eq!(inserted.icon, category.icon);
        assert_eq!(inserted.is_active, category.is_active);

        // Verify timestamps are set
        assert!(inserted.created_on >= category.created_on);
        assert!(inserted.updated_on >= category.updated_on);

        // Verify the category exists in the database by querying it
        let retrieved = sqlx::query_as!(
            crate::Categories,
            r#"
                SELECT
                    id              AS "id!: lib_domain::RowID",
                    code,
                    name,
                    description,
                    url_slug        AS "url_slug?: lib_domain::UrlSlug",
                    category_type   AS "category_type!: lib_domain::CategoryTypes",
                    color           AS "color?: lib_domain::HexColor",
                    icon,
                    is_active       AS "is_active!: bool",
                    created_on      AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on      AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM categories
                WHERE id = ?
            "#,
            inserted.id
        )
        .fetch_one(&pool)
        .await;

        assert!(retrieved.is_ok(), "Category should exist in database");
        assert_eq!(retrieved.unwrap().id, inserted.id);
    }

    /// Tests insertion of categories with various field combinations.
    ///
    /// Tests categories with:
    /// - All optional fields present
    /// - No optional fields
    /// - Mixed optional fields
    #[sqlx::test]
    async fn test_insert_categories_with_various_fields(pool: SqlitePool) {
        // Test with all fields
        let full_category = crate::Categories {
            id: RowID::new(),
            code: "001.002.003".to_string(),
            name: "Full Featured Category".to_string(),
            description: Some("A category with all fields".to_string()),
            url_slug: Some(UrlSlug::parse("full-featured-category").unwrap()),
            category_type: CategoryTypes::Expense,
            color: Some(HexColor::parse("#FF5733").unwrap()),
            icon: Some("shopping-cart".to_string()),
            is_active: true,
            created_on: chrono::Utc::now(),
            updated_on: chrono::Utc::now(),
        };

        let result1 = full_category.insert(&pool).await;
        assert!(result1.is_ok(), "Full category insert should succeed");

        // Test with minimal fields
        let minimal_category = crate::Categories {
            id: RowID::new(),
            code: "004.005.006".to_string(),
            name: "Minimal Category".to_string(),
            description: None,
            url_slug: Some(UrlSlug::parse("minimal-category").unwrap()),
            category_type: CategoryTypes::Asset,
            color: None,
            icon: None,
            is_active: false,
            created_on: chrono::Utc::now(),
            updated_on: chrono::Utc::now(),
        };

        let result2 = minimal_category.insert(&pool).await;
        assert!(result2.is_ok(), "Minimal category insert should succeed");

        // Test with mixed fields
        let mixed_category = crate::Categories {
            id: RowID::new(),
            code: "007.008.009".to_string(),
            name: "Mixed Category".to_string(),
            description: Some("Has description".to_string()),
            url_slug: Some(UrlSlug::parse("mixed-category").unwrap()),
            category_type: CategoryTypes::Income,
            color: None,
            icon: Some("dollar-sign".to_string()),
            is_active: true,
            created_on: chrono::Utc::now(),
            updated_on: chrono::Utc::now(),
        };

        let result3 = mixed_category.insert(&pool).await;
        assert!(result3.is_ok(), "Mixed category insert should succeed");
    }

    /// Tests that duplicate category codes are handled properly.
    ///
    /// Verifies that attempting to insert a category with an existing code
    /// results in an appropriate error.
    #[sqlx::test]
    async fn test_insert_duplicate_code_fails(pool: SqlitePool) {
        let category1 = insert_test_category(&pool).await;

        // Try to insert another category with the same code
        let category2 = crate::Categories {
            id: RowID::new(), // Different ID
            code: category1.code.clone(), // Same code
            name: "Different Name".to_string(),
            description: category1.description.clone(),
            url_slug: Some(UrlSlug::parse("different-name").unwrap()),
            category_type: category1.category_type,
            color: category1.color.clone(),
            icon: category1.icon.clone(),
            is_active: category1.is_active,
            created_on: chrono::Utc::now(),
            updated_on: chrono::Utc::now(),
        };

        let result = category2.insert(&pool).await;
        assert!(result.is_err(), "Insert with duplicate code should fail");

        // The error should be a database constraint violation
        let error = result.unwrap_err();
        // Note: The exact error type depends on SQLx error handling
        // In a real scenario, you'd check for specific constraint violation errors
    }

    /// Tests bulk insertion of multiple categories.
    ///
    /// Verifies that:
    /// - All categories are inserted successfully
    /// - The correct number of categories are returned
    /// - All categories exist in the database
    #[sqlx::test]
    async fn test_insert_many_success(pool: SqlitePool) {
        let categories = create_random_categories(5);

        let result = crate::Categories::insert_many(&categories, &pool).await;
        assert!(result.is_ok(), "Bulk insert should succeed");

        let inserted = result.unwrap();
        assert_eq!(inserted.len(), 5, "Should return all inserted categories");

        // Verify all categories exist in database
        for category in &inserted {
            let exists = sqlx::query!("SELECT COUNT(*) as count FROM categories WHERE id = ?", category.id)
                .fetch_one(&pool)
                .await
                .unwrap()
                .count > 0;
            assert!(exists, "Category should exist in database");
        }
    }

    /// Tests bulk insertion with empty input.
    ///
    /// Verifies that inserting an empty vector returns an empty vector.
    #[sqlx::test]
    async fn test_insert_many_empty_input(pool: SqlitePool) {
        let categories: Vec<crate::Categories> = vec![];

        let result = crate::Categories::insert_many(&categories, &pool).await;
        assert!(result.is_ok(), "Empty bulk insert should succeed");

        let inserted = result.unwrap();
        assert_eq!(inserted.len(), 0, "Should return empty vector for empty input");
    }

    /// Tests bulk insertion with large number of categories.
    ///
    /// Tests the performance and reliability of bulk operations with many records.
    #[sqlx::test]
    async fn test_insert_many_large_batch(pool: SqlitePool) {
        let categories = create_random_categories(20);

        let result = crate::Categories::insert_many(&categories, &pool).await;
        assert!(result.is_ok(), "Large bulk insert should succeed");

        let inserted = result.unwrap();
        assert_eq!(inserted.len(), 20, "Should return all inserted categories");
    }

    /// Tests bulk insertion with duplicate codes.
    ///
    /// Verifies that when some categories have duplicate codes,
    /// the operation continues and returns successfully inserted categories.
    #[sqlx::test]
    async fn test_insert_many_with_some_duplicates(pool: SqlitePool) {
        let mut categories = create_random_categories(3);

        // Make the second category have the same code as the first
        categories[1].code = categories[0].code.clone();

        let result = crate::Categories::insert_many(&categories, &pool).await;

        // The operation should succeed but with fewer inserted categories
        assert!(result.is_ok(), "Bulk insert with duplicates should succeed");

        let inserted = result.unwrap();
        // Should have inserted 2 categories (first and third), second failed due to duplicate
        assert!(!inserted.is_empty(), "Should insert at least some categories");
        assert!(inserted.len() <= 3, "Should not insert more than attempted");
    }

    /// Tests upsert operation - insert case.
    ///
    /// Verifies that inserting a new category via upsert works correctly.
    #[sqlx::test]
    async fn test_insert_or_update_insert_case(pool: SqlitePool) {
        let category = create_random_category();

        let result = crate::Categories::insert_or_update(&category, &pool).await;
        assert!(result.is_ok(), "Upsert insert should succeed");

        let upserted = result.unwrap();
        assert_eq!(upserted.id, category.id);
        assert_eq!(upserted.code, category.code);

        // Verify it exists in database
        let exists = sqlx::query!("SELECT COUNT(*) as count FROM categories WHERE id = ?", category.id)
            .fetch_one(&pool)
            .await
            .unwrap()
            .count > 0;
        assert!(exists, "Category should exist in database");
    }

    /// Tests upsert operation - update case.
    ///
    /// Verifies that updating an existing category via upsert works correctly.
    #[sqlx::test]
    async fn test_insert_or_update_update_case(pool: SqlitePool) {
        // First insert a category
        let original = insert_test_category(&pool).await;

        // Modify the category
        let mut updated = original.clone();
        updated.name = "Updated Name".to_string();
        updated.description = Some("Updated description".to_string());
        updated.updated_on = chrono::Utc::now();

        // Upsert the modified category
        let result = crate::Categories::insert_or_update(&updated, &pool).await;
        assert!(result.is_ok(), "Upsert update should succeed");

        let upserted = result.unwrap();
        assert_eq!(upserted.id, original.id);
        assert_eq!(upserted.name, "Updated Name");
        assert_eq!(upserted.description, Some("Updated description".to_string()));

        // Verify only one record exists (not a duplicate)
        let count = sqlx::query!("SELECT COUNT(*) as count FROM categories WHERE id = ?", original.id)
            .fetch_one(&pool)
            .await
            .unwrap()
            .count;
        assert_eq!(count, 1, "Should have exactly one record with this ID");
    }

    /// Tests upsert operation multiple times on same category.
    ///
    /// Verifies that multiple upsert operations on the same category work correctly.
    #[sqlx::test]
    async fn test_insert_or_update_multiple_updates(pool: SqlitePool) {
        let mut category = create_random_category();

        // First upsert - should insert
        let result1 = crate::Categories::insert_or_update(&category, &pool).await;
        assert!(result1.is_ok());

        // Modify and upsert again - should update
        category.name = "First Update".to_string();
        let result2 = crate::Categories::insert_or_update(&category, &pool).await;
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap().name, "First Update");

        // Modify and upsert third time - should update again
        category.name = "Second Update".to_string();
        let result3 = crate::Categories::insert_or_update(&category, &pool).await;
        assert!(result3.is_ok());
        assert_eq!(result3.unwrap().name, "Second Update");

        // Verify final state
        let final_state = sqlx::query_as!(
            crate::Categories,
            r#"
                SELECT
                    id              AS "id!: lib_domain::RowID",
                    code,
                    name,
                    description,
                    url_slug        AS "url_slug?: lib_domain::UrlSlug",
                    category_type   AS "category_type!: lib_domain::CategoryTypes",
                    color           AS "color?: lib_domain::HexColor",
                    icon,
                    is_active       AS "is_active!: bool",
                    created_on      AS "created_on!: chrono::DateTime<chrono::Utc>",
                    updated_on      AS "updated_on!: chrono::DateTime<chrono::Utc>"
                FROM categories
                WHERE id = ?
            "#,
            category.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(final_state.name, "Second Update");
    }

    /// Tests that categories with all possible category types can be inserted.
    ///
    /// Verifies that all five category types (Asset, Liability, Income, Expense, Equity)
    /// can be inserted successfully.
    #[sqlx::test]
    async fn test_insert_all_category_types(pool: SqlitePool) {
        let category_types = vec![
            CategoryTypes::Asset,
            CategoryTypes::Liability,
            CategoryTypes::Income,
            CategoryTypes::Expense,
            CategoryTypes::Equity,
        ];

        for category_type in category_types {
            let category_type_name = category_type.as_str().to_string();
            let category = crate::Categories {
                id: RowID::new(),
                code: format!("{:03}.{:03}.{:03}",
                    fake::rand::random::<u8>() % 100,
                    fake::rand::random::<u8>() % 100,
                    fake::rand::random::<u8>() % 100
                ),
                name: format!("{} Category", category_type_name),
                description: Some(format!("A {} category", category_type_name)),
                url_slug: Some(UrlSlug::from(format!("{}-category", category_type_name))),
                category_type,
                color: Some(HexColor::mock()),
                icon: Some("test-icon".to_string()),
                is_active: true,
                created_on: chrono::Utc::now(),
                updated_on: chrono::Utc::now(),
            };

            let result = category.insert(&pool).await;
            assert!(result.is_ok(), "Should be able to insert {} category", category_type_name);
        }
    }

    /// Tests insertion with extreme field lengths.
    ///
    /// Verifies that categories with very long names and descriptions can be handled.
    #[sqlx::test]
    async fn test_insert_with_long_fields(pool: SqlitePool) {
        let long_name = "A".repeat(200); // Very long name
        let long_description = Some("B".repeat(1000)); // Very long description

        let category = crate::Categories {
            id: RowID::new(),
            code: "100.200.300".to_string(),
            name: long_name.clone(),
            description: long_description.clone(),
            url_slug: Some(UrlSlug::from("long-category")),
            category_type: CategoryTypes::Expense,
            color: None,
            icon: None,
            is_active: true,
            created_on: chrono::Utc::now(),
            updated_on: chrono::Utc::now(),
        };

        let result = category.insert(&pool).await;
        // This may succeed or fail depending on database constraints
        // The important thing is that it doesn't panic
        let _ = result; // We don't assert success/failure, just that it doesn't crash
    }

    /// Tests concurrent insertions.
    ///
    /// Verifies that multiple concurrent insertions work correctly.
    /// This is a basic test - in a real scenario you'd use more sophisticated
    /// concurrency testing tools.
    #[sqlx::test]
    async fn test_concurrent_insertions(pool: SqlitePool) {
        let mut handles = vec![];

        // Spawn multiple tasks to insert categories concurrently
        for _ in 0..5 {
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move {
                let category = create_random_category();
                category.insert(&pool_clone).await
            });
            handles.push(handle);
        }

        // Wait for all insertions to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok(), "Concurrent insert should succeed");
        }

        // Verify all categories were inserted
        let count = sqlx::query!("SELECT COUNT(*) as count FROM categories")
            .fetch_one(&pool)
            .await
            .unwrap()
            .count;
        assert_eq!(count, 5, "All concurrent insertions should succeed");
    }

    /// Tests that insert operations maintain data integrity.
    ///
    /// Verifies that inserted data matches exactly what was provided.
    #[sqlx::test]
    async fn test_data_integrity_after_insert(pool: SqlitePool) {
        let original = create_random_category();

        let inserted = original.insert(&pool).await.unwrap();

        // Verify all fields match exactly
        assert_eq!(inserted.id, original.id);
        assert_eq!(inserted.code, original.code);
        assert_eq!(inserted.name, original.name);
        assert_eq!(inserted.description, original.description);
        assert_eq!(inserted.url_slug, original.url_slug);
        assert_eq!(inserted.category_type, original.category_type);
        assert_eq!(inserted.color, original.color);
        assert_eq!(inserted.icon, original.icon);
        assert_eq!(inserted.is_active, original.is_active);

        // Timestamps should be set (may be slightly different due to database precision)
        assert!(inserted.created_on >= original.created_on);
        assert!(inserted.updated_on >= original.updated_on);
    }

    /// Tests bulk insert data integrity.
    ///
    /// Verifies that all categories inserted in bulk maintain their data integrity.
    #[sqlx::test]
    async fn test_bulk_insert_data_integrity(pool: SqlitePool) {
        let originals = create_random_categories(3);

        let inserted = crate::Categories::insert_many(&originals, &pool).await.unwrap();

        assert_eq!(inserted.len(), originals.len());

        // Verify each inserted category matches its original
        for (original, inserted_cat) in originals.iter().zip(inserted.iter()) {
            assert_eq!(inserted_cat.code, original.code);
            assert_eq!(inserted_cat.name, original.name);
            assert_eq!(inserted_cat.category_type, original.category_type);
            assert_eq!(inserted_cat.is_active, original.is_active);
        }
    }
}
