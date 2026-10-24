/*!
 * Checkout. The quote is advisory; placing the order prices it again on the server and refuses a total that moved
 * since the shopper saw it (FR-11.4), checks and reserves stock, numbers the invoice, and starts a payment with the
 * gateway. Only the gateway's signed callback marks it paid.
 */
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::crypto;
use crate::error::{codes, ApiError};
use crate::extractors::AdminTx;
use crate::integrations::{PaymentRequest, PaymentState};
use crate::inventory;
use crate::response::ApiEnvelope;
use crate::state::AppState;
use crate::validate::required_text;

use super::cart;
use super::context::{ShopTx, Storefront, STOREFRONT_COLUMNS};
use super::loyalty;
use super::orders::{self, Buyer, OrderView, PAYMENT_COLUMNS};
use super::pricing::{self, PriceRequest, PricedOrder};
use super::promotions;

pub const PAYMENT_SIGNATURE_HEADER: &str = "x-gateway-signature";
const METHODS: [&str; 5] = ["card", "bank_transfer", "ussd", "mobile_wallet", "pay_on_delivery"];

#[derive(Debug, Deserialize)]
pub struct QuoteRequest {
    pub shipping_zone_id: Uuid,
    pub coupon_code: Option<String>,
    #[serde(default)]
    pub points: i64,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlaceOrderRequest {
    pub idempotency_key: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub full_name: Option<String>,
    pub delivery_address: Value,
    pub shipping_zone_id: Uuid,
    pub payment_method: String,
    pub coupon_code: Option<String>,
    #[serde(default)]
    pub points: i64,
    /// The total the shopper was shown; a different one is refused with the new quote.
    pub expected_total_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct RetryPaymentRequest {
    pub method: String,
    pub idempotency_key: String,
    pub access_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentStart {
    pub id: Uuid,
    pub method: String,
    pub status: String,
    pub amount_minor: i64,
    pub redirect_url: Option<String>,
    pub instructions: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct PlacedOrder {
    pub order: OrderView,
    pub payment: PaymentStart,
    /// For a guest, the secret that opens this order later. Shown once.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

fn window(storefront: &Storefront, method: &str) -> Option<DateTime<Utc>> {
    let minutes = match method {
        "pay_on_delivery" => return None,
        "ussd" => storefront.ussd_window_minutes,
        "bank_transfer" => storefront.bank_transfer_window_minutes,
        _ => storefront.reservation_minutes,
    };
    return Some(Utc::now() + Duration::minutes(i64::from(minutes)));
}

/// Pay on delivery is limited by order value, the zone, and whether the store wants an account (FR-12.5).
async fn check_method(conn: &mut PgConnection, storefront: &Storefront, method: &str, total_minor: i64, zone_id: Uuid, shopper_id: Option<Uuid>) -> Result<(), ApiError> {
    if !METHODS.contains(&method) {
        return Err(ApiError::bad_request_with_code("payment_method must be card, bank_transfer, ussd, mobile_wallet or pay_on_delivery", "VALIDATION_FAILED"));
    }
    if method != "pay_on_delivery" {
        return Ok(());
    }
    let zone_allows: bool = sqlx::query_scalar("SELECT allows_pay_on_delivery FROM atlas.shipping_zones WHERE id = $1")
        .bind(zone_id)
        .fetch_one(conn)
        .await?;
    let reason = match storefront.pay_on_delivery_max_minor {
        None => Some("This shop doesn't offer pay on delivery."),
        Some(max) if total_minor > max => Some("This order is over the pay on delivery limit."),
        _ if !zone_allows => Some("Pay on delivery isn't available in this delivery zone."),
        _ if storefront.pay_on_delivery_requires_account && shopper_id.is_none() => Some("Sign in to pay on delivery."),
        _ => None,
    };
    return match reason {
        Some(message) => Err(ApiError::conflict_with_code(message, "PAY_ON_DELIVERY_UNAVAILABLE")),
        None => Ok(()),
    };
}

async fn start_payment(
    conn: &mut PgConnection,
    state: &AppState,
    tenant_id: Uuid,
    order_id: Uuid,
    method: &str,
    amount_minor: i64,
    currency: &str,
    email: &str,
    idempotency_key: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<PaymentStart, ApiError> {
    let (gateway_ref, redirect_url, instructions) = if method == "pay_on_delivery" {
        (None, None, Some("Pay the courier when your order arrives.".to_string()))
    } else {
        let init = state
            .integrations
            .payments
            .initiate(&PaymentRequest {
                method: method.to_string(),
                amount_minor,
                currency: currency.to_string(),
                idempotency_key: idempotency_key.to_string(),
                email: email.to_string(),
            })
            .await
            .map_err(|err| ApiError::conflict_with_code(format!("The payment couldn't be started: {}", err), "PAYMENT_START_FAILED"))?;
        (Some(init.gateway_ref), init.redirect_url, init.instructions)
    };
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.online_payments (id, tenant_id, order_id, method, amount_minor, idempotency_key, gateway_ref, redirect_url)
         VALUES ($1, $2, $3, $4::atlas.online_payment_method, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(order_id)
    .bind(method)
    .bind(amount_minor)
    .bind(idempotency_key)
    .bind(&gateway_ref)
    .bind(&redirect_url)
    .execute(conn)
    .await?;
    return Ok(PaymentStart {
        id,
        method: method.to_string(),
        status: "pending".to_string(),
        amount_minor,
        redirect_url,
        instructions,
        expires_at,
    });
}

fn total_changed(quote: &PricedOrder) -> Response {
    return (
        StatusCode::CONFLICT,
        Json(json!({
            "success": false,
            "code": "TOTAL_CHANGED",
            "message": "Prices, discounts or stock changed since you looked. Check the new total and confirm.",
            "data": { "quote": quote },
        })),
    )
        .into_response();
}

async fn buyer_defaults(tx: &mut ShopTx) -> Result<Option<(String, String, Option<String>)>, ApiError> {
    let Some(shopper_id) = tx.shopper_id else { return Ok(None) };
    return sqlx::query_as("SELECT email, full_name, phone FROM atlas.shoppers WHERE id = $1")
        .bind(shopper_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(ApiError::from);
}

// === Handlers

/// `POST /api/v1/shop/:handle/checkout/quote`. The breakdown for the current cart; reserves nothing.
pub async fn quote(State(state): State<AppState>, mut tx: ShopTx, Json(body): Json<QuoteRequest>) -> Result<Json<ApiEnvelope<PricedOrder>>, ApiError> {
    let storefront = tx.storefront.clone();
    orders::expire_overdue(&mut tx, state.integrations.payments.as_ref(), &storefront).await?;
    let lines = cart::lines(&mut tx).await?;
    let email = match body.email.clone() {
        Some(email) => Some(email),
        None => buyer_defaults(&mut tx).await?.map(|(email, _, _)| email),
    };
    let shopper_id = tx.shopper_id;
    let priced = pricing::price(
        &mut tx,
        &storefront,
        &PriceRequest { shopper_id, email: email.as_deref(), lines: &lines, coupon_code: body.coupon_code.as_deref(), shipping_zone_id: body.shipping_zone_id, points: body.points },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Quote calculated.", priced)));
}

/// `POST /api/v1/shop/:handle/checkout`. Places the order from the cart and starts its payment.
pub async fn place_order(State(state): State<AppState>, mut tx: ShopTx, Json(body): Json<PlaceOrderRequest>) -> Result<Response, ApiError> {
    let storefront = tx.storefront.clone();
    let (tenant_id, store_id, shopper_id) = (tx.tenant_id, tx.store_id, tx.shopper_id);
    let key = required_text("idempotency_key", &body.idempotency_key, 100)?;
    orders::expire_overdue(&mut tx, state.integrations.payments.as_ref(), &storefront).await?;

    // The same key again is the same order, not a second one
    let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM atlas.shop_orders WHERE store_id = $1 AND idempotency_key = $2")
        .bind(store_id)
        .bind(&key)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(order_id) = existing {
        let order = orders::view(&mut tx, order_id).await?;
        if order.header.shopper_id != shopper_id {
            return Err(ApiError::conflict_with_code("That checkout key was already used.", codes::IDEMPOTENCY_CONFLICT));
        }
        let payment = order.payments.last().cloned().ok_or_else(|| ApiError::Internal("order without a payment".to_string()))?;
        tx.commit().await?;
        let start = PaymentStart {
            id: payment.id,
            method: payment.method,
            status: payment.status,
            amount_minor: payment.amount_minor,
            redirect_url: payment.redirect_url,
            instructions: None,
            expires_at: order.header.payment_expires_at,
        };
        return Ok(Json(ApiEnvelope::ok("Order already placed.", PlacedOrder { order, payment: start, access_token: None })).into_response());
    }

    let defaults = buyer_defaults(&mut tx).await?;
    let email = body.email.clone().or_else(|| defaults.as_ref().map(|d| d.0.clone())).map(|e| e.trim().to_lowercase()).filter(|e| e.contains('@'));
    let full_name = body.full_name.clone().or_else(|| defaults.as_ref().map(|d| d.1.clone())).filter(|n| !n.trim().is_empty());
    let phone = body.phone.clone().or_else(|| defaults.as_ref().and_then(|d| d.2.clone())).filter(|p| !p.trim().is_empty());
    let (Some(email), Some(full_name), Some(phone)) = (email, full_name, phone) else {
        return Err(ApiError::bad_request_with_code("Email, full name and phone are needed to check out.", "VALIDATION_FAILED"));
    };
    if !body.delivery_address.is_object() || body.delivery_address.get("line1").and_then(Value::as_str).is_none_or(|v| v.trim().is_empty()) {
        return Err(ApiError::bad_request_with_code("delivery_address needs at least line1.", "VALIDATION_FAILED"));
    }

    let lines = cart::lines(&mut tx).await?;
    let product_ids: Vec<Uuid> = lines.iter().map(|(id, _)| *id).collect();
    inventory::service::lock_products(&mut tx, store_id, &product_ids).await?;
    let priced = pricing::price(
        &mut tx,
        &storefront,
        &PriceRequest {
            shopper_id,
            email: Some(&email),
            lines: &lines,
            coupon_code: body.coupon_code.as_deref(),
            shipping_zone_id: body.shipping_zone_id,
            points: body.points,
        },
    )
    .await?;
    if let (Some(_), Some(error)) = (body.coupon_code.as_deref().filter(|c| !c.trim().is_empty()), &priced.coupon_error) {
        return Err(ApiError::conflict_with_code(error.clone(), "COUPON_NOT_APPLICABLE"));
    }
    if let Some(short) = priced.lines.iter().find(|l| l.quantity > l.available) {
        return Err(ApiError::conflict_with_code(
            format!("Only {} of {} left.", short.available.normalize(), short.product_name),
            codes::PRODUCT_OUT_OF_STOCK,
        ));
    }
    if priced.total_minor != body.expected_total_minor {
        return Ok(total_changed(&priced));
    }
    check_method(&mut tx, &storefront, &body.payment_method, priced.total_minor, priced.shipping_zone_id, shopper_id).await?;

    let shop_prefix = orders::prefix(&mut tx, store_id).await?;
    let order_number = format!("{}-W{:06}", shop_prefix, orders::next_number(&mut tx, tenant_id, store_id, "order").await?);
    let invoice_number = format!("INV-{}-{:08}", shop_prefix, orders::next_number(&mut tx, tenant_id, store_id, "invoice").await?);
    let buyer = Buyer { full_name: &full_name, email: &email, phone: &phone, address: &body.delivery_address };
    let snapshot = orders::invoice_snapshot(&mut tx, store_id, &invoice_number, &order_number, &buyer, &priced).await?;
    let expires_at = window(&storefront, &body.payment_method);
    let access_token = crypto::generate_opaque_token();
    let customer_id: Option<Uuid> = match shopper_id {
        Some(id) => sqlx::query_scalar("SELECT customer_id FROM atlas.shoppers WHERE id = $1").bind(id).fetch_optional(&mut *tx).await?,
        None => None,
    };

    let order_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.shop_orders
            (id, tenant_id, store_id, order_number, invoice_number, shopper_id, customer_id, email, phone, full_name,
             delivery_address, shipping_zone_id, weight_kg, payment_method, payment_expires_at, prices_include_tax, currency,
             subtotal_minor, discount_minor, shipping_minor, tax_minor, total_minor, coupon_code, points_redeemed, points_value_minor,
             idempotency_key, access_token_hash, invoice_snapshot)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14::atlas.online_payment_method, $15, $16, $17,
                 $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28)",
    )
    .bind(order_id)
    .bind(tenant_id)
    .bind(store_id)
    .bind(&order_number)
    .bind(&invoice_number)
    .bind(shopper_id)
    .bind(customer_id)
    .bind(&email)
    .bind(&phone)
    .bind(&full_name)
    .bind(&body.delivery_address)
    .bind(priced.shipping_zone_id)
    .bind(priced.weight_kg)
    .bind(&body.payment_method)
    .bind(expires_at)
    .bind(priced.prices_include_tax)
    .bind(&priced.currency)
    .bind(priced.subtotal_minor)
    .bind(priced.discount_minor)
    .bind(priced.shipping_minor)
    .bind(priced.tax_minor)
    .bind(priced.total_minor)
    .bind(body.coupon_code.as_deref().map(|c| c.trim().to_uppercase()).filter(|c| !c.is_empty()))
    .bind(priced.points_redeemed as i32)
    .bind(priced.points_value_minor)
    .bind(&key)
    .bind(crypto::hash_lookup_token(&access_token))
    .bind(&snapshot)
    .execute(&mut *tx)
    .await?;

    for line in &priced.lines {
        sqlx::query(
            "INSERT INTO atlas.shop_order_lines
                (id, tenant_id, order_id, product_id, product_name, sku, quantity, unit_price_minor, discount_minor, net_minor,
                 tax_rate_basis_points, tax_minor, total_minor, unit_cost_minor)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(order_id)
        .bind(line.product_id)
        .bind(&line.product_name)
        .bind(&line.sku)
        .bind(line.quantity)
        .bind(line.unit_price_minor)
        .bind(line.discount_minor)
        .bind(line.net_minor)
        .bind(line.tax_rate_basis_points as i32)
        .bind(line.tax_minor)
        .bind(line.total_minor)
        .bind(line.unit_cost_minor)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO atlas.stock_reservations (id, tenant_id, store_id, order_id, product_id, quantity, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(store_id)
        .bind(order_id)
        .bind(line.product_id)
        .bind(line.quantity)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;
    }
    promotions::redeem(&mut tx, tenant_id, order_id, shopper_id, &email, &priced.promotions).await?;
    if let Some(shopper_id) = shopper_id {
        loyalty::record_redeem(&mut tx, tenant_id, shopper_id, order_id, priced.points_redeemed).await?;
    }
    orders::record_event(&mut tx, tenant_id, order_id, "pending_payment", None, Some("Order placed")).await?;
    let payment = start_payment(&mut tx, &state, tenant_id, order_id, &body.payment_method, priced.total_minor, &priced.currency, &email, &key, expires_at).await?;
    cart::clear(&mut tx).await?;
    let order = orders::view(&mut tx, order_id).await?;
    let mail = orders::invoice_mail(&mut tx, order_id).await?;
    tx.commit().await?;
    orders::send(state.email.as_ref(), Some(mail)).await;

    let access_token = if shopper_id.is_none() { Some(access_token) } else { None };
    return Ok(Json(ApiEnvelope::ok("Order placed.", PlacedOrder { order, payment, access_token })).into_response());
}

/// `POST /api/v1/shop/:handle/orders/:order_id/payments`. Retry, or switch method, without re-entering the order.
pub async fn retry_payment(
    State(state): State<AppState>,
    Path((_handle, order_id)): Path<(String, Uuid)>,
    mut tx: ShopTx,
    Json(body): Json<RetryPaymentRequest>,
) -> Result<Json<ApiEnvelope<PaymentStart>>, ApiError> {
    let storefront = tx.storefront.clone();
    orders::expire_overdue(&mut tx, state.integrations.payments.as_ref(), &storefront).await?;
    let order = orders::authorize(&mut tx, order_id, body.access_token.as_deref()).await?;
    if order.status != "pending_payment" {
        return Err(ApiError::conflict_with_code("This order isn't waiting for payment.", "ORDER_NOT_PAYABLE"));
    }
    let key = required_text("idempotency_key", &body.idempotency_key, 100)?;
    let shopper_id = tx.shopper_id;
    check_method(&mut tx, &storefront, &body.method, order.total_minor, order.shipping_zone_id, shopper_id.or(order.shopper_id)).await?;
    let repeated = sqlx::query_as::<_, orders::OnlinePayment>(&format!(
        "SELECT {PAYMENT_COLUMNS} FROM atlas.online_payments WHERE order_id = $1 AND idempotency_key = $2"
    ))
    .bind(order_id)
    .bind(&key)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(payment) = repeated {
        tx.commit().await?;
        return Ok(Json(ApiEnvelope::ok(
            "Payment already started.",
            PaymentStart {
                id: payment.id,
                method: payment.method,
                status: payment.status,
                amount_minor: payment.amount_minor,
                redirect_url: payment.redirect_url,
                instructions: None,
                expires_at: order.payment_expires_at,
            },
        )));
    }
    sqlx::query("UPDATE atlas.online_payments SET status = 'failed', failure_reason = 'Replaced by a new attempt' WHERE order_id = $1 AND status = 'pending'")
        .bind(order_id)
        .execute(&mut *tx)
        .await?;
    // Switching to pay on delivery holds the stock until collection
    let expires_at = if body.method == "pay_on_delivery" { None } else { order.payment_expires_at };
    sqlx::query("UPDATE atlas.shop_orders SET payment_method = $2::atlas.online_payment_method, payment_expires_at = $3 WHERE id = $1")
        .bind(order_id)
        .bind(&body.method)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE atlas.stock_reservations SET expires_at = $2 WHERE order_id = $1 AND status = 'active'")
        .bind(order_id)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;
    let tenant_id = tx.tenant_id;
    let payment = start_payment(&mut tx, &state, tenant_id, order_id, &body.method, order.total_minor, &order.currency, &order.email, &key, expires_at).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Payment started.", payment)));
}

pub async fn storefront_for(conn: &mut PgConnection, store_id: Uuid) -> Result<Storefront, ApiError> {
    return sqlx::query_as::<_, Storefront>(&format!("SELECT {STOREFRONT_COLUMNS} FROM atlas.storefronts WHERE store_id = $1"))
        .bind(store_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::not_found("This store has no storefront."));
}

/**
 * `POST /api/v1/shop-webhooks/payments`. The gateway's signed callback, the only thing that marks a payment
 * succeeded (FR-12.2). Repeats are harmless. Money that arrives for a cancelled order is refunded at once.
 */
pub async fn payment_webhook(State(state): State<AppState>, headers: HeaderMap, mut tx: AdminTx, body: Bytes) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let signature = headers.get(PAYMENT_SIGNATURE_HEADER).and_then(|v| v.to_str().ok());
    let event = state
        .integrations
        .payments
        .parse_webhook(signature, &body)
        .map_err(|err| ApiError::Unauthorized(format!("Rejected webhook: {}", err)))?;
    let found: Option<(Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT p.id, p.order_id, p.status::text FROM atlas.online_payments p WHERE p.gateway_ref = $1 FOR UPDATE",
    )
    .bind(&event.gateway_ref)
    .fetch_optional(&mut *tx)
    .await?;
    let (payment_id, order_id, status) = found.ok_or_else(|| ApiError::not_found("Unknown payment."))?;
    let order = orders::header(&mut tx, order_id).await?;
    let storefront = storefront_for(&mut tx, order.store_id).await?;
    let mut mail = None;
    if status == "pending" || status == "expired" {
        match event.state {
            PaymentState::Succeeded if order.status == "cancelled" => {
                let _ = state.integrations.payments.refund(&event.gateway_ref, order.total_minor).await;
                sqlx::query("UPDATE atlas.online_payments SET status = 'refunded', confirmed_at = now() WHERE id = $1").bind(payment_id).execute(&mut *tx).await?;
            }
            PaymentState::Succeeded => {
                sqlx::query("UPDATE atlas.online_payments SET status = 'succeeded', confirmed_at = now() WHERE id = $1").bind(payment_id).execute(&mut *tx).await?;
                orders::complete(&mut tx, &storefront, order_id).await?;
                mail = Some(orders::status_mail(&mut tx, order_id, "Payment received").await?);
            }
            PaymentState::Failed => {
                sqlx::query("UPDATE atlas.online_payments SET status = 'failed', failure_reason = $2 WHERE id = $1")
                    .bind(payment_id)
                    .bind(event.reason.as_deref().unwrap_or("Declined"))
                    .execute(&mut *tx)
                    .await?;
            }
            PaymentState::Pending => {}
        }
    }
    tx.commit().await?;
    orders::send(state.email.as_ref(), mail).await;
    return Ok(Json(ApiEnvelope::ok("Webhook received.", json!({ "payment_id": payment_id }))));
}

// === Shopper order views

#[derive(Debug, Deserialize)]
pub struct AccessQuery {
    pub access_token: Option<String>,
    /// `text` for a printable invoice instead of the snapshot.
    pub format: Option<String>,
}

/// `GET /api/v1/shop/:handle/orders`. The signed-in shopper's orders, newest first.
pub async fn my_orders(mut tx: ShopTx) -> Result<Json<ApiEnvelope<Vec<orders::OrderHeader>>>, ApiError> {
    let shopper_id = tx.require_shopper()?;
    let store_id = tx.store_id;
    let list = sqlx::query_as::<_, orders::OrderHeader>(&format!(
        "SELECT {} FROM atlas.shop_orders WHERE shopper_id = $1 AND store_id = $2 ORDER BY created_at DESC",
        orders::ORDER_COLUMNS
    ))
    .bind(shopper_id)
    .bind(store_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Orders fetched.", list)));
}

/// `GET /api/v1/shop/:handle/orders/:order_id?access_token=`. Status history, payments and tracking included.
pub async fn show_order(
    State(state): State<AppState>,
    Path((_handle, order_id)): Path<(String, Uuid)>,
    mut tx: ShopTx,
    Query(query): Query<AccessQuery>,
) -> Result<Json<ApiEnvelope<OrderView>>, ApiError> {
    let storefront = tx.storefront.clone();
    orders::expire_overdue(&mut tx, state.integrations.payments.as_ref(), &storefront).await?;
    orders::authorize(&mut tx, order_id, query.access_token.as_deref()).await?;
    let order = orders::view(&mut tx, order_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Order fetched.", order)));
}

/// `GET /api/v1/shop/:handle/orders/:order_id/invoice?access_token=&format=`. The issued invoice, or its text; a receipt once paid.
pub async fn invoice(Path((_handle, order_id)): Path<(String, Uuid)>, mut tx: ShopTx, Query(query): Query<AccessQuery>) -> Result<Response, ApiError> {
    orders::authorize(&mut tx, order_id, query.access_token.as_deref()).await?;
    let (snapshot, receipt_number): (Value, Option<String>) = sqlx::query_as("SELECT invoice_snapshot, receipt_number FROM atlas.shop_orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    if query.format.as_deref() == Some("text") {
        return Ok(([(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")], orders::render(&snapshot, receipt_number.as_deref())).into_response());
    }
    return Ok(Json(ApiEnvelope::ok("Invoice fetched.", json!({ "invoice": snapshot, "receipt_number": receipt_number }))).into_response());
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TrackingEvent {
    pub status: String,
    pub note: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct Tracking {
    pub order_status: String,
    pub courier: Option<String>,
    pub tracking_number: Option<String>,
    pub events: Vec<TrackingEvent>,
}

/// `GET /api/v1/shop/:handle/orders/:order_id/tracking?access_token=`.
pub async fn tracking(Path((_handle, order_id)): Path<(String, Uuid)>, mut tx: ShopTx, Query(query): Query<AccessQuery>) -> Result<Json<ApiEnvelope<Tracking>>, ApiError> {
    let order = orders::authorize(&mut tx, order_id, query.access_token.as_deref()).await?;
    let shipment: Option<(Uuid, String, String)> = sqlx::query_as("SELECT id, courier, tracking_number FROM atlas.shipments WHERE order_id = $1 ORDER BY created_at DESC LIMIT 1")
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await?;
    let events = match &shipment {
        Some((shipment_id, _, _)) => sqlx::query_as::<_, TrackingEvent>(
            "SELECT status::text AS status, note, occurred_at FROM atlas.shipment_events WHERE shipment_id = $1 ORDER BY occurred_at, id",
        )
        .bind(shipment_id)
        .fetch_all(&mut *tx)
        .await?,
        None => Vec::new(),
    };
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok(
        "Tracking fetched.",
        Tracking {
            order_status: order.status,
            courier: shipment.as_ref().map(|s| s.1.clone()),
            tracking_number: shipment.map(|s| s.2),
            events,
        },
    )));
}
