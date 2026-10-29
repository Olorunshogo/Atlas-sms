use axum::extract::{Path, Query};
use axum::Json;
use uuid::Uuid;

use crate::approvals;
use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::notifications::{self, NotificationKind};
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{
    CreatePurchaseOrderRequest, EditPurchaseOrderRequest, GoodsReceivedNoteData, ListPurchaseOrdersQuery,
    ListPurchaseReturnsQuery, PurchaseOrder, PurchaseOrderDetail, PurchaseOrderLine, PurchaseReturn, ReceiveGoodsRequest,
    RecordPurchaseReturnRequest,
};
use super::service;

async fn assert_at(tx: &mut TenantTx, store_id: Uuid, permission: &str) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, Some(store_id), permission).await;
}

async fn audit_order(tx: &mut TenantTx, order: &PurchaseOrder, action: &str) -> Result<(), ApiError> {
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    return audit::record(tx, tenant_id, user_id, "purchase_order", order.id, action, serde_json::to_value(order).ok()).await;
}

// === Purchase orders

/// `GET /api/v1/purchase-orders?store_id=`. Requires `purchase.create` at that store.
pub async fn list_purchase_orders(
    mut tx: TenantTx,
    Query(query): Query<ListPurchaseOrdersQuery>,
) -> Result<Json<ApiEnvelope<Vec<PurchaseOrder>>>, ApiError> {
    assert_at(&mut tx, query.store_id, "purchase.create").await?;
    let orders = service::list_purchase_orders(&mut tx, query.store_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase orders fetched.", orders)));
}

/// `POST /api/v1/purchase-orders`. Requires `purchase.create` at the order's store. Starts as `draft`.
pub async fn create_purchase_order(
    mut tx: TenantTx,
    Json(body): Json<CreatePurchaseOrderRequest>,
) -> Result<Json<ApiEnvelope<PurchaseOrder>>, ApiError> {
    assert_at(&mut tx, body.store_id, "purchase.create").await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let order = service::create_purchase_order(&mut tx, tenant_id, user_id, &body).await?;
    audit_order(&mut tx, &order, "purchase_order.create").await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase order created.", order)));
}

/// `GET /api/v1/purchase-orders/:id`. Lines with outstanding quantities, charges and totals.
pub async fn show_purchase_order(Path(purchase_order_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<PurchaseOrderDetail>>, ApiError> {
    let store_id = service::purchase_order_store_id(&mut tx, purchase_order_id).await?;
    assert_at(&mut tx, store_id, "purchase.create").await?;
    let order = service::get_purchase_order(&mut tx, purchase_order_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase order fetched.", order)));
}

/// `GET /api/v1/purchase-orders/:id/lines`.
pub async fn list_purchase_order_lines(
    Path(purchase_order_id): Path<Uuid>,
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<Vec<PurchaseOrderLine>>>, ApiError> {
    let store_id = service::purchase_order_store_id(&mut tx, purchase_order_id).await?;
    assert_at(&mut tx, store_id, "purchase.create").await?;
    let lines = service::list_purchase_order_lines(&mut tx, purchase_order_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase order lines fetched.", lines)));
}

/// `PATCH /api/v1/purchase-orders/:id`. Before approval only; a submitted order goes back to `draft`.
pub async fn edit_purchase_order(
    Path(purchase_order_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<EditPurchaseOrderRequest>,
) -> Result<Json<ApiEnvelope<PurchaseOrder>>, ApiError> {
    let store_id = service::purchase_order_store_id(&mut tx, purchase_order_id).await?;
    assert_at(&mut tx, store_id, "purchase.create").await?;
    let tenant_id = tx.tenant_id;
    let before = audit::snapshot(&mut tx, "purchase_orders", "id", purchase_order_id).await?;
    let order = service::edit_purchase_order(&mut tx, tenant_id, purchase_order_id, &body).await?;
    let user_id = tx.user_id;
    audit::record_edit(
        &mut tx,
        audit::Edit { tenant_id, actor_user_id: user_id, entity_type: "purchase_order", entity_id: order.id, action: "purchase_order.edit", table: "purchase_orders", key: "id", before },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase order updated.", order)));
}

/// `POST /api/v1/purchase-orders/:id/submit`. `draft` -> `submitted`; `purchase.approve` holders are notified.
pub async fn submit_purchase_order(Path(purchase_order_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<PurchaseOrder>>, ApiError> {
    let store_id = service::purchase_order_store_id(&mut tx, purchase_order_id).await?;
    assert_at(&mut tx, store_id, "purchase.create").await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let order = service::submit_purchase_order(&mut tx, tenant_id, purchase_order_id).await?;
    audit_order(&mut tx, &order, "purchase_order.submit").await?;
    notifications::service::purchase_order(&mut tx, tenant_id, user_id, order.id, NotificationKind::PurchaseOrderSubmitted).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase order submitted for approval.", order)));
}

/// `POST /api/v1/purchase-orders/:id/approve`. Requires `purchase.approve` at the store. `submitted` -> `ordered`.
pub async fn approve_purchase_order(Path(purchase_order_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<PurchaseOrder>>, ApiError> {
    let store_id = service::purchase_order_store_id(&mut tx, purchase_order_id).await?;
    assert_at(&mut tx, store_id, "purchase.approve").await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let author = service::get_purchase_order(&mut tx, purchase_order_id).await?.created_by_user_id;
    approvals::assert_may_approve(&mut tx, store_id, user_id, author).await?;
    let order = service::approve_purchase_order(&mut tx, tenant_id, user_id, purchase_order_id).await?;
    audit_order(&mut tx, &order, "purchase_order.approve").await?;
    notifications::service::purchase_order(&mut tx, tenant_id, user_id, order.id, NotificationKind::PurchaseOrderApproved).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase order approved.", order)));
}

/// `POST /api/v1/purchase-orders/:id/cancel`. Cancels whatever is still outstanding.
pub async fn cancel_purchase_order(Path(purchase_order_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<PurchaseOrder>>, ApiError> {
    let store_id = service::purchase_order_store_id(&mut tx, purchase_order_id).await?;
    assert_at(&mut tx, store_id, "purchase.create").await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let order = service::cancel_purchase_order(&mut tx, tenant_id, user_id, purchase_order_id).await?;
    audit_order(&mut tx, &order, "purchase_order.cancel").await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase order cancelled.", order)));
}

/// `POST /api/v1/purchase-orders/:id/receive`. Requires `purchase.create`; an over-receipt also needs `purchase.approve`.
pub async fn receive_goods(
    Path(purchase_order_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<ReceiveGoodsRequest>,
) -> Result<Json<ApiEnvelope<GoodsReceivedNoteData>>, ApiError> {
    let store_id = service::purchase_order_store_id(&mut tx, purchase_order_id).await?;
    assert_at(&mut tx, store_id, "purchase.create").await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let can_over_receive = roles::service::has_permission(&mut tx, user_id, tenant_id, Some(store_id), "purchase.approve").await?;
    let grn = service::receive_goods(&mut tx, tenant_id, purchase_order_id, user_id, can_over_receive, &body).await?;
    let over_received = body.lines.iter().any(|l| l.over_receipt_reason.is_some());
    let action = if over_received { "goods_received_note.receive_over" } else { "goods_received_note.receive" };
    audit::record(&mut tx, tenant_id, user_id, "goods_received_note", grn.id, action, serde_json::to_value(&grn).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Goods received.", grn)));
}

// === Purchase returns (FR-23)

/// `GET /api/v1/purchase-returns?store_id=&supplier_id=`. Requires `purchase.create` at the store.
pub async fn list_purchase_returns(
    mut tx: TenantTx,
    Query(query): Query<ListPurchaseReturnsQuery>,
) -> Result<Json<ApiEnvelope<Vec<PurchaseReturn>>>, ApiError> {
    assert_at(&mut tx, query.store_id, "purchase.create").await?;
    let returns = service::list_purchase_returns(&mut tx, &query).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase returns fetched.", returns)));
}

/// `GET /api/v1/purchase-returns/:id`.
pub async fn show_purchase_return(Path(return_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<PurchaseReturn>>, ApiError> {
    let store_id = service::purchase_return_store_id(&mut tx, return_id).await?;
    assert_at(&mut tx, store_id, "purchase.create").await?;
    let purchase_return = service::get_purchase_return(&mut tx, return_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase return fetched.", purchase_return)));
}

/// `POST /api/v1/purchase-returns`. Records a `pending` return; stock and balance wait for `confirm`.
pub async fn record_purchase_return(
    mut tx: TenantTx,
    Json(body): Json<RecordPurchaseReturnRequest>,
) -> Result<Json<ApiEnvelope<PurchaseReturn>>, ApiError> {
    let store_id = service::purchase_order_store_id(&mut tx, body.purchase_order_id).await?;
    assert_at(&mut tx, store_id, "purchase.create").await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let purchase_return = service::record_purchase_return(&mut tx, tenant_id, user_id, &body).await?;
    audit::record(&mut tx, tenant_id, user_id, "purchase_return", purchase_return.id, "purchase_return.create", serde_json::to_value(&purchase_return).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase return recorded.", purchase_return)));
}

/// `POST /api/v1/purchase-returns/:id/confirm`. Moves the stock out and takes it off the supplier's balance.
pub async fn confirm_purchase_return(Path(return_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<PurchaseReturn>>, ApiError> {
    let store_id = service::purchase_return_store_id(&mut tx, return_id).await?;
    assert_at(&mut tx, store_id, "purchase.create").await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let author = service::get_purchase_return(&mut tx, return_id).await?.created_by_user_id;
    approvals::assert_may_approve(&mut tx, store_id, user_id, author).await?;
    let purchase_return = service::confirm_purchase_return(&mut tx, tenant_id, user_id, return_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "purchase_return", return_id, "purchase_return.confirm", serde_json::to_value(&purchase_return).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase return confirmed.", purchase_return)));
}

/// `POST /api/v1/purchase-returns/:id/cancel`.
pub async fn cancel_purchase_return(Path(return_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<PurchaseReturn>>, ApiError> {
    let store_id = service::purchase_return_store_id(&mut tx, return_id).await?;
    assert_at(&mut tx, store_id, "purchase.create").await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let purchase_return = service::cancel_purchase_return(&mut tx, tenant_id, return_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "purchase_return", return_id, "purchase_return.cancel", serde_json::to_value(&purchase_return).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Purchase return cancelled.", purchase_return)));
}
