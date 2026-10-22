use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;

use super::engine::Discount;

// === Inputs

/// Exactly one of `amount_minor` or `percent_basis_points` (1000 = 10%). Any non-zero discount needs a reason.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscountInput {
    pub amount_minor: Option<i64>,
    pub percent_basis_points: Option<i64>,
    pub reason: Option<String>,
}

impl DiscountInput {
    pub fn to_engine(&self) -> Result<Option<Discount>, ApiError> {
        return match (self.amount_minor, self.percent_basis_points) {
            (Some(_), Some(_)) => Err(ApiError::bad_request(
                "a discount takes either amount_minor or percent_basis_points, not both",
            )),
            (Some(amount), None) if amount < 0 => Err(ApiError::bad_request("discount amount must not be negative")),
            (Some(0), None) | (None, Some(0)) | (None, None) => Ok(None),
            (Some(amount), None) => Ok(Some(Discount::Amount(amount))),
            (None, Some(bp)) if !(0..=10_000).contains(&bp) => {
                Err(ApiError::bad_request("percent_basis_points must be between 0 and 10000"))
            }
            (None, Some(bp)) => Ok(Some(Discount::BasisPoints(bp))),
        };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartLineInput {
    /// Optional client-generated line id (offline carts mint their own).
    pub id: Option<Uuid>,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub unit_id: Uuid,
    pub quantity: Decimal,
    /// Only set when the cashier charges something other than the resolved price.
    pub unit_price_amount_minor: Option<i64>,
    pub price_override_reason: Option<String>,
    pub discount: Option<DiscountInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuoteRequest {
    pub customer_id: Option<Uuid>,
    pub lines: Vec<CartLineInput>,
    pub cart_discount: Option<DiscountInput>,
}

// === Outputs

#[derive(Debug, Clone, Serialize)]
pub struct PricedLine {
    pub line_number: i32,
    pub client_line_id: Option<Uuid>,
    pub product_id: Uuid,
    pub product_name: String,
    pub sku: Option<String>,
    pub is_returnable: bool,
    pub stock_item_id: Option<Uuid>,
    pub unit_id: Uuid,
    pub unit_name: String,
    pub quantity: Decimal,
    pub base_quantity: Decimal,
    /// What the price rules say; `unit_price_amount_minor` is what is charged.
    pub list_unit_price_amount_minor: Option<i64>,
    pub unit_price_amount_minor: i64,
    pub price_rule_id: Option<Uuid>,
    pub price_overridden: bool,
    pub price_override_reason: Option<String>,
    pub gross_amount_minor: i64,
    pub line_discount_amount_minor: i64,
    pub allocated_cart_discount_amount_minor: i64,
    pub net_amount_minor: i64,
    pub tax_rate_id: Option<Uuid>,
    pub tax_rate_basis_points: i64,
    pub tax_amount_minor: i64,
    pub line_total_amount_minor: i64,
    pub on_hand: Decimal,
    #[serde(skip)]
    #[allow(dead_code)]
    pub discount_reason: Option<String>,
}

/// A rule the cart breaks. A quote reports every one; live checkout refuses on the first, offline sync records them and accepts.
#[derive(Debug, Clone, Serialize)]
pub struct Violation {
    pub code: &'static str,
    pub message: String,
    pub line_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Quote {
    pub store_id: Uuid,
    pub currency: String,
    pub prices_include_tax: bool,
    pub customer_id: Option<Uuid>,
    pub customer_group_id: Option<Uuid>,
    pub lines: Vec<PricedLine>,
    pub subtotal_amount_minor: i64,
    pub discount_amount_minor: i64,
    pub cart_discount_amount_minor: i64,
    pub discount_reason: Option<String>,
    pub tax_amount_minor: i64,
    pub total_amount_minor: i64,
    /// What a cash-paying customer owes after cash rounding; equal to the total when the store doesn't round.
    pub cash_total_amount_minor: i64,
    pub cash_rounding_increment_minor: i64,
    pub price_overridden: bool,
    pub priced_at: DateTime<Utc>,
    pub violations: Vec<Violation>,
}

/// A store's pricing and checkout settings, loaded once per quote or sale.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StorePricing {
    pub id: Uuid,
    pub currency: String,
    pub is_active: bool,
    pub prices_include_tax: bool,
    pub default_tax_rate_id: Option<Uuid>,
    pub cash_rounding_increment_minor: i64,
    pub max_cashier_discount_basis_points: i32,
    pub allow_negative_stock: bool,
}

// === Tax rates

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TaxRate {
    pub id: Uuid,
    pub name: String,
    pub rate_basis_points: i32,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaxRateRequest {
    pub name: String,
    pub rate_basis_points: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaxRateRequest {
    pub name: Option<String>,
    pub is_active: Option<bool>,
}
