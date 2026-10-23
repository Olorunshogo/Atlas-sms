use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// === Reads

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StockLevel {
    pub product_id: Uuid,
    pub product_name: String,
    pub on_hand: Decimal,
    pub low_stock_threshold: Decimal,
}

#[derive(Debug, Serialize)]
pub struct LocationQuantity {
    pub store_id: Uuid,
    pub store_name: String,
    pub quantity: Decimal,
}

/**
 * One product across every store the caller may see. `total_quantity` is
 * computed from `locations` at read time, never read from a column.
 */
#[derive(Debug, Serialize)]
pub struct ProductInventory {
    pub product_id: Uuid,
    pub product_name: String,
    pub total_quantity: Decimal,
    pub locations: Vec<LocationQuantity>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StockMovement {
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

// === Writes

#[derive(Debug, Deserialize)]
pub struct InitialStockRequest {
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    /// Zero is allowed: it lists the product at the store with no stock yet.
    pub quantity: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct RestockRequest {
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    /// Always positive.
    pub quantity: Decimal,
    /// Supplier or delivery note, defaults to "Supplier delivery".
    pub reason: Option<String>,
    /// Loose pointer to a purchase order or delivery document.
    pub reference_id: Option<Uuid>,
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
