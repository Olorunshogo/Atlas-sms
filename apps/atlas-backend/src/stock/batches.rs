/*!
 * FR-14 batches, expiry and unsellable goods at a store.
 * Writes need `stock.adjust` at the store, reads `stock.view`.
 */
use axum::extract::Query;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::approvals;
use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::{require_visible, required_text};

const WRITE_OFF_CONDITIONS: [&str; 4] = ["opened", "damaged", "defective", "expired"];

// === Types

#[derive(Debug, Deserialize)]
pub struct CreateBatchRequest {
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub batch_number: String,
    pub expiry_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Batch {
    pub id: Uuid,
    pub product_id: Uuid,
    pub batch_number: Option<String>,
    pub expiry_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct BatchesQuery {
    pub store_id: Uuid,
    pub product_id: Option<Uuid>,
}

/// A batch with its on-hand at one store. `status` is `ok`, `near_expiry` or `expired`.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BatchStock {
    pub stock_item_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub batch_number: Option<String>,
    pub expiry_date: Option<NaiveDate>,
    pub days_to_expiry: Option<i32>,
    pub on_hand: Decimal,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ExpiryQuery {
    pub store_id: Uuid,
    /// Defaults to the store's `expiry_warning_days`.
    pub within_days: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WriteOffRequest {
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub quantity: Decimal,
    /// `opened` | `damaged` | `defective` | `expired`.
    pub condition: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct UnsellableQuery {
    pub store_id: Uuid,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UnsellableStock {
    pub id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub quantity: Decimal,
    pub condition: String,
    pub source: String,
    pub reference_id: Option<Uuid>,
    pub reason: String,
    pub recorded_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

async fn assert_at(tx: &mut TenantTx, store_id: Uuid, permission: &str) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, Some(store_id), permission).await;
}

/// On-hand per batch at a store; `warning_days` decides `near_expiry`, `only_expiring` keeps just the batches within it.
async fn batch_stock(
    tx: &mut TenantTx,
    store_id: Uuid,
    product_id: Option<Uuid>,
    warning_days: Option<i32>,
    only_expiring: bool,
) -> Result<Vec<BatchStock>, ApiError> {
    return sqlx::query_as::<_, BatchStock>(
        "WITH today AS (SELECT (now() AT TIME ZONE timezone)::date AS d, expiry_warning_days FROM atlas.stores WHERE id = $1)
         SELECT si.id AS stock_item_id, si.product_id, p.name AS product_name, si.batch_number, si.expiry_date,
                (si.expiry_date - today.d)::int AS days_to_expiry,
                SUM(m.quantity) AS on_hand,
                CASE
                    WHEN si.expiry_date < today.d THEN 'expired'
                    WHEN si.expiry_date <= today.d + COALESCE($3, today.expiry_warning_days) THEN 'near_expiry'
                    ELSE 'ok'
                END AS status
         FROM atlas.stock_movements m
         JOIN atlas.stock_items si ON si.id = m.stock_item_id
         JOIN atlas.products p ON p.id = si.product_id
         CROSS JOIN today
         WHERE m.store_id = $1 AND ($2::uuid IS NULL OR m.product_id = $2)
         GROUP BY si.id, si.product_id, p.name, si.batch_number, si.expiry_date, today.d, today.expiry_warning_days
         HAVING SUM(m.quantity) > 0
            AND (NOT $4 OR si.expiry_date <= today.d + COALESCE($3, today.expiry_warning_days))
         ORDER BY si.expiry_date NULLS LAST, p.name, si.id",
    )
    .bind(store_id)
    .bind(product_id)
    .bind(warning_days)
    .bind(only_expiring)
    .fetch_all(&mut **tx)
    .await
    .map_err(ApiError::from);
}

// === Handlers

/// `POST /api/v1/stock/batches`. Registers a batch so opening stock or a count can name it; receiving goods creates batches itself.
pub async fn create_batch(mut tx: TenantTx, Json(body): Json<CreateBatchRequest>) -> Result<Json<ApiEnvelope<Batch>>, ApiError> {
    assert_at(&mut tx, body.store_id, "stock.adjust").await?;
    require_visible(&mut tx, "products", body.product_id, "Product").await?;
    if !inventory::batches::tracks_batches(&mut tx, body.product_id).await? {
        return Err(ApiError::bad_request_with_code("This product isn't batch-tracked.", "VALIDATION_FAILED"));
    }
    let batch_number = required_text("batch_number", &body.batch_number, 64)?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let id = inventory::batches::find_or_create_batch(&mut tx, tenant_id, body.product_id, &batch_number, body.expiry_date).await?;
    let batch = sqlx::query_as::<_, Batch>("SELECT id, product_id, batch_number, expiry_date FROM atlas.stock_items WHERE id = $1")
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    audit::record(&mut tx, tenant_id, user_id, "stock_item", batch.id, "stock_item.create", serde_json::to_value(&batch).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Batch recorded.", batch)));
}

/// `GET /api/v1/stock/batches?store_id=&product_id=`. Batches with stock at the store, earliest expiry first.
pub async fn list_batches(mut tx: TenantTx, Query(query): Query<BatchesQuery>) -> Result<Json<ApiEnvelope<Vec<BatchStock>>>, ApiError> {
    assert_at(&mut tx, query.store_id, "stock.view").await?;
    let batches = batch_stock(&mut tx, query.store_id, query.product_id, None, false).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Batches fetched.", batches)));
}

/// `GET /api/v1/stock/expiry?store_id=&within_days=`. Expired and near-expiry batches still on hand.
pub async fn list_expiring(mut tx: TenantTx, Query(query): Query<ExpiryQuery>) -> Result<Json<ApiEnvelope<Vec<BatchStock>>>, ApiError> {
    assert_at(&mut tx, query.store_id, "stock.view").await?;
    if query.within_days.is_some_and(|days| days < 0) {
        return Err(ApiError::bad_request_with_code("within_days must not be negative", "VALIDATION_FAILED"));
    }
    let batches = batch_stock(&mut tx, query.store_id, None, query.within_days, true).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Expiring stock fetched.", batches)));
}

/// `POST /api/v1/stock/write-offs`. Takes goods out of saleable stock and records them as unsellable; can't overdraw on-hand.
pub async fn write_off(mut tx: TenantTx, Json(body): Json<WriteOffRequest>) -> Result<Response, ApiError> {
    assert_at(&mut tx, body.store_id, "stock.adjust").await?;
    if !WRITE_OFF_CONDITIONS.contains(&body.condition.as_str()) {
        return Err(ApiError::bad_request_with_code("condition must be opened, damaged, defective or expired", "VALIDATION_FAILED"));
    }
    if body.quantity <= Decimal::ZERO {
        return Err(ApiError::bad_request("quantity must be positive"));
    }
    let reason = required_text("reason", &body.reason, 500)?;
    inventory::batches::check_batch_given(&mut tx, body.product_id, body.stock_item_id).await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let measure = approvals::Measure::Quantity(body.quantity);
    if approvals::needs_approval(&mut tx, tenant_id, Some(body.store_id), user_id, approvals::Operation::WriteOff, measure).await? {
        let held = approvals::hold(
            &mut tx,
            approvals::Hold {
                tenant_id,
                store_id: Some(body.store_id),
                operation: approvals::Operation::WriteOff,
                subject_id: body.product_id,
                payload: serde_json::to_value(&body).map_err(|e| ApiError::Internal(e.to_string()))?,
                summary: format!("Write off {} ({}): {}", body.quantity.normalize(), body.condition, reason),
                requested_by_user_id: user_id,
                message: "Write-off held for approval.",
            },
        )
        .await?;
        tx.commit().await?;
        return Ok(held);
    }
    let record = apply_write_off(&mut tx, tenant_id, user_id, &body, None).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Stock written off.", record)).into_response());
}

/// Posts the write-off, records the unsellable stock and its audit row. Also how an approved request applies.
pub async fn apply_write_off(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    body: &WriteOffRequest,
    approval: Option<audit::Approval>,
) -> Result<UnsellableStock, ApiError> {
    let reason = required_text("reason", &body.reason, 500)?;
    let lines = [PostingLine { product_id: body.product_id, stock_item_id: body.stock_item_id, unit_id: None, quantity: body.quantity }];
    let movements = inventory::service::post(
        &mut *conn,
        Posting {
            tenant_id,
            store_id: body.store_id,
            movement_type: MovementType::WriteOff,
            reason: &reason,
            reference_id: None,
            actor_user_id: user_id,
            policy: NegativeStockPolicy::Reject,
            source: "write_off",
            lines: &lines,
        },
    )
    .await?;

    let record = sqlx::query_as::<_, UnsellableStock>(
        "INSERT INTO atlas.unsellable_stock
            (id, tenant_id, store_id, product_id, stock_item_id, quantity, condition, source, reference_id, reason, recorded_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7::atlas.item_condition, 'write_off', $8, $9, $10)
         RETURNING id, product_id, stock_item_id, quantity, condition::text AS condition, source, reference_id, reason, recorded_by_user_id, created_at",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(body.store_id)
    .bind(body.product_id)
    .bind(body.stock_item_id)
    .bind(body.quantity)
    .bind(&body.condition)
    .bind(movements[0].id)
    .bind(&reason)
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await?;
    let after = audit::with_approval(serde_json::to_value(&record).ok(), approval);
    audit::record(conn, tenant_id, user_id, "unsellable_stock", record.id, "stock.write_off", after).await?;
    return Ok(record);
}

/// `GET /api/v1/stock/unsellable?store_id=`. Write-offs and non-restocked returns, newest first.
pub async fn list_unsellable(mut tx: TenantTx, Query(query): Query<UnsellableQuery>) -> Result<Json<ApiEnvelope<Vec<UnsellableStock>>>, ApiError> {
    assert_at(&mut tx, query.store_id, "stock.view").await?;
    let records = sqlx::query_as::<_, UnsellableStock>(
        "SELECT id, product_id, stock_item_id, quantity, condition::text AS condition, source, reference_id, reason, recorded_by_user_id, created_at
         FROM atlas.unsellable_stock WHERE store_id = $1 ORDER BY created_at DESC, id DESC",
    )
    .bind(query.store_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Unsellable stock fetched.", records)));
}
