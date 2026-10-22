use axum::extract::{Path, Query};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::checkout::{self, CheckoutMode};
use super::models::{
    CloseShiftRequest, CreateSaleRequest, OpenShiftRequest, Sale, SaleDetail, SaleLine, Shift, VoidSaleRequest,
};
use super::service;

/**
 * `POST /api/v1/shifts`.
 * Requires `sale.create` at the store - operating a till starts with opening its shift.
 */
pub async fn open_shift(
    mut tx: TenantTx,
    Json(body): Json<OpenShiftRequest>,
) -> Result<Json<ApiEnvelope<Shift>>, ApiError> {
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
    let shift = service::open_shift(&mut tx, tenant_id, user_id, &body).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "shift",
        shift.id,
        "shift.open",
        serde_json::to_value(&shift).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Shift opened.", shift)));
}

/**
 * `POST /api/v1/shifts/:id/close`.
 * Requires `sale.create` at the shift's own store - the same gate as opening it.
 */
pub async fn close_shift(
    Path(shift_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<CloseShiftRequest>,
) -> Result<Json<ApiEnvelope<Shift>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = service::shift_store_id(&mut tx, shift_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.create").await?;
    let shift = service::close_shift(&mut tx, shift_id, user_id, &body).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "shift",
        shift.id,
        "shift.close",
        serde_json::to_value(&shift).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Shift closed.", shift)));
}

#[derive(Deserialize)]
pub struct ListSalesQuery {
    pub shift_id: Uuid,
}

/**
 * `GET /api/v1/sales?shift_id=`.
 * Requires `sale.create` at the shift's store - the till's own running list.
 */
pub async fn list_sales(
    mut tx: TenantTx,
    Query(query): Query<ListSalesQuery>,
) -> Result<Json<ApiEnvelope<Vec<Sale>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = service::shift_store_id(&mut tx, query.shift_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.create").await?;
    let sales = service::list_sales_for_shift(&mut tx, query.shift_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sales fetched.", sales)));
}

/**
 * `POST /api/v1/sales`.
 * Requires `sale.create` at the store. Prices, discounts and tax are computed here, payments are checked against the amount due, and stock is taken through the inventory service - see `checkout::create_sale`.
 * Idempotent by `body.id`, the same function the offline-sync endpoint calls.
 */
pub async fn create_sale(
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
    let sale = checkout::create_sale(&mut tx, tenant_id, user_id, &body, CheckoutMode::Live).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sale recorded.", sale)));
}

/**
 * `POST /api/v1/sales/:id/void`.
 * Requires `sale.void` at the sale's store. Same-shift cancellation only - see `service::void_sale`.
 */
pub async fn void_sale(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<VoidSaleRequest>,
) -> Result<Json<ApiEnvelope<Sale>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = crate::payments::service::sale_store_id(&mut tx, sale_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.void").await?;
    let sale = service::void_sale(&mut tx, tenant_id, sale_id, user_id, &body.reason).await?;
    audit::record(&mut tx, tenant_id, user_id, "sale", sale.id, "sale.void", serde_json::to_value(&sale).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sale voided.", sale)));
}

/**
 * `GET /api/v1/sales/:id/lines`.
 * Requires `sale.create` at the sale's store.
 */
pub async fn list_sale_lines(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<Vec<SaleLine>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = crate::payments::service::sale_store_id(&mut tx, sale_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.create").await?;
    let lines = service::list_sale_lines(&mut tx, sale_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sale lines fetched.", lines)));
}
