use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::http::Json;
use crate::response::ApiEnvelope;

use super::models::Tenant;
use super::service;

/// `GET /api/v1/tenants/me`. Returns the caller's own tenant, resolved from
/// the `TenantTx` scope - there is no "get any tenant by id" here, only
/// "get mine".
pub async fn me(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Tenant>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let tenant = service::get_tenant(&mut tx, tenant_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Tenant fetched.", tenant)))
}
