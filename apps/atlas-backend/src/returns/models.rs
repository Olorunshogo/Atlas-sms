use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::payments::models::Refund;

pub const CONDITIONS: [&str; 5] = ["resellable", "opened", "damaged", "defective", "expired"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnLineInput {
    pub sale_line_id: Uuid,
    pub quantity: Decimal,
    /// `resellable` | `opened` | `damaged` | `defective` | `expired`.
    pub condition: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefundMethod {
    /// Back through the sale's own payments, non-cash first and cash last.
    Original,
    /// Onto the sale's customer's store credit balance.
    StoreCredit,
    /// Out through one named payment method, e.g. cash for a card sale whose card can't be refunded at the till.
    PaymentMethod,
}

impl RefundMethod {
    pub fn as_str(self) -> &'static str {
        return match self {
            RefundMethod::Original => "original",
            RefundMethod::StoreCredit => "store_credit",
            RefundMethod::PaymentMethod => "payment_method",
        };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundInput {
    pub method: RefundMethod,
    pub payment_method_id: Option<Uuid>,
    pub reference: Option<String>,
}

/// `id` is client-generated, like a sale's, so a retried return is recorded once.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReturnRequest {
    pub id: Uuid,
    pub device_id: Option<String>,
    pub reason_code: String,
    pub notes: Option<String>,
    pub lines: Vec<ReturnLineInput>,
    pub refund: RefundInput,
}

#[derive(Debug, Deserialize)]
pub struct RejectReturnRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ListReturnsQuery {
    pub store_id: Option<Uuid>,
    pub status: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Return {
    pub id: Uuid,
    pub store_id: Uuid,
    pub sale_id: Uuid,
    pub return_number: Option<String>,
    pub status: String,
    pub reason_code: String,
    pub notes: Option<String>,
    pub refund_method: String,
    pub refund_payment_method_id: Option<Uuid>,
    pub outside_window: bool,
    pub refund_total_minor: i64,
    pub requested_by_user_id: Uuid,
    pub approved_by_user_id: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_by_user_id: Option<Uuid>,
    pub rejection_reason: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub device_id: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip)]
    pub refund_reference: Option<String>,
    #[serde(skip)]
    pub request_hash: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ReturnLine {
    pub id: Uuid,
    pub return_id: Uuid,
    pub sale_line_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub stock_item_id: Option<Uuid>,
    pub unit_id: Uuid,
    pub unit_name: String,
    pub quantity: Decimal,
    pub base_quantity: Decimal,
    pub condition: String,
    pub restock: bool,
    pub refund_amount_minor: i64,
    pub tax_refund_minor: i64,
    pub stock_movement_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReturnDetail {
    #[serde(flatten)]
    pub header: Return,
    pub lines: Vec<ReturnLine>,
    pub refunds: Vec<Refund>,
}

#[derive(Debug, Serialize)]
pub struct ReturnsPage {
    pub items: Vec<Return>,
    pub next_cursor: Option<Uuid>,
}
