use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::response::ApiEnvelope;
use crate::state::AppState;

use super::models::{InviteRequest, InviteResponseData, Role};
use super::service;
use super::service::InviteOutcome;

/**
 * `GET /api/v1/roles`.
 * System roles plus the caller's tenant's own custom roles - never another tenant's custom roles.
 */
pub async fn list_roles(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<Role>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let roles = service::list_roles(&mut tx, tenant_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Roles fetched.", roles)));
}

/**
 * `POST /api/v1/roles/invite`.
 * Requires `staff.invite` at the target scope (tenant-wide when `store_id` is omitted, that store when it's given), then `assert_grantable` - the caller can never hand out a role with permissions they don't hold themselves.
 * See `docs/atlas/auth-flow.md`'s per-role provisioning table for who this lets invite whom.
 */
pub async fn invite(
    State(state): State<AppState>,
    mut tx: TenantTx,
    Json(body): Json<InviteRequest>,
) -> Result<Json<ApiEnvelope<InviteResponseData>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;

    service::assert_permission(&mut tx, user_id, tenant_id, body.store_id, "staff.invite").await?;
    service::assert_grantable(&mut tx, user_id, tenant_id, body.store_id, body.role_id).await?;

    let outcome = service::invite(&mut tx, tenant_id, body.store_id, body.role_id, &body.email, user_id).await?;

    let data = match outcome {
        InviteOutcome::GrantedExisting => {
            state
                .email
                .send(
                    &body.email,
                    "You've been added to a new team on Atlas",
                    "Log in to your existing Atlas account to see your new access.",
                )
                .await
                .map_err(|err| ApiError::Internal(err.to_string()))?;
            InviteResponseData::GrantedExisting
        }
        InviteOutcome::CreatedInvite { token, email } => {
            state
                .email
                .send(
                    &email,
                    "You've been invited to Atlas",
                    &format!(
                        "You've been invited to join a team on Atlas. Use this token to accept: {}",
                        token
                    ),
                )
                .await
                .map_err(|err| ApiError::Internal(err.to_string()))?;
            InviteResponseData::Invited
        }
    };

    // `body.role_id` stands in for a proper role_grants/invites row id - the
    // service layer doesn't return one today. Good enough to answer "who
    // was invited to what role, by whom, when"; tighten once it does.
    audit::record(
        &mut tx,
        tenant_id,
        user_id,
        "role_grant",
        body.role_id,
        "staff.invite",
        Some(json!({ "email": body.email, "store_id": body.store_id })),
    )
    .await?;

    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Invite sent.", data)));
}
