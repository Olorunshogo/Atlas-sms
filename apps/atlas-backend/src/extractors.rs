use std::ops::{Deref, DerefMut};

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::decode_access_token;
use crate::error::{codes, ApiError};
use crate::state::AppState;

// === TenantTx

/**
 * The row-level-security enforcement mechanism.
 * Every tenant-scoped handler takes a `TenantTx` instead of a raw pool connection, so it is structurally impossible to write an unscoped query - there is no other way to get a connection out of this extractor.
 *
 * `user_id` and `tenant_id` come from a verified access-token JWT (`Authorization: Bearer <token>`, decoded via `auth::decode_access_token`, see `auth::jwt::Claims`), never a client-supplied header.
 * A header is something a caller can set to any value; a signed claim isn't.
 *
 * `store_id` is the one piece that legitimately comes from a plain header (`x-store-id`, optional): it's the caller picking which store's data to view, not an identity claim.
 * A role grant scoped to a different store still gets rejected downstream, by `roles::service::assert_permission` or by RLS itself.
 */
pub struct TenantTx {
    tx: Transaction<'static, Postgres>,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    /**
     * Mutations still take their store from the body or the entity they touch.
     * Tenant-level resources with no store of their own (customers) check permissions against this, so a store-scoped grant can match.
     */
    pub store_id: Option<Uuid>,
}

impl Deref for TenantTx {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        return &self.tx;
    }
}

impl DerefMut for TenantTx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return &mut self.tx;
    }
}

impl TenantTx {
    /**
     * Commits the underlying transaction.
     * Handlers call this explicitly once their writes are done, rather than relying on drop, so a forgotten commit surfaces as "nothing persisted" in testing instead of silently succeeding.
     */
    pub async fn commit(self) -> Result<(), ApiError> {
        return self.tx.commit().await.map_err(ApiError::from);
    }
}

impl<S> FromRequestParts<S> for TenantTx
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let claims = bearer_claims(parts, &app_state)?;
        let store_id = header_uuid(parts, "x-store-id")?;

        let mut tx = app_state.pool.begin().await.map_err(ApiError::from)?;

        sqlx::query("SELECT set_config('app.current_tenant_id', $1, true)")
            .bind(claims.tenant_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(ApiError::from)?;

        if let Some(store_id) = store_id {
            sqlx::query("SELECT set_config('app.current_store_id', $1, true)")
                .bind(store_id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(ApiError::from)?;
        }

        // Read per request, not from the token, so verifying takes effect on the very next call
        let verified: Option<bool> = sqlx::query_scalar("SELECT is_email_verified FROM atlas.users WHERE id = $1 AND deleted_at IS NULL")
            .bind(claims.sub)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ApiError::from)?;
        match verified {
            Some(true) => {}
            Some(false) => {
                return Err(ApiError::forbidden_with_code("Verify your email to continue.", codes::EMAIL_NOT_VERIFIED));
            }
            None => return Err(ApiError::Unauthorized("Unknown user.".to_string())),
        }

        return Ok(TenantTx {
            tx,
            user_id: claims.sub,
            tenant_id: claims.tenant_id,
            store_id,
        });
    }
}

// === AdminTx

/**
 * Unscoped sibling of `TenantTx`, for pre-tenant / cross-tenant admin handlers - signup, login, and refresh (no session exists yet to decode a claim from), and platform-role operations that intentionally cross tenants.
 * Sets `app.bypass_tenant = 'on'` for the transaction, the only way to satisfy the `FORCE ROW LEVEL SECURITY` write check when no single tenant is in scope.
 * Deliberately only reachable by routing a handler through `AdminTx` at all - never a runtime toggle a `TenantTx` handler can flip.
 */
pub struct AdminTx {
    tx: Transaction<'static, Postgres>,
}

impl Deref for AdminTx {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        return &self.tx;
    }
}

impl DerefMut for AdminTx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return &mut self.tx;
    }
}

impl AdminTx {
    pub async fn commit(self) -> Result<(), ApiError> {
        return self.tx.commit().await.map_err(ApiError::from);
    }
}

impl<S> FromRequestParts<S> for AdminTx
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let mut tx = app_state.pool.begin().await.map_err(ApiError::from)?;

        sqlx::query("SELECT set_config('app.bypass_tenant', 'on', true)")
            .execute(&mut *tx)
            .await
            .map_err(ApiError::from)?;

        return Ok(AdminTx { tx });
    }
}

// === Helpers

fn bearer_claims(parts: &Parts, state: &AppState) -> Result<crate::auth::Claims, ApiError> {
    let header = parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| ApiError::Unauthorized("missing Authorization header".to_string()))?
        .to_str()
        .map_err(|_| ApiError::Unauthorized("invalid Authorization header".to_string()))?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::Unauthorized("expected a Bearer token".to_string()))?;

    return decode_access_token(token, &state.jwt_secret);
}

fn header_uuid(parts: &Parts, name: &str) -> Result<Option<Uuid>, ApiError> {
    return match parts.headers.get(name) {
        None => Ok(None),
        Some(value) => {
            let value = value
                .to_str()
                .map_err(|_| ApiError::bad_request(format!("invalid {}", name)))?;
            Uuid::parse_str(value)
                .map(Some)
                .map_err(|_| ApiError::bad_request(format!("invalid {}", name)))
        }
    };
}
