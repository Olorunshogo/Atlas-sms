/*!
 * Request context for the public storefront. `ShopTx` resolves `{handle}` to a storefront (the one lookup that has to
 * cross tenants, since the tenant isn't known yet), then scopes the transaction to that tenant exactly as `TenantTx`
 * does. A shopper token is optional; it is signed with a key derived from the staff secret, so a shopper token never
 * validates as a staff token or the other way round.
 */
use std::ops::{Deref, DerefMut};

use axum::extract::{FromRef, FromRequestParts, RawPathParams};
use axum::http::request::Parts;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

pub const CART_TOKEN_HEADER: &str = "x-cart-token";
const SHOPPER_TOKEN_DAYS: i64 = 30;

pub const STOREFRONT_COLUMNS: &str = "store_id, tenant_id, handle, is_enabled, reservation_minutes, ussd_window_minutes,
     bank_transfer_window_minutes, guest_cart_days, pay_on_delivery_max_minor, pay_on_delivery_requires_account,
     loyalty_minor_per_point, loyalty_point_value_minor, loyalty_max_redeem_basis_points";

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Storefront {
    pub store_id: Uuid,
    pub tenant_id: Uuid,
    pub handle: String,
    pub is_enabled: bool,
    pub reservation_minutes: i32,
    pub ussd_window_minutes: i32,
    pub bank_transfer_window_minutes: i32,
    pub guest_cart_days: i32,
    pub pay_on_delivery_max_minor: Option<i64>,
    pub pay_on_delivery_requires_account: bool,
    pub loyalty_minor_per_point: Option<i64>,
    pub loyalty_point_value_minor: i64,
    pub loyalty_max_redeem_basis_points: i32,
}

// === Shopper tokens

#[derive(Debug, Serialize, Deserialize)]
struct ShopperClaims {
    sub: Uuid,
    tenant_id: Uuid,
    aud: String,
    exp: usize,
}

const SHOPPER_AUDIENCE: &str = "atlas-shopper";

fn shopper_key(secret: &[u8]) -> Vec<u8> {
    let mut key = secret.to_vec();
    key.extend_from_slice(b":shopper");
    return key;
}

pub fn issue_shopper_token(shopper_id: Uuid, tenant_id: Uuid, secret: &[u8]) -> Result<String, ApiError> {
    let claims = ShopperClaims {
        sub: shopper_id,
        tenant_id,
        aud: SHOPPER_AUDIENCE.to_string(),
        exp: (Utc::now() + Duration::days(SHOPPER_TOKEN_DAYS)).timestamp() as usize,
    };
    return encode(&Header::default(), &claims, &EncodingKey::from_secret(&shopper_key(secret)))
        .map_err(|err| ApiError::Internal(format!("failed to issue shopper token: {}", err)));
}

fn decode_shopper_token(token: &str, secret: &[u8]) -> Result<ShopperClaims, ApiError> {
    let mut validation = Validation::default();
    validation.set_audience(&[SHOPPER_AUDIENCE]);
    return decode::<ShopperClaims>(token, &DecodingKey::from_secret(&shopper_key(secret)), &validation)
        .map(|data| data.claims)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired shopper session.".to_string()));
}

// === ShopTx

pub struct ShopTx {
    tx: Transaction<'static, Postgres>,
    pub storefront: Storefront,
    pub tenant_id: Uuid,
    pub store_id: Uuid,
    pub shopper_id: Option<Uuid>,
    pub cart_token: Option<String>,
}

impl Deref for ShopTx {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        return &self.tx;
    }
}

impl DerefMut for ShopTx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return &mut self.tx;
    }
}

impl ShopTx {
    pub async fn commit(self) -> Result<(), ApiError> {
        return self.tx.commit().await.map_err(ApiError::from);
    }

    pub fn require_shopper(&self) -> Result<Uuid, ApiError> {
        return self.shopper_id.ok_or_else(|| ApiError::Unauthorized("Sign in to your account first.".to_string()));
    }
}

impl<S> FromRequestParts<S> for ShopTx
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let params = RawPathParams::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::not_found("Shop not found."))?;
        let handle = params
            .iter()
            .find(|(name, _)| *name == "handle")
            .map(|(_, value)| value.to_lowercase())
            .ok_or_else(|| ApiError::not_found("Shop not found."))?;

        let mut tx = app_state.pool.begin().await?;
        sqlx::query("SELECT set_config('app.bypass_tenant', 'on', true)").execute(&mut *tx).await?;
        let storefront = sqlx::query_as::<_, Storefront>(&format!(
            "SELECT {STOREFRONT_COLUMNS} FROM atlas.storefronts f
             WHERE lower(f.handle) = $1 AND f.is_enabled
               AND EXISTS (SELECT 1 FROM atlas.stores s WHERE s.id = f.store_id AND s.deleted_at IS NULL AND s.is_active)"
        ))
        .bind(&handle)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("Shop not found."))?;
        sqlx::query("SELECT set_config('app.bypass_tenant', 'off', true)").execute(&mut *tx).await?;
        sqlx::query("SELECT set_config('app.current_tenant_id', $1, true)")
            .bind(storefront.tenant_id.to_string())
            .execute(&mut *tx)
            .await?;

        let shopper_id = match parts.headers.get(axum::http::header::AUTHORIZATION) {
            None => None,
            Some(value) => {
                let token = value
                    .to_str()
                    .ok()
                    .and_then(|v| v.strip_prefix("Bearer "))
                    .ok_or_else(|| ApiError::Unauthorized("expected a Bearer token".to_string()))?;
                let claims = decode_shopper_token(token, &app_state.jwt_secret)?;
                if claims.tenant_id != storefront.tenant_id {
                    return Err(ApiError::Unauthorized("This session belongs to another shop.".to_string()));
                }
                Some(claims.sub)
            }
        };
        let cart_token = parts
            .headers
            .get(CART_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);

        return Ok(ShopTx {
            tx,
            tenant_id: storefront.tenant_id,
            store_id: storefront.store_id,
            storefront,
            shopper_id,
            cart_token,
        });
    }
}
