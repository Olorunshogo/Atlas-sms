use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::http::{Json, Path, Query};
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{
    AdjustStockRequest, InitialStockRequest, ProductInventory, RestockRequest, StockLevel,
    StockMovement, TransferStockRequest,
};
use super::service::{self, Actor};

const DEFAULT_MOVEMENT_LIMIT: i64 = 50;
const MAX_MOVEMENT_LIMIT: i64 = 200;

fn actor(tx: &TenantTx) -> Actor {
    Actor {
        tenant_id: tx.tenant_id,
        user_id: tx.user_id,
    }
}

// === Reads

#[derive(Deserialize)]
pub struct OnHandQuery {
    pub store_id: Uuid,
    pub product_id: Option<Uuid>,
    #[serde(default)]
    pub low_stock: bool,
    pub max_quantity: Option<Decimal>,
}

/// `GET /api/v1/stock/on-hand?store_id=`. Requires `stock.view` at that store.
pub async fn on_hand(
    mut tx: TenantTx,
    Query(query): Query<OnHandQuery>,
) -> Result<Json<ApiEnvelope<Vec<StockLevel>>>, ApiError> {
    let who = actor(&tx);
    roles::service::assert_permission(
        &mut tx,
        who.user_id,
        who.tenant_id,
        Some(query.store_id),
        "stock.view",
    )
    .await?;
    let levels = service::on_hand_for_store(
        &mut tx,
        query.store_id,
        query.product_id,
        query.low_stock,
        query.max_quantity,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Stock levels fetched.", levels)))
}

#[derive(Deserialize)]
pub struct InventoryQuery {
    pub store_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    #[serde(default)]
    pub low_stock: bool,
}

/**
 * `GET /api/v1/stock/inventory`. Every product with its quantity per store,
 * across the stores the caller holds `stock.view` at. `store_id` narrows to
 * one of them.
 */
pub async fn inventory(
    mut tx: TenantTx,
    Query(query): Query<InventoryQuery>,
) -> Result<Json<ApiEnvelope<Vec<ProductInventory>>>, ApiError> {
    let who = actor(&tx);
    let mut store_ids = service::viewable_store_ids(&mut tx, who, "stock.view").await?;
    if let Some(store_id) = query.store_id {
        if !store_ids.contains(&store_id) {
            return Err(ApiError::forbidden("Missing permission: stock.view"));
        }
        store_ids = vec![store_id];
    }
    let products =
        service::inventory_by_product(&mut tx, &store_ids, query.product_id, query.low_stock)
            .await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Inventory fetched.", products)))
}

/// `GET /api/v1/stock/products/{product_id}`. One product across stores.
pub async fn product_inventory(
    mut tx: TenantTx,
    Path(product_id): Path<Uuid>,
) -> Result<Json<ApiEnvelope<ProductInventory>>, ApiError> {
    let who = actor(&tx);
    let store_ids = service::viewable_store_ids(&mut tx, who, "stock.view").await?;
    let inventory = service::product_inventory(&mut tx, &store_ids, product_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok(
        "Product inventory fetched.",
        inventory,
    )))
}

#[derive(Deserialize)]
pub struct MovementsQuery {
    pub store_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    pub limit: Option<i64>,
}

/// `GET /api/v1/stock/movements`. The audit trail, newest first.
pub async fn movements(
    mut tx: TenantTx,
    Query(query): Query<MovementsQuery>,
) -> Result<Json<ApiEnvelope<Vec<StockMovement>>>, ApiError> {
    let who = actor(&tx);
    let store_ids = service::viewable_store_ids(&mut tx, who, "stock.view").await?;
    let limit = query
        .limit
        .unwrap_or(DEFAULT_MOVEMENT_LIMIT)
        .clamp(1, MAX_MOVEMENT_LIMIT);
    let movements =
        service::list_movements(&mut tx, &store_ids, query.store_id, query.product_id, limit)
            .await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Stock movements fetched.", movements)))
}

// === Writes

#[derive(Serialize)]
pub struct InitialStockResponse {
    pub movement: Option<StockMovement>,
}

/// `POST /api/v1/stock/inventory`. Requires `stock.adjust` at the store.
pub async fn initial_stock(
    mut tx: TenantTx,
    Json(body): Json<InitialStockRequest>,
) -> Result<Json<ApiEnvelope<InitialStockResponse>>, ApiError> {
    let who = actor(&tx);
    roles::service::assert_permission(
        &mut tx,
        who.user_id,
        who.tenant_id,
        Some(body.store_id),
        "stock.adjust",
    )
    .await?;
    let movement = service::initial_stock(&mut tx, who, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok(
        "Product added to store.",
        InitialStockResponse { movement },
    )))
}

/// `POST /api/v1/stock/restocks`. Incoming supplier stock. Requires `stock.adjust` at the store.
pub async fn restock(
    mut tx: TenantTx,
    Json(body): Json<RestockRequest>,
) -> Result<Json<ApiEnvelope<StockMovement>>, ApiError> {
    let who = actor(&tx);
    roles::service::assert_permission(
        &mut tx,
        who.user_id,
        who.tenant_id,
        Some(body.store_id),
        "stock.adjust",
    )
    .await?;
    let movement = service::restock(&mut tx, who, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Stock received.", movement)))
}

/// `POST /api/v1/stock/adjustments`. Requires `stock.adjust` at the store
/// named in the body - a stock count correction or damage write-off.
pub async fn adjust(
    mut tx: TenantTx,
    Json(body): Json<AdjustStockRequest>,
) -> Result<Json<ApiEnvelope<StockMovement>>, ApiError> {
    let who = actor(&tx);
    roles::service::assert_permission(
        &mut tx,
        who.user_id,
        who.tenant_id,
        Some(body.store_id),
        "stock.adjust",
    )
    .await?;
    let movement = service::adjust_stock(&mut tx, who, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Stock adjusted.", movement)))
}

#[derive(Serialize)]
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
    let who = actor(&tx);
    for store_id in [body.from_store_id, body.to_store_id] {
        roles::service::assert_permission(
            &mut tx,
            who.user_id,
            who.tenant_id,
            Some(store_id),
            "stock.transfer",
        )
        .await?;
    }
    let (out_movement, in_movement) = service::transfer_stock(&mut tx, who, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok(
        "Stock transferred.",
        TransferResponse {
            out_movement,
            in_movement,
        },
    )))
}
