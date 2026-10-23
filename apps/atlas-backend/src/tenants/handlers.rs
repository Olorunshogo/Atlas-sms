use axum::Json;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{Tenant, UpdateBusinessDetailsRequest};
use super::service;

/**
 * `GET /api/v1/tenants/me`.
 * Returns the caller's own tenant, resolved from the `TenantTx` scope - there is no "get any tenant by id" here, only "get mine".
 */
pub async fn get_current_tenant(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Tenant>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let tenant = service::get_tenant(&mut tx, tenant_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Tenant fetched.", tenant)));
}

/**
 * `PATCH /api/v1/tenants/me`.
 * Requires `tenant.manage`. The legal name and tax identification number shown on receipts.
 */
pub async fn update_business_details(
    mut tx: TenantTx,
    Json(body): Json<UpdateBusinessDetailsRequest>,
) -> Result<Json<ApiEnvelope<Tenant>>, ApiError> {
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "tenant.manage").await?;
    let tenant = service::update_business_details(
        &mut tx,
        tenant_id,
        body.legal_name.as_deref(),
        body.tax_identification_number.as_deref(),
    )
    .await?;
    audit::record(&mut tx, tenant_id, user_id, "tenant", tenant_id, "tenant.business_details", serde_json::to_value(&tenant).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Business details updated.", tenant)));
}
