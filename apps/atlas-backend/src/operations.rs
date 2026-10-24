/*!
 * FR-30 operational views for administrators (`audit.view`): problems that didn't fail a request but need a person -
 * dropped notifications, emails that never went out, and stock oversold by offline sales.
 */
use axum::extract::{Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct IssuesQuery {
    /// `open` (default), `resolved` or `all`.
    pub status: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OperationalIssue {
    pub id: Uuid,
    pub kind: String,
    pub detail: String,
    pub reference_id: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct StockExceptionsQuery {
    pub store_id: Option<Uuid>,
    #[serde(default)]
    pub include_resolved: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StockException {
    pub id: Uuid,
    pub store_id: Uuid,
    pub product_id: Uuid,
    pub stock_item_id: Option<Uuid>,
    pub movement_id: Uuid,
    pub on_hand_before: Decimal,
    pub requested: Decimal,
    pub source: String,
    pub reference_id: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/issues", get(list_issues))
        .route("/issues/{id}/resolve", post(resolve_issue))
        .route("/stock-exceptions", get(list_stock_exceptions))
        .route("/stock-exceptions/{id}/resolve", post(resolve_stock_exception));
}

async fn assert_admin(tx: &mut TenantTx) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::assert_permission(tx, user_id, tenant_id, None, "audit.view").await;
}

/// `GET /api/v1/operations/issues?status=&kind=`. Newest first.
pub async fn list_issues(mut tx: TenantTx, Query(query): Query<IssuesQuery>) -> Result<Json<ApiEnvelope<Vec<OperationalIssue>>>, ApiError> {
    assert_admin(&mut tx).await?;
    let status = query.status.as_deref().unwrap_or("open");
    if !["open", "resolved", "all"].contains(&status) {
        return Err(ApiError::bad_request_with_code("status must be open, resolved or all", "VALIDATION_FAILED"));
    }
    let tenant_id = tx.tenant_id;
    let issues = sqlx::query_as::<_, OperationalIssue>(
        "SELECT id, kind, detail, reference_id, resolved_at, created_at FROM atlas.operational_issues
         WHERE tenant_id = $1
           AND ($2 = 'all' OR ($2 = 'open') = (resolved_at IS NULL))
           AND ($3::text IS NULL OR kind = $3)
         ORDER BY created_at DESC, id DESC
         LIMIT 200",
    )
    .bind(tenant_id)
    .bind(status)
    .bind(query.kind.as_deref())
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Operational issues fetched.", issues)));
}

/// `POST /api/v1/operations/issues/:id/resolve`.
pub async fn resolve_issue(Path(issue_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<OperationalIssue>>, ApiError> {
    assert_admin(&mut tx).await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let issue = sqlx::query_as::<_, OperationalIssue>(
        "UPDATE atlas.operational_issues SET resolved_at = COALESCE(resolved_at, now())
         WHERE id = $1 AND tenant_id = $2
         RETURNING id, kind, detail, reference_id, resolved_at, created_at",
    )
    .bind(issue_id)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Issue not found."))?;
    audit::record(&mut tx, tenant_id, user_id, "operational_issue", issue_id, "operations.resolve_issue", serde_json::to_value(&issue).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Issue resolved.", issue)));
}

/// `GET /api/v1/operations/stock-exceptions?store_id=&include_resolved=`. Sales that took stock below zero while offline.
pub async fn list_stock_exceptions(
    mut tx: TenantTx,
    Query(query): Query<StockExceptionsQuery>,
) -> Result<Json<ApiEnvelope<Vec<StockException>>>, ApiError> {
    assert_admin(&mut tx).await?;
    let tenant_id = tx.tenant_id;
    let exceptions = sqlx::query_as::<_, StockException>(
        "SELECT id, store_id, product_id, stock_item_id, movement_id, on_hand_before, requested, source, reference_id, resolved_at, created_at
         FROM atlas.stock_exceptions
         WHERE tenant_id = $1 AND ($2::uuid IS NULL OR store_id = $2) AND ($3 OR resolved_at IS NULL)
         ORDER BY created_at DESC
         LIMIT 200",
    )
    .bind(tenant_id)
    .bind(query.store_id)
    .bind(query.include_resolved)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Stock exceptions fetched.", exceptions)));
}

/// `POST /api/v1/operations/stock-exceptions/:id/resolve`. Marks it handled (usually after a count fixed the balance).
pub async fn resolve_stock_exception(Path(exception_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<StockException>>, ApiError> {
    assert_admin(&mut tx).await?;
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let exception = sqlx::query_as::<_, StockException>(
        "UPDATE atlas.stock_exceptions SET resolved_at = COALESCE(resolved_at, now()), resolved_by_user_id = COALESCE(resolved_by_user_id, $3)
         WHERE id = $1 AND tenant_id = $2
         RETURNING id, store_id, product_id, stock_item_id, movement_id, on_hand_before, requested, source, reference_id, resolved_at, created_at",
    )
    .bind(exception_id)
    .bind(tenant_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Stock exception not found."))?;
    audit::record(&mut tx, tenant_id, user_id, "stock_exception", exception_id, "operations.resolve_stock_exception", serde_json::to_value(&exception).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Stock exception resolved.", exception)));
}
