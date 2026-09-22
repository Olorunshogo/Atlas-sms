use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{AdjustStockRequest, StockLevel, StockMovement, TransferStockRequest};
use super::service;

#[derive(Deserialize)]
pub struct OnHandQuery {
    pub store_id: Uuid,
}

/// `GET /api/v1/stock/on-hand?store_id=`. Requires `stock.view` at that store.
pub async fn on_hand(
    mut tx: TenantTx,
    Query(query): Query<OnHandQuery>,
) -> Result<Json<ApiEnvelope<Vec<StockLevel>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(
        &mut tx,
        user_id,
        tenant_id,
        Some(query.store_id),
        "stock.view",
    )
    .await?;
    let levels = service::on_hand_for_store(&mut tx, query.store_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Stock levels fetched.", levels)))
}

/// `POST /api/v1/stock/adjustments`. Requires `stock.adjust` at the store
/// named in the body - a stock count correction or damage write-off.
pub async fn adjust(
    mut tx: TenantTx,
    Json(body): Json<AdjustStockRequest>,
) -> Result<Json<ApiEnvelope<StockMovement>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(
        &mut tx,
        user_id,
        tenant_id,
        Some(body.store_id),
        "stock.adjust",
    )
    .await?;
    let movement = service::adjust_stock(&mut tx, tenant_id, user_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Stock adjusted.", movement)))
}

#[derive(serde::Serialize)]
pub struct TransferResponse {
    pub out_movement: StockMovement,
    pub in_movement: StockMovement,
}

/// `POST /api/v1/stock/transfers`. Requires `stock.transfer` at BOTH the
/// source and destination store - moving stock into a branch is as much a
/// receiving-store decision as a sending-store one.
pub async fn transfer(
    mut tx: TenantTx,
    Json(body): Json<TransferStockRequest>,
) -> Result<Json<ApiEnvelope<TransferResponse>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(
        &mut tx,
        user_id,
        tenant_id,
        Some(body.from_store_id),
        "stock.transfer",
    )
    .await?;
    roles::service::assert_permission(
        &mut tx,
        user_id,
        tenant_id,
        Some(body.to_store_id),
        "stock.transfer",
    )
    .await?;
    let (out_movement, in_movement) =
        service::transfer_stock(&mut tx, tenant_id, user_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok(
        "Stock transferred.",
        TransferResponse {
            out_movement,
            in_movement,
        },
    )))
}
