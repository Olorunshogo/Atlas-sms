/*!
 * Shopper accounts: sign-up, log-in, and social sign-in through the configured identity provider. A shopper is also
 * a customer of the business, so till and online history meet in one customer record. Any of these merges a guest
 * cart sent as `x-cart-token`.
 */
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto;
use crate::customers::service::normalize_phone;
use crate::error::ApiError;
use crate::response::ApiEnvelope;
use crate::state::AppState;
use crate::validate::required_text;

use super::cart;
use super::context::{issue_shopper_token, ShopTx};
use super::loyalty;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ShopperProfile {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub email: String,
    pub full_name: String,
    pub phone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ShopperSession {
    pub token: String,
    pub shopper: ShopperProfile,
}

#[derive(Debug, Serialize)]
pub struct ShopperAccount {
    #[serde(flatten)]
    pub shopper: ShopperProfile,
    pub loyalty_points: i64,
    pub pending_loyalty_points: i64,
}

#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub phone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct SocialRequest {
    /// `google` or `facebook`.
    pub provider: String,
    /// The provider's sign-in token, checked on the server.
    pub token: String,
}

const PROFILE_COLUMNS: &str = "id, customer_id, email, full_name, phone";

fn clean_email(email: &str) -> Result<String, ApiError> {
    let email = email.trim().to_lowercase();
    if !email.contains('@') || email.len() > 254 {
        return Err(ApiError::bad_request_with_code("a valid email is required", "VALIDATION_FAILED"));
    }
    return Ok(email);
}

async fn create_shopper(tx: &mut ShopTx, email: &str, full_name: &str, phone: Option<&str>, password_hash: Option<String>) -> Result<ShopperProfile, ApiError> {
    let tenant_id = tx.tenant_id;
    let customer_id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.customers (id, tenant_id, full_name, email, phone) VALUES ($1, $2, $3, $4, $5)")
        .bind(customer_id)
        .bind(tenant_id)
        .bind(full_name)
        .bind(email)
        .bind(phone)
        .execute(&mut **tx)
        .await?;
    return sqlx::query_as::<_, ShopperProfile>(&format!(
        "INSERT INTO atlas.shoppers (id, tenant_id, customer_id, email, full_name, phone, password_hash)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING {PROFILE_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(customer_id)
    .bind(email)
    .bind(full_name)
    .bind(phone)
    .bind(password_hash)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            ApiError::conflict_with_code("An account with that email already exists. Log in instead.", "EMAIL_ALREADY_REGISTERED")
        }
        other => ApiError::from(other),
    });
}

async fn start_session(mut tx: ShopTx, state: &AppState, shopper: ShopperProfile, message: &str) -> Result<Json<ApiEnvelope<ShopperSession>>, ApiError> {
    if let Some(token) = tx.cart_token.clone() {
        let (tenant_id, store_id) = (tx.tenant_id, tx.store_id);
        cart::merge_guest_cart(&mut tx, tenant_id, store_id, &token, shopper.id).await?;
    }
    let token = issue_shopper_token(shopper.id, tx.tenant_id, &state.jwt_secret)?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok(message, ShopperSession { token, shopper })));
}

/// `POST /api/v1/shop/:handle/auth/signup`.
pub async fn signup(State(state): State<AppState>, mut tx: ShopTx, Json(body): Json<SignupRequest>) -> Result<Json<ApiEnvelope<ShopperSession>>, ApiError> {
    let email = clean_email(&body.email)?;
    let full_name = required_text("full_name", &body.full_name, 200)?;
    if body.password.len() < 8 {
        return Err(ApiError::bad_request_with_code("password must be at least 8 characters", "VALIDATION_FAILED"));
    }
    let phone = body.phone.as_deref().and_then(normalize_phone);
    let hash = crypto::hash_secret(&body.password).map_err(|e| ApiError::Internal(e.to_string()))?;
    let shopper = create_shopper(&mut tx, &email, &full_name, phone.as_deref(), Some(hash)).await?;
    return start_session(tx, &state, shopper, "Account created.").await;
}

/// `POST /api/v1/shop/:handle/auth/login`.
pub async fn login(State(state): State<AppState>, mut tx: ShopTx, Json(body): Json<LoginRequest>) -> Result<Json<ApiEnvelope<ShopperSession>>, ApiError> {
    let email = clean_email(&body.email)?;
    let found: Option<(Uuid, Option<String>)> = sqlx::query_as("SELECT id, password_hash FROM atlas.shoppers WHERE lower(email) = $1")
        .bind(&email)
        .fetch_optional(&mut *tx)
        .await?;
    let wrong = || ApiError::bad_request("Wrong email or password.");
    let (shopper_id, hash) = found.ok_or_else(wrong)?;
    let hash = hash.ok_or_else(wrong)?;
    if !crypto::verify_secret(&body.password, &hash).map_err(|e| ApiError::Internal(e.to_string()))? {
        return Err(wrong());
    }
    let shopper = sqlx::query_as::<_, ShopperProfile>(&format!("SELECT {PROFILE_COLUMNS} FROM atlas.shoppers WHERE id = $1"))
        .bind(shopper_id)
        .fetch_one(&mut *tx)
        .await?;
    return start_session(tx, &state, shopper, "Logged in.").await;
}

/**
 * `POST /api/v1/shop/:handle/auth/social`. The provider confirms who the token belongs to; that identity signs in an
 * account already linked to it, links an account with the same email, or creates a new one.
 */
pub async fn social(State(state): State<AppState>, mut tx: ShopTx, Json(body): Json<SocialRequest>) -> Result<Json<ApiEnvelope<ShopperSession>>, ApiError> {
    let identity = state
        .integrations
        .social
        .verify(&body.provider, &body.token)
        .await
        .map_err(|err| ApiError::Unauthorized(format!("Social sign-in failed: {}", err)))?;
    let tenant_id = tx.tenant_id;
    let linked: Option<Uuid> = sqlx::query_scalar("SELECT shopper_id FROM atlas.shopper_identities WHERE tenant_id = $1 AND provider = $2 AND subject = $3")
        .bind(tenant_id)
        .bind(&body.provider)
        .bind(&identity.subject)
        .fetch_optional(&mut *tx)
        .await?;
    let shopper_id = match linked {
        Some(id) => id,
        None => {
            let email = clean_email(&identity.email)?;
            let by_email: Option<Uuid> = sqlx::query_scalar("SELECT id FROM atlas.shoppers WHERE lower(email) = $1")
                .bind(&email)
                .fetch_optional(&mut *tx)
                .await?;
            let id = match by_email {
                Some(id) => id,
                None => create_shopper(&mut tx, &email, identity.full_name.trim(), None, None).await?.id,
            };
            sqlx::query("INSERT INTO atlas.shopper_identities (tenant_id, provider, subject, shopper_id) VALUES ($1, $2, $3, $4)")
                .bind(tenant_id)
                .bind(&body.provider)
                .bind(&identity.subject)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            id
        }
    };
    let shopper = sqlx::query_as::<_, ShopperProfile>(&format!("SELECT {PROFILE_COLUMNS} FROM atlas.shoppers WHERE id = $1"))
        .bind(shopper_id)
        .fetch_one(&mut *tx)
        .await?;
    return start_session(tx, &state, shopper, "Signed in.").await;
}

/// `GET /api/v1/shop/:handle/account`.
pub async fn me(mut tx: ShopTx) -> Result<Json<ApiEnvelope<ShopperAccount>>, ApiError> {
    let shopper_id = tx.require_shopper()?;
    let shopper = sqlx::query_as::<_, ShopperProfile>(&format!("SELECT {PROFILE_COLUMNS} FROM atlas.shoppers WHERE id = $1"))
        .bind(shopper_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Unknown shopper.".to_string()))?;
    let (loyalty_points, pending_loyalty_points) = loyalty::balance(&mut tx, shopper_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Account fetched.", ShopperAccount { shopper, loyalty_points, pending_loyalty_points })));
}
