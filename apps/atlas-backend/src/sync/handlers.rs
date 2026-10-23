use axum::Json;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::sales::checkout::{self, CheckoutMode};
use crate::sales::{CreateSaleRequest, SaleDetail};

/**
 * `POST /api/v1/sync/sales`.
 * What `atlas-pos` calls once it's back online, once per queued sale it built up offline - see docs/Atlas/README.md's offline requirements and the `sales` table's migration comment.
 *
 * Deliberately the same underlying `create_sale` as the live `POST /api/v1/sales` path: idempotent by the sale's client-generated `id`, so calling this twice for the same sale (a retried request after a flaky reconnect) is a no-op the second time, not a duplicate sale or a double stock deduction.
 * What differs is `CheckoutMode::Sync`: the sale already happened, so a since-closed shift, short stock, or a device price that no longer matches the rules is recorded (a `stock_exceptions` row, an audit entry) and accepted rather than refused.
 */
pub async fn sync_sale(
    mut tx: TenantTx,
    Json(body): Json<CreateSaleRequest>,
) -> Result<Json<ApiEnvelope<SaleDetail>>, ApiError> {
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
    let sale = checkout::create_sale(&mut tx, tenant_id, user_id, &body, CheckoutMode::Sync).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sale synced.", sale)));
}
