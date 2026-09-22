use axum::Json;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::sales::service as sales_service;
use crate::sales::{CreateSaleRequest, Sale};

/**
 * `POST /api/v1/sync/sales`.
 * What `atlas-pos` calls once it's back online, once per queued sale it built up offline - see docs/Atlas/README.md's offline requirements and the `sales` table's migration comment.
 *
 * Deliberately the same underlying `create_sale` as the live `POST /api/v1/sales` path: idempotent by the sale's client-generated `id`, so calling this twice for the same sale (a retried request after a flaky reconnect) is a no-op the second time, not a duplicate sale or a double stock deduction.
 * The only thing this endpoint adds over the live path is being named for what it's for - a device catching up, not a cashier ringing something up right now.
 */
pub async fn sync_sale(
    mut tx: TenantTx,
    Json(body): Json<CreateSaleRequest>,
) -> Result<Json<ApiEnvelope<Sale>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(
        &mut tx,
        user_id,
        tenant_id,
        Some(body.store_id),
        "sale.create",
    )
    .await?;
    let sale = sales_service::create_sale(&mut tx, tenant_id, user_id, &body).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sale synced.", sale)));
}
