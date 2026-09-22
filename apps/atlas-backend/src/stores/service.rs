use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::Store;

pub async fn list_stores(conn: &mut PgConnection, tenant_id: Uuid) -> Result<Vec<Store>, ApiError> {
    sqlx::query_as::<_, Store>(
        "SELECT id, tenant_id, name, slug, currency, is_active
         FROM atlas.stores
         WHERE tenant_id = $1 AND deleted_at IS NULL
         ORDER BY created_at",
    )
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn create_store(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    name: &str,
    slug: &str,
) -> Result<Store, ApiError> {
    sqlx::query_as::<_, Store>(
        "INSERT INTO atlas.stores (id, tenant_id, name, slug)
         VALUES ($1, $2, $3, $4)
         RETURNING id, tenant_id, name, slug, currency, is_active",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(name)
    .bind(slug)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from)
}
