use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;

use super::models::{CreatePriceRuleRequest, PriceRule};

const COLUMNS: &str = "id, product_id, store_id, customer_group_id, min_quantity, unit_price_amount_minor, priority, starts_at, ends_at";

/** Foreign ids bypass RLS, so each one is read back through the scoped transaction. */
async fn require_visible(
    conn: &mut PgConnection,
    table: &str,
    id: Uuid,
    label: &str,
) -> Result<(), ApiError> {
    let found: Option<i32> =
        sqlx::query_scalar(&format!("SELECT 1 FROM atlas.{table} WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?;
    found
        .map(|_| ())
        .ok_or_else(|| ApiError::NotFound(format!("{label} not found.")))
}

pub async fn list(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    product_id: Option<Uuid>,
) -> Result<Vec<PriceRule>, ApiError> {
    sqlx::query_as::<_, PriceRule>(&format!(
        "SELECT {COLUMNS} FROM atlas.price_rules
         WHERE tenant_id = $1 AND ($2::uuid IS NULL OR product_id = $2)
         ORDER BY product_id, created_at"
    ))
    .bind(tenant_id)
    .bind(product_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

pub async fn create(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreatePriceRuleRequest,
) -> Result<PriceRule, ApiError> {
    let invalid =
        |message: &str| ApiError::bad_request_with_code(message.to_string(), "VALIDATION_FAILED");

    if payload.unit_price_amount_minor < 0 {
        return Err(invalid("unit_price_amount_minor cannot be negative"));
    }
    let min_quantity = payload.min_quantity.unwrap_or(Decimal::ONE);
    if min_quantity <= Decimal::ZERO {
        return Err(invalid("min_quantity must be greater than zero"));
    }
    if let (Some(starts), Some(ends)) = (payload.starts_at, payload.ends_at) {
        if ends <= starts {
            return Err(invalid("ends_at must be after starts_at"));
        }
    }

    require_visible(conn, "products", payload.product_id, "Product").await?;
    if let Some(store_id) = payload.store_id {
        require_visible(conn, "stores", store_id, "Store").await?;
    }
    if let Some(group_id) = payload.customer_group_id {
        require_visible(conn, "customer_groups", group_id, "Customer group").await?;
    }

    sqlx::query_as::<_, PriceRule>(&format!(
        "INSERT INTO atlas.price_rules
            (id, tenant_id, product_id, store_id, customer_group_id, min_quantity,
             unit_price_amount_minor, priority, starts_at, ends_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING {COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.product_id)
    .bind(payload.store_id)
    .bind(payload.customer_group_id)
    .bind(min_quantity)
    .bind(payload.unit_price_amount_minor)
    .bind(payload.priority)
    .bind(payload.starts_at)
    .bind(payload.ends_at)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from)
}

/**
 * Safe to hard-delete: sale lines copy the price they sold at, so removing
 * a rule never changes history.
 */
pub async fn delete(conn: &mut PgConnection, rule_id: Uuid) -> Result<(), ApiError> {
    let result = sqlx::query("DELETE FROM atlas.price_rules WHERE id = $1")
        .bind(rule_id)
        .execute(conn)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Price rule not found.".to_string()));
    }
    Ok(())
}
