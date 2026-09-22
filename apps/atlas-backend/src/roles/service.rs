use chrono::{Duration, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::crypto;
use crate::error::ApiError;

use super::models::Role;

pub enum InviteOutcome {
    GrantedExisting,
    /// Carries the raw (unhashed) token - only ever handed to the caller once, to put in an email link.
    /// Never stored or logged as-is.
    CreatedInvite { token: String, email: String },
}

pub async fn list_roles(conn: &mut PgConnection, tenant_id: Uuid) -> Result<Vec<Role>, ApiError> {
    return sqlx::query_as::<_, Role>(
        "SELECT id, tenant_id, name, permissions::text[] AS permissions, is_system
         FROM atlas.roles
         WHERE tenant_id IS NULL OR tenant_id = $1
         ORDER BY is_system DESC, name",
    )
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

/**
 * The one generic permission check every handler that gates on a capability calls, rather than each action writing its own bespoke role assertion.
 * `store_id` is `None` for a tenant-wide action (e.g. opening a new store); a caller's grant satisfies the check when it is a platform grant, a tenant-wide grant matching `tenant_id`, a store grant matching `store_id`, or a store-group grant covering `store_id` (the V2 regional-manager case, already join-able even before the UI to manage it exists).
 */
pub async fn assert_permission(
    conn: &mut PgConnection,
    user_id: Uuid,
    tenant_id: Uuid,
    store_id: Option<Uuid>,
    permission: &str,
) -> Result<(), ApiError> {
    let granted: Option<i32> = sqlx::query_scalar(
        r#"
        SELECT 1
        FROM atlas.role_grants rg
        JOIN atlas.roles r ON r.id = rg.role_id
        WHERE rg.user_id = $1
          AND rg.status = 'active'
          AND $4::atlas.permission = ANY(r.permissions)
          AND (
                rg.scope = 'platform'
             OR (rg.scope = 'tenant' AND rg.tenant_id = $2)
             OR (rg.scope = 'store' AND rg.store_id = $3)
             OR EXISTS (
                  SELECT 1 FROM atlas.store_group_grants sgg
                  WHERE sgg.role_grant_id = rg.id AND sgg.store_id = $3
                )
          )
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(store_id)
    .bind(permission)
    .fetch_optional(conn)
    .await?;

    return match granted {
        Some(_) => Ok(()),
        None => Err(ApiError::forbidden(format!(
            "Missing permission: {}",
            permission
        ))),
    };
}

/**
 * The union of every permission a caller effectively holds at this scope, across however many role grants they have that reach it.
 * Backs `assert_grantable`: a granter's power to hand out a role is the permissions they hold, not any single grant's permissions in isolation (an admin who is also a store's stock_clerk still has the admin-level union when inviting at that store).
 */
async fn effective_permissions(
    conn: &mut PgConnection,
    user_id: Uuid,
    tenant_id: Uuid,
    store_id: Option<Uuid>,
) -> Result<Vec<String>, ApiError> {
    return sqlx::query_scalar(
        r#"
        SELECT DISTINCT unnest(r.permissions)::text
        FROM atlas.role_grants rg
        JOIN atlas.roles r ON r.id = rg.role_id
        WHERE rg.user_id = $1
          AND rg.status = 'active'
          AND (
                rg.scope = 'platform'
             OR (rg.scope = 'tenant' AND rg.tenant_id = $2)
             OR (rg.scope = 'store' AND rg.store_id = $3)
             OR EXISTS (
                  SELECT 1 FROM atlas.store_group_grants sgg
                  WHERE sgg.role_grant_id = rg.id AND sgg.store_id = $3
                )
          )
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(store_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

/**
 * "A granter can never hand out more than they hold" - the rule that makes the dynamic role system safe without a rigid hierarchy table.
 * Compares `target_role_id`'s full permission set against the granter's effective permissions (above) at the same scope; any permission the target role has that the granter doesn't is a 403.
 */
pub async fn assert_grantable(
    conn: &mut PgConnection,
    granter_user_id: Uuid,
    tenant_id: Uuid,
    store_id: Option<Uuid>,
    target_role_id: Uuid,
) -> Result<(), ApiError> {
    let target_permissions: Vec<String> =
        sqlx::query_scalar("SELECT permissions::text[] FROM atlas.roles WHERE id = $1")
            .bind(target_role_id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or_else(|| ApiError::bad_request("Unknown role."))?;

    let granter_permissions = effective_permissions(conn, granter_user_id, tenant_id, store_id).await?;

    let ungranted = target_permissions.iter().find(|permission| !granter_permissions.contains(permission));

    return match ungranted {
        None => Ok(()),
        Some(permission) => Err(ApiError::forbidden(format!(
            "You cannot grant a role with the \"{}\" permission, which you do not hold yourself.",
            permission
        ))),
    };
}

/**
 * The two-branch invite (docs/atlas/auth-flow.md #5).
 * An existing, verified user has nothing left to prove - they get the `role_grants` row directly, `status = 'active'`, no token needed.
 * A brand-new email goes through `atlas.invites`; accepting it (`auth::handlers::accept_invite`) is what actually creates the user and the grant, in one transaction.
 *
 * Callers must run `assert_grantable` first - this function trusts that the containment check already happened, it doesn't repeat it.
 */
pub async fn invite(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    store_id: Option<Uuid>,
    role_id: Uuid,
    email: &str,
    invited_by_user_id: Uuid,
) -> Result<InviteOutcome, ApiError> {
    let scope = if store_id.is_some() { "store" } else { "tenant" };

    let existing_user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM atlas.users WHERE email = $1 AND deleted_at IS NULL")
            .bind(email)
            .fetch_optional(&mut *conn)
            .await?;

    if let Some(user_id) = existing_user_id {
        sqlx::query(
            "INSERT INTO atlas.role_grants (id, user_id, role_id, scope, tenant_id, store_id, status, granted_by_user_id)
             VALUES ($1, $2, $3, $4::atlas.role_scope, $5, $6, 'active', $7)
             ON CONFLICT (user_id, role_id, tenant_id, store_id) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(role_id)
        .bind(scope)
        .bind(tenant_id)
        .bind(store_id)
        .bind(invited_by_user_id)
        .execute(conn)
        .await?;

        return Ok(InviteOutcome::GrantedExisting);
    }

    let token = crypto::generate_opaque_token();
    let token_hash = crypto::hash_lookup_token(&token);
    let expires_at = Utc::now() + Duration::days(7);

    sqlx::query(
        "INSERT INTO atlas.invites (id, tenant_id, store_id, role_id, email, token_hash, invited_by_user_id, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(store_id)
    .bind(role_id)
    .bind(email)
    .bind(&token_hash)
    .bind(invited_by_user_id)
    .bind(expires_at)
    .execute(conn)
    .await?;

    return Ok(InviteOutcome::CreatedInvite { token, email: email.to_string() });
}
