use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit;
use crate::crypto;
use crate::error::{codes, ApiError};
use crate::extractors::AdminTx;
use crate::response::ApiEnvelope;
use crate::state::AppState;

use super::cookies::{self, client_type, ClientType};
use super::models::{
    AcceptInviteRequest, AuthenticatedUser, ChoiceErrorBody, ForgotPasswordRequest, LoginRequest, PlatformInviteRequest,
    RefreshRequest, ResetPasswordRequest, SessionData, SessionProfile, SignupRequest, SignupResponseData,
    ResendVerificationRequest, SwitchTenantRequest, TenantChoice, TenantChoiceList, VerifyEmailRequest,
};
use super::{jwt, otp, password};

/// Fixed id of the seeded `owner` system role (`atlas.roles`, migration `20260922000009_seed_system_roles.sql`) - signup always grants this one.
fn owner_role_id() -> Uuid {
    return Uuid::parse_str("00000000-0000-7000-8000-000000000001").expect("valid constant uuid");
}

fn platform_admin_role_id() -> Uuid {
    return Uuid::parse_str("00000000-0000-7000-8000-000000000007").expect("valid constant uuid");
}

// === Signup

/**
 * `POST /api/v1/auth/signup`.
 * Creates the user and a brand-new tenant in one transaction, grants the caller `owner` on it (pending until email verification), and emails a 6-digit OTP.
 * Runs entirely inside an `AdminTx`: no tenant exists yet for a `TenantTx` to scope to.
 */
pub async fn signup(
    State(state): State<AppState>,
    mut tx: AdminTx,
    Json(payload): Json<SignupRequest>,
) -> Result<(StatusCode, Json<ApiEnvelope<SignupResponseData>>), ApiError> {
    validate_signup(&payload)?;

    let existing_verified: Option<bool> =
        sqlx::query_scalar("SELECT is_email_verified FROM atlas.users WHERE email = $1")
            .bind(&payload.email)
            .fetch_optional(&mut *tx)
            .await?;

    if let Some(is_verified) = existing_verified {
        return if is_verified {
            Err(ApiError::bad_request_with_code(
                "This email is already registered. Please log in.",
                "EMAIL_ALREADY_REGISTERED",
            ))
        } else {
            Err(ApiError::bad_request_with_code(
                "This email is registered but not yet verified. Please verify your email.",
                codes::EMAIL_NOT_VERIFIED,
            ))
        };
    }

    let password_hash = password::hash_password(&payload.password)?;
    let user_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO atlas.users (id, full_name, email, phone, password_hash, is_email_verified)
         VALUES ($1, $2, $3, $4, $5, false)",
    )
    .bind(user_id)
    .bind(payload.full_name.trim())
    .bind(&payload.email)
    .bind(&payload.phone)
    .bind(&password_hash)
    .execute(&mut *tx)
    .await?;

    let tenant_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO atlas.tenants (id, name, status, created_by_user_id)
         VALUES ($1, $2, 'pending', $3)",
    )
    .bind(tenant_id)
    .bind(payload.tenant_name.trim())
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    crate::payments::service::seed_default_methods(&mut tx, tenant_id).await?;

    sqlx::query(
        "INSERT INTO atlas.role_grants (id, user_id, role_id, scope, tenant_id, status, granted_by_user_id)
         VALUES ($1, $2, $3, 'tenant', $4, 'pending', $2)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(owner_role_id())
    .bind(tenant_id)
    .execute(&mut *tx)
    .await?;

    send_otp(&mut tx, &state, user_id, &payload.email, "signup_verification", "Verify your Atlas account").await?;

    tx.commit().await?;

    let data = SignupResponseData { email: payload.email };

    return Ok((
        StatusCode::CREATED,
        Json(ApiEnvelope::ok(
            "Registration successful! Please check your email for the verification code.",
            data,
        )),
    ));
}

fn validate_signup(payload: &SignupRequest) -> Result<(), ApiError> {
    if payload.full_name.trim().is_empty() {
        return Err(ApiError::bad_request("full_name is required"));
    }
    if !payload.email.contains('@') {
        return Err(ApiError::bad_request("a valid email is required"));
    }
    if payload.password.len() < 8 {
        return Err(ApiError::bad_request("password must be at least 8 characters"));
    }
    if payload.phone.trim().is_empty() {
        return Err(ApiError::bad_request("phone is required"));
    }
    if payload.tenant_name.trim().is_empty() {
        return Err(ApiError::bad_request("tenant_name is required"));
    }
    return Ok(());
}

/// Shared by signup and forgot-password - generates, hashes, stores and emails a 6-digit OTP under the given `purpose`.
async fn send_otp(
    tx: &mut AdminTx,
    state: &AppState,
    user_id: Uuid,
    email: &str,
    purpose: &str,
    subject: &str,
) -> Result<(), ApiError> {
    let code = otp::generate_code();
    let code_hash = otp::hash_code(&code)?;
    let expires_at = Utc::now() + Duration::minutes(10);

    sqlx::query(
        "INSERT INTO atlas.email_otps (id, user_id, code_hash, purpose, expires_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(&code_hash)
    .bind(purpose)
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;

    return state
        .email
        .send(email, subject, &format!("Your verification code is {}. It expires in 10 minutes.", code))
        .await
        .map_err(|err| ApiError::Internal(err.to_string()));
}

// === Verify email

const RESEND_COOLDOWN_SECONDS: i64 = 60;

/**
 * `POST /api/v1/auth/resend-verification`.
 * Same answer whether or not the email has an unverified account, so it can't be used to probe for accounts. One code per minute per user.
 */
pub async fn resend_verification(
    State(state): State<AppState>,
    mut tx: AdminTx,
    Json(payload): Json<ResendVerificationRequest>,
) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    let user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM atlas.users WHERE email = $1 AND NOT is_email_verified AND deleted_at IS NULL")
            .bind(&payload.email)
            .fetch_optional(&mut *tx)
            .await?;

    if let Some(user_id) = user_id {
        let recent: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM atlas.email_otps
             WHERE user_id = $1 AND purpose = 'signup_verification' AND created_at > now() - make_interval(secs => $2)
             LIMIT 1",
        )
        .bind(user_id)
        .bind(RESEND_COOLDOWN_SECONDS as f64)
        .fetch_optional(&mut *tx)
        .await?;
        if recent.is_some() {
            return Err(ApiError::TooManyRequests("A code was just sent. Wait a minute before asking for another.".to_string()));
        }
        send_otp(&mut tx, &state, user_id, &payload.email, "signup_verification", "Verify your Atlas account").await?;
    }

    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("If that account still needs verifying, a new code is on its way.", ())));
}

/**
 * `POST /api/v1/auth/verify-email`.
 * Consumes the OTP, marks the user verified, and activates every `pending` role grant they hold (in practice, exactly the one signup created).
 */
pub async fn verify_email(
    State(state): State<AppState>,
    mut tx: AdminTx,
    Json(payload): Json<VerifyEmailRequest>,
) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM atlas.users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::bad_request("Invalid email or code."))?;

    consume_otp(&mut tx, &state.pool, user_id, "signup_verification", &payload.code).await?;

    sqlx::query("UPDATE atlas.users SET is_email_verified = true, updated_at = now() WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "UPDATE atlas.role_grants SET status = 'active', updated_at = now()
         WHERE user_id = $1 AND status = 'pending'",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    return Ok(Json(ApiEnvelope::ok("Email verified successfully.", ())));
}

/// A code is dead after this many wrong guesses, independent of IP-based rate limiting - a 6-digit code is only ~1,000,000 possibilities, small enough that an attacker who rotates IPs would otherwise get unlimited tries against a fixed target email.
const MAX_OTP_ATTEMPTS: i32 = 5;

/**
 * Shared by verify-email and reset-password: looks up the most recent unconsumed, unexpired OTP for `purpose`, checks it against `code`, and marks it consumed.
 * Returns the OTP's own id (unused by most callers, but kept so a future audit trail can reference exactly which code was spent).
 */
async fn consume_otp(
    tx: &mut AdminTx,
    pool: &PgPool,
    user_id: Uuid,
    purpose: &str,
    code: &str,
) -> Result<Uuid, ApiError> {
    let otp_row: Option<(Uuid, String, i32)> = sqlx::query_as(
        "SELECT id, code_hash, attempts FROM atlas.email_otps
         WHERE user_id = $1 AND purpose = $2
           AND consumed_at IS NULL AND expires_at > now()
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(user_id)
    .bind(purpose)
    .fetch_optional(&mut **tx)
    .await?;

    let (otp_id, code_hash, attempts) =
        otp_row.ok_or_else(|| ApiError::bad_request("Invalid or expired code."))?;

    if attempts >= MAX_OTP_ATTEMPTS {
        return Err(ApiError::bad_request("Invalid or expired code."));
    }

    if !otp::verify_code(code, &code_hash)? {
        // Committed against `pool` directly, NOT `tx`: the caller's own
        // transaction rolls back on this error (nothing else it did should
        // persist), but a wrong guess must still count against the attempt
        // budget even though the request as a whole fails - running this
        // on `tx` would have made the increment disappear on every single
        // wrong guess, defeating the lockout entirely.
        sqlx::query("UPDATE atlas.email_otps SET attempts = attempts + 1 WHERE id = $1")
            .bind(otp_id)
            .execute(pool)
            .await?;
        return Err(ApiError::bad_request("Invalid or expired code."));
    }

    sqlx::query("UPDATE atlas.email_otps SET consumed_at = now() WHERE id = $1")
        .bind(otp_id)
        .execute(&mut **tx)
        .await?;

    return Ok(otp_id);
}

// === Login

/**
 * `POST /api/v1/auth/login`.
 * Verifies the password, resolves which tenant to authenticate into (the sole one if the user has only one active grant, otherwise the caller must name `tenant_id`), and issues an access JWT plus an opaque refresh token - delivered per `X-Client-Type`, see `cookies::client_type`'s doc comment.
 */
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut tx: AdminTx,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let user: Option<(Uuid, String, String, String, bool)> = sqlx::query_as(
        "SELECT id, full_name, email, password_hash, is_email_verified
         FROM atlas.users WHERE email = $1 AND deleted_at IS NULL",
    )
    .bind(&payload.email)
    .fetch_optional(&mut *tx)
    .await?;

    let (user_id, full_name, email, password_hash, is_email_verified) =
        user.ok_or_else(|| ApiError::bad_request("Wrong email or password."))?;

    if !password::verify_password(&payload.password, &password_hash)? {
        return Err(ApiError::bad_request("Wrong email or password."));
    }

    // Store-scoped staff (cashier, store manager) hold a tenant_id with scope
    // 'store'. Counting only scope = 'tenant' locked them out of login.
    let choices = list_grant_tenants(&mut tx, user_id).await?;
    let platform_admin = is_platform_admin(&mut tx, user_id).await?;

    let tenant_id = match (payload.tenant_id, choices.as_slice(), platform_admin) {
        (Some(requested), choices, _) if choices.iter().any(|choice| choice.id == requested) => requested,
        (Some(requested), _, true) => {
            let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM atlas.tenants WHERE id = $1")
                .bind(requested)
                .fetch_optional(&mut *tx)
                .await?;
            exists.ok_or_else(|| ApiError::bad_request("Unknown tenant."))?
        }
        (Some(_), _, false) => return Err(ApiError::forbidden("You have no active role in that tenant.")),
        (None, [only], _) => only.id,
        (None, [], true) => {
            let tenants = list_all_tenants(&mut tx).await?;
            return Ok(tenant_choice_response(
                "PLATFORM_TENANT_REQUIRED",
                "Choose a tenant to enter.",
                tenants,
            ));
        }
        (None, [], false) => {
            return Err(ApiError::conflict_with_code(
                "This account has no active role in any tenant.",
                "ROLE_NOT_FOUND",
            ))
        }
        (None, _, _) => {
            return Ok(tenant_choice_response(
                "TENANT_REQUIRED",
                "This account belongs to more than one tenant. Specify tenant_id.",
                choices,
            ));
        }
    };

    let (access_token, refresh_token) = issue_session(&mut tx, user_id, tenant_id, &state).await?;
    audit::record(&mut tx, tenant_id, user_id, "user", user_id, "auth.login", None).await?;
    tx.commit().await?;

    let user = AuthenticatedUser { id: user_id, full_name, email, email_verified: is_email_verified };
    return Ok(session_response(client_type(&headers), "Logged in.", access_token, refresh_token, tenant_id, user));
}

/// Issues a fresh access JWT and a fresh (stored, hashed) opaque refresh token - the one write path both `login` and `refresh` share.
async fn issue_session(
    tx: &mut AdminTx,
    user_id: Uuid,
    tenant_id: Uuid,
    state: &AppState,
) -> Result<(String, String), ApiError> {
    let access_token = jwt::issue_access_token(user_id, tenant_id, &state.jwt_secret)?;
    let refresh_token = crypto::generate_opaque_token();
    let refresh_token_hash = crypto::hash_lookup_token(&refresh_token);
    let expires_at = Utc::now() + Duration::days(7);

    sqlx::query(
        "INSERT INTO atlas.refresh_tokens (id, user_id, tenant_id, token_hash, expires_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(tenant_id)
    .bind(&refresh_token_hash)
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;

    return Ok((access_token, refresh_token));
}

/**
 * Builds the login/refresh response per `X-Client-Type`: `Web` gets `Set-Cookie` headers and no tokens in the JSON body, `App` gets the tokens in the body and no cookies.
 * See `SessionData`'s doc comment for why this distinction is load-bearing, not cosmetic.
 */
fn session_response(
    client: ClientType,
    message: &str,
    access_token: String,
    refresh_token: String,
    tenant_id: Uuid,
    user: AuthenticatedUser,
) -> Response {
    let data = match client {
        ClientType::Web => SessionData { access_token: None, refresh_token: None, tenant_id, user },
        ClientType::App => {
            SessionData { access_token: Some(access_token.clone()), refresh_token: Some(refresh_token.clone()), tenant_id, user }
        }
    };

    let mut response = Json(ApiEnvelope::ok(message, data)).into_response();

    if client == ClientType::Web {
        let access_cookie = cookies::build_cookie(cookies::ACCESS_TOKEN_COOKIE, &access_token, 15 * 60);
        let refresh_cookie = cookies::build_cookie(cookies::REFRESH_TOKEN_COOKIE, &refresh_token, 7 * 24 * 60 * 60);
        let response_headers = response.headers_mut();
        response_headers.append(
            header::SET_COOKIE,
            HeaderValue::from_str(&access_cookie).expect("cookie value is always valid ascii"),
        );
        response_headers.append(
            header::SET_COOKIE,
            HeaderValue::from_str(&refresh_cookie).expect("cookie value is always valid ascii"),
        );
    }

    return response;
}

// === Refresh

/**
 * `POST /api/v1/auth/refresh`.
 * Looks the refresh token up by its SHA-256 lookup hash (see `crypto::hash_lookup_token`), issues a fresh access token for the same tenant, and rotates the refresh token itself (revokes the old row, inserts a new one) so a leaked refresh token used once by an attacker and once by the real client is detectable later (both would present the same now-revoked token).
 *
 * For `Web`, the incoming refresh token is read from the cookie the proxy forwards, not the body - `RefreshRequest.refresh_token` is `App`'s path.
 */
pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut tx: AdminTx,
    Json(payload): Json<RefreshRequest>,
) -> Result<Response, ApiError> {
    let client = client_type(&headers);

    let presented_token = match client {
        ClientType::App => payload.refresh_token,
        ClientType::Web => cookie_value(&headers, cookies::REFRESH_TOKEN_COOKIE)
            .ok_or_else(|| ApiError::Unauthorized("Missing refresh token cookie.".to_string()))?,
    };

    let lookup_hash = crypto::hash_lookup_token(&presented_token);

    let row: Option<(Uuid, Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, user_id, tenant_id FROM atlas.refresh_tokens
         WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > now()",
    )
    .bind(&lookup_hash)
    .fetch_optional(&mut *tx)
    .await?;

    let (old_token_id, user_id, tenant_id) =
        row.ok_or_else(|| ApiError::Unauthorized("Invalid or expired refresh token.".to_string()))?;

    let user: (String, String, bool) = sqlx::query_as("SELECT full_name, email, is_email_verified FROM atlas.users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

    sqlx::query("UPDATE atlas.refresh_tokens SET revoked_at = now() WHERE id = $1")
        .bind(old_token_id)
        .execute(&mut *tx)
        .await?;

    let (access_token, refresh_token) = issue_session(&mut tx, user_id, tenant_id, &state).await?;
    tx.commit().await?;

    let authenticated_user = AuthenticatedUser { id: user_id, full_name: user.0, email: user.1, email_verified: user.2 };
    return Ok(session_response(client, "Token refreshed.", access_token, refresh_token, tenant_id, authenticated_user));
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    return cookie_header.split(';').map(str::trim).find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| value.to_string())
    });
}

// === Accept invite

/**
 * `POST /api/v1/auth/invites/:token/accept`.
 * Looks the invite up by its lookup hash, creates the user if none exists yet for its email (pre-verified - receiving the link already proved control of the email, no separate OTP step), grants the invited role, and marks the invite accepted.
 * Routes to login next, same as signup. Never auto-logs-in: the user's first real action should be proving they can log in with what they just set.
 */
pub async fn accept_invite(
    Path(token): Path<String>,
    mut tx: AdminTx,
    Json(payload): Json<AcceptInviteRequest>,
) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    let token_hash = crypto::hash_lookup_token(&token);

    let invite: Option<(Uuid, Option<Uuid>, Option<Uuid>, Uuid, String)> = sqlx::query_as(
        "SELECT id, tenant_id, store_id, role_id, email FROM atlas.invites
         WHERE token_hash = $1 AND accepted_at IS NULL AND expires_at > now()",
    )
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await?;

    let (invite_id, tenant_id, store_id, role_id, email) =
        invite.ok_or_else(|| ApiError::bad_request("Invalid or expired invite."))?;

    let existing_user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM atlas.users WHERE email = $1 AND deleted_at IS NULL")
            .bind(&email)
            .fetch_optional(&mut *tx)
            .await?;

    let user_id = match existing_user_id {
        Some(id) => id,
        None => {
            if payload.password.len() < 8 {
                return Err(ApiError::bad_request("password must be at least 8 characters"));
            }
            let user_id = Uuid::now_v7();
            let password_hash = password::hash_password(&payload.password)?;

            sqlx::query(
                "INSERT INTO atlas.users (id, full_name, email, phone, password_hash, is_email_verified)
                 VALUES ($1, $2, $3, $4, $5, true)",
            )
            .bind(user_id)
            .bind(payload.full_name.trim())
            .bind(&email)
            .bind(payload.phone.trim())
            .bind(&password_hash)
            .execute(&mut *tx)
            .await?;

            user_id
        }
    };

    let scope = if tenant_id.is_none() {
        "platform"
    } else if store_id.is_some() {
        "store"
    } else {
        "tenant"
    };

    sqlx::query(
        "INSERT INTO atlas.role_grants (id, user_id, role_id, scope, tenant_id, store_id, status, granted_by_user_id)
         VALUES ($1, $2, $3, $4::atlas.role_scope, $5, $6, 'active', $2)
         ON CONFLICT (user_id, role_id, tenant_id, store_id) DO NOTHING",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(role_id)
    .bind(scope)
    .bind(tenant_id)
    .bind(store_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE atlas.invites SET accepted_at = now() WHERE id = $1")
        .bind(invite_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    return Ok(Json(ApiEnvelope::ok("Invite accepted. Please log in.", ())));
}

// === Forgot / reset password

/**
 * `POST /api/v1/auth/forgot-password`.
 * Always the same response whether or not the email exists - an existence-dependent response is an account enumeration oracle.
 */
pub async fn forgot_password(
    State(state): State<AppState>,
    mut tx: AdminTx,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    let user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM atlas.users WHERE email = $1 AND deleted_at IS NULL")
            .bind(&payload.email)
            .fetch_optional(&mut *tx)
            .await?;

    if let Some(user_id) = user_id {
        send_otp(&mut tx, &state, user_id, &payload.email, "password_reset", "Reset your Atlas password").await?;
    }

    tx.commit().await?;

    return Ok(Json(ApiEnvelope::ok(
        "If that email has an account, a reset code has been sent.",
        (),
    )));
}

/**
 * `POST /api/v1/auth/reset-password`.
 * Consumes the OTP, updates the password.
 * Does not log the user in - same "prove you can use it" reasoning as signup and accept-invite.
 */
pub async fn reset_password(
    State(state): State<AppState>,
    mut tx: AdminTx,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM atlas.users WHERE email = $1 AND deleted_at IS NULL")
        .bind(&payload.email)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::bad_request("Invalid email or code."))?;

    consume_otp(&mut tx, &state.pool, user_id, "password_reset", &payload.code).await?;

    if payload.new_password.len() < 8 {
        return Err(ApiError::bad_request("password must be at least 8 characters"));
    }

    let password_hash = password::hash_password(&payload.new_password)?;

    sqlx::query("UPDATE atlas.users SET password_hash = $1, updated_at = now() WHERE id = $2")
        .bind(&password_hash)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // Revoking every outstanding refresh token is the point of a password
    // reset: a session started with the old, possibly-compromised password
    // should not survive it.
    sqlx::query("UPDATE atlas.refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    return Ok(Json(ApiEnvelope::ok("Password reset. Please log in.", ())));
}

// === Logout

/**
 * `POST /api/v1/auth/logout`.
 * Revokes just the presented refresh token, not every session the caller holds - logging out of one device/tenant shouldn't silently sign them out elsewhere.
 * Idempotent: an unknown or already-revoked token still returns success, both to avoid an enumeration oracle and because "log out" should feel like it worked even on a double-click or an already-expired session.
 */
pub async fn logout(
    headers: HeaderMap,
    mut tx: AdminTx,
    Json(payload): Json<RefreshRequest>,
) -> Result<Response, ApiError> {
    let client = client_type(&headers);

    let presented_token = match client {
        ClientType::App => Some(payload.refresh_token),
        ClientType::Web => cookie_value(&headers, cookies::REFRESH_TOKEN_COOKIE),
    };

    if let Some(token) = presented_token {
        let lookup_hash = crypto::hash_lookup_token(&token);
        sqlx::query(
            "UPDATE atlas.refresh_tokens SET revoked_at = now()
             WHERE token_hash = $1 AND revoked_at IS NULL",
        )
        .bind(&lookup_hash)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let mut response = Json(ApiEnvelope::ok("Logged out.", ())).into_response();

    if client == ClientType::Web {
        let response_headers = response.headers_mut();
        response_headers.append(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookies::clear_cookie(cookies::ACCESS_TOKEN_COOKIE))
                .expect("cookie value is always valid ascii"),
        );
        response_headers.append(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookies::clear_cookie(cookies::REFRESH_TOKEN_COOKIE))
                .expect("cookie value is always valid ascii"),
        );
    }

    return Ok(response);
}

// === Switch tenant

/**
 * `POST /api/v1/auth/switch-tenant`.
 * For a user holding active grants in more than one tenant: re-issues a session for a different one of their tenants without a full re-login.
 * Proves identity via the current access token rather than a fresh password, then checks the target tenant is one the caller actually holds an active grant in - the same containment posture as `roles::service`, just checked here directly since there's no tenant-scoped `TenantTx` yet at this point (that's the reason this endpoint exists at all).
 *
 * Deliberately does not revoke the previous tenant's refresh token - nothing about switching implies logging out of the tenant switched away from; a user can hold sessions in two tenants at once.
 */
pub async fn switch_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut tx: AdminTx,
    Json(payload): Json<SwitchTenantRequest>,
) -> Result<Response, ApiError> {
    let claims = jwt::decode_access_token(&bearer_token(&headers)?, &state.jwt_secret)?;

    let has_grant: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM atlas.role_grants
         WHERE user_id = $1 AND tenant_id = $2 AND status = 'active'
         LIMIT 1",
    )
    .bind(claims.sub)
    .bind(payload.tenant_id)
    .fetch_optional(&mut *tx)
    .await?;

    if has_grant.is_none() {
        let platform_admin = is_platform_admin(&mut tx, claims.sub).await?;
        if !platform_admin {
            return Err(ApiError::forbidden("You have no active role in that tenant."));
        }
        let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM atlas.tenants WHERE id = $1")
            .bind(payload.tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
        if exists.is_none() {
            return Err(ApiError::bad_request("Unknown tenant."));
        }
    }

    let user: (String, String, bool) =
        sqlx::query_as("SELECT full_name, email, is_email_verified FROM atlas.users WHERE id = $1")
            .bind(claims.sub)
            .fetch_one(&mut *tx)
            .await?;

    let (access_token, refresh_token) = issue_session(&mut tx, claims.sub, payload.tenant_id, &state).await?;
    audit::record(&mut tx, payload.tenant_id, claims.sub, "user", claims.sub, "auth.switch_tenant", None).await?;
    tx.commit().await?;

    let authenticated_user = AuthenticatedUser { id: claims.sub, full_name: user.0, email: user.1, email_verified: user.2 };
    return Ok(session_response(
        client_type(&headers),
        "Switched tenant.",
        access_token,
        refresh_token,
        payload.tenant_id,
        authenticated_user,
    ));
}

// === Session profile

/**
 * `GET /api/v1/auth/me`.
 * The marketing header and the POS home screen both need the signed-in user, the tenants they can switch to, and whether they are platform staff.
 */
pub async fn get_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut tx: AdminTx,
) -> Result<Json<ApiEnvelope<SessionProfile>>, ApiError> {
    let claims = jwt::decode_access_token(&bearer_token(&headers)?, &state.jwt_secret)?;
    let user: (String, String, bool) = sqlx::query_as("SELECT full_name, email, is_email_verified FROM atlas.users WHERE id = $1 AND deleted_at IS NULL")
        .bind(claims.sub)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Unknown user.".to_string()))?;

    let tenants = list_grant_tenants(&mut tx, claims.sub).await?;
    let platform_admin = is_platform_admin(&mut tx, claims.sub).await?;
    tx.commit().await?;

    return Ok(Json(ApiEnvelope::ok(
        "Session fetched.",
        SessionProfile {
            user: AuthenticatedUser { id: claims.sub, full_name: user.0, email: user.1, email_verified: user.2 },
            tenant_id: claims.tenant_id,
            is_platform_admin: platform_admin,
            tenants,
        },
    )));
}

/**
 * `GET /api/v1/auth/platform/tenants`.
 * Every tenant, for a platform admin choosing which business to enter.
 * A tenant owner does not get this list.
 */
pub async fn list_platform_tenants(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut tx: AdminTx,
) -> Result<Json<ApiEnvelope<Vec<TenantChoice>>>, ApiError> {
    let claims = jwt::decode_access_token(&bearer_token(&headers)?, &state.jwt_secret)?;
    let platform_admin = is_platform_admin(&mut tx, claims.sub).await?;
    if !platform_admin {
        return Err(ApiError::forbidden("Platform staff only."));
    }
    let tenants = list_all_tenants(&mut tx).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Tenants fetched.", tenants)));
}

/**
 * `POST /api/v1/auth/platform/invite`.
 * Only an existing platform admin can add another. An email that already has an account gets the grant immediately. A new email gets an invite link.
 */
pub async fn invite_platform_staff(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut tx: AdminTx,
    Json(payload): Json<PlatformInviteRequest>,
) -> Result<Json<ApiEnvelope<()>>, ApiError> {
    let claims = jwt::decode_access_token(&bearer_token(&headers)?, &state.jwt_secret)?;
    let platform_admin = is_platform_admin(&mut tx, claims.sub).await?;
    if !platform_admin {
        return Err(ApiError::forbidden("Platform staff only."));
    }

    let email = payload.email.trim().to_lowercase();
    if !email.contains('@') {
        return Err(ApiError::bad_request("Invalid email address."));
    }

    let role_id = platform_admin_role_id();
    let existing_user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM atlas.users WHERE email = $1 AND deleted_at IS NULL")
            .bind(&email)
            .fetch_optional(&mut *tx)
            .await?;

    if let Some(user_id) = existing_user_id {
        sqlx::query(
            "INSERT INTO atlas.role_grants (id, user_id, role_id, scope, tenant_id, store_id, status, granted_by_user_id)
             SELECT $1, $2, $3, 'platform', NULL, NULL, 'active', $4
             WHERE NOT EXISTS (
                 SELECT 1 FROM atlas.role_grants
                 WHERE user_id = $2 AND role_id = $3 AND scope = 'platform' AND deleted_at IS NULL
             )",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(role_id)
        .bind(claims.sub)
        .execute(&mut *tx)
        .await?;

        state
            .email
            .send(
                &email,
                "You've been added as Atlas platform staff",
                "Log in to your existing Atlas account to use platform access.",
            )
            .await
            .map_err(|err| ApiError::Internal(err.to_string()))?;
    } else {
        let token = crypto::generate_opaque_token();
        let token_hash = crypto::hash_lookup_token(&token);
        let expires_at = Utc::now() + Duration::days(7);

        sqlx::query(
            "INSERT INTO atlas.invites (id, tenant_id, store_id, role_id, email, token_hash, invited_by_user_id, expires_at)
             VALUES ($1, NULL, NULL, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::now_v7())
        .bind(role_id)
        .bind(&email)
        .bind(&token_hash)
        .bind(claims.sub)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;

        state
            .email
            .send(
                &email,
                "You've been invited to Atlas",
                &format!("You've been invited to join Atlas as platform staff. Use this token to accept: {}", token),
            )
            .await
            .map_err(|err| ApiError::Internal(err.to_string()))?;
    }

    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Invitation sent.", ())));
}

async fn list_grant_tenants(tx: &mut AdminTx, user_id: Uuid) -> Result<Vec<TenantChoice>, ApiError> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT DISTINCT t.id, t.name
         FROM atlas.role_grants rg
         JOIN atlas.tenants t ON t.id = rg.tenant_id
         WHERE rg.user_id = $1 AND rg.deleted_at IS NULL AND rg.tenant_id IS NOT NULL
           AND (rg.status = 'active' OR (rg.status = 'pending' AND rg.granted_by_user_id = rg.user_id))
         ORDER BY t.name",
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await?;

    return Ok(rows.into_iter().map(|(id, name)| TenantChoice { id, name }).collect());
}

async fn list_all_tenants(tx: &mut AdminTx) -> Result<Vec<TenantChoice>, ApiError> {
    let rows: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, name FROM atlas.tenants ORDER BY name")
            .fetch_all(&mut **tx)
            .await?;
    return Ok(rows.into_iter().map(|(id, name)| TenantChoice { id, name }).collect());
}

async fn is_platform_admin(tx: &mut AdminTx, user_id: Uuid) -> Result<bool, ApiError> {
    let found: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM atlas.role_grants
         WHERE user_id = $1 AND scope = 'platform' AND status = 'active' AND deleted_at IS NULL
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await?;
    return Ok(found.is_some());
}

fn tenant_choice_response(code: &'static str, message: &str, tenants: Vec<TenantChoice>) -> Response {
    let body = ChoiceErrorBody {
        message: message.to_string(),
        success: false,
        code,
        data: TenantChoiceList { tenants },
    };
    return (StatusCode::CONFLICT, Json(body)).into_response();
}

fn bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let header = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| ApiError::Unauthorized("missing Authorization header".to_string()))?
        .to_str()
        .map_err(|_| ApiError::Unauthorized("invalid Authorization header".to_string()))?;

    return header
        .strip_prefix("Bearer ")
        .map(str::to_string)
        .ok_or_else(|| ApiError::Unauthorized("expected a Bearer token".to_string()));
}
