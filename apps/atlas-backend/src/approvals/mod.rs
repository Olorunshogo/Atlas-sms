/*!
 * FR-09 and FR-29 approvals. A policy sets the limit for an operation, business-wide or for one store (the store's
 * own wins). A price change or stock change past its limit, raised by someone without the approve permission, is held
 * as a request and only applied when someone approves it. Refunds and expenses keep their own workflows and only read
 * their limit from here.
 */
mod policies;
mod requests;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};
use crate::response::ApiEnvelope;
use crate::roles;
use crate::state::AppState;

pub fn policies_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(policies::list_policies).put(policies::set_policy))
        .route("/{id}", delete(policies::delete_policy));
}

pub fn requests_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(requests::list_requests))
        .route("/{id}", get(requests::show_request))
        .route("/{id}/approve", post(requests::approve_request))
        .route("/{id}/reject", post(requests::reject_request))
        .route("/{id}/cancel", post(requests::cancel_request));
}

// === Operations and limits

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "approval_operation", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    PriceChange,
    StockAdjustment,
    WriteOff,
    Refund,
    Expense,
}

impl Operation {
    /// Who may approve a held request, or go past the limit without one. `None` for operations with their own workflow.
    pub fn approve_permission(self) -> Option<&'static str> {
        return match self {
            Operation::PriceChange => Some("price.approve"),
            Operation::StockAdjustment | Operation::WriteOff => Some("stock.approve"),
            Operation::Refund | Operation::Expense => None,
        };
    }
}

#[derive(Debug, Clone, Copy, sqlx::FromRow)]
pub struct Limit {
    pub limit_amount_minor: Option<i64>,
    pub limit_quantity: Option<Decimal>,
    pub limit_basis_points: Option<i32>,
}

/// The store's own policy if it has one, else the business-wide one; `None` means no limit.
pub async fn limit(conn: &mut PgConnection, tenant_id: Uuid, store_id: Option<Uuid>, operation: Operation) -> Result<Option<Limit>, ApiError> {
    return sqlx::query_as::<_, Limit>(
        "SELECT limit_amount_minor, limit_quantity, limit_basis_points FROM atlas.approval_policies
         WHERE tenant_id = $1 AND operation = $2 AND (store_id = $3 OR store_id IS NULL)
         ORDER BY store_id NULLS LAST
         LIMIT 1",
    )
    .bind(tenant_id)
    .bind(operation)
    .bind(store_id)
    .fetch_optional(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn amount_limit(conn: &mut PgConnection, tenant_id: Uuid, store_id: Uuid, operation: Operation) -> Result<Option<i64>, ApiError> {
    return Ok(limit(conn, tenant_id, Some(store_id), operation).await?.and_then(|l| l.limit_amount_minor));
}

/// How far a change goes, in the unit its operation's limit is set in.
pub enum Measure {
    Quantity(Decimal),
    BasisPoints(i64),
}

/**
 * True when the change is past its limit and the caller can't approve it themselves - the caller should then `hold`
 * it instead of applying it. A store change checks the permission at that store; a business-wide one, business-wide.
 */
pub async fn needs_approval(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    store_id: Option<Uuid>,
    user_id: Uuid,
    operation: Operation,
    measure: Measure,
) -> Result<bool, ApiError> {
    let Some(limit) = limit(conn, tenant_id, store_id, operation).await? else { return Ok(false) };
    let over = match measure {
        Measure::Quantity(quantity) => limit.limit_quantity.is_some_and(|max| quantity.abs() > max),
        Measure::BasisPoints(points) => limit.limit_basis_points.is_some_and(|max| points.abs() > i64::from(max)),
    };
    if !over {
        return Ok(false);
    }
    let Some(permission) = operation.approve_permission() else { return Ok(false) };
    return Ok(!roles::service::has_permission(conn, user_id, tenant_id, store_id, permission).await?);
}

// === Holding a change

pub struct Hold<'a> {
    pub tenant_id: Uuid,
    pub store_id: Option<Uuid>,
    pub operation: Operation,
    /// The product the change is about.
    pub subject_id: Uuid,
    /// The request body, replayed on approval.
    pub payload: Value,
    pub summary: String,
    pub requested_by_user_id: Uuid,
    pub message: &'a str,
}

/// Records the held change, tells whoever can approve it, and answers `202 Accepted` with the request.
pub async fn hold(conn: &mut PgConnection, hold: Hold<'_>) -> Result<Response, ApiError> {
    let request = requests::insert(conn, &hold).await?;
    crate::audit::record(
        conn,
        hold.tenant_id,
        hold.requested_by_user_id,
        "approval_request",
        request.id,
        "approval_request.create",
        serde_json::to_value(&request).ok(),
    )
    .await?;
    crate::notifications::service::approval(conn, hold.tenant_id, hold.requested_by_user_id, request.id, crate::notifications::models::NotificationKind::ApprovalRequested).await;
    return Ok((StatusCode::ACCEPTED, Json(ApiEnvelope::ok(hold.message, request))).into_response());
}

// === Self-approval

/// When the store turns self-approval off, whoever raised a transaction can't approve it.
pub async fn assert_may_approve(conn: &mut PgConnection, store_id: Uuid, approver_id: Uuid, author_id: Uuid) -> Result<(), ApiError> {
    if approver_id != author_id {
        return Ok(());
    }
    let allowed: Option<bool> = sqlx::query_scalar("SELECT allow_self_approval FROM atlas.stores WHERE id = $1")
        .bind(store_id)
        .fetch_optional(conn)
        .await?;
    if allowed.unwrap_or(true) {
        return Ok(());
    }
    return Err(ApiError::conflict_with_code(
        "This store requires someone else to approve what you raised.",
        codes::APPROVAL_SELF_FORBIDDEN,
    ));
}
