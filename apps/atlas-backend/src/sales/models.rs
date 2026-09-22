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

#[derive(Debug, Deserialize)]
pub struct SaleLineInput {
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub unit_id: Uuid,
    pub quantity: Decimal,
    pub unit_price_amount_minor: i64,
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
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SaleLine {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub unit_id: Uuid,
    pub quantity: Decimal,
    pub unit_price_amount_minor: i64,
    pub line_total_amount_minor: i64,
}
