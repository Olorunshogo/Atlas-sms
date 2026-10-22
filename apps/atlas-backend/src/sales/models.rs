use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::payments::models::Payment;
use crate::pricing::models::{CartLineInput, DiscountInput};

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

/// A closed shift with its drawer reconciliation: expected cash against what was counted.
#[derive(Debug, Serialize)]
pub struct ClosedShift {
    #[serde(flatten)]
    pub shift: Shift,
    pub cash: crate::reports::models::ShiftCashSummary,
}

// === Checkout input

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentInputStatus {
    /// Money already in hand (cash counted, card approved, transfer seen).
    #[default]
    Succeeded,
    /// Started but unconfirmed, e.g. a card terminal still processing. The sale waits in `awaiting_payment`.
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInput {
    /// Client-generated so a retried request can't record the payment twice.
    pub id: Option<Uuid>,
    pub payment_method_id: Uuid,
    /// The amount applied to the sale. For cash, change is `tendered_amount_minor - amount_minor`.
    pub amount_minor: i64,
    pub tendered_amount_minor: Option<i64>,
    pub reference: Option<String>,
    #[serde(default)]
    pub status: PaymentInputStatus,
}

/**
 * `id` is client-generated (UUIDv7, minted on the POS device) - the one sale input where the caller picks the primary key, because it doubles as the idempotency key for offline sync.
 * Prices, discounts and tax are not taken from the client: lines carry only what was scanned and how many, plus an optional price override the server checks against `sale.price_override`. `expected_total_amount_minor` is what the cashier saw; on the live path a mismatch is a 409 so the till re-shows the server's total before charging.
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSaleRequest {
    pub id: Uuid,
    pub store_id: Uuid,
    pub shift_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub device_id: String,
    pub client_created_at: Option<DateTime<Utc>>,
    pub lines: Vec<CartLineInput>,
    pub cart_discount: Option<DiscountInput>,
    #[serde(default)]
    pub payments: Vec<PaymentInput>,
    pub expected_total_amount_minor: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct VoidSaleRequest {
    pub reason: String,
}

// === Sale records

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Sale {
    pub id: Uuid,
    pub store_id: Uuid,
    pub shift_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub cashier_user_id: Uuid,
    pub device_id: String,
    pub status: String,
    pub currency: String,
    pub subtotal_amount_minor: i64,
    pub discount_amount_minor: i64,
    pub cart_discount_amount_minor: i64,
    pub discount_reason: Option<String>,
    pub tax_amount_minor: i64,
    pub total_amount_minor: i64,
    pub rounding_adjustment_minor: i64,
    pub amount_paid_minor: i64,
    pub amount_refunded_minor: i64,
    pub payment_status: String,
    pub prices_include_tax: bool,
    pub price_overridden: bool,
    pub client_created_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub voided_at: Option<DateTime<Utc>>,
    pub voided_by_user_id: Option<Uuid>,
    #[serde(skip)]
    pub request_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SaleLine {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub line_number: i32,
    pub product_id: Uuid,
    pub product_name: String,
    pub sku: Option<String>,
    pub stock_item_id: Option<Uuid>,
    pub unit_id: Uuid,
    pub unit_name: String,
    pub quantity: Decimal,
    pub base_quantity: Decimal,
    pub list_unit_price_amount_minor: Option<i64>,
    pub unit_price_amount_minor: i64,
    pub price_rule_id: Option<Uuid>,
    pub price_overridden: bool,
    pub price_override_reason: Option<String>,
    pub line_discount_amount_minor: i64,
    pub allocated_cart_discount_amount_minor: i64,
    pub discount_reason: Option<String>,
    pub net_amount_minor: i64,
    pub tax_rate_id: Option<Uuid>,
    pub tax_rate_basis_points: i32,
    pub tax_amount_minor: i64,
    pub line_total_amount_minor: i64,
}

/// A sale with everything under it - what checkout returns and what history and receipts read.
#[derive(Debug, Clone, Serialize)]
pub struct SaleDetail {
    #[serde(flatten)]
    pub sale: Sale,
    pub lines: Vec<SaleLine>,
    pub payments: Vec<Payment>,
}
