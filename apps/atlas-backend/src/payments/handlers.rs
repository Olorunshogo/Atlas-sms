use axum::extract::{Path, Query};
use axum::Json;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use super::models::{
    ConfirmRefundRequest, CreatePaymentRequest, ListPaymentsQuery, Payment, Refund,
    RefundRequestedData, RequestRefundRequest,
};
use super::service;

/**
 * `GET /api/v1/payments?sale_id=`.
 * Tenant-wide visibility, same reasoning as `stores`/`products`.
 */
pub async fn list_payments(
    mut tx: TenantTx,
    Query(query): Query<ListPaymentsQuery>,
) -> Result<Json<ApiEnvelope<Vec<Payment>>>, ApiError> {
    let payments = service::list_payments_for_sale(&mut tx, query.sale_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payments fetched.", payments)));
}

/**
 * `POST /api/v1/payments`.
 * Requires `sale.create` at the sale's own store.
 */
pub async fn create_payment(
    mut tx: TenantTx,
    Json(body): Json<CreatePaymentRequest>,
) -> Result<Json<ApiEnvelope<Payment>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = service::sale_store_id(&mut tx, body.sale_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.create").await?;
    let payment = service::create_payment(&mut tx, tenant_id, &body).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "payment",
        payment.id,
        "payment.create",
        serde_json::to_value(&payment).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payment recorded.", payment)));
}

/**
 * `POST /api/v1/payments/:id/refund`.
 * Requires `sale.refund` at the payment's own store - the first half of the two-step confirm flow, see `service::request_refund`'s doc comment.
 * The raw confirmation token is returned directly to the caller (a staff member at the till, not the customer's email) - there's nowhere else it needs to go.
 */
pub async fn request_refund(
    Path(payment_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<RequestRefundRequest>,
) -> Result<Json<ApiEnvelope<RefundRequestedData>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = service::payment_store_id(&mut tx, payment_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.refund").await?;
    let (refund, confirmation_token) =
        service::request_refund(&mut tx, tenant_id, payment_id, body.amount_minor, user_id).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "refund",
        refund.id,
        "refund.request",
        serde_json::to_value(&refund).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok(
        "Refund requested - confirm to execute it.",
        RefundRequestedData { refund, confirmation_token },
    )));
}

/**
 * `POST /api/v1/payments/refunds/:id/confirm`.
 * Same `sale.refund` gate as requesting it.
 */
pub async fn confirm_refund(
    Path(refund_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<ConfirmRefundRequest>,
) -> Result<Json<ApiEnvelope<Refund>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = service::refund_store_id(&mut tx, refund_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.refund").await?;
    let refund = service::confirm_refund(&mut tx, refund_id, &body.confirmation_token).await?;
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "refund",
        refund.id,
        "refund.confirm",
        serde_json::to_value(&refund).ok(),
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Refund executed.", refund)));
}
