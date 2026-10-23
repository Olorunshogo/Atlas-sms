use axum::extract::{Path, Query};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{AdjustStockRequest, MovementRecord, ProductInventory, StockLevel, StockMovement, TransferStockRequest};
use super::service;

#[derive(Deserialize)]
pub struct OnHandQuery {
    pub store_id: Uuid,
}

/**
 * `GET /api/v1/stock/on-hand?store_id=`.
 * Requires `stock.view` at that store.
 */
pub async fn get_stock_on_hand(
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
    return Ok(Json(ApiEnvelope::ok("Stock levels fetched.", levels)));
}

/**
 * `POST /api/v1/stock/adjustments`.
 * Requires `stock.adjust` at the store named in the body - a stock count correction or damage write-off.
 */
pub async fn adjust_stock(
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
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "stock_movement",
        movement.id,
        "stock.adjust",
        serde_json::to_value(&movement).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Stock adjusted.", movement)));
}

#[derive(serde::Serialize)]
pub struct TransferResponse {
    pub out_movement: StockMovement,
    pub in_movement: StockMovement,
}

/**
 * `POST /api/v1/stock/transfers`.
 * Requires `stock.transfer` at BOTH the source and destination store - moving stock into a branch is as much a receiving-store decision as a sending-store one.
 */
pub async fn transfer_stock(
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
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "stock_movement",
        out_movement.id,
        "stock.transfer_out",
        serde_json::to_value(&out_movement).ok(),
    )
    .await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "stock_movement",
        in_movement.id,
        "stock.transfer_in",
        serde_json::to_value(&in_movement).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok(
        "Stock transferred.",
        TransferResponse {
            out_movement,
            in_movement,
        },
    )));
}

// === Cross-store reads

const DEFAULT_MOVEMENT_LIMIT: i64 = 50;
const MAX_MOVEMENT_LIMIT: i64 = 200;

/// The stores the caller may view stock at, or a 403 when that is nowhere (the same rule the sales search uses).
async fn viewable_scope(tx: &mut TenantTx) -> Result<roles::service::StoreScope, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let scope = roles::service::stores_with_permission(tx, user_id, tenant_id, "stock.view").await?;
    if scope == roles::service::StoreScope::Stores(Vec::new()) {
        return Err(ApiError::forbidden("Missing permission: stock.view"));
    }
    return Ok(scope);
}

#[derive(Deserialize)]
pub struct InventoryQuery {
    pub store_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    #[serde(default)]
    pub low_stock: bool,
}

/**
 * `GET /api/v1/stock/inventory?store_id=&product_id=&low_stock=`.
 * Every product with its on-hand at each store the caller holds `stock.view` at; `store_id` narrows to one of them, `low_stock` keeps locations at or below their listing's threshold.
 */
pub async fn get_inventory(mut tx: TenantTx, Query(query): Query<InventoryQuery>) -> Result<Json<ApiEnvelope<Vec<ProductInventory>>>, ApiError> {
    let scope = viewable_scope(&mut tx).await?;
    if let Some(store_id) = query.store_id {
        if !scope.includes(store_id) {
            return Err(ApiError::forbidden("Missing permission: stock.view"));
        }
    }
    let tenant_id = tx.tenant_id;
    let products = service::inventory_by_product(&mut tx, tenant_id, scope.as_filter(), query.store_id, query.product_id, query.low_stock).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Inventory fetched.", products)));
}

/// `GET /api/v1/stock/products/:product_id`. One product across the stores the caller may view, with its total.
pub async fn get_product_inventory(Path(product_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<ProductInventory>>, ApiError> {
    let scope = viewable_scope(&mut tx).await?;
    let tenant_id = tx.tenant_id;
    let inventory = service::product_inventory(&mut tx, tenant_id, scope.as_filter(), product_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Product inventory fetched.", inventory)));
}

#[derive(Deserialize)]
pub struct MovementsQuery {
    pub store_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    pub limit: Option<i64>,
}

/// `GET /api/v1/stock/movements?store_id=&product_id=&limit=`. The audit trail, newest first (default 50, at most 200).
pub async fn list_movements(mut tx: TenantTx, Query(query): Query<MovementsQuery>) -> Result<Json<ApiEnvelope<Vec<MovementRecord>>>, ApiError> {
    let scope = viewable_scope(&mut tx).await?;
    if let Some(store_id) = query.store_id {
        if !scope.includes(store_id) {
            return Err(ApiError::forbidden("Missing permission: stock.view"));
        }
    }
    let tenant_id = tx.tenant_id;
    let limit = query.limit.unwrap_or(DEFAULT_MOVEMENT_LIMIT).clamp(1, MAX_MOVEMENT_LIMIT);
    let movements = service::list_movements(&mut tx, tenant_id, scope.as_filter(), query.store_id, query.product_id, limit).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Stock movements fetched.", movements)));
}
