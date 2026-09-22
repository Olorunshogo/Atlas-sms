use axum::extract::{Path, Query};
use axum::Json;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{
    CreatePurchaseOrderRequest, CreateSupplierRequest, GoodsReceivedNoteData, ListPurchaseOrdersQuery,
    PurchaseOrder, PurchaseOrderLine, ReceiveGoodsRequest, Supplier, UpdateSupplierRequest,
};
use super::service;

// === Suppliers

/**
 * `GET /api/v1/suppliers`.
 * Tenant-wide visibility, same reasoning as `stores`/`products`.
 */
pub async fn list_suppliers(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<Supplier>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let suppliers = service::list_suppliers(&mut tx, tenant_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Suppliers fetched.", suppliers)));
}

/**
 * `POST /api/v1/suppliers`.
 * Requires `supplier.manage`.
 */
pub async fn create_supplier(
    mut tx: TenantTx,
    Json(body): Json<CreateSupplierRequest>,
) -> Result<Json<ApiEnvelope<Supplier>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "supplier.manage").await?;
    let supplier = service::create_supplier(&mut tx, tenant_id, &body.name, body.phone.as_deref(), body.email.as_deref()).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "supplier",
        supplier.id,
        "supplier.create",
        serde_json::to_value(&supplier).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Supplier added.", supplier)));
}

/**
 * `PATCH /api/v1/suppliers/:id`.
 * Requires `supplier.manage`.
 */
pub async fn update_supplier(
    Path(supplier_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdateSupplierRequest>,
) -> Result<Json<ApiEnvelope<Supplier>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "supplier.manage").await?;
    let supplier = service::update_supplier(&mut tx, supplier_id, &body).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "supplier",
        supplier.id,
        "supplier.update",
        serde_json::to_value(&supplier).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Supplier updated.", supplier)));
}

// === Purchase orders

/**
 * `GET /api/v1/purchase-orders?store_id=`.
 * Requires `purchase.create` at that store - the same permission that creates one also lists them.
 */
pub async fn list_purchase_orders(
    mut tx: TenantTx,
    Query(query): Query<ListPurchaseOrdersQuery>,
) -> Result<Json<ApiEnvelope<Vec<PurchaseOrder>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(query.store_id), "purchase.create").await?;
    let orders = service::list_purchase_orders(&mut tx, query.store_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase orders fetched.", orders)));
}

/**
 * `POST /api/v1/purchase-orders`.
 * Requires `purchase.create` at the order's own store.
 */
pub async fn create_purchase_order(
    mut tx: TenantTx,
    Json(body): Json<CreatePurchaseOrderRequest>,
) -> Result<Json<ApiEnvelope<PurchaseOrder>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(body.store_id), "purchase.create").await?;
    let order = service::create_purchase_order(&mut tx, tenant_id, user_id, &body).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "purchase_order",
        order.id,
        "purchase_order.create",
        serde_json::to_value(&order).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase order created.", order)));
}

/**
 * `GET /api/v1/purchase-orders/:id/lines`.
 * Requires `purchase.create` at the order's own store.
 */
pub async fn list_purchase_order_lines(
    Path(purchase_order_id): Path<Uuid>,
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<Vec<PurchaseOrderLine>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = service::purchase_order_store_id(&mut tx, purchase_order_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "purchase.create").await?;
    let lines = service::list_purchase_order_lines(&mut tx, purchase_order_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase order lines fetched.", lines)));
}

/**
 * `POST /api/v1/purchase-orders/:id/receive`.
 * Requires `purchase.create` at the order's own store - see `service::receive_goods`'s doc comment for what one call here actually does (stock movements, supplier balance, order status, all together).
 */
pub async fn receive_goods(
    Path(purchase_order_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<ReceiveGoodsRequest>,
) -> Result<Json<ApiEnvelope<GoodsReceivedNoteData>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = service::purchase_order_store_id(&mut tx, purchase_order_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "purchase.create").await?;
    let grn = service::receive_goods(&mut tx, tenant_id, purchase_order_id, user_id, &body).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "goods_received_note",
        grn.id,
        "goods_received_note.receive",
        serde_json::to_value(&grn).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Goods received.", grn)));
}
