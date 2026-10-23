/*!
 * FR-09 price history: every selling-price rule added or removed and every cost change, with the price it replaced, who changed it and from when it applies.
 * Completed sales are untouched by any of this - sale lines copy the price they sold at.
 */
use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::validate::require_visible;

use super::selling::PriceRule;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PriceChange {
    pub id: Uuid,
    pub product_id: Uuid,
    pub store_id: Option<Uuid>,
    pub customer_group_id: Option<Uuid>,
    pub min_quantity: Option<Decimal>,
    pub kind: String,
    pub previous_amount_minor: Option<i64>,
    pub new_amount_minor: Option<i64>,
    pub price_rule_id: Option<Uuid>,
    pub effective_from: DateTime<Utc>,
    pub changed_by_user_id: Uuid,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PriceHistoryQuery {
    pub store_id: Option<Uuid>,
}

struct Change {
    product_id: Uuid,
    store_id: Option<Uuid>,
    customer_group_id: Option<Uuid>,
    min_quantity: Option<Decimal>,
    kind: &'static str,
    previous: Option<i64>,
    new: Option<i64>,
    price_rule_id: Option<Uuid>,
    effective_from: DateTime<Utc>,
}

async fn insert(conn: &mut PgConnection, tenant_id: Uuid, user_id: Uuid, change: Change) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO atlas.price_changes
            (id, tenant_id, product_id, store_id, customer_group_id, min_quantity, kind,
             previous_amount_minor, new_amount_minor, price_rule_id, effective_from, changed_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7::atlas.price_kind, $8, $9, $10, $11, $12)",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(change.product_id)
    .bind(change.store_id)
    .bind(change.customer_group_id)
    .bind(change.min_quantity)
    .bind(change.kind)
    .bind(change.previous)
    .bind(change.new)
    .bind(change.price_rule_id)
    .bind(change.effective_from)
    .bind(user_id)
    .execute(conn)
    .await?;
    return Ok(());
}

/// The previous price is the best still-running rule for the same product, store, group and quantity tier.
pub async fn record_rule_created(conn: &mut PgConnection, tenant_id: Uuid, user_id: Uuid, rule: &PriceRule) -> Result<(), ApiError> {
    let previous: Option<i64> = sqlx::query_scalar(
        "SELECT unit_price_amount_minor FROM atlas.price_rules
         WHERE product_id = $1 AND id <> $2
           AND store_id IS NOT DISTINCT FROM $3
           AND customer_group_id IS NOT DISTINCT FROM $4
           AND min_quantity = $5
           AND (ends_at IS NULL OR ends_at > now())
         ORDER BY priority DESC, created_at DESC
         LIMIT 1",
    )
    .bind(rule.product_id)
    .bind(rule.id)
    .bind(rule.store_id)
    .bind(rule.customer_group_id)
    .bind(rule.min_quantity)
    .fetch_optional(&mut *conn)
    .await?;
    return insert(
        conn,
        tenant_id,
        user_id,
        Change {
            product_id: rule.product_id,
            store_id: rule.store_id,
            customer_group_id: rule.customer_group_id,
            min_quantity: Some(rule.min_quantity),
            kind: "selling",
            previous,
            new: Some(rule.unit_price_amount_minor),
            price_rule_id: Some(rule.id),
            effective_from: rule.starts_at.unwrap_or_else(Utc::now),
        },
    )
    .await;
}

pub async fn record_rule_deleted(conn: &mut PgConnection, tenant_id: Uuid, user_id: Uuid, rule: &PriceRule) -> Result<(), ApiError> {
    return insert(
        conn,
        tenant_id,
        user_id,
        Change {
            product_id: rule.product_id,
            store_id: rule.store_id,
            customer_group_id: rule.customer_group_id,
            min_quantity: Some(rule.min_quantity),
            kind: "selling",
            previous: Some(rule.unit_price_amount_minor),
            new: None,
            price_rule_id: Some(rule.id),
            effective_from: Utc::now(),
        },
    )
    .await;
}

pub async fn record_cost_change(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    product_id: Uuid,
    store_id: Uuid,
    previous: Option<i64>,
    new: Option<i64>,
) -> Result<(), ApiError> {
    return insert(
        conn,
        tenant_id,
        user_id,
        Change {
            product_id,
            store_id: Some(store_id),
            customer_group_id: None,
            min_quantity: None,
            kind: "cost",
            previous,
            new,
            price_rule_id: None,
            effective_from: Utc::now(),
        },
    )
    .await;
}

/// `GET /api/v1/products/:id/price-history?store_id=`. Newest first; `store_id` keeps that store's changes plus the all-store ones.
pub async fn list_price_history(
    Path(product_id): Path<Uuid>,
    mut tx: TenantTx,
    Query(query): Query<PriceHistoryQuery>,
) -> Result<Json<ApiEnvelope<Vec<PriceChange>>>, ApiError> {
    require_visible(&mut tx, "products", product_id, "Product").await?;
    let changes = sqlx::query_as::<_, PriceChange>(
        "SELECT id, product_id, store_id, customer_group_id, min_quantity, kind::text AS kind, previous_amount_minor,
                new_amount_minor, price_rule_id, effective_from, changed_by_user_id, changed_at
         FROM atlas.price_changes
         WHERE product_id = $1 AND ($2::uuid IS NULL OR store_id IS NULL OR store_id = $2)
         ORDER BY changed_at DESC, id DESC",
    )
    .bind(product_id)
    .bind(query.store_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Price history fetched.", changes)));
}
