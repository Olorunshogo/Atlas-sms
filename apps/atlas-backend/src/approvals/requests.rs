/*!
 * Held changes waiting for a decision. Approving replays the stored request body inside the approver's transaction:
 * if it no longer fits (the stock has since been sold, the product removed), the approval fails and the request stays
 * pending. The applied change keeps the requester as its author and names the request and approver (FR-27).
 */
use std::collections::BTreeMap;

use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::audit;
use crate::error::{codes, ApiError};
use crate::extractors::TenantTx;
use crate::notifications::models::NotificationKind;
use crate::notifications::service as notifications;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::required_text;

use super::{Hold, Operation};

const REQUEST_COLUMNS: &str = "id, store_id, operation, status::text AS status, subject_id, payload, summary,
     requested_by_user_id, requested_at, decided_by_user_id, decided_at, decision_note, result_entity_type, result_id";

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub store_id: Option<Uuid>,
    pub operation: Operation,
    /// `pending` | `approved` | `rejected` | `cancelled`.
    pub status: String,
    pub subject_id: Uuid,
    pub payload: Value,
    pub summary: String,
    pub requested_by_user_id: Uuid,
    pub requested_at: DateTime<Utc>,
    pub decided_by_user_id: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decision_note: Option<String>,
    /// What approving created: `stock_movement`, `unsellable_stock` or `price_rule`.
    pub result_entity_type: Option<String>,
    pub result_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct RequestsQuery {
    /// `pending` (default), `approved`, `rejected`, `cancelled` or `all`.
    pub status: Option<String>,
    pub store_id: Option<Uuid>,
    pub operation: Option<Operation>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}

pub async fn insert(conn: &mut PgConnection, hold: &Hold<'_>) -> Result<ApprovalRequest, ApiError> {
    return sqlx::query_as::<_, ApprovalRequest>(&format!(
        "INSERT INTO atlas.approval_requests (id, tenant_id, store_id, operation, subject_id, payload, summary, requested_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING {}",
        REQUEST_COLUMNS
    ))
    .bind(Uuid::now_v7())
    .bind(hold.tenant_id)
    .bind(hold.store_id)
    .bind(hold.operation)
    .bind(hold.subject_id)
    .bind(&hold.payload)
    .bind(&hold.summary)
    .bind(hold.requested_by_user_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

async fn fetch(conn: &mut PgConnection, request_id: Uuid, lock: bool) -> Result<ApprovalRequest, ApiError> {
    return sqlx::query_as::<_, ApprovalRequest>(&format!(
        "SELECT {} FROM atlas.approval_requests WHERE id = $1{}",
        REQUEST_COLUMNS,
        if lock { " FOR UPDATE" } else { "" }
    ))
    .bind(request_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Approval request not found."));
}

async fn may_decide(tx: &mut TenantTx, request: &ApprovalRequest) -> Result<bool, ApiError> {
    let Some(permission) = request.operation.approve_permission() else { return Ok(false) };
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    return roles::service::has_permission(tx, user_id, tenant_id, request.store_id, permission).await;
}

/// The requester sees their own requests; anyone who could decide one sees it too. Others get a 404, not a 403.
async fn visible(tx: &mut TenantTx, request_id: Uuid, lock: bool) -> Result<ApprovalRequest, ApiError> {
    let request = fetch(tx, request_id, lock).await?;
    if request.requested_by_user_id == tx.user_id || may_decide(tx, &request).await? {
        return Ok(request);
    }
    return Err(ApiError::not_found("Approval request not found."));
}

fn not_pending() -> ApiError {
    return ApiError::conflict_with_code("This request has already been decided.", codes::APPROVAL_NOT_PENDING);
}

/// `GET /api/v1/approval-requests?status=&store_id=&operation=&limit=`. Newest first; only requests the caller raised or may decide.
pub async fn list_requests(mut tx: TenantTx, Query(query): Query<RequestsQuery>) -> Result<Json<ApiEnvelope<Vec<ApprovalRequest>>>, ApiError> {
    let status = query.status.as_deref().unwrap_or("pending");
    if !["pending", "approved", "rejected", "cancelled", "all"].contains(&status) {
        return Err(ApiError::bad_request("status must be pending, approved, rejected, cancelled or all"));
    }
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let candidates = sqlx::query_as::<_, ApprovalRequest>(&format!(
        "SELECT {} FROM atlas.approval_requests
         WHERE tenant_id = $1
           AND ($2 = 'all' OR status = $2::atlas.approval_request_status)
           AND ($3::uuid IS NULL OR store_id = $3)
           AND ($4::atlas.approval_operation IS NULL OR operation = $4)
         ORDER BY requested_at DESC, id DESC
         LIMIT $5",
        REQUEST_COLUMNS
    ))
    .bind(tx.tenant_id)
    .bind(status)
    .bind(query.store_id)
    .bind(query.operation)
    .bind(MAX_LIMIT)
    .fetch_all(&mut *tx)
    .await?;

    let mut decidable: BTreeMap<(Option<Uuid>, &'static str), bool> = BTreeMap::new();
    let mut requests = Vec::new();
    for request in candidates {
        let mine = request.requested_by_user_id == tx.user_id;
        let allowed = match request.operation.approve_permission() {
            _ if mine => true,
            None => false,
            Some(permission) => match decidable.get(&(request.store_id, permission)) {
                Some(known) => *known,
                None => {
                    let can = may_decide(&mut tx, &request).await?;
                    decidable.insert((request.store_id, permission), can);
                    can
                }
            },
        };
        if allowed {
            requests.push(request);
        }
        if requests.len() as i64 >= limit {
            break;
        }
    }
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Approval requests fetched.", requests)));
}

/// `GET /api/v1/approval-requests/:id`.
pub async fn show_request(Path(request_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<ApprovalRequest>>, ApiError> {
    let request = visible(&mut tx, request_id, false).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Approval request fetched.", request)));
}

/// Replays the held change as its requester and says what it created.
async fn apply(tx: &mut TenantTx, request: &ApprovalRequest) -> Result<(&'static str, Uuid), ApiError> {
    let tenant_id = tx.tenant_id;
    let approval = audit::Approval { request_id: request.id, approved_by_user_id: tx.user_id };
    let invalid = |err: serde_json::Error| ApiError::Internal(format!("held request body no longer reads: {}", err));
    return match request.operation {
        Operation::StockAdjustment => {
            let body: crate::stock::AdjustStockRequest = serde_json::from_value(request.payload.clone()).map_err(invalid)?;
            let movement = crate::stock::apply_adjustment(tx, tenant_id, request.requested_by_user_id, &body, Some(approval)).await?;
            Ok(("stock_movement", movement.id))
        }
        Operation::WriteOff => {
            let body: crate::stock::WriteOffRequest = serde_json::from_value(request.payload.clone()).map_err(invalid)?;
            let record = crate::stock::apply_write_off(tx, tenant_id, request.requested_by_user_id, &body, Some(approval)).await?;
            Ok(("unsellable_stock", record.id))
        }
        Operation::PriceChange => {
            let body: crate::products::CreatePriceRuleRequest = serde_json::from_value(request.payload.clone()).map_err(invalid)?;
            let rule = crate::products::apply_price_rule(tx, tenant_id, request.requested_by_user_id, request.subject_id, &body, Some(approval)).await?;
            Ok(("price_rule", rule.id))
        }
        Operation::Refund | Operation::Expense => Err(ApiError::Internal("refunds and expenses aren't held here".to_string())),
    };
}

/**
 * `POST /api/v1/approval-requests/:id/approve`. Needs the operation's approve permission (`price.approve` or
 * `stock.approve`) where the change applies. The requester can't approve their own request when the store turns
 * self-approval off, and never for a business-wide one.
 */
pub async fn approve_request(Path(request_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<ApprovalRequest>>, ApiError> {
    let request = visible(&mut tx, request_id, true).await?;
    if request.status != "pending" {
        return Err(not_pending());
    }
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    if let Some(permission) = request.operation.approve_permission() {
        roles::service::assert_permission(&mut tx, user_id, tenant_id, request.store_id, permission).await?;
    }
    match request.store_id {
        Some(store_id) => super::assert_may_approve(&mut tx, store_id, user_id, request.requested_by_user_id).await?,
        None if user_id == request.requested_by_user_id => {
            return Err(ApiError::conflict_with_code("Someone else has to approve a business-wide change you raised.", codes::APPROVAL_SELF_FORBIDDEN));
        }
        None => {}
    }
    let (result_entity_type, result_id) = apply(&mut tx, &request).await?;
    let decided = sqlx::query_as::<_, ApprovalRequest>(&format!(
        "UPDATE atlas.approval_requests
         SET status = 'approved', decided_by_user_id = $2, decided_at = now(), result_entity_type = $3, result_id = $4
         WHERE id = $1 AND status = 'pending'
         RETURNING {}",
        REQUEST_COLUMNS
    ))
    .bind(request_id)
    .bind(user_id)
    .bind(result_entity_type)
    .bind(result_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(not_pending)?;
    audit::record(&mut tx, tenant_id, user_id, "approval_request", request_id, "approval_request.approve", serde_json::to_value(&decided).ok()).await?;
    notifications::approval(&mut tx, tenant_id, user_id, request_id, NotificationKind::ApprovalDecided).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Request approved and applied.", decided)));
}

/// `POST /api/v1/approval-requests/:id/reject` with `{ "reason" }`. Same permission and self-approval rules as approving.
pub async fn reject_request(
    Path(request_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<RejectRequest>,
) -> Result<Json<ApiEnvelope<ApprovalRequest>>, ApiError> {
    let reason = required_text("reason", &body.reason, 500)?;
    let request = visible(&mut tx, request_id, true).await?;
    if request.status != "pending" {
        return Err(not_pending());
    }
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    if let Some(permission) = request.operation.approve_permission() {
        roles::service::assert_permission(&mut tx, user_id, tenant_id, request.store_id, permission).await?;
    }
    if let Some(store_id) = request.store_id {
        super::assert_may_approve(&mut tx, store_id, user_id, request.requested_by_user_id).await?;
    }
    let decided = decide(&mut tx, request_id, "rejected", Some(&reason)).await?;
    audit::record(&mut tx, tenant_id, user_id, "approval_request", request_id, "approval_request.reject", serde_json::to_value(&decided).ok()).await?;
    notifications::approval(&mut tx, tenant_id, user_id, request_id, NotificationKind::ApprovalDecided).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Request rejected.", decided)));
}

/// `POST /api/v1/approval-requests/:id/cancel`. Only the requester, only while it's pending.
pub async fn cancel_request(Path(request_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<ApprovalRequest>>, ApiError> {
    let request = visible(&mut tx, request_id, true).await?;
    if request.requested_by_user_id != tx.user_id {
        return Err(ApiError::forbidden("Only whoever raised a request can cancel it."));
    }
    if request.status != "pending" {
        return Err(not_pending());
    }
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let decided = decide(&mut tx, request_id, "cancelled", None).await?;
    audit::record(&mut tx, tenant_id, user_id, "approval_request", request_id, "approval_request.cancel", serde_json::to_value(&decided).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Request cancelled.", decided)));
}

async fn decide(tx: &mut TenantTx, request_id: Uuid, status: &str, note: Option<&str>) -> Result<ApprovalRequest, ApiError> {
    let user_id = tx.user_id;
    return sqlx::query_as::<_, ApprovalRequest>(&format!(
        "UPDATE atlas.approval_requests
         SET status = $2::atlas.approval_request_status, decided_by_user_id = $3, decided_at = now(), decision_note = $4
         WHERE id = $1 AND status = 'pending'
         RETURNING {}",
        REQUEST_COLUMNS
    ))
    .bind(request_id)
    .bind(status)
    .bind(user_id)
    .bind(note)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(not_pending);
}
