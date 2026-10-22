use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::StorePricing;

pub struct ResolvedPrice {
    pub price_rule_id: Uuid,
    /// Per base unit, as the rule is written.
    pub base_unit_price_minor: i64,
}

/**
 * The winning `price_rules` row for a product at a store, for a customer group and quantity, at a moment in time - the resolution the migration 015 comment describes.
 * Most specific wins (a store-specific rule beats an all-stores one, a group rule beats an everyone rule), then higher `priority`, then the larger quantity break, then the longest-running rule. `None` means the product has no price here.
 */
pub async fn resolve_base_price(
    conn: &mut PgConnection,
    product_id: Uuid,
    store_id: Uuid,
    customer_group_id: Option<Uuid>,
    base_quantity: Decimal,
    at: DateTime<Utc>,
) -> Result<Option<ResolvedPrice>, ApiError> {
    let row: Option<(Uuid, i64)> = sqlx::query_as(
        "SELECT id, unit_price_amount_minor
         FROM atlas.price_rules
         WHERE product_id = $1
           AND (store_id IS NULL OR store_id = $2)
           AND (customer_group_id IS NULL OR customer_group_id = $3)
           AND min_quantity <= $4
           AND (starts_at IS NULL OR starts_at <= $5)
           AND (ends_at IS NULL OR ends_at > $5)
         ORDER BY (store_id IS NOT NULL) DESC,
                  (customer_group_id IS NOT NULL) DESC,
                  priority DESC,
                  min_quantity DESC,
                  starts_at ASC NULLS FIRST,
                  id
         LIMIT 1",
    )
    .bind(product_id)
    .bind(store_id)
    .bind(customer_group_id)
    .bind(base_quantity)
    .bind(at)
    .fetch_optional(conn)
    .await?;

    return Ok(row.map(|(price_rule_id, base_unit_price_minor)| ResolvedPrice {
        price_rule_id,
        base_unit_price_minor,
    }));
}

/// The tax rate that applies to a product at a store: the product's own active rate, else the store's active default, else none (0 bp). A retired rate falls through to the next one.
pub async fn effective_tax_rate(
    conn: &mut PgConnection,
    product_tax_rate_id: Option<Uuid>,
    store: &StorePricing,
) -> Result<(Option<Uuid>, i64), ApiError> {
    for rate_id in [product_tax_rate_id, store.default_tax_rate_id].into_iter().flatten() {
        let bp: Option<i32> =
            sqlx::query_scalar("SELECT rate_basis_points FROM atlas.tax_rates WHERE id = $1 AND is_active")
                .bind(rate_id)
                .fetch_optional(&mut *conn)
                .await?;
        if let Some(bp) = bp {
            return Ok((Some(rate_id), i64::from(bp)));
        }
    }
    return Ok((None, 0));
}

pub async fn load_store_pricing(conn: &mut PgConnection, store_id: Uuid) -> Result<StorePricing, ApiError> {
    return sqlx::query_as::<_, StorePricing>(
        "SELECT id, currency, is_active, prices_include_tax, default_tax_rate_id,
                cash_rounding_increment_minor, max_cashier_discount_basis_points, allow_negative_stock
         FROM atlas.stores WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(store_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Store not found."));
}
