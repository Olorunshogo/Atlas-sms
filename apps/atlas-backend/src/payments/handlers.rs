use axum::extract::{Path, Query};
use axum::Json;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use crate::sales::settlement;

use super::models::{
    ConfirmRefundRequest, CreatePaymentMethodRequest, ListPaymentsQuery, Payment, PaymentMethod, Refund,
    RefundRequestedData, RequestRefundRequest, ResolvePaymentRequest, ReversePaymentRequest,
    UpdatePaymentMethodRequest,
};
use super::service;

/**
 * `GET /api/v1/payments?sale_id=`.
 * Requires `sale.create` at the sale's store.
 */
pub async fn list_payments(
    mut tx: TenantTx,
    Query(query): Query<ListPaymentsQuery>,
) -> Result<Json<ApiEnvelope<Vec<Payment>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = service::sale_store_id(&mut tx, query.sale_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.create").await?;
    let payments = service::list_payments_for_sale(&mut tx, query.sale_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payments fetched.", payments)));
}

/**
 * `POST /api/v1/payments/:id/status`.
 * Requires `payment.manage` at the payment's store. Confirms or fails a pending payment; confirming the last one a sale was waiting on completes the sale.
 */
pub async fn resolve_payment(
    Path(payment_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<ResolvePaymentRequest>,
) -> Result<Json<ApiEnvelope<Payment>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = service::payment_store_id(&mut tx, payment_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "payment.manage").await?;
    let payment = settlement::resolve_pending(
        &mut tx,
        tenant_id,
        payment_id,
        user_id,
        body.status,
        body.reference.as_deref(),
        body.reason.as_deref(),
    )
    .await?;
    audit::record(&mut tx, tenant_id, user_id, "payment", payment.id, "payment.resolve", serde_json::to_value(&payment).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payment updated.", payment)));
}

/**
 * `POST /api/v1/payments/:id/reverse`.
 * Requires `payment.manage` at the payment's store - a reversal takes money off a sale's paid total, so it's a supervisor action.
 */
pub async fn reverse_payment(
    Path(payment_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<ReversePaymentRequest>,
) -> Result<Json<ApiEnvelope<Payment>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = service::payment_store_id(&mut tx, payment_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "payment.manage").await?;
    let payment = settlement::reverse(&mut tx, tenant_id, payment_id, user_id, &body.reason).await?;
    audit::record(&mut tx, tenant_id, user_id, "payment", payment.id, "payment.reverse", serde_json::to_value(&payment).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payment reversed.", payment)));
}

// === Payment methods

/// `GET /api/v1/payment-methods`. Any tenant member - every till lists them at checkout.
pub async fn list_methods(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<PaymentMethod>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let methods = service::list_methods(&mut tx, tenant_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payment methods fetched.", methods)));
}

/**
 * `POST /api/v1/payment-methods`.
 * Requires `store.update` tenant-wide: payment methods are shared by every store, so configuring one is not a single-branch decision.
 */
pub async fn create_method(
    mut tx: TenantTx,
    Json(body): Json<CreatePaymentMethodRequest>,
) -> Result<Json<ApiEnvelope<PaymentMethod>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "store.update").await?;
    let method = service::create_method(&mut tx, tenant_id, &body).await?;
    audit::record(&mut tx, tenant_id, user_id, "payment_method", method.id, "payment_method.create", serde_json::to_value(&method).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payment method created.", method)));
}

/// `PATCH /api/v1/payment-methods/:id`. Same gate as create.
pub async fn update_method(
    Path(method_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<UpdatePaymentMethodRequest>,
) -> Result<Json<ApiEnvelope<PaymentMethod>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "store.update").await?;
    let method = service::update_method(&mut tx, method_id, &body).await?;
    audit::record(&mut tx, tenant_id, user_id, "payment_method", method.id, "payment_method.update", serde_json::to_value(&method).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payment method updated.", method)));
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
