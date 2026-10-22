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
