use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::Tenant;

pub async fn get_tenant(conn: &mut PgConnection, tenant_id: Uuid) -> Result<Tenant, ApiError> {
    return sqlx::query_as::<_, Tenant>(
        "SELECT id, name, status::text AS status, legal_name, tax_identification_number, created_at
         FROM atlas.tenants
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::NotFound("Tenant not found.".to_string()));
}

/// Business details printed on every receipt. Takes effect for receipts issued from now on; issued receipts keep their snapshot.
pub async fn update_business_details(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    legal_name: Option<&str>,
    tax_identification_number: Option<&str>,
) -> Result<Tenant, ApiError> {
    sqlx::query(
        "UPDATE atlas.tenants
         SET legal_name = COALESCE($2, legal_name),
             tax_identification_number = COALESCE($3, tax_identification_number),
             updated_at = now()
         WHERE id = $1",
    )
    .bind(tenant_id)
    .bind(legal_name.map(str::trim))
    .bind(tax_identification_number.map(str::trim))
    .execute(&mut *conn)
    .await?;
    return get_tenant(conn, tenant_id).await;
}
