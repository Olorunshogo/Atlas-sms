use axum::extract::{Path, Query};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{
    CloseShiftRequest, CreateSaleRequest, OpenShiftRequest, Sale, SaleLine, Shift,
};
use super::service;

/// `POST /api/v1/shifts`. Requires `sale.create` at the store - operating
/// a till starts with opening its shift.
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
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Shift opened.", shift)))
}

/// `POST /api/v1/shifts/:id/close`.
pub async fn close_shift(
    Path(shift_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<CloseShiftRequest>,
) -> Result<Json<ApiEnvelope<Shift>>, ApiError> {
    let user_id = tx.user_id;
    let shift = service::close_shift(&mut tx, shift_id, user_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Shift closed.", shift)))
}

#[derive(Deserialize)]
pub struct ListSalesQuery {
    pub shift_id: Uuid,
}

/// `GET /api/v1/sales?shift_id=`.
pub async fn list(
    mut tx: TenantTx,
    Query(query): Query<ListSalesQuery>,
) -> Result<Json<ApiEnvelope<Vec<Sale>>>, ApiError> {
    let sales = service::list_sales_for_shift(&mut tx, query.shift_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Sales fetched.", sales)))
}

/// `POST /api/v1/sales`. Requires `sale.create` at the store. Idempotent by
/// `body.id` - see `service::create_sale`'s doc comment; this is also the
/// function the offline-sync endpoint calls.
pub async fn create(
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
    let sale = service::create_sale(&mut tx, tenant_id, user_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Sale recorded.", sale)))
}

/// `GET /api/v1/sales/:id/lines`.
pub async fn lines(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<Vec<SaleLine>>>, ApiError> {
    let lines = service::list_sale_lines(&mut tx, sale_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Sale lines fetched.", lines)))
}
