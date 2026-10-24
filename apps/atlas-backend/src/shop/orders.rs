/*!
 * Online orders after checkout: completing a paid order, expiring an unpaid one, cancelling, the order and invoice
 * views, and the emails that go with each step.
 *
 * Lifecycle (FR-15): `pending_payment` -> `paid` -> `processing` -> `shipped` -> `delivered`, with `cancelled` and
 * `expired` as exits. Every change is kept in `shop_order_events`. Stock is held by reservations until payment and
 * only then leaves through the inventory service, as a `sale` movement referencing the order.
 */
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::crypto;
use crate::email::EmailProvider;
use crate::error::ApiError;
use crate::integrations::{PaymentGateway, PaymentState};
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};
use crate::sales::receipts;

use super::context::{ShopTx, Storefront};
use super::loyalty;
use super::pricing::PricedOrder;
use super::promotions;

pub const ORDER_COLUMNS: &str = "id, store_id, order_number, invoice_number, receipt_number, shopper_id, email, phone, full_name,
     status::text AS status, delivery_address, shipping_zone_id, weight_kg, payment_method::text AS payment_method, payment_expires_at,
     currency, subtotal_minor, discount_minor, shipping_minor, tax_minor, total_minor, coupon_code, points_redeemed,
     points_value_minor, paid_at, shipped_at, delivered_at, cancelled_at, created_at";

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct OrderHeader {
    pub id: Uuid,
    pub store_id: Uuid,
    pub order_number: String,
    pub invoice_number: String,
    pub receipt_number: Option<String>,
    pub shopper_id: Option<Uuid>,
    pub email: String,
    pub phone: String,
    pub full_name: String,
    pub status: String,
    pub delivery_address: Value,
    pub shipping_zone_id: Uuid,
    pub weight_kg: Decimal,
    pub payment_method: String,
    pub payment_expires_at: Option<DateTime<Utc>>,
    pub currency: String,
    pub subtotal_minor: i64,
    pub discount_minor: i64,
    pub shipping_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub coupon_code: Option<String>,
    pub points_redeemed: i32,
    pub points_value_minor: i64,
    pub paid_at: Option<DateTime<Utc>>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrderLine {
    pub product_id: Uuid,
    pub product_name: String,
    pub sku: Option<String>,
    pub quantity: Decimal,
    pub unit_price_minor: i64,
    pub discount_minor: i64,
    pub net_minor: i64,
    pub tax_rate_basis_points: i32,
    pub tax_minor: i64,
    pub total_minor: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrderEvent {
    pub status: String,
    pub actor_user_id: Option<Uuid>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct OnlinePayment {
    pub id: Uuid,
    pub method: String,
    pub status: String,
    pub amount_minor: i64,
    pub gateway_ref: Option<String>,
    pub redirect_url: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Shipment {
    pub id: Uuid,
    pub courier: String,
    pub tracking_number: String,
    pub label_url: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct OrderView {
    #[serde(flatten)]
    pub header: OrderHeader,
    pub lines: Vec<OrderLine>,
    pub events: Vec<OrderEvent>,
    pub payments: Vec<OnlinePayment>,
    pub shipment: Option<Shipment>,
}

pub const PAYMENT_COLUMNS: &str =
    "id, method::text AS method, status::text AS status, amount_minor, gateway_ref, redirect_url, failure_reason, created_at, confirmed_at";

// === Reading

pub async fn header(conn: &mut PgConnection, order_id: Uuid) -> Result<OrderHeader, ApiError> {
    return sqlx::query_as::<_, OrderHeader>(&format!("SELECT {ORDER_COLUMNS} FROM atlas.shop_orders WHERE id = $1"))
        .bind(order_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::not_found("Order not found."));
}

pub async fn view(conn: &mut PgConnection, order_id: Uuid) -> Result<OrderView, ApiError> {
    let header = header(conn, order_id).await?;
    let lines = sqlx::query_as::<_, OrderLine>(
        "SELECT product_id, product_name, sku, quantity, unit_price_minor, discount_minor, net_minor, tax_rate_basis_points, tax_minor, total_minor
         FROM atlas.shop_order_lines WHERE order_id = $1 ORDER BY id",
    )
    .bind(order_id)
    .fetch_all(&mut *conn)
    .await?;
    let events = sqlx::query_as::<_, OrderEvent>(
        "SELECT status::text AS status, actor_user_id, note, created_at FROM atlas.shop_order_events WHERE order_id = $1 ORDER BY created_at, id",
    )
    .bind(order_id)
    .fetch_all(&mut *conn)
    .await?;
    let payments = sqlx::query_as::<_, OnlinePayment>(&format!(
        "SELECT {PAYMENT_COLUMNS} FROM atlas.online_payments WHERE order_id = $1 ORDER BY created_at, id"
    ))
    .bind(order_id)
    .fetch_all(&mut *conn)
    .await?;
    let shipment = sqlx::query_as::<_, Shipment>(
        "SELECT id, courier, tracking_number, label_url, status::text AS status, created_at, updated_at
         FROM atlas.shipments WHERE order_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(order_id)
    .fetch_optional(conn)
    .await?;
    return Ok(OrderView { header, lines, events, payments, shipment });
}

/// A shopper sees their own orders; a guest needs the secret link token they were given at checkout.
pub async fn authorize(tx: &mut ShopTx, order_id: Uuid, access_token: Option<&str>) -> Result<OrderHeader, ApiError> {
    let found = header(tx, order_id).await?;
    let owner = tx.shopper_id.is_some() && found.shopper_id == tx.shopper_id;
    let token_ok = match access_token {
        Some(token) => {
            let expected: String = sqlx::query_scalar("SELECT access_token_hash FROM atlas.shop_orders WHERE id = $1")
                .bind(order_id)
                .fetch_one(&mut **tx)
                .await?;
            expected == crypto::hash_lookup_token(token)
        }
        None => false,
    };
    if found.store_id != tx.store_id || !(owner || token_ok) {
        return Err(ApiError::not_found("Order not found."));
    }
    return Ok(found);
}

// === Numbering and invoices

pub async fn next_number(conn: &mut PgConnection, tenant_id: Uuid, store_id: Uuid, kind: &str) -> Result<i64, ApiError> {
    return sqlx::query_scalar(
        "INSERT INTO atlas.shop_number_sequences (store_id, kind, tenant_id, last_value) VALUES ($1, $2, $3, 1)
         ON CONFLICT (store_id, kind) DO UPDATE SET last_value = atlas.shop_number_sequences.last_value + 1
         RETURNING last_value",
    )
    .bind(store_id)
    .bind(kind)
    .bind(tenant_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn prefix(conn: &mut PgConnection, store_id: Uuid) -> Result<String, ApiError> {
    return receipts::store_prefix(conn, store_id).await;
}

pub struct Buyer<'a> {
    pub full_name: &'a str,
    pub email: &'a str,
    pub phone: &'a str,
    pub address: &'a Value,
}

/// The issued invoice, kept as written and never edited (FR-14.3). Tax is broken down by rate.
pub async fn invoice_snapshot(conn: &mut PgConnection, store_id: Uuid, invoice_number: &str, order_number: &str, buyer: &Buyer<'_>, priced: &PricedOrder) -> Result<Value, ApiError> {
    let (store_name, address, phone): (String, Option<String>, Option<String>) = sqlx::query_as("SELECT name, address, phone FROM atlas.stores WHERE id = $1")
        .bind(store_id)
        .fetch_one(conn)
        .await?;
    let mut by_rate: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
    for line in &priced.lines {
        *by_rate.entry(line.tax_rate_basis_points).or_default() += line.tax_minor;
    }
    return Ok(json!({
        "invoice_number": invoice_number,
        "order_number": order_number,
        "issued_at": Utc::now(),
        "currency": priced.currency,
        "prices_include_tax": priced.prices_include_tax,
        "seller": { "name": store_name, "address": address, "phone": phone },
        "buyer": { "name": buyer.full_name, "email": buyer.email, "phone": buyer.phone, "address": buyer.address },
        "lines": priced.lines.iter().map(|l| json!({
            "product_id": l.product_id, "name": l.product_name, "sku": l.sku, "quantity": l.quantity,
            "unit_price_minor": l.unit_price_minor, "discount_minor": l.discount_minor, "net_minor": l.net_minor,
            "tax_rate_basis_points": l.tax_rate_basis_points, "tax_minor": l.tax_minor, "total_minor": l.total_minor,
        })).collect::<Vec<Value>>(),
        "promotions": priced.promotions,
        "points_redeemed": priced.points_redeemed,
        "points_value_minor": priced.points_value_minor,
        "tax_by_rate": by_rate.iter().map(|(rate, tax)| json!({ "rate_basis_points": rate, "tax_minor": tax })).collect::<Vec<Value>>(),
        "subtotal_minor": priced.subtotal_minor,
        "discount_minor": priced.discount_minor,
        "shipping_minor": priced.shipping_minor,
        "tax_minor": priced.tax_minor,
        "total_minor": priced.total_minor,
    }));
}

fn money(minor: i64) -> String {
    return format!("{}.{:02}", minor / 100, (minor % 100).abs());
}

/// Plain-text invoice, or receipt when `receipt_number` is given.
pub fn render(snapshot: &Value, receipt_number: Option<&str>) -> String {
    let text = |v: &Value| v.as_str().unwrap_or("").to_string();
    let amount = |v: &Value| money(v.as_i64().unwrap_or(0));
    let mut out = String::new();
    out.push_str(&format!("{}\n", text(&snapshot["seller"]["name"])));
    match receipt_number {
        Some(number) => out.push_str(&format!("RECEIPT {} (invoice {})\n", number, text(&snapshot["invoice_number"]))),
        None => out.push_str(&format!("INVOICE {}\n", text(&snapshot["invoice_number"]))),
    }
    out.push_str(&format!("Order {}\nBill to: {} <{}>\n\n", text(&snapshot["order_number"]), text(&snapshot["buyer"]["name"]), text(&snapshot["buyer"]["email"])));
    for line in snapshot["lines"].as_array().into_iter().flatten() {
        out.push_str(&format!("{} x {}  {}\n", line["quantity"].as_str().unwrap_or("0"), text(&line["name"]), amount(&line["total_minor"])));
    }
    out.push_str(&format!("\nSubtotal {}\nDiscounts -{}\nShipping {}\n", amount(&snapshot["subtotal_minor"]), amount(&snapshot["discount_minor"]), amount(&snapshot["shipping_minor"])));
    for rate in snapshot["tax_by_rate"].as_array().into_iter().flatten() {
        let bp = rate["rate_basis_points"].as_i64().unwrap_or(0);
        out.push_str(&format!("Tax {}.{:02}% {}\n", bp / 100, bp % 100, amount(&rate["tax_minor"])));
    }
    out.push_str(&format!("Total {} {}\n", text(&snapshot["currency"]), amount(&snapshot["total_minor"])));
    return out;
}

pub async fn record_event(conn: &mut PgConnection, tenant_id: Uuid, order_id: Uuid, status: &str, actor_user_id: Option<Uuid>, note: Option<&str>) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO atlas.shop_order_events (id, tenant_id, order_id, status, actor_user_id, note)
         VALUES ($1, $2, $3, $4::atlas.shop_order_status, $5, $6)",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(order_id)
    .bind(status)
    .bind(actor_user_id)
    .bind(note)
    .execute(conn)
    .await?;
    return Ok(());
}

// === Emails

/// What to tell the customer once the transaction behind it has committed. Sending never fails the request.
pub struct Mail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

pub async fn send(email: &dyn EmailProvider, mail: Option<Mail>) {
    if let Some(mail) = mail {
        if let Err(err) = email.send(&mail.to, &mail.subject, &mail.body).await {
            eprintln!("order email to {} failed: {}", mail.to, err.0);
        }
    }
}

async fn order_mail(conn: &mut PgConnection, order_id: Uuid, subject: &str, receipt: bool) -> Result<Mail, ApiError> {
    let (email, order_number, snapshot, receipt_number): (String, String, Value, Option<String>) =
        sqlx::query_as("SELECT email, order_number, invoice_snapshot, receipt_number FROM atlas.shop_orders WHERE id = $1")
            .bind(order_id)
            .fetch_one(conn)
            .await?;
    let receipt_number = if receipt { receipt_number } else { None };
    return Ok(Mail {
        to: email,
        subject: format!("{} - order {}", subject, order_number),
        body: render(&snapshot, receipt_number.as_deref()),
    });
}

pub async fn invoice_mail(conn: &mut PgConnection, order_id: Uuid) -> Result<Mail, ApiError> {
    return order_mail(conn, order_id, "Your invoice", false).await;
}

pub async fn status_mail(conn: &mut PgConnection, order_id: Uuid, subject: &str) -> Result<Mail, ApiError> {
    let header = header(conn, order_id).await?;
    let mut mail = order_mail(conn, order_id, subject, header.receipt_number.is_some()).await?;
    mail.body = format!("Order {} is now {}.\n\n{}", header.order_number, header.status.replace('_', " "), mail.body);
    return Ok(mail);
}

// === Completing payment

/// Who online stock movements are recorded against when no staff member acted: the business's owner.
async fn system_actor(conn: &mut PgConnection, tenant_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar("SELECT created_by_user_id FROM atlas.tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(conn)
        .await
        .map_err(ApiError::from);
}

/// Posts the order's goods out of stock first-expiry-first-out, against the order.
async fn post_stock(conn: &mut PgConnection, tenant_id: Uuid, store_id: Uuid, order_id: Uuid, order_number: &str) -> Result<(), ApiError> {
    let lines: Vec<(Uuid, Decimal)> = sqlx::query_as("SELECT product_id, quantity FROM atlas.shop_order_lines WHERE order_id = $1 ORDER BY id")
        .bind(order_id)
        .fetch_all(&mut *conn)
        .await?;
    let actor = system_actor(conn, tenant_id).await?;
    let product_ids: Vec<Uuid> = lines.iter().map(|(id, _)| *id).collect();
    inventory::service::lock_products(conn, store_id, &product_ids).await?;
    let mut posting = Vec::with_capacity(lines.len());
    for (product_id, quantity) in lines {
        if inventory::batches::tracks_batches(conn, product_id).await? {
            let (allocations, remainder) = inventory::batches::allocate_fefo(conn, store_id, product_id, quantity, false).await?;
            for (stock_item_id, take) in allocations {
                posting.push(PostingLine { product_id, stock_item_id, unit_id: None, quantity: take });
            }
            if remainder > Decimal::ZERO {
                posting.push(PostingLine { product_id, stock_item_id: None, unit_id: None, quantity: remainder });
            }
        } else {
            posting.push(PostingLine { product_id, stock_item_id: None, unit_id: None, quantity });
        }
    }
    let reason = format!("Online order {}", order_number);
    inventory::service::post(
        conn,
        Posting {
            tenant_id,
            store_id,
            movement_type: MovementType::Sale,
            reason: &reason,
            reference_id: Some(order_id),
            actor_user_id: actor,
            // The goods are paid for; a shortfall is surfaced as a stock exception rather than refusing the payment
            policy: NegativeStockPolicy::AllowAndRecordException,
            source: "online_sale",
            lines: &posting,
        },
    )
    .await?;
    return Ok(());
}

/**
 * Marks the order paid: issues its receipt number, turns the reservations into a real stock movement, and keeps the
 * order's place in the lifecycle (a pay-on-delivery order collected after delivery stays `delivered`).
 */
pub async fn complete(conn: &mut PgConnection, storefront: &Storefront, order_id: Uuid) -> Result<(), ApiError> {
    let order = header(conn, order_id).await?;
    if order.paid_at.is_some() {
        return Ok(());
    }
    let receipt_seq = next_number(conn, storefront.tenant_id, order.store_id, "receipt").await?;
    let receipt_number = format!("{}-R{:06}", prefix(conn, order.store_id).await?, receipt_seq);
    sqlx::query(
        "UPDATE atlas.shop_orders SET paid_at = now(), receipt_number = $2,
             status = CASE WHEN status IN ('pending_payment', 'expired') THEN 'paid'::atlas.shop_order_status ELSE status END
         WHERE id = $1",
    )
    .bind(order_id)
    .bind(&receipt_number)
    .execute(&mut *conn)
    .await?;
    sqlx::query("UPDATE atlas.stock_reservations SET status = 'converted' WHERE order_id = $1 AND status IN ('active', 'released')")
        .bind(order_id)
        .execute(&mut *conn)
        .await?;
    post_stock(conn, storefront.tenant_id, order.store_id, order_id, &order.order_number).await?;
    if order.status == "pending_payment" || order.status == "expired" {
        record_event(conn, storefront.tenant_id, order_id, "paid", None, Some("Payment confirmed")).await?;
    }
    loyalty::earn(conn, storefront, order_id).await?;
    return Ok(());
}

// === Expiry

/// Releases an unpaid order's hold on stock, promotions and points.
pub async fn release(conn: &mut PgConnection, order_id: Uuid) -> Result<(), ApiError> {
    sqlx::query("UPDATE atlas.stock_reservations SET status = 'released' WHERE order_id = $1 AND status = 'active'")
        .bind(order_id)
        .execute(&mut *conn)
        .await?;
    promotions::release(conn, order_id).await?;
    loyalty::reverse(conn, order_id).await?;
    return Ok(());
}

/**
 * Expires unpaid orders past their payment window. Each still-pending attempt is checked with the gateway first
 * (FR-12.4); one that turns out to have succeeded completes the order instead.
 */
pub async fn expire_overdue(conn: &mut PgConnection, gateway: &dyn PaymentGateway, storefront: &Storefront) -> Result<u32, ApiError> {
    let overdue: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM atlas.shop_orders
         WHERE store_id = $1 AND status = 'pending_payment' AND payment_expires_at IS NOT NULL AND payment_expires_at < now()
         FOR UPDATE SKIP LOCKED",
    )
    .bind(storefront.store_id)
    .fetch_all(&mut *conn)
    .await?;
    let mut expired = 0;
    for order_id in overdue {
        let pending: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, gateway_ref FROM atlas.online_payments WHERE order_id = $1 AND status = 'pending' AND gateway_ref IS NOT NULL",
        )
        .bind(order_id)
        .fetch_all(&mut *conn)
        .await?;
        let mut paid = false;
        for (payment_id, gateway_ref) in &pending {
            if let Ok(PaymentState::Succeeded) = gateway.verify(gateway_ref).await {
                sqlx::query("UPDATE atlas.online_payments SET status = 'succeeded', confirmed_at = now() WHERE id = $1")
                    .bind(payment_id)
                    .execute(&mut *conn)
                    .await?;
                paid = true;
                break;
            }
        }
        if paid {
            complete(conn, storefront, order_id).await?;
            continue;
        }
        sqlx::query("UPDATE atlas.online_payments SET status = 'expired' WHERE order_id = $1 AND status = 'pending'")
            .bind(order_id)
            .execute(&mut *conn)
            .await?;
        sqlx::query("UPDATE atlas.shop_orders SET status = 'expired' WHERE id = $1").bind(order_id).execute(&mut *conn).await?;
        release(conn, order_id).await?;
        record_event(conn, storefront.tenant_id, order_id, "expired", None, Some("Payment window passed")).await?;
        expired += 1;
    }
    return Ok(expired);
}

// === Cancelling

/**
 * Cancels an order that hasn't shipped. Unpaid: its holds are released. Paid: the payment is refunded through the
 * gateway and the goods go back on the shelf as a `sale_void` against the order.
 */
pub async fn cancel(
    conn: &mut PgConnection,
    gateway: &dyn PaymentGateway,
    storefront: &Storefront,
    order_id: Uuid,
    actor_user_id: Option<Uuid>,
    reason: &str,
) -> Result<OrderHeader, ApiError> {
    let order = header(conn, order_id).await?;
    if !["pending_payment", "paid", "processing"].contains(&order.status.as_str()) {
        return Err(ApiError::conflict_with_code("Only an order that hasn't shipped can be cancelled.", "ORDER_NOT_CANCELLABLE"));
    }
    if order.paid_at.is_some() {
        let paid: Option<(Uuid, Option<String>, i64)> =
            sqlx::query_as("SELECT id, gateway_ref, amount_minor FROM atlas.online_payments WHERE order_id = $1 AND status = 'succeeded'")
                .bind(order_id)
                .fetch_optional(&mut *conn)
                .await?;
        if let Some((payment_id, gateway_ref, amount)) = paid {
            if let Some(gateway_ref) = gateway_ref {
                gateway
                    .refund(&gateway_ref, amount)
                    .await
                    .map_err(|err| ApiError::conflict_with_code(format!("The refund failed: {}", err), "REFUND_FAILED"))?;
            }
            sqlx::query("UPDATE atlas.online_payments SET status = 'refunded' WHERE id = $1").bind(payment_id).execute(&mut *conn).await?;
        }
        let sold: Vec<(Uuid, Option<Uuid>, Decimal)> = sqlx::query_as(
            "SELECT product_id, stock_item_id, quantity FROM atlas.stock_movements WHERE reference_id = $1 AND movement_type = 'sale' ORDER BY id",
        )
        .bind(order_id)
        .fetch_all(&mut *conn)
        .await?;
        if !sold.is_empty() {
            let lines: Vec<PostingLine> = sold
                .into_iter()
                .map(|(product_id, stock_item_id, quantity)| PostingLine { product_id, stock_item_id, unit_id: None, quantity: -quantity })
                .collect();
            let actor = match actor_user_id {
                Some(actor) => actor,
                None => system_actor(conn, storefront.tenant_id).await?,
            };
            let note = format!("Online order {} cancelled", order.order_number);
            inventory::service::post(
                conn,
                Posting {
                    tenant_id: storefront.tenant_id,
                    store_id: order.store_id,
                    movement_type: MovementType::SaleVoid,
                    reason: &note,
                    reference_id: Some(order_id),
                    actor_user_id: actor,
                    policy: NegativeStockPolicy::Allow,
                    source: "online_cancel",
                    lines: &lines,
                },
            )
            .await?;
        }
    } else {
        sqlx::query("UPDATE atlas.online_payments SET status = 'expired' WHERE order_id = $1 AND status = 'pending'")
            .bind(order_id)
            .execute(&mut *conn)
            .await?;
    }
    sqlx::query("UPDATE atlas.shop_orders SET status = 'cancelled', cancelled_at = now() WHERE id = $1")
        .bind(order_id)
        .execute(&mut *conn)
        .await?;
    release(conn, order_id).await?;
    record_event(conn, storefront.tenant_id, order_id, "cancelled", actor_user_id, Some(reason)).await?;
    return header(conn, order_id).await;
}
