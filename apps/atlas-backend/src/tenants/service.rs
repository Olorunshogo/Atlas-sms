use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::Tenant;

pub async fn get_tenant(conn: &mut PgConnection, tenant_id: Uuid) -> Result<Tenant, ApiError> {
    return sqlx::query_as::<_, Tenant>(
        "SELECT id, name, status::text AS status, created_at
         FROM atlas.tenants
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::NotFound("Tenant not found.".to_string()));
}
