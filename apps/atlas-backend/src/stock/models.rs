use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StockLevel {
    pub product_id: Uuid,
    pub on_hand: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdjustStockRequest {
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    /// Signed - positive adds stock, negative removes it (a stock count
    /// correction, damage write-off, etc).
    pub quantity: Decimal,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct RequestTransferRequest {
    pub from_store_id: Uuid,
    pub to_store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    /// Always positive - the direction is `from_store_id` -> `to_store_id`.
    pub quantity: Decimal,
    pub reason: String,
}

#[derive(Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "stock_transfer_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum StockTransferStatus {
    Requested,
    Approved,
    Rejected,
    Dispatched,
    Received,
    Cancelled,
}

/**
 * One stage of FR-22's workflow. Stock leaves `from_store_id` only at
 * `dispatch` and arrives at `to_store_id` only at `receive` - the gap
 * between the two is the documented in-transit state, so it is never
 * counted as available at both stores at once.
 */
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StockTransfer {
    pub id: Uuid,
    pub from_store_id: Uuid,
    pub to_store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub quantity: Decimal,
    pub reason: String,
    pub status: StockTransferStatus,
    pub requested_by_user_id: Uuid,
    pub requested_at: DateTime<Utc>,
    pub approved_by_user_id: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub dispatched_at: Option<DateTime<Utc>>,
    pub received_at: Option<DateTime<Utc>>,
    pub cancelled_by_user_id: Option<Uuid>,
    pub cancelled_at: Option<DateTime<Utc>>,
}

pub use crate::inventory::StockMovement;

// === Cross-store reads

#[derive(Debug, Serialize)]
pub struct LocationQuantity {
    pub store_id: Uuid,
    pub store_name: String,
    pub quantity: Decimal,
}

/// One product across the stores the caller may see. `total_quantity` is summed from `locations` at read time, never stored.
#[derive(Debug, Serialize)]
pub struct ProductInventory {
    pub product_id: Uuid,
    pub product_name: String,
    pub total_quantity: Decimal,
    pub locations: Vec<LocationQuantity>,
}

/// One row of the audit trail, with who made the change and when.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MovementRecord {
    pub id: Uuid,
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub movement_type: String,
    pub quantity: Decimal,
    pub reason: String,
    pub reference_id: Option<Uuid>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}
