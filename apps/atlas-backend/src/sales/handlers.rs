use axum::extract::{Path, Query};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;

use crate::payments::models::Payment;

use super::checkout::{self, CheckoutMode};
use super::models::{
    ClosedShift, CloseShiftRequest, CreateSaleRequest, OpenShiftRequest, PaymentInput, Sale, SaleDetail, SaleLine, Shift,
    VoidSaleRequest,
};
use super::query::{self, SalesPage, SalesQuery};
use super::{receipts, service, settlement};

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
 * Requires `sale.create` at the shift's own store - the same gate as opening it. The response carries the drawer reconciliation: expected cash (float plus cash taken, less cash refunded) against the counted amount.
 */
pub async fn close_shift(
    Path(shift_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<CloseShiftRequest>,
) -> Result<Json<ApiEnvelope<ClosedShift>>, ApiError> {
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
    let cash = crate::reports::service::shift_cash_summary(&mut tx, shift.id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Shift closed.", ClosedShift { shift, cash })));
}

/**
 * `GET /api/v1/sales?store_id=&shift_id=&from=&to=&cashier_user_id=&customer_id=&status=&payment_status=&q=&cursor=&limit=`.
 * With `sale.view`, searches every store that permission reaches; without it, only the caller's own sales in still-open shifts. See `query::search`.
 */
pub async fn list_sales(
    mut tx: TenantTx,
    Query(query): Query<SalesQuery>,
) -> Result<Json<ApiEnvelope<SalesPage>>, ApiError> {
    let page = query::search(&mut tx, &query).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sales fetched.", page)));
}

/// `GET /api/v1/sales/:id` - the sale with its lines and payments. `sale.view` at its store, or the cashier's own sale while the shift is open.
pub async fn get_sale(Path(sale_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<SaleDetail>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let sale = service::get_sale(&mut tx, sale_id).await?;
    query::assert_can_view(&mut tx, user_id, tenant_id, &sale).await?;
    let detail = service::sale_detail(&mut tx, sale).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sale fetched.", detail)));
}

/**
 * `GET /api/v1/sales/by-receipt/:receipt_number`.
 * Requires `sale.view` or `sale.return` at the sale's store - this is how a return starts, from the receipt the customer brings back.
 */
pub async fn get_by_receipt(
    Path(receipt_number): Path<String>,
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<SaleDetail>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let sale_id = query::find_by_receipt(&mut tx, &receipt_number).await?;
    let sale = service::get_sale(&mut tx, sale_id).await?;
    if !roles::service::has_permission(&mut tx, user_id, tenant_id, Some(sale.store_id), "sale.return").await? {
        query::assert_can_view(&mut tx, user_id, tenant_id, &sale).await?;
    }
    let detail = service::sale_detail(&mut tx, sale).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sale fetched.", detail)));
}

#[derive(Deserialize)]
pub struct ReceiptQuery {
    pub format: Option<String>,
    pub width: Option<usize>,
}

/**
 * `GET /api/v1/sales/:id/receipt?format=json|text&width=32|48`.
 * Same access rule as reading the sale. Viewing doesn't count as printing; `text` is the fixed-width render for a thermal printer, and is marked as a reprint once the receipt has already been printed.
 */
pub async fn get_receipt(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
    Query(params): Query<ReceiptQuery>,
) -> Result<Response, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let sale = service::get_sale(&mut tx, sale_id).await?;
    query::assert_can_view(&mut tx, user_id, tenant_id, &sale).await?;
    let view = receipts::view(&mut tx, sale_id).await?;
    tx.commit().await?;
    if params.format.as_deref() == Some("text") {
        let copy = if view.print_count > 0 { Some(view.print_count as i32 + 1) } else { None };
        let text = receipts::render_text(&view.snapshot, params.width.unwrap_or(32), copy);
        return Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], text).into_response());
    }
    return Ok(Json(ApiEnvelope::ok("Receipt fetched.", view)).into_response());
}

#[derive(Deserialize)]
pub struct PrintReceiptRequest {
    pub device_id: Option<String>,
    pub reason: Option<String>,
    pub width: Option<usize>,
}

/**
 * `POST /api/v1/sales/:id/receipt/prints`.
 * The first print needs only the right to read the sale. Every print after that is a reprint and needs `receipt.reprint` at the store; reprints are numbered, labelled on the paper, and audited.
 */
pub async fn print_receipt(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<PrintReceiptRequest>,
) -> Result<Json<ApiEnvelope<receipts::PrintedReceipt>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let sale = service::get_sale(&mut tx, sale_id).await?;
    let already_printed = receipts::print_count(&mut tx, sale_id).await? > 0;
    let can_reprint = roles::service::has_permission(&mut tx, user_id, tenant_id, Some(sale.store_id), "receipt.reprint").await?;
    if already_printed && !can_reprint {
        return Err(ApiError::forbidden("Missing permission: receipt.reprint"));
    }
    if !already_printed && !can_reprint {
        query::assert_can_view(&mut tx, user_id, tenant_id, &sale).await?;
    }
    let printed = receipts::record_print(
        &mut tx,
        tenant_id,
        sale_id,
        user_id,
        body.device_id.as_deref(),
        body.reason.as_deref(),
        body.width.unwrap_or(32),
    )
    .await?;
    if printed.kind == "reprint" {
        audit::record_change(
            &mut tx,
            audit::Change {
                tenant_id,
                actor_user_id: user_id,
                device_id: body.device_id.as_deref(),
                entity_type: "sale",
                entity_id: sale_id,
                action: "receipt.reprint",
                before: None,
                after: Some(serde_json::json!({ "copy_number": printed.copy_number, "reason": body.reason })),
            },
        )
        .await?;
    }
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Receipt printed.", printed)));
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

/// `GET /api/v1/sales/:id/lines`. Same access rule as reading the sale.
pub async fn list_sale_lines(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
) -> Result<Json<ApiEnvelope<Vec<SaleLine>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let sale = service::get_sale(&mut tx, sale_id).await?;
    query::assert_can_view(&mut tx, user_id, tenant_id, &sale).await?;
    let lines = service::list_sale_lines(&mut tx, sale_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Sale lines fetched.", lines)));
}

/**
 * `POST /api/v1/sales/:id/payments`.
 * Requires `sale.create` at the sale's store. Pays down a sale still awaiting payment or a pay-later balance; a payment beyond what's owed is `PAYMENT_OVERPAID`. Once the sale is covered it completes and its stock is taken.
 */
pub async fn add_payment(
    Path(sale_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<PaymentInput>,
) -> Result<Json<ApiEnvelope<Payment>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let store_id = crate::payments::service::sale_store_id(&mut tx, sale_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(store_id), "sale.create").await?;
    let payment = settlement::add_payment(&mut tx, tenant_id, sale_id, user_id, &body).await?;
    audit::record(&mut tx, tenant_id, user_id, "payment", payment.id, "payment.record", serde_json::to_value(&payment).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payment recorded.", payment)));
}
