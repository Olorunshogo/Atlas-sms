/*!
 * Loyalty points, as a signed ledger. Points are earned on what was paid for goods after discounts (no shipping, no
 * tax) once an order is delivered and paid, and only count once the store's return window has passed. Redeeming is
 * capped at a share of the discounted goods; cancelling or expiring an order reverses everything it touched.
 */
use axum::Json;
use chrono::{DateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::pricing::engine;
use crate::response::ApiEnvelope;

use super::context::{ShopTx, Storefront};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LoyaltyEntry {
    pub id: Uuid,
    pub order_id: Option<Uuid>,
    pub kind: String,
    pub points: i32,
    pub available_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct LoyaltyAccount {
    pub enabled: bool,
    pub available_points: i64,
    /// Earned but still inside the return window.
    pub pending_points: i64,
    pub point_value_minor: i64,
    pub entries: Vec<LoyaltyEntry>,
}

pub async fn balance(conn: &mut PgConnection, shopper_id: Uuid) -> Result<(i64, i64), ApiError> {
    let (available, pending): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(points) FILTER (WHERE available_at <= now()), 0)::bigint,
                COALESCE(SUM(points) FILTER (WHERE available_at > now()), 0)::bigint
         FROM atlas.loyalty_entries WHERE shopper_id = $1",
    )
    .bind(shopper_id)
    .fetch_one(conn)
    .await?;
    return Ok((available, pending));
}

/// The most points this order may spend: the storefront's share of the discounted goods, in whole points.
pub fn max_redeemable(storefront: &Storefront, discounted_goods_minor: i64) -> i64 {
    let cap_minor = engine::round_minor(
        Decimal::from(discounted_goods_minor) * Decimal::from(storefront.loyalty_max_redeem_basis_points) / Decimal::from(engine::BASIS_POINTS),
    );
    return (cap_minor / storefront.loyalty_point_value_minor).max(0);
}

pub async fn record_redeem(conn: &mut PgConnection, tenant_id: Uuid, shopper_id: Uuid, order_id: Uuid, points: i64) -> Result<(), ApiError> {
    if points <= 0 {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO atlas.loyalty_entries (id, tenant_id, shopper_id, order_id, kind, points, available_at)
         VALUES ($1, $2, $3, $4, 'redeem', $5, now())",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(shopper_id)
    .bind(order_id)
    .bind(-(points as i32))
    .execute(conn)
    .await?;
    return Ok(());
}

/**
 * Earns points for a delivered, paid order, once. Usable from the end of the store's return window.
 * `eligible_minor` is the goods after discounts, without shipping or tax.
 */
pub async fn earn(conn: &mut PgConnection, storefront: &Storefront, order_id: Uuid) -> Result<(), ApiError> {
    let Some(minor_per_point) = storefront.loyalty_minor_per_point else { return Ok(()) };
    let row: Option<(Uuid, Uuid, i64, bool)> = sqlx::query_as(
        "SELECT o.tenant_id, o.shopper_id,
                COALESCE((SELECT SUM(l.net_minor - CASE WHEN o.prices_include_tax THEN l.tax_minor ELSE 0 END)
                          FROM atlas.shop_order_lines l WHERE l.order_id = o.id), 0)::bigint - o.points_value_minor,
                EXISTS (SELECT 1 FROM atlas.loyalty_entries e WHERE e.order_id = o.id AND e.kind = 'earn')
         FROM atlas.shop_orders o
         WHERE o.id = $1 AND o.shopper_id IS NOT NULL AND o.status = 'delivered' AND o.paid_at IS NOT NULL",
    )
    .bind(order_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((tenant_id, shopper_id, eligible_minor, already)) = row else { return Ok(()) };
    let points = (eligible_minor.max(0) / minor_per_point).to_i32().unwrap_or(0);
    if already || points <= 0 {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO atlas.loyalty_entries (id, tenant_id, shopper_id, order_id, kind, points, available_at)
         SELECT $1, $2, $3, $4, 'earn', $5, now() + make_interval(days => s.return_window_days)
         FROM atlas.stores s WHERE s.id = $6",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(shopper_id)
    .bind(order_id)
    .bind(points)
    .bind(storefront.store_id)
    .execute(conn)
    .await?;
    return Ok(());
}

/// Undoes every point an order earned or spent, with one balancing entry.
pub async fn reverse(conn: &mut PgConnection, order_id: Uuid) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO atlas.loyalty_entries (id, tenant_id, shopper_id, order_id, kind, points, available_at)
         SELECT gen_random_uuid(), tenant_id, shopper_id, order_id, 'reverse', -SUM(points)::int, now()
         FROM atlas.loyalty_entries WHERE order_id = $1
         GROUP BY tenant_id, shopper_id, order_id
         HAVING SUM(points) <> 0",
    )
    .bind(order_id)
    .execute(conn)
    .await?;
    return Ok(());
}

/// `GET /api/v1/shop/:handle/account/loyalty`.
pub async fn account(mut tx: ShopTx) -> Result<Json<ApiEnvelope<LoyaltyAccount>>, ApiError> {
    let shopper_id = tx.require_shopper()?;
    let (available_points, pending_points) = balance(&mut tx, shopper_id).await?;
    let entries = sqlx::query_as::<_, LoyaltyEntry>(
        "SELECT id, order_id, kind::text AS kind, points, available_at, created_at FROM atlas.loyalty_entries
         WHERE shopper_id = $1 ORDER BY created_at DESC, id DESC LIMIT 100",
    )
    .bind(shopper_id)
    .fetch_all(&mut *tx)
    .await?;
    let account = LoyaltyAccount {
        enabled: tx.storefront.loyalty_minor_per_point.is_some(),
        available_points,
        pending_points,
        point_value_minor: tx.storefront.loyalty_point_value_minor,
        entries,
    };
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Loyalty account fetched.", account)));
}
