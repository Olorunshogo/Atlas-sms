use axum::extract::{Path, Query};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::notifications::{self, NotificationKind};
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{
    AdjustStockRequest, MovementRecord, ProductInventory, RequestTransferRequest, StockLevel,
    StockMovement, StockTransfer,
};
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

// === Inter-store transfers (FR-22)
//
// Staged: request -> approve/reject -> dispatch -> receive, with cancel as
// an exit before dispatch. See `service.rs`'s module doc for why the stages
// exist - stock must never be countable as available at both stores at once.

async fn audit_transfer(tx: &mut TenantTx, transfer: &StockTransfer, action: &str) -> Result<(), ApiError> {
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    return audit::record(
        tx,
        tenant_id,
        user_id,
        "stock_transfer",
        transfer.id,
        action,
        serde_json::to_value(transfer).ok(),
    )
    .await;
}

/**
 * `POST /api/v1/stock/transfers`.
 * Requires `stock.transfer` at the sending store - the destination store confirms at `approve`, not here.
 */
pub async fn request_transfer(
    mut tx: TenantTx,
    Json(body): Json<RequestTransferRequest>,
) -> Result<Json<ApiEnvelope<StockTransfer>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(body.from_store_id), "stock.transfer").await?;
    let transfer = service::request_transfer(&mut tx, tenant_id, user_id, &body).await?;
    audit_transfer(&mut tx, &transfer, "stock_transfer.request").await?;
    notifications::service::transfer(&mut tx, tenant_id, user_id, transfer.id, NotificationKind::TransferRequested).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Transfer requested.", transfer)));
}

/// `GET /api/v1/stock/transfers?store_id=`. Every transfer touching that store, either direction, newest first.
pub async fn list_transfers(
    mut tx: TenantTx,
    Query(query): Query<OnHandQuery>,
) -> Result<Json<ApiEnvelope<Vec<StockTransfer>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(query.store_id), "stock.view").await?;
    let transfers = service::list_transfers(&mut tx, query.store_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Transfers fetched.", transfers)));
}

/**
 * `POST /api/v1/stock/transfers/:id/approve`.
 * Requires `stock.transfer` at BOTH stores - moving stock into a branch is as much a receiving-store decision as a sending-store one.
 */
pub async fn approve_transfer(Path(transfer_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<StockTransfer>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let existing = service::get_transfer(&mut tx, transfer_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.from_store_id), "stock.transfer").await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.to_store_id), "stock.transfer").await?;
    let transfer = service::approve_transfer(&mut tx, tenant_id, user_id, transfer_id).await?;
    audit_transfer(&mut tx, &transfer, "stock_transfer.approve").await?;
    notifications::service::transfer(&mut tx, tenant_id, user_id, transfer.id, NotificationKind::TransferApproved).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Transfer approved.", transfer)));
}

/// `POST /api/v1/stock/transfers/:id/reject`. Requires `stock.transfer` at either store.
pub async fn reject_transfer(Path(transfer_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<StockTransfer>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let existing = service::get_transfer(&mut tx, transfer_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.to_store_id), "stock.transfer").await?;
    let transfer = service::reject_transfer(&mut tx, tenant_id, transfer_id).await?;
    audit_transfer(&mut tx, &transfer, "stock_transfer.reject").await?;
    notifications::service::transfer(&mut tx, tenant_id, user_id, transfer.id, NotificationKind::TransferRejected).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Transfer rejected.", transfer)));
}

/// `POST /api/v1/stock/transfers/:id/cancel`. Requires `stock.transfer` at the sending store - not available once dispatched.
pub async fn cancel_transfer(Path(transfer_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<StockTransfer>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let existing = service::get_transfer(&mut tx, transfer_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.from_store_id), "stock.transfer").await?;
    let transfer = service::cancel_transfer(&mut tx, tenant_id, user_id, transfer_id).await?;
    audit_transfer(&mut tx, &transfer, "stock_transfer.cancel").await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Transfer cancelled.", transfer)));
}

/**
 * `POST /api/v1/stock/transfers/:id/dispatch`.
 * Requires `stock.transfer` at the sending store. Stock actually leaves here - see `service::dispatch_transfer`.
 */
pub async fn dispatch_transfer(Path(transfer_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<StockTransfer>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let existing = service::get_transfer(&mut tx, transfer_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.from_store_id), "stock.transfer").await?;
    let transfer = service::dispatch_transfer(&mut tx, tenant_id, user_id, transfer_id).await?;
    audit_transfer(&mut tx, &transfer, "stock_transfer.dispatch").await?;
    notifications::service::transfer(&mut tx, tenant_id, user_id, transfer.id, NotificationKind::TransferDispatched).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Transfer dispatched.", transfer)));
}

/**
 * `POST /api/v1/stock/transfers/:id/receive`.
 * Requires `stock.transfer` at the receiving store. Stock actually arrives here - see `service::receive_transfer`.
 */
pub async fn receive_transfer(Path(transfer_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<StockTransfer>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let existing = service::get_transfer(&mut tx, transfer_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.to_store_id), "stock.transfer").await?;
    let transfer = service::receive_transfer(&mut tx, tenant_id, user_id, transfer_id).await?;
    audit_transfer(&mut tx, &transfer, "stock_transfer.receive").await?;
    notifications::service::transfer(&mut tx, tenant_id, user_id, transfer.id, NotificationKind::TransferReceived).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Transfer received.", transfer)));
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
    pub category_id: Option<Uuid>,
    /// Name or SKU, case-insensitive substring.
    pub q: Option<String>,
    #[serde(default)]
    pub low_stock: bool,
}

/**
 * `GET /api/v1/stock/inventory?store_id=&product_id=&category_id=&q=&low_stock=`.
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
    let products = service::inventory_by_product(&mut tx, tenant_id, scope.as_filter(), query.store_id, query.product_id, query.category_id, query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()), query.low_stock).await?;
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
