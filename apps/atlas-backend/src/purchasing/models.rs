use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// === Purchase orders

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PurchaseOrder {
    pub id: Uuid,
    pub store_id: Uuid,
    pub supplier_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PurchaseOrderLine {
    pub id: Uuid,
    pub purchase_order_id: Uuid,
    pub product_id: Uuid,
    pub quantity_ordered: Decimal,
    pub quantity_received: Decimal,
    pub quantity_cancelled: Decimal,
    /// What is still expected: ordered less received and cancelled, never below zero.
    pub quantity_outstanding: Decimal,
    pub unit_cost_amount_minor: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PurchaseOrderCharge {
    pub id: Uuid,
    pub description: String,
    pub amount_minor: i64,
}

/// `GET /purchase-orders/:id`: the order with its lines, charges, totals and who moved it along.
#[derive(Debug, Serialize)]
pub struct PurchaseOrderDetail {
    pub id: Uuid,
    pub store_id: Uuid,
    pub supplier_id: Uuid,
    pub status: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub approved_by_user_id: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub lines: Vec<PurchaseOrderLine>,
    pub charges: Vec<PurchaseOrderCharge>,
    pub lines_total_amount_minor: i64,
    pub charges_total_amount_minor: i64,
    pub total_amount_minor: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PurchaseOrderHeader {
    pub id: Uuid,
    pub store_id: Uuid,
    pub supplier_id: Uuid,
    pub status: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub approved_by_user_id: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct PurchaseOrderLineInput {
    pub product_id: Uuid,
    pub quantity_ordered: Decimal,
    pub unit_cost_amount_minor: i64,
}

/// An extra cost on the order (delivery, handling), owed with the first receipt.
#[derive(Debug, Deserialize)]
pub struct PurchaseOrderChargeInput {
    pub description: String,
    pub amount_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreatePurchaseOrderRequest {
    pub store_id: Uuid,
    pub supplier_id: Uuid,
    pub lines: Vec<PurchaseOrderLineInput>,
    #[serde(default)]
    pub charges: Vec<PurchaseOrderChargeInput>,
}

#[derive(Debug, Deserialize)]
pub struct ListPurchaseOrdersQuery {
    pub store_id: Uuid,
}

/// Before approval only. Editing a submitted order sends it back to `draft`. Lines and charges, when present, replace the old ones.
#[derive(Debug, Deserialize)]
pub struct EditPurchaseOrderRequest {
    pub supplier_id: Option<Uuid>,
    pub lines: Option<Vec<PurchaseOrderLineInput>>,
    pub charges: Option<Vec<PurchaseOrderChargeInput>>,
}

// === Goods received notes

#[derive(Debug, Deserialize)]
pub struct ReceiveLineInput {
    pub purchase_order_line_id: Uuid,
    pub quantity_received: Decimal,
    /// Required for batch-tracked products (FR-14); a batch number seen before tops that batch up.
    pub batch_number: Option<String>,
    pub expiry_date: Option<chrono::NaiveDate>,
    /// Receiving more than is outstanding needs this and `purchase.approve` (FR-21's authorized exception).
    pub over_receipt_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReceiveGoodsRequest {
    pub lines: Vec<ReceiveLineInput>,
}

#[derive(Debug, Serialize)]
pub struct GoodsReceivedNoteData {
    pub id: Uuid,
    pub purchase_order: PurchaseOrder,
}

// === Purchase returns (FR-23)

#[derive(Debug, Deserialize)]
pub struct PurchaseReturnLineInput {
    pub goods_received_note_line_id: Uuid,
    pub quantity: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct RecordPurchaseReturnRequest {
    pub purchase_order_id: Uuid,
    pub reason: String,
    pub lines: Vec<PurchaseReturnLineInput>,
}

#[derive(Debug, Deserialize)]
pub struct ListPurchaseReturnsQuery {
    pub store_id: Uuid,
    pub supplier_id: Option<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PurchaseReturnLine {
    pub id: Uuid,
    pub goods_received_note_line_id: Uuid,
    pub product_id: Uuid,
    pub quantity: Decimal,
    pub unit_cost_amount_minor: i64,
    pub amount_minor: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PurchaseReturn {
    pub id: Uuid,
    pub store_id: Uuid,
    pub supplier_id: Uuid,
    pub purchase_order_id: Uuid,
    pub reason: String,
    /// `pending` until confirmed; only a confirmed return has moved stock and the supplier balance.
    pub status: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub confirmed_by_user_id: Option<Uuid>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    #[sqlx(skip)]
    pub total_amount_minor: i64,
    #[sqlx(skip)]
    pub lines: Vec<PurchaseReturnLine>,
}
