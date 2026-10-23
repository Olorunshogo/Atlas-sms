use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PriceRule {
    pub id: Uuid,
    pub product_id: Uuid,
    pub store_id: Option<Uuid>,
    pub customer_group_id: Option<Uuid>,
    pub min_quantity: Decimal,
    pub unit_price_amount_minor: i64,
    pub priority: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

/**
 * `store_id` and `customer_group_id` left out mean "applies to all". The
 * price is per base unit of the product, in minor currency units.
 */
#[derive(Debug, Deserialize)]
pub struct CreatePriceRuleRequest {
    pub product_id: Uuid,
    pub store_id: Option<Uuid>,
    pub customer_group_id: Option<Uuid>,
    /// Quantity tier this price starts at, in base units. Defaults to 1.
    pub min_quantity: Option<Decimal>,
    pub unit_price_amount_minor: i64,
    #[serde(default)]
    pub priority: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}
