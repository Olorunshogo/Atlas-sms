use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StockLevel {
    pub product_id: Uuid,
    pub on_hand: Decimal,
}

#[derive(Debug, Deserialize)]
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
pub struct TransferStockRequest {
    pub from_store_id: Uuid,
    pub to_store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    /// Always positive - the direction is `from_store_id` -> `to_store_id`.
    pub quantity: Decimal,
    pub reason: String,
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
