use serde::{Deserialize, Serialize};
use uuid::Uuid;

// === Suppliers

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Supplier {
    pub id: Uuid,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub balance_amount_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateSupplierRequest {
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
}

/// Deliberately no `balance_amount_minor` field - the migration's own comment is explicit that it's adjusted by purchase orders and payments in the service layer, never set directly by a caller.
#[derive(Debug, Deserialize)]
pub struct UpdateSupplierRequest {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

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
    pub quantity_ordered: rust_decimal::Decimal,
    pub quantity_received: rust_decimal::Decimal,
    pub unit_cost_amount_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct PurchaseOrderLineInput {
    pub product_id: Uuid,
    pub quantity_ordered: rust_decimal::Decimal,
    pub unit_cost_amount_minor: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreatePurchaseOrderRequest {
    pub store_id: Uuid,
    pub supplier_id: Uuid,
    pub lines: Vec<PurchaseOrderLineInput>,
}

#[derive(Debug, Deserialize)]
pub struct ListPurchaseOrdersQuery {
    pub store_id: Uuid,
}

// === Goods received notes

#[derive(Debug, Deserialize)]
pub struct ReceiveLineInput {
    pub purchase_order_line_id: Uuid,
    pub quantity_received: rust_decimal::Decimal,
    /// Required for batch-tracked products (FR-14); a batch number seen before tops that batch up.
    pub batch_number: Option<String>,
    pub expiry_date: Option<chrono::NaiveDate>,
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

// === Purchase order lifecycle (FR-21)

/// `draft` only - edit before approval. Lines, when present, replace the order's lines wholesale.
#[derive(Debug, Deserialize)]
pub struct EditPurchaseOrderRequest {
    pub supplier_id: Option<Uuid>,
    pub lines: Option<Vec<PurchaseOrderLineInput>>,
}

// === Purchase returns (FR-23)

#[derive(Debug, Deserialize)]
pub struct PurchaseReturnLineInput {
    pub goods_received_note_line_id: Uuid,
    pub quantity: rust_decimal::Decimal,
}

#[derive(Debug, Deserialize)]
pub struct RecordPurchaseReturnRequest {
    pub purchase_order_id: Uuid,
    pub reason: String,
    pub lines: Vec<PurchaseReturnLineInput>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PurchaseReturnLine {
    pub id: Uuid,
    pub goods_received_note_line_id: Uuid,
    pub product_id: Uuid,
    pub quantity: rust_decimal::Decimal,
    pub unit_cost_amount_minor: i64,
}

#[derive(Debug, Serialize)]
pub struct PurchaseReturn {
    pub id: Uuid,
    pub store_id: Uuid,
    pub supplier_id: Uuid,
    pub purchase_order_id: Uuid,
    pub reason: String,
    pub lines: Vec<PurchaseReturnLine>,
}
