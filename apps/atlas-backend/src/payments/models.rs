use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub method: String,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePaymentRequest {
    pub sale_id: Uuid,
    /// `cash` | `card` | `transfer` | `wallet` - matches `atlas.payment_method`.
    pub method: String,
    pub amount_minor: i64,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
pub struct ListPaymentsQuery {
    pub sale_id: Uuid,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Refund {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub amount_minor: i64,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct RequestRefundRequest {
    pub amount_minor: i64,
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
