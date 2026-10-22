use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::pricing::models::TaxRate;

#[derive(Debug, Serialize)]
pub struct CatalogUnit {
    pub unit_id: Uuid,
    pub name: String,
    pub abbreviation: String,
    pub factor_to_base: Decimal,
    /// Price for one of this unit to a walk-in customer right now; `None` if no rule prices it.
    pub unit_price_amount_minor: Option<i64>,
    pub barcodes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CatalogItem {
    pub product_id: Uuid,
    pub name: String,
    pub sku: Option<String>,
    pub identity_mode: String,
    pub is_returnable: bool,
    pub base_unit_id: Uuid,
    pub tax_rate_id: Option<Uuid>,
    pub tax_rate_basis_points: i64,
    pub on_hand: Decimal,
    pub units: Vec<CatalogUnit>,
}

#[derive(Debug, Serialize)]
pub struct BarcodeMatch {
    pub unit_id: Uuid,
    pub item: CatalogItem,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SnapshotPriceRule {
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

/// Everything an offline till needs to keep selling: the listed products with units and barcodes, every price rule that could apply here, and the tax rates.
#[derive(Debug, Serialize)]
pub struct CatalogSnapshot {
    pub store_id: Uuid,
    pub currency: String,
    pub prices_include_tax: bool,
    pub cash_rounding_increment_minor: i64,
    pub generated_at: DateTime<Utc>,
    pub items: Vec<CatalogItem>,
    pub price_rules: Vec<SnapshotPriceRule>,
    pub tax_rates: Vec<TaxRate>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotQuery {
    pub updated_since: Option<DateTime<Utc>>,
}
