use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ReconciliationQuery {
    pub store_id: Uuid,
    pub shift_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MethodTotals {
    pub payment_method_id: Option<Uuid>,
    pub name: String,
    pub kind: String,
    pub count: i64,
    /// Money taken and still counting toward the sale (succeeded, including payments later refunded).
    pub collected_minor: i64,
    pub refunded_minor: i64,
    pub reversed_minor: i64,
    pub pending_minor: i64,
    pub failed_minor: i64,
    pub net_minor: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SalesTotals {
    pub completed_count: i64,
    pub awaiting_payment_count: i64,
    pub voided_count: i64,
    /// What completed sales were for, cash rounding included.
    pub due_minor: i64,
    pub amount_paid_minor: i64,
    pub outstanding_minor: i64,
    pub refunded_minor: i64,
}

/**
 * Both sides of the books over the same set of sales. `variance_minor` is what payments say was netted minus what sales say was netted, and should be zero; `discrepancies` names any sale whose own paid figure disagrees with its payment rows.
 */
#[derive(Debug, Serialize)]
pub struct Reconciliation {
    pub store_id: Uuid,
    pub shift_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub by_method: Vec<MethodTotals>,
    pub sales: SalesTotals,
    /// Refunds paid out without a payment to come back through, i.e. to store credit.
    pub non_payment_refunds_minor: i64,
    pub payments_net_minor: i64,
    pub sales_net_minor: i64,
    pub variance_minor: i64,
    pub discrepancies: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct ShiftCashSummary {
    pub opening_cash_minor: i64,
    pub cash_collected_minor: i64,
    pub cash_refunded_minor: i64,
    pub expected_cash_minor: i64,
    pub counted_cash_minor: Option<i64>,
    /// Counted minus expected: negative is a shortage.
    pub variance_minor: Option<i64>,
    pub by_method: Vec<MethodTotals>,
}
