//! Category query operations for the Personal Ledger database.
//!
//! This module provides functions for querying category records from the SQLite database.
//! It supports finding single records, bulk retrievals, filtered searches, and paginated results.
//!
//! All query operations are read-only and ensure data integrity by using explicit column selection.
//!
//! The module follows these key principles:
//! - **Efficiency**: Explicit column selection and indexed queries where possible
//! - **Flexibility**: Support for filtering, sorting, and pagination
//! - **Observability**: Detailed tracing from TRACE to INFO levels
//! - **Safety**: No sensitive data exposure; proper error handling

use lib_domain as domain;

impl crate::Categories {
    /// Finds a category by its unique ID.
    ///
    /// # Arguments
    /// * `id` - The unique identifier of the category.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Option<Self>>` containing the category if found, or `None` if not found.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    pub async fn find_by_id(
        id: domain::RowID,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Option<Self>> {
        let category = sqlx::query_as!(
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
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(category)
    }

    /// Finds a category by its unique code.
    ///
    /// # Arguments
    /// * `code` - The unique code of the category (e.g., "FOO.BAR.BAZ").
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Option<Self>>` containing the category if found, or `None` if not found.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    pub async fn find_by_code(
        code: &str,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Option<Self>> {
        let category = sqlx::query_as!(
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
                WHERE code = ?
            "#,
            code
        )
        .fetch_optional(pool)
        .await?;

        Ok(category)
    }

    /// Finds a category by its URL slug.
    ///
    /// # Arguments
    /// * `slug` - The URL slug of the category.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Option<Self>>` containing the category if found, or `None` if not found.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    pub async fn find_by_url_slug(
        slug: &domain::UrlSlug,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Option<Self>> {
        let category = sqlx::query_as!(
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
                WHERE url_slug = ?
            "#,
            slug
        )
        .fetch_optional(pool)
        .await?;

        Ok(category)
    }

    /// Finds categories by name (case-insensitive partial match).
    ///
    /// Since category names may not be unique, this returns a vector of matching categories.
    ///
    /// # Arguments
    /// * `name` - The name to search for (partial match).
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Vec<Self>>` containing all matching categories.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, ERROR on database failures.
    #[tracing::instrument(
        name = "Find categories by name",
        level = "debug",
        skip(pool),
        fields(
            search_name = %name,
            operation = "find_by_name"
        ),
        err
    )]
    pub async fn find_by_name(
        name: &str,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        tracing::trace!(
            search_name = %name,
            "Starting find categories by name operation"
        );

        let name_pattern = format!("%{}%", name);

        tracing::debug!(
            search_name = %name,
            "Executing query to find categories by name"
        );

        let categories = sqlx::query_as!(
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
                WHERE name LIKE ?
                ORDER BY created_on DESC
            "#,
            name_pattern
        )
        .fetch_all(pool)
        .await?;

        tracing::info!(
            search_name = %name,
            category_count = %categories.len(),
            "Found categories by name"
        );

        Ok(categories)
    }

    /// Finds all categories.
    ///
    /// # Arguments
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Vec<Self>>` containing all categories.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs INFO with the number of categories retrieved.
    pub async fn find_all(
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        let categories = sqlx::query_as!(
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
                ORDER BY created_on DESC
            "#
        )
        .fetch_all(pool)
        .await?;

        tracing::info!("Retrieved {} categories from database", categories.len());

        Ok(categories)
    }

    /// Finds all active categories.
    ///
    /// # Arguments
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Vec<Self>>` containing all active categories.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs INFO with the number of categories retrieved.
    pub async fn find_all_active(
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        let categories = sqlx::query_as!(
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
                WHERE is_active = true
                ORDER BY created_on DESC
            "#
        )
        .fetch_all(pool)
        .await?;

        tracing::info!("Retrieved {} active categories from database", categories.len());

        Ok(categories)
    }

    /// Finds all inactive categories.
    ///
    /// # Arguments
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Vec<Self>>` containing all inactive categories.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, ERROR on database failures.
    #[tracing::instrument(
        name = "Find inactive categories",
        level = "debug",
        skip(pool),
        fields(operation = "find_inactive"),
        err
    )]
    pub async fn find_inactive(
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        tracing::trace!("Starting find inactive categories operation");

        tracing::debug!("Executing query to find inactive categories");

        let categories = sqlx::query_as!(
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
                WHERE is_active = false
                ORDER BY created_on DESC
            "#
        )
        .fetch_all(pool)
        .await?;

        tracing::info!(
            category_count = %categories.len(),
            "Found inactive categories"
        );

        Ok(categories)
    }

    /// Finds categories by type.
    ///
    /// # Arguments
    /// * `category_type` - The category type to filter by.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Vec<Self>>` containing all categories of the specified type.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs INFO with the number of categories retrieved.
    pub async fn find_by_type(
        category_type: domain::CategoryTypes,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        let categories = sqlx::query_as!(
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
                WHERE category_type = ?
                ORDER BY created_on DESC
            "#,
            category_type
        )
        .fetch_all(pool)
        .await?;

        tracing::info!("Retrieved {} categories of type {} from database", categories.len(), category_type);

        Ok(categories)
    }

    /// Finds active categories by type.
    ///
    /// # Arguments
    /// * `category_type` - The category type to filter by.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<Vec<Self>>` containing all active categories of the specified type.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs INFO with the number of categories retrieved.
    pub async fn find_active_by_type(
        category_type: domain::CategoryTypes,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<Vec<Self>> {
        let categories = sqlx::query_as!(
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
                WHERE category_type = ? AND is_active = true
                ORDER BY created_on DESC
            "#,
            category_type
        )
        .fetch_all(pool)
        .await?;

        tracing::info!("Retrieved {} active categories of type {} from database", categories.len(), category_type);

        Ok(categories)
    }

    /// Finds all categories with pagination.
    ///
    /// # Arguments
    /// * `offset` - The number of records to skip.
    /// * `limit` - The maximum number of records to return.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<(Vec<Self>, i32)>` containing the categories and total count.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, ERROR on database failures.
    #[tracing::instrument(
        name = "Find all categories with pagination",
        level = "debug",
        skip(pool),
        fields(
            offset = %offset,
            limit = %limit,
            operation = "find_all_with_pagination"
        ),
        err
    )]
    pub async fn find_all_with_pagination(
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        tracing::trace!(
            offset = %offset,
            limit = %limit,
            "Starting find all categories with pagination operation"
        );

        tracing::debug!(
            offset = %offset,
            limit = %limit,
            "Executing paginated query for all categories"
        );

        let (categories, total_count) = Self::find_all_with_pagination_internal(offset, limit, pool).await?;

        tracing::info!(
            offset = %offset,
            limit = %limit,
            category_count = %categories.len(),
            total_count = %total_count,
            "Found all categories with pagination"
        );

        Ok((categories, total_count))
    }

    /// Finds all active categories with pagination.
    ///
    /// # Arguments
    /// * `offset` - The number of records to skip.
    /// * `limit` - The maximum number of records to return.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<(Vec<Self>, i32)>` containing the categories and total count.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, ERROR on database failures.
    #[tracing::instrument(
        name = "Find active categories with pagination",
        level = "debug",
        skip(pool),
        fields(
            offset = %offset,
            limit = %limit,
            operation = "find_active_with_pagination"
        ),
        err
    )]
    pub async fn find_active_with_pagination(
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        tracing::trace!(
            offset = %offset,
            limit = %limit,
            "Starting find active categories with pagination operation"
        );

        tracing::debug!(
            offset = %offset,
            limit = %limit,
            "Executing paginated query for active categories"
        );

        let (categories, total_count) = Self::find_all_active_with_pagination_internal(offset, limit, pool).await?;

        tracing::info!(
            offset = %offset,
            limit = %limit,
            category_count = %categories.len(),
            total_count = %total_count,
            "Found active categories with pagination"
        );

        Ok((categories, total_count))
    }

    /// Finds all inactive categories with pagination.
    ///
    /// # Arguments
    /// * `offset` - The number of records to skip.
    /// * `limit` - The maximum number of records to return.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<(Vec<Self>, i32)>` containing the categories and total count.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, ERROR on database failures.
    #[tracing::instrument(
        name = "Find inactive categories with pagination",
        level = "debug",
        skip(pool),
        fields(
            offset = %offset,
            limit = %limit,
            operation = "find_inactive_with_pagination"
        ),
        err
    )]
    pub async fn find_inactive_with_pagination(
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        tracing::trace!(
            offset = %offset,
            limit = %limit,
            "Starting find inactive categories with pagination operation"
        );

        tracing::debug!(
            offset = %offset,
            limit = %limit,
            "Executing paginated query for inactive categories"
        );

        let (categories, total_count) = Self::find_all_inactive_with_pagination_internal(offset, limit, pool).await?;

        tracing::info!(
            offset = %offset,
            limit = %limit,
            category_count = %categories.len(),
            total_count = %total_count,
            "Found inactive categories with pagination"
        );

        Ok((categories, total_count))
    }

    /// Finds categories by type with pagination.
    ///
    /// # Arguments
    /// * `category_type` - The category type to filter by.
    /// * `offset` - The number of records to skip.
    /// * `limit` - The maximum number of records to return.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<(Vec<Self>, i32)>` containing the categories and total count.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, ERROR on database failures.
    #[tracing::instrument(
        name = "Find categories by type with pagination",
        level = "debug",
        skip(pool),
        fields(
            category_type = %category_type.as_str(),
            offset = %offset,
            limit = %limit,
            operation = "find_by_type_with_pagination"
        ),
        err
    )]
    pub async fn find_by_type_with_pagination(
        category_type: domain::CategoryTypes,
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        let category_type_str = category_type.as_str();

        tracing::trace!(
            category_type = %category_type_str,
            offset = %offset,
            limit = %limit,
            "Starting find categories by type with pagination operation"
        );

        tracing::debug!(
            category_type = %category_type_str,
            offset = %offset,
            limit = %limit,
            "Executing paginated query for categories by type"
        );

        let (categories, total_count) = Self::find_by_type_with_pagination_internal(category_type, offset, limit, pool).await?;

        tracing::info!(
            category_type = %category_type_str,
            offset = %offset,
            limit = %limit,
            category_count = %categories.len(),
            total_count = %total_count,
            "Found categories by type with pagination"
        );

        Ok((categories, total_count))
    }

    /// Finds active categories by type with pagination.
    ///
    /// # Arguments
    /// * `category_type` - The category type to filter by.
    /// * `offset` - The number of records to skip.
    /// * `limit` - The maximum number of records to return.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<(Vec<Self>, i32)>` containing the categories and total count.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs TRACE for operation start, DEBUG for query execution, INFO on success, ERROR on database failures.
    #[tracing::instrument(
        name = "Find active categories by type with pagination",
        level = "debug",
        skip(pool),
        fields(
            category_type = %category_type.as_str(),
            offset = %offset,
            limit = %limit,
            operation = "find_active_by_type_with_pagination"
        ),
        err
    )]
    pub async fn find_active_by_type_with_pagination(
        category_type: domain::CategoryTypes,
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        let category_type_str = category_type.as_str();

        tracing::trace!(
            category_type = %category_type_str,
            offset = %offset,
            limit = %limit,
            "Starting find active categories by type with pagination operation"
        );

        tracing::debug!(
            category_type = %category_type_str,
            offset = %offset,
            limit = %limit,
            "Executing paginated query for active categories by type"
        );

        let (categories, total_count) = Self::find_active_by_type_with_pagination_internal(category_type, offset, limit, pool).await?;

        tracing::info!(
            category_type = %category_type_str,
            offset = %offset,
            limit = %limit,
            category_count = %categories.len(),
            total_count = %total_count,
            "Found active categories by type with pagination"
        );

        Ok((categories, total_count))
    }

    /// Finds categories with advanced filters and pagination.
    ///
    /// # Arguments
    /// * `category_type_filter` - Optional filter by category type.
    /// * `is_active_filter` - Optional filter by active status.
    /// * `sort_by` - Optional sort field (not implemented yet).
    /// * `sort_desc` - Optional sort direction (not implemented yet).
    /// * `offset` - The number of records to skip.
    /// * `limit` - The maximum number of records to return.
    /// * `pool` - A reference to the SQLite database connection pool.
    ///
    /// # Returns
    /// Returns a `DatabaseResult<(Vec<Self>, i32)>` containing the categories and total count.
    ///
    /// # Errors
    /// This function will return an error if a database connection or query execution error occurs.
    ///
    /// # Tracing
    /// Logs INFO with the number of categories retrieved.
    pub async fn find_with_filters(
        category_type_filter: Option<domain::CategoryTypes>,
        is_active_filter: Option<bool>,
        sort_by: Option<&str>,
        sort_desc: Option<bool>,
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        // For now, implement a simpler version that handles the most common cases
        // TODO: Implement full dynamic filtering when needed

        let (categories, total_count) = match (category_type_filter, is_active_filter) {
            (Some(category_type), Some(_is_active)) => {
                Self::find_active_by_type_with_pagination(category_type, offset, limit, pool).await?
            }
            (Some(category_type), None) => {
                Self::find_by_type_with_pagination(category_type, offset, limit, pool).await?
            }
            (None, Some(is_active)) => {
                if is_active {
                    Self::find_active_with_pagination(offset, limit, pool).await?
                } else {
                    Self::find_inactive_with_pagination(offset, limit, pool).await?
                }
            }
            (None, None) => {
                Self::find_all_with_pagination(offset, limit, pool).await?
            }
        };

        Ok((categories, total_count))
    }

    /// Helper method to find all categories with pagination
    async fn find_all_with_pagination_internal(
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        let total_count: i32 = sqlx::query_scalar("SELECT COUNT(*) as count FROM categories")
            .fetch_one(pool)
            .await?;

        let categories = sqlx::query_as!(
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
                ORDER BY created_on DESC
                LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        Ok((categories, total_count))
    }

    /// Helper method to find all active categories with pagination
    async fn find_all_active_with_pagination_internal(
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        let total_count: i32 = sqlx::query_scalar("SELECT COUNT(*) as count FROM categories WHERE is_active = true")
            .fetch_one(pool)
            .await?;

        let categories = sqlx::query_as!(
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
                WHERE is_active = true
                ORDER BY created_on DESC
                LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        Ok((categories, total_count))
    }

    /// Helper method to find all inactive categories with pagination
    async fn find_all_inactive_with_pagination_internal(
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        let total_count: i32 = sqlx::query_scalar("SELECT COUNT(*) as count FROM categories WHERE is_active = false")
            .fetch_one(pool)
            .await?;

        let categories = sqlx::query_as!(
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
                WHERE is_active = false
                ORDER BY created_on DESC
                LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        Ok((categories, total_count))
    }

    /// Helper method to find categories by type with pagination
    async fn find_by_type_with_pagination_internal(
        category_type: domain::CategoryTypes,
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        let total_count: i32 = sqlx::query_scalar("SELECT COUNT(*) as count FROM categories WHERE category_type = ?")
            .bind(&category_type)
            .fetch_one(pool)
            .await?;

        let categories = sqlx::query_as!(
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
                WHERE category_type = ?
                ORDER BY created_on DESC
                LIMIT ? OFFSET ?
            "#,
            category_type,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        Ok((categories, total_count))
    }

    /// Helper method to find active categories by type with pagination
    async fn find_active_by_type_with_pagination_internal(
        category_type: domain::CategoryTypes,
        offset: i32,
        limit: i32,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> crate::DatabaseResult<(Vec<Self>, i32)> {
        let total_count: i32 = sqlx::query_scalar("SELECT COUNT(*) as count FROM categories WHERE category_type = ? AND is_active = true")
            .bind(&category_type)
            .fetch_one(pool)
            .await?;

        let categories = sqlx::query_as!(
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
                WHERE category_type = ? AND is_active = true
                ORDER BY created_on DESC
                LIMIT ? OFFSET ?
            "#,
            category_type,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        Ok((categories, total_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use fake::Fake;

    /// Helper function to insert a test category
    async fn insert_test_category(pool: &SqlitePool, category: &crate::Categories) -> domain::RowID {
        let id_str = category.id.to_string();
        let url_slug_str = category.url_slug.as_ref().map(|s| s.to_string());
        let category_type_str = category.category_type.as_str();
        let color_str = category.color.as_ref().map(|c| c.to_string());
        let created_on_str = category.created_on.to_rfc3339();
        let updated_on_str = category.updated_on.to_rfc3339();

        sqlx::query!(
            r#"
            INSERT INTO categories (
                id, code, name, description, url_slug, category_type,
                color, icon, is_active, created_on, updated_on
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id_str,
            category.code,
            category.name,
            category.description,
            url_slug_str,
            category_type_str,
            color_str,
            category.icon,
            category.is_active,
            created_on_str,
            updated_on_str
        )
        .execute(pool)
        .await
        .unwrap();

        category.id
    }

    mod basic_finds {
        use super::*;

        #[sqlx::test]
        async fn test_find_by_id(pool: SqlitePool) {
            let category = crate::Categories::mock();
            insert_test_category(&pool, &category).await;

            let result = crate::Categories::find_by_id(category.id, &pool).await;
            assert!(result.is_ok());
            let found = result.unwrap();
            assert!(found.is_some());
            assert_eq!(found.unwrap().id, category.id);
        }

        #[sqlx::test]
        async fn test_find_by_code(pool: SqlitePool) {
            let category = crate::Categories::mock();
            insert_test_category(&pool, &category).await;

            let result = crate::Categories::find_by_code(&category.code, &pool).await;
            assert!(result.is_ok());
            let found = result.unwrap();
            assert!(found.is_some());
            assert_eq!(found.unwrap().code, category.code);
        }

        #[sqlx::test]
        async fn test_find_by_url_slug(pool: SqlitePool) {
            let mut category = crate::Categories::mock();
            category.url_slug = Some(domain::UrlSlug::from("test-slug"));
            insert_test_category(&pool, &category).await;

            let result = crate::Categories::find_by_url_slug(category.url_slug.as_ref().unwrap(), &pool).await;
            assert!(result.is_ok());
            let found = result.unwrap();
            assert!(found.is_some());
            assert_eq!(found.unwrap().url_slug, category.url_slug);
        }

        #[sqlx::test]
        async fn test_find_by_name(pool: SqlitePool) {
            let category = crate::Categories::mock();
            insert_test_category(&pool, &category).await;

            let result = crate::Categories::find_by_name(&category.name, &pool).await;
            assert!(result.is_ok());
            let found = result.unwrap();
            assert!(!found.is_empty());
            assert!(found.iter().any(|c| c.id == category.id));
        }

        #[sqlx::test]
        async fn test_find_all(pool: SqlitePool) {
            let category1 = crate::Categories::mock();
            let category2 = crate::Categories::mock();
            insert_test_category(&pool, &category1).await;
            insert_test_category(&pool, &category2).await;

            let result = crate::Categories::find_all(&pool).await;
            assert!(result.is_ok());
            let categories = result.unwrap();
            assert!(categories.len() >= 2);
        }

        #[sqlx::test]
        async fn test_find_all_active(pool: SqlitePool) {
            let mut active_category = crate::Categories::mock();
            active_category.is_active = true;
            let mut inactive_category = crate::Categories::mock();
            inactive_category.is_active = false;
            insert_test_category(&pool, &active_category).await;
            insert_test_category(&pool, &inactive_category).await;

            let result = crate::Categories::find_all_active(&pool).await;
            assert!(result.is_ok());
            let categories = result.unwrap();
            assert!(categories.iter().all(|c| c.is_active));
        }

        #[sqlx::test]
        async fn test_find_inactive(pool: SqlitePool) {
            let mut active_category = crate::Categories::mock();
            active_category.is_active = true;
            let mut inactive_category = crate::Categories::mock();
            inactive_category.is_active = false;
            insert_test_category(&pool, &active_category).await;
            insert_test_category(&pool, &inactive_category).await;

            let result = crate::Categories::find_inactive(&pool).await;
            assert!(result.is_ok());
            let categories = result.unwrap();
            assert!(categories.iter().all(|c| !c.is_active));
        }
    }

    mod pagination {
        use super::*;

        #[sqlx::test]
        async fn test_find_all_with_pagination(pool: SqlitePool) {
            // Insert multiple categories
            for _ in 0..5 {
                let category = crate::Categories::mock();
                insert_test_category(&pool, &category).await;
            }

            let (categories, total_count) = crate::Categories::find_all_with_pagination(0, 2, &pool).await.unwrap();
            assert_eq!(categories.len(), 2);
            assert!(total_count >= 5);
        }

        #[sqlx::test]
        async fn test_find_active_with_pagination(pool: SqlitePool) {
            // Insert mix of active and inactive
            for i in 0..5 {
                let mut category = crate::Categories::mock();
                category.is_active = i % 2 == 0;
                insert_test_category(&pool, &category).await;
            }

            let (categories, total_count) = crate::Categories::find_active_with_pagination(0, 2, &pool).await.unwrap();
            assert!(categories.len() <= 2);
            assert!(categories.iter().all(|c| c.is_active));
            assert!(total_count >= 3); // At least 3 active categories
        }

        #[sqlx::test]
        async fn test_find_inactive_with_pagination(pool: SqlitePool) {
            // Insert mix of active and inactive
            for i in 0..5 {
                let mut category = crate::Categories::mock();
                category.is_active = i % 2 == 0;
                insert_test_category(&pool, &category).await;
            }

            let (categories, total_count) = crate::Categories::find_inactive_with_pagination(0, 2, &pool).await.unwrap();
            assert!(categories.len() <= 2);
            assert!(categories.iter().all(|c| !c.is_active));
            assert!(total_count >= 2); // At least 2 inactive categories
        }
    }
}
