use serde::Deserialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::http::{Json, Path, Query};
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{
    CloseShiftRequest, CreateReturnRequest, CreateSaleRequest, OpenShiftRequest, ReturnReceipt,
    Sale, SaleLine, SaleReceipt, SaleReturnLine, Shift, VoidReceipt, VoidSaleRequest,
};
use super::{returns, service};
use crate::stock::service::Actor;

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
/// function the offline-sync endpoint calls. Returns the receipt (sale plus lines).
pub async fn create(
    mut tx: TenantTx,
    Json(body): Json<CreateSaleRequest>,
) -> Result<Json<ApiEnvelope<SaleReceipt>>, ApiError> {
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
    let lines = service::list_sale_lines(&mut tx, sale.id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok(
        "Sale recorded.",
        SaleReceipt { sale, lines },
    )))
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

/// `GET /api/v1/sales/{id}`. The receipt: the sale plus its lines.
pub async fn get(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<SaleReceipt>>, ApiError> {
    let sale = service::fetch_sale(&mut tx, sale_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Sale not found.".to_string()))?;
    let lines = service::list_sale_lines(&mut tx, sale_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok(
        "Sale fetched.",
        SaleReceipt { sale, lines },
    )))
}

/// `POST /api/v1/sales/{id}/void`. Requires `sale.void` at the sale's store.
pub async fn void(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<VoidSaleRequest>,
) -> Result<Json<ApiEnvelope<VoidReceipt>>, ApiError> {
    let actor = Actor {
        tenant_id: tx.tenant_id,
        user_id: tx.user_id,
    };
    let store_id = returns::sale_store_id(&mut tx, sale_id).await?;
    roles::service::assert_permission(
        &mut tx,
        actor.user_id,
        actor.tenant_id,
        Some(store_id),
        "sale.void",
    )
    .await?;
    let receipt = returns::void_sale(&mut tx, actor, sale_id, &body.reason).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Sale voided.", receipt)))
}

/// `POST /api/v1/sales/{id}/returns`. Requires `sale.refund` at the sale's store.
pub async fn create_return(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<CreateReturnRequest>,
) -> Result<Json<ApiEnvelope<ReturnReceipt>>, ApiError> {
    let actor = Actor {
        tenant_id: tx.tenant_id,
        user_id: tx.user_id,
    };
    let store_id = returns::sale_store_id(&mut tx, sale_id).await?;
    roles::service::assert_permission(
        &mut tx,
        actor.user_id,
        actor.tenant_id,
        Some(store_id),
        "sale.refund",
    )
    .await?;
    let receipt = returns::create_return(&mut tx, actor, sale_id, &body).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Return recorded.", receipt)))
}

/// `GET /api/v1/sales/{id}/returns`.
pub async fn list_returns(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<Vec<SaleReturnLine>>>, ApiError> {
    let returns = returns::list_returns(&mut tx, sale_id).await?;
    tx.commit().await?;
    Ok(Json(ApiEnvelope::ok("Returns fetched.", returns)))
}
