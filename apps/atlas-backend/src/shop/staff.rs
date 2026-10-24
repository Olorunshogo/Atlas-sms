/*!
 * Staff handling of online orders (`order.manage` at the store): picking, shipping through the courier, delivery,
 * cancelling, and confirming pay-on-delivery cash. Also the courier's signed status callback.
 */
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::{AdminTx, TenantTx};
use crate::integrations::{ShipmentRequest, ShipmentState};
use crate::response::ApiEnvelope;
use crate::roles;
use crate::state::AppState;

use super::checkout::storefront_for;
use super::loyalty;
use super::orders::{self, OrderHeader, OrderView};

pub const COURIER_SIGNATURE_HEADER: &str = "x-courier-signature";

#[derive(Debug, Deserialize)]
pub struct ListOrdersQuery {
    pub store_id: Uuid,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CancelRequest {
    pub reason: String,
}

async fn order_at(tx: &mut TenantTx, order_id: Uuid) -> Result<OrderHeader, ApiError> {
    let order = orders::header(tx, order_id).await?;
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(tx, user_id, tenant_id, Some(order.store_id), "order.manage").await?;
    return Ok(order);
}

async fn finish(mut tx: TenantTx, state: &AppState, order_id: Uuid, action: &str, mail_subject: Option<&str>) -> Result<Json<ApiEnvelope<OrderView>>, ApiError> {
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let view = orders::view(&mut tx, order_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "shop_order", order_id, action, serde_json::to_value(&view.header).ok()).await?;
    let mail = match mail_subject {
        Some(subject) => Some(orders::status_mail(&mut tx, order_id, subject).await?),
        None => None,
    };
    tx.commit().await?;
    orders::send(state.email.as_ref(), mail).await;
    return Ok(Json(ApiEnvelope::ok("Order updated.", view)));
}

fn conflict(message: &str) -> ApiError {
    return ApiError::conflict_with_code(message, "ORDER_STATUS_INVALID");
}

/// `GET /api/v1/shop-orders?store_id=&status=`.
pub async fn list_orders(mut tx: TenantTx, Query(query): Query<ListOrdersQuery>) -> Result<Json<ApiEnvelope<Vec<OrderHeader>>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(query.store_id), "order.manage").await?;
    let list = sqlx::query_as::<_, OrderHeader>(&format!(
        "SELECT {} FROM atlas.shop_orders WHERE store_id = $1 AND ($2::text IS NULL OR status::text = $2) ORDER BY created_at DESC LIMIT 200",
        orders::ORDER_COLUMNS
    ))
    .bind(query.store_id)
    .bind(query.status.as_deref())
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Orders fetched.", list)));
}

/// `GET /api/v1/shop-orders/:id`.
pub async fn show_order(Path(order_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<OrderView>>, ApiError> {
    order_at(&mut tx, order_id).await?;
    let view = orders::view(&mut tx, order_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Order fetched.", view)));
}

/// `POST /api/v1/shop-orders/:id/process`. `paid` -> `processing`: being picked and packed.
pub async fn process(State(state): State<AppState>, Path(order_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<OrderView>>, ApiError> {
    let order = order_at(&mut tx, order_id).await?;
    if order.status != "paid" {
        return Err(conflict("Only a paid order can be processed."));
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    sqlx::query("UPDATE atlas.shop_orders SET status = 'processing' WHERE id = $1").bind(order_id).execute(&mut *tx).await?;
    orders::record_event(&mut tx, tenant_id, order_id, "processing", Some(user_id), None).await?;
    return finish(tx, &state, order_id, "shop_order.process", None).await;
}

/**
 * `POST /api/v1/shop-orders/:id/ship`. Books the courier and records the tracking number. A paid order ships from
 * `paid` or `processing`; a pay-on-delivery order ships before it is paid.
 */
pub async fn ship(State(state): State<AppState>, Path(order_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<OrderView>>, ApiError> {
    let order = order_at(&mut tx, order_id).await?;
    let shippable = order.status == "paid" || order.status == "processing" || (order.status == "pending_payment" && order.payment_method == "pay_on_delivery");
    if !shippable {
        return Err(conflict("This order can't be shipped yet."));
    }
    let booking = state
        .integrations
        .courier
        .create_shipment(&ShipmentRequest { order_number: order.order_number.clone(), address: order.delivery_address.clone(), weight_kg: order.weight_kg })
        .await
        .map_err(|err| ApiError::conflict_with_code(format!("The courier couldn't take the shipment: {}", err), "COURIER_FAILED"))?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let shipment_id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.shipments (id, tenant_id, order_id, courier, tracking_number, label_url) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(shipment_id)
        .bind(tenant_id)
        .bind(order_id)
        .bind(state.integrations.courier.name())
        .bind(&booking.tracking_number)
        .bind(&booking.label_url)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO atlas.shipment_events (id, tenant_id, shipment_id, status, note) VALUES ($1, $2, $3, 'created', 'Booked with the courier')")
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(shipment_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE atlas.shop_orders SET status = 'shipped', shipped_at = now() WHERE id = $1").bind(order_id).execute(&mut *tx).await?;
    orders::record_event(&mut tx, tenant_id, order_id, "shipped", Some(user_id), Some(&format!("Tracking {}", booking.tracking_number))).await?;
    return finish(tx, &state, order_id, "shop_order.ship", Some("Your order has shipped")).await;
}

async fn mark_delivered(conn: &mut PgConnection, state_store: Uuid, tenant_id: Uuid, order_id: Uuid, actor: Option<Uuid>) -> Result<(), ApiError> {
    sqlx::query("UPDATE atlas.shop_orders SET status = 'delivered', delivered_at = now() WHERE id = $1").bind(order_id).execute(&mut *conn).await?;
    orders::record_event(conn, tenant_id, order_id, "delivered", actor, None).await?;
    let storefront = storefront_for(conn, state_store).await?;
    loyalty::earn(conn, &storefront, order_id).await?;
    return Ok(());
}

/// `POST /api/v1/shop-orders/:id/deliver`. For deliveries the courier doesn't report.
pub async fn deliver(State(state): State<AppState>, Path(order_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<OrderView>>, ApiError> {
    let order = order_at(&mut tx, order_id).await?;
    if order.status != "shipped" {
        return Err(conflict("Only a shipped order can be delivered."));
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    mark_delivered(&mut tx, order.store_id, tenant_id, order_id, Some(user_id)).await?;
    return finish(tx, &state, order_id, "shop_order.deliver", Some("Your order was delivered")).await;
}

/// `POST /api/v1/shop-orders/:id/cancel`. Refunds a paid order through the gateway and restocks it.
pub async fn cancel(State(state): State<AppState>, Path(order_id): Path<Uuid>, mut tx: TenantTx, Json(body): Json<CancelRequest>) -> Result<Json<ApiEnvelope<OrderView>>, ApiError> {
    let order = order_at(&mut tx, order_id).await?;
    let reason = body.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::bad_request_with_code("reason is required", "VALIDATION_FAILED"));
    }
    let storefront = storefront_for(&mut tx, order.store_id).await?;
    let user_id = tx.user_id;
    orders::cancel(&mut tx, state.integrations.payments.as_ref(), &storefront, order_id, Some(user_id), reason).await?;
    return finish(tx, &state, order_id, "shop_order.cancel", Some("Your order was cancelled")).await;
}

/// `POST /api/v1/shop-orders/:id/collect-payment`. Staff confirm the cash for a pay-on-delivery order.
pub async fn collect_payment(State(state): State<AppState>, Path(order_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<OrderView>>, ApiError> {
    let order = order_at(&mut tx, order_id).await?;
    if order.payment_method != "pay_on_delivery" || order.paid_at.is_some() || order.status == "cancelled" {
        return Err(conflict("Only an unpaid pay-on-delivery order can be collected."));
    }
    let updated = sqlx::query("UPDATE atlas.online_payments SET status = 'succeeded', confirmed_at = now() WHERE order_id = $1 AND status = 'pending' AND method = 'pay_on_delivery'")
        .bind(order_id)
        .execute(&mut *tx)
        .await?;
    if updated.rows_affected() == 0 {
        return Err(conflict("There's no pending pay-on-delivery payment on this order."));
    }
    let storefront = storefront_for(&mut tx, order.store_id).await?;
    orders::complete(&mut tx, &storefront, order_id).await?;
    return finish(tx, &state, order_id, "shop_order.collect_payment", Some("Payment received")).await;
}

/**
 * `POST /api/v1/shop-webhooks/couriers`. The courier's signed status update. In transit is recorded; delivered
 * moves the order to `delivered` and tells the customer.
 */
pub async fn courier_webhook(State(state): State<AppState>, headers: HeaderMap, mut tx: AdminTx, body: Bytes) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let signature = headers.get(COURIER_SIGNATURE_HEADER).and_then(|v| v.to_str().ok());
    let event = state
        .integrations
        .courier
        .parse_webhook(signature, &body)
        .map_err(|err| ApiError::Unauthorized(format!("Rejected webhook: {}", err)))?;
    let found: Option<(Uuid, Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT s.id, s.tenant_id, s.order_id, s.status::text FROM atlas.shipments s WHERE s.tracking_number = $1 FOR UPDATE",
    )
    .bind(&event.tracking_number)
    .fetch_optional(&mut *tx)
    .await?;
    let (shipment_id, tenant_id, order_id, current) = found.ok_or_else(|| ApiError::not_found("Unknown shipment."))?;
    let status = match event.state {
        ShipmentState::InTransit => "in_transit",
        ShipmentState::Delivered => "delivered",
        ShipmentState::Failed => "failed",
    };
    let mut mail = None;
    if current != "delivered" && current != status {
        sqlx::query("UPDATE atlas.shipments SET status = $2::atlas.shipment_status, updated_at = now() WHERE id = $1")
            .bind(shipment_id)
            .bind(status)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO atlas.shipment_events (id, tenant_id, shipment_id, status, note) VALUES ($1, $2, $3, $4::atlas.shipment_status, $5)")
            .bind(Uuid::now_v7())
            .bind(tenant_id)
            .bind(shipment_id)
            .bind(status)
            .bind(&event.note)
            .execute(&mut *tx)
            .await?;
        let order = orders::header(&mut tx, order_id).await?;
        match event.state {
            ShipmentState::Delivered if order.status == "shipped" => {
                mark_delivered(&mut tx, order.store_id, tenant_id, order_id, None).await?;
                mail = Some(orders::status_mail(&mut tx, order_id, "Your order was delivered").await?);
            }
            ShipmentState::InTransit => mail = Some(orders::status_mail(&mut tx, order_id, "Your order is on its way").await?),
            ShipmentState::Failed => {
                orders::record_event(&mut tx, tenant_id, order_id, "shipped", None, Some(event.note.as_deref().unwrap_or("Delivery attempt failed"))).await?;
            }
            _ => {}
        }
    }
    tx.commit().await?;
    orders::send(state.email.as_ref(), mail).await;
    return Ok(Json(ApiEnvelope::ok("Webhook received.", json!({ "shipment_id": shipment_id }))));
}
