/*!
 * FR-12 physical stock counts: counted vs recorded quantities are visible as a pending count, and stock moves only when it is approved.
 * Every write needs `stock.adjust` at the count's store.
 */
use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::audit;
use crate::error::{codes, ApiError};
use crate::extractors::TenantTx;
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};
use crate::notifications::{self, NotificationKind};
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::{require_visible, required_text};

const COUNT_COLUMNS: &str = "id, store_id, status::text AS status, reason, created_by_user_id, created_at, decided_by_user_id, decided_at";

// === Types

#[derive(Debug, Deserialize)]
pub struct CountLineInput {
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub counted_quantity: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CreateCountRequest {
    pub store_id: Uuid,
    pub reason: String,
    pub lines: Vec<CountLineInput>,
}

#[derive(Debug, Deserialize)]
pub struct ListCountsQuery {
    pub store_id: Uuid,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CountLine {
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub counted_quantity: Decimal,
    pub recorded_quantity: Decimal,
    pub variance: Decimal,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StockCount {
    pub id: Uuid,
    pub store_id: Uuid,
    pub status: String,
    pub reason: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub decided_by_user_id: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
    #[sqlx(skip)]
    pub lines: Vec<CountLine>,
}

// === Service

async fn load(conn: &mut PgConnection, count_id: Uuid) -> Result<StockCount, ApiError> {
    let mut count = sqlx::query_as::<_, StockCount>(&format!("SELECT {COUNT_COLUMNS} FROM atlas.stock_counts WHERE id = $1"))
        .bind(count_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| ApiError::not_found("Stock count not found."))?;
    count.lines = sqlx::query_as::<_, CountLine>(
        "SELECT product_id, stock_item_id, counted_quantity, recorded_quantity, variance
         FROM atlas.stock_count_lines WHERE stock_count_id = $1 ORDER BY id",
    )
    .bind(count_id)
    .fetch_all(conn)
    .await?;
    return Ok(count);
}

async fn assert_adjust(tx: &mut TenantTx, store_id: Uuid) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, Some(store_id), "stock.adjust").await;
}

/// Claims `pending` -> `status` so two approvers can't both post the variance.
async fn decide(conn: &mut PgConnection, count_id: Uuid, user_id: Uuid, status: &str) -> Result<(), ApiError> {
    let claimed: Option<Uuid> = sqlx::query_scalar(
        "UPDATE atlas.stock_counts SET status = $3::atlas.stock_count_status, decided_by_user_id = $2, decided_at = now()
         WHERE id = $1 AND status = 'pending'
         RETURNING id",
    )
    .bind(count_id)
    .bind(user_id)
    .bind(status)
    .fetch_optional(conn)
    .await?;
    return claimed
        .map(|_| ())
        .ok_or_else(|| ApiError::conflict_with_code("Only a pending stock count can be approved or rejected.", codes::STOCK_COUNT_NOT_PENDING));
}

// === Handlers

/// `POST /api/v1/stock/counts`. Snapshots the recorded quantity of each counted line; stock is unchanged until approval.
pub async fn create_count(mut tx: TenantTx, Json(body): Json<CreateCountRequest>) -> Result<Json<ApiEnvelope<StockCount>>, ApiError> {
    assert_adjust(&mut tx, body.store_id).await?;
    let reason = required_text("reason", &body.reason, 500)?;
    if body.lines.is_empty() {
        return Err(ApiError::bad_request_with_code("a count needs at least one line", "VALIDATION_FAILED"));
    }
    if body.lines.iter().any(|line| line.counted_quantity < Decimal::ZERO) {
        return Err(ApiError::bad_request_with_code("counted_quantity must not be negative", "VALIDATION_FAILED"));
    }
    require_visible(&mut tx, "stores", body.store_id, "Store").await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);

    let count_id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.stock_counts (id, tenant_id, store_id, reason, created_by_user_id) VALUES ($1, $2, $3, $4, $5)")
        .bind(count_id)
        .bind(tenant_id)
        .bind(body.store_id)
        .bind(&reason)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    for line in &body.lines {
        require_visible(&mut tx, "products", line.product_id, "Product").await?;
        let recorded = inventory::service::on_hand(&mut tx, body.store_id, line.product_id, line.stock_item_id).await?;
        sqlx::query(
            "INSERT INTO atlas.stock_count_lines (id, tenant_id, stock_count_id, product_id, stock_item_id, counted_quantity, recorded_quantity)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(count_id)
        .bind(line.product_id)
        .bind(line.stock_item_id)
        .bind(line.counted_quantity)
        .bind(recorded)
        .execute(&mut *tx)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                ApiError::bad_request_with_code("each product or batch can only be counted once per count", "VALIDATION_FAILED")
            }
            other => ApiError::from(other),
        })?;
    }

    let count = load(&mut tx, count_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "stock_count", count_id, "stock_count.create", serde_json::to_value(&count).ok()).await?;
    notifications::service::stock_count(&mut tx, tenant_id, user_id, count_id, NotificationKind::StockCountSubmitted).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Stock count recorded.", count)));
}

/// `GET /api/v1/stock/counts?store_id=`. Requires `stock.view` at the store. Newest first, lines included.
pub async fn list_counts(mut tx: TenantTx, Query(query): Query<ListCountsQuery>) -> Result<Json<ApiEnvelope<Vec<StockCount>>>, ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(query.store_id), "stock.view").await?;
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM atlas.stock_counts WHERE store_id = $1 ORDER BY created_at DESC")
        .bind(query.store_id)
        .fetch_all(&mut *tx)
        .await?;
    let mut counts = Vec::with_capacity(ids.len());
    for id in ids {
        counts.push(load(&mut tx, id).await?);
    }
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Stock counts fetched.", counts)));
}

/// `POST /api/v1/stock/counts/:id/approve`. Posts each non-zero variance as an adjustment; the count is the authority, so it isn't blocked by on-hand.
pub async fn approve_count(Path(count_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<StockCount>>, ApiError> {
    let existing = load(&mut tx, count_id).await?;
    assert_adjust(&mut tx, existing.store_id).await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    decide(&mut tx, count_id, user_id, "approved").await?;

    let lines: Vec<PostingLine> = existing
        .lines
        .iter()
        .filter(|line| line.variance != Decimal::ZERO)
        .map(|line| PostingLine {
            product_id: line.product_id,
            stock_item_id: line.stock_item_id,
            unit_id: None,
            quantity: line.variance,
        })
        .collect();
    if !lines.is_empty() {
        let reason = format!("Stock count: {}", existing.reason);
        inventory::service::post(
            &mut tx,
            Posting {
                tenant_id,
                store_id: existing.store_id,
                movement_type: MovementType::Adjustment,
                reason: &reason,
                reference_id: Some(count_id),
                actor_user_id: user_id,
                policy: NegativeStockPolicy::Allow,
                source: "stock_count",
                lines: &lines,
            },
        )
        .await?;
    }

    let count = load(&mut tx, count_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "stock_count", count_id, "stock_count.approve", serde_json::to_value(&count).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Stock count approved.", count)));
}

/// `POST /api/v1/stock/counts/:id/reject`. Leaves stock untouched.
pub async fn reject_count(Path(count_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<StockCount>>, ApiError> {
    let existing = load(&mut tx, count_id).await?;
    assert_adjust(&mut tx, existing.store_id).await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    decide(&mut tx, count_id, user_id, "rejected").await?;
    let count = load(&mut tx, count_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "stock_count", count_id, "stock_count.reject", serde_json::to_value(&count).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Stock count rejected.", count)));
}
