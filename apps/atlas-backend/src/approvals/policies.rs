/*!
 * FR-29 and FR-30 approval policy management. A business-wide policy needs `tenant.manage`; a store's own needs
 * `store.update` at that store. Every change is audited with what it replaced.
 */
use axum::extract::{Path, Query};
use axum::Json;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::roles;
use crate::validate::require_visible;

use super::Operation;

const POLICY_COLUMNS: &str =
    "id, store_id, operation, limit_amount_minor, limit_quantity, limit_basis_points, updated_by_user_id, created_at, updated_at";

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ApprovalPolicy {
    pub id: Uuid,
    /// `None` for the business-wide policy.
    pub store_id: Option<Uuid>,
    pub operation: Operation,
    pub limit_amount_minor: Option<i64>,
    pub limit_quantity: Option<Decimal>,
    pub limit_basis_points: Option<i32>,
    pub updated_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PoliciesQuery {
    pub store_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct SetPolicyRequest {
    pub store_id: Option<Uuid>,
    pub operation: Operation,
    pub limit_amount_minor: Option<i64>,
    pub limit_quantity: Option<Decimal>,
    pub limit_basis_points: Option<i32>,
}

async fn assert_manage(tx: &mut TenantTx, store_id: Option<Uuid>) -> Result<(), ApiError> {
    let (user_id, tenant_id) = (tx.user_id, tx.tenant_id);
    let permission = if store_id.is_some() { "store.update" } else { "tenant.manage" };
    return roles::service::assert_permission(tx, user_id, tenant_id, store_id, permission).await;
}

/// Each operation is limited in one unit; anything else is refused rather than silently ignored.
fn check_limit(body: &SetPolicyRequest) -> Result<(), ApiError> {
    let invalid = |message: &str| Err(ApiError::bad_request_with_code(message.to_string(), "VALIDATION_FAILED"));
    let (amount, quantity, points) = (body.limit_amount_minor, body.limit_quantity, body.limit_basis_points);
    let given = [amount.is_some(), quantity.is_some(), points.is_some()].iter().filter(|g| **g).count();
    if given != 1 {
        return invalid("give exactly one of limit_amount_minor, limit_quantity or limit_basis_points");
    }
    return match body.operation {
        Operation::PriceChange if points.is_some_and(|p| p > 0) => Ok(()),
        Operation::PriceChange => invalid("a price change policy takes a positive limit_basis_points"),
        Operation::StockAdjustment | Operation::WriteOff if quantity.is_some_and(|q| q > Decimal::ZERO) => Ok(()),
        Operation::StockAdjustment | Operation::WriteOff => invalid("a stock policy takes a positive limit_quantity"),
        Operation::Refund | Operation::Expense if amount.is_some_and(|a| a > 0) => Ok(()),
        Operation::Refund | Operation::Expense => invalid("a refund or expense policy takes a positive limit_amount_minor"),
    };
}

/**
 * `GET /api/v1/approval-policies?store_id=`. Without `store_id`, every policy in the business (`tenant.manage`);
 * with it, the business-wide policies plus that store's own (`store.update` there).
 */
pub async fn list_policies(mut tx: TenantTx, Query(query): Query<PoliciesQuery>) -> Result<Json<ApiEnvelope<Vec<ApprovalPolicy>>>, ApiError> {
    assert_manage(&mut tx, query.store_id).await?;
    let policies = sqlx::query_as::<_, ApprovalPolicy>(&format!(
        "SELECT {} FROM atlas.approval_policies
         WHERE tenant_id = $1 AND ($2::uuid IS NULL OR store_id IS NULL OR store_id = $2)
         ORDER BY operation, store_id NULLS FIRST",
        POLICY_COLUMNS
    ))
    .bind(tx.tenant_id)
    .bind(query.store_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Approval policies fetched.", policies)));
}

/// `PUT /api/v1/approval-policies`. Creates or replaces the policy for that operation and scope.
pub async fn set_policy(mut tx: TenantTx, Json(body): Json<SetPolicyRequest>) -> Result<Json<ApiEnvelope<ApprovalPolicy>>, ApiError> {
    assert_manage(&mut tx, body.store_id).await?;
    check_limit(&body)?;
    if let Some(store_id) = body.store_id {
        require_visible(&mut tx, "stores", store_id, "Store").await?;
    }
    let (tenant_id, user_id) = (tx.tenant_id, tx.user_id);
    let before = sqlx::query_as::<_, ApprovalPolicy>(&format!(
        "SELECT {} FROM atlas.approval_policies WHERE tenant_id = $1 AND store_id IS NOT DISTINCT FROM $2 AND operation = $3",
        POLICY_COLUMNS
    ))
    .bind(tenant_id)
    .bind(body.store_id)
    .bind(body.operation)
    .fetch_optional(&mut *tx)
    .await?;
    let policy = sqlx::query_as::<_, ApprovalPolicy>(&format!(
        "INSERT INTO atlas.approval_policies
            (id, tenant_id, store_id, operation, limit_amount_minor, limit_quantity, limit_basis_points, updated_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (tenant_id, store_id, operation) DO UPDATE
         SET limit_amount_minor = EXCLUDED.limit_amount_minor,
             limit_quantity = EXCLUDED.limit_quantity,
             limit_basis_points = EXCLUDED.limit_basis_points,
             updated_by_user_id = EXCLUDED.updated_by_user_id,
             updated_at = now()
         RETURNING {}",
        POLICY_COLUMNS
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(body.store_id)
    .bind(body.operation)
    .bind(body.limit_amount_minor)
    .bind(body.limit_quantity)
    .bind(body.limit_basis_points)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    audit::record_change(
        &mut tx,
        audit::Change {
            tenant_id,
            actor_user_id: user_id,
            device_id: None,
            entity_type: "approval_policy",
            entity_id: policy.id,
            action: "approval_policy.set",
            before: before.and_then(|b| serde_json::to_value(&b).ok()),
            after: serde_json::to_value(&policy).ok(),
        },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Approval policy saved.", policy)));
}

/// `DELETE /api/v1/approval-policies/:id`. The operation then falls back to the business-wide policy, or no limit.
pub async fn delete_policy(Path(policy_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<ApprovalPolicy>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let policy = sqlx::query_as::<_, ApprovalPolicy>(&format!(
        "SELECT {} FROM atlas.approval_policies WHERE id = $1 AND tenant_id = $2",
        POLICY_COLUMNS
    ))
    .bind(policy_id)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("Approval policy not found."))?;
    assert_manage(&mut tx, policy.store_id).await?;
    sqlx::query("DELETE FROM atlas.approval_policies WHERE id = $1").bind(policy_id).execute(&mut *tx).await?;
    let user_id = tx.user_id;
    audit::record_change(
        &mut tx,
        audit::Change {
            tenant_id,
            actor_user_id: user_id,
            device_id: None,
            entity_type: "approval_policy",
            entity_id: policy.id,
            action: "approval_policy.delete",
            before: serde_json::to_value(&policy).ok(),
            after: None,
        },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Approval policy removed.", policy)));
}
