use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// === Shifts

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Shift {
    pub id: Uuid,
    pub store_id: Uuid,
    pub opened_by_user_id: Uuid,
    pub opened_at: DateTime<Utc>,
    pub opening_cash_amount_minor: i64,
    pub closed_by_user_id: Option<Uuid>,
    pub closed_at: Option<DateTime<Utc>>,
    pub closing_cash_amount_minor: Option<i64>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenShiftRequest {
    pub store_id: Uuid,
    #[serde(default)]
    pub opening_cash_amount_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct CloseShiftRequest {
    pub closing_cash_amount_minor: i64,
}

// === Sales

/**
 * No price here on purpose: the server resolves it from `price_rules`. A
 * `unit_price_amount_minor` sent by an older client is ignored (serde
 * skips unknown fields), never trusted.
 */
#[derive(Debug, Deserialize)]
pub struct SaleLineInput {
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub unit_id: Uuid,
    /// In `unit_id`'s unit; converted to the product's base unit server-side.
    pub quantity: Decimal,
}

/**
 * `id` is client-generated (UUIDv7, minted on the POS device) - the one
 * request in this API where the caller picks the primary key, because it
 * doubles as the idempotency token for offline sync. See
 * `sales::service::create_sale`'s doc comment.
 */
#[derive(Debug, Deserialize)]
pub struct CreateSaleRequest {
    pub id: Uuid,
    pub store_id: Uuid,
    pub shift_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub device_id: String,
    #[serde(default)]
    pub discount_amount_minor: i64,
    #[serde(default)]
    pub tax_amount_minor: i64,
    pub lines: Vec<SaleLineInput>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Sale {
    pub id: Uuid,
    pub store_id: Uuid,
    pub shift_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub cashier_user_id: Uuid,
    pub device_id: String,
    pub status: String,
    pub subtotal_amount_minor: i64,
    pub discount_amount_minor: i64,
    pub tax_amount_minor: i64,
    pub total_amount_minor: i64,
    pub created_at: DateTime<Utc>,
    pub voided_at: Option<DateTime<Utc>>,
}

/// A sale together with its lines - the receipt returned by `POST /sales`.
#[derive(Debug, Serialize)]
pub struct SaleReceipt {
    #[serde(flatten)]
    pub sale: Sale,
    pub lines: Vec<SaleLine>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SaleLine {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub unit_id: Uuid,
    pub quantity: Decimal,
    /// What actually left stock, in the product's base unit.
    pub base_quantity: Decimal,
    pub unit_price_amount_minor: i64,
    pub line_total_amount_minor: i64,
}

// === Voids and returns

#[derive(Debug, Deserialize)]
pub struct VoidSaleRequest {
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct VoidReceipt {
    pub sale: Sale,
    /// The whole sale total goes back to the customer.
    pub refund_amount_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct ReturnLineInput {
    pub sale_line_id: Uuid,
    /// In the sale line's own unit, up to what has not been returned yet.
    pub quantity: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CreateReturnRequest {
    pub lines: Vec<ReturnLineInput>,
    pub reason: String,
    /// False for goods that cannot be resold (damaged): the refund is still
    /// recorded but stock is not increased.
    #[serde(default = "default_true")]
    pub return_to_stock: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SaleReturnLine {
    pub id: Uuid,
    pub return_id: Uuid,
    pub sale_id: Uuid,
    pub sale_line_id: Uuid,
    pub quantity: Decimal,
    pub base_quantity: Decimal,
    pub refund_amount_minor: i64,
    pub returned_to_stock: bool,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ReturnReceipt {
    pub return_id: Uuid,
    pub sale_id: Uuid,
    pub refund_amount_minor: i64,
    pub lines: Vec<SaleReturnLine>,
}
