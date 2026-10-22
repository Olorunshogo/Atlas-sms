use axum::extract::Query;
use axum::Json;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{Reconciliation, ReconciliationQuery};
use super::service;

/**
 * `GET /api/v1/reports/payments/reconciliation?store_id=&shift_id=&from=&to=`.
 * Requires `report.view` or `sale.view` at the store.
 */
pub async fn reconciliation(
    mut tx: TenantTx,
    Query(query): Query<ReconciliationQuery>,
) -> Result<Json<ApiEnvelope<Reconciliation>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    if !roles::service::has_permission(&mut tx, user_id, tenant_id, Some(query.store_id), "report.view").await? {
        roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(query.store_id), "sale.view").await?;
    }
    let report = service::reconcile(&mut tx, &query).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Reconciliation generated.", report)));
}
