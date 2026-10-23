use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub payment_method_id: Option<Uuid>,
    /// The method's kind (`cash`, `card`, `transfer`, ...); `method_name` is the tenant's label for it.
    pub method: String,
    pub method_name: String,
    pub amount_minor: i64,
    pub tendered_amount_minor: Option<i64>,
    pub change_amount_minor: i64,
    pub refunded_amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub reference: Option<String>,
    pub status_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PaymentMethod {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub kind: String,
    pub requires_reference: bool,
    pub is_active: bool,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreatePaymentMethodRequest {
    pub code: String,
    pub name: String,
    /// `cash` | `card` | `transfer` | `wallet` | `store_credit` | `other` - matches `atlas.payment_method`.
    pub kind: String,
    #[serde(default)]
    pub requires_reference: bool,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePaymentMethodRequest {
    pub name: Option<String>,
    pub requires_reference: Option<bool>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ListPaymentsQuery {
    pub sale_id: Uuid,
}

/// Money returned to a customer for one sale - through one of its payments, to store credit, or out through another method.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Refund {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub return_id: Option<Uuid>,
    pub payment_id: Option<Uuid>,
    pub payment_method_id: Option<Uuid>,
    pub method_name: Option<String>,
    pub method_kind: Option<String>,
    pub amount_minor: i64,
    pub status: String,
    pub reason: Option<String>,
    pub reference: Option<String>,
    pub requested_by_user_id: Uuid,
    pub approved_by_user_id: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RequestRefundRequest {
    pub amount_minor: i64,
    pub reason: String,
}

/// The raw confirmation token is only ever returned once, here - it's never stored (only its hash is), so this response is the one chance a caller has to capture it before confirming.
#[derive(Debug, Serialize)]
pub struct RefundRequestedData {
    pub refund: Refund,
    pub confirmation_token: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmRefundRequest {
    pub confirmation_token: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolvePaymentRequest {
    /// `succeeded` | `failed`.
    pub status: crate::sales::settlement::PaymentOutcome,
    pub reference: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReversePaymentRequest {
    pub reason: String,
}
