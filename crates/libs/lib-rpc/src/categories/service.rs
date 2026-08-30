use crate::categories::proto;
use crate::error::RpcError; // Needed for error type conversion into tonic::Status
use lib_database as database;

pub struct CategoriesService {
    database_pool: std::sync::Arc<sqlx::SqlitePool>,
}

impl From<database::Categories> for proto::Category {
    fn from(category: database::Categories) -> Self {
        use prost_types::Timestamp;

        Self {
            id: category.id.to_string(),
            code: category.code,
            name: category.name,
            description: category.description,
            url_slug: category.url_slug.map(|s| s.to_string()),
            category_type: category.category_type as i32,
            color: category.color.map(|c| c.to_string()),
            icon: category.icon,
            is_active: category.is_active,
            created_on: Some(Timestamp {
                seconds: category.created_on.timestamp(),
                nanos: category.created_on.timestamp_subsec_nanos() as i32,
            }),
            updated_on: Some(Timestamp {
                seconds: category.updated_on.timestamp(),
                nanos: category.updated_on.timestamp_subsec_nanos() as i32,
            }),
        }
    }
}

impl CategoriesService {
    pub fn new(database_pool: std::sync::Arc<sqlx::SqlitePool>) -> Self {
        Self { database_pool }
    }

    pub fn database_ref(&self) -> &sqlx::SqlitePool {
        &self.database_pool
    }
}

#[tonic::async_trait]
impl proto::CategoriesService for CategoriesService {
    async fn category_activate(
        &self,
        request: tonic::Request<proto::CategoryActivateRequest>,
    ) -> Result<tonic::Response<proto::CategoryActivateResponse>, tonic::Status> {
        // The Ok wrapper is necessary here to convert the RpcResult into a tonic::Status
        Ok(crate::categories::activate::activate_category(self, request).await?)
    }

    async fn category_create(
        &self,
        request: tonic::Request<proto::CategoryCreateRequest>,
    ) -> Result<tonic::Response<proto::CategoryCreateResponse>, tonic::Status> {
        // Ok(crate::categories::)
        unimplemented!();
    }

    async fn categories_create_batch(
        &self,
        request: tonic::Request<proto::CategoriesCreateBatchRequest>,
    ) -> Result<tonic::Response<proto::CategoriesCreateBatchResponse>, tonic::Status> {
        unimplemented!();
    }

    async fn category_deactivate(
        &self,
        request: tonic::Request<proto::CategoryDeactivateRequest>,
    ) -> Result<tonic::Response<proto::CategoryDeactivateResponse>, tonic::Status> {
        unimplemented!();
    }

    async fn category_delete(
        &self,
        request: tonic::Request<proto::CategoryDeleteRequest>,
    ) -> Result<tonic::Response<proto::CategoryDeleteResponse>, tonic::Status> {
        unimplemented!();
    }

    async fn categories_delete_batch(
        &self,
        request: tonic::Request<proto::CategoriesDeleteBatchRequest>,
    ) -> Result<tonic::Response<proto::CategoriesDeleteBatchResponse>, tonic::Status> {
        unimplemented!();
    }

    async fn categories_list(
        &self,
        request: tonic::Request<proto::CategoriesListRequest>,
    ) -> Result<tonic::Response<proto::CategoriesListResponse>, tonic::Status> {
        unimplemented!();
    }

    async fn category_get(
        &self,
        request: tonic::Request<proto::CategoryGetRequest>,
    ) -> Result<tonic::Response<proto::CategoryGetResponse>, tonic::Status> {
        unimplemented!();
    }

    async fn category_get_by_code(
        &self,
        request: tonic::Request<proto::CategoryGetByCodeRequest>,
    ) -> Result<tonic::Response<proto::CategoryGetByCodeResponse>, tonic::Status> {
        unimplemented!();
    }

    async fn category_get_by_slug(
        &self,
        request: tonic::Request<proto::CategoryGetBySlugRequest>,
    ) -> Result<tonic::Response<proto::CategoryGetBySlugResponse>, tonic::Status> {
        unimplemented!();
    }

    async fn category_update(
        &self,
        request: tonic::Request<proto::CategoryUpdateRequest>,
    ) -> Result<tonic::Response<proto::CategoryUpdateResponse>, tonic::Status> {
        unimplemented!();
    }
}
