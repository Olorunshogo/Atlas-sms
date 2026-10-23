mod cookies;
mod handlers;
mod jwt;
mod models;
mod otp;
mod password;

pub use jwt::{decode_access_token, Claims};

use axum::routing::{get, post};
use axum::Router;

use crate::rate_limit;
use crate::state::AppState;

/**
 * Every route here is public or session-bootstrapping (no `TenantTx` / permission check has run yet), so the whole router - not a curated subset - sits behind the per-IP rate limiter.
 * `verify-email` and `reset-password` additionally have their own attempt-counter on the OTP itself (`handlers::consume_otp`), since IP limiting alone doesn't stop an attacker who rotates IPs against one fixed target email.
 */
pub fn router(state: AppState) -> Router<AppState> {
    return Router::new()
        .route("/signup", post(handlers::signup))
        .route("/verify-email", post(handlers::verify_email))
        .route("/login", post(handlers::login))
        .route("/me", get(handlers::get_session))
        .route("/refresh", post(handlers::refresh))
        .route("/logout", post(handlers::logout))
        .route("/switch-tenant", post(handlers::switch_tenant))
        .route("/platform/tenants", get(handlers::list_platform_tenants))
        .route("/platform/invite", post(handlers::invite_platform_staff))
        .route("/invites/{token}/accept", post(handlers::accept_invite))
        .route("/forgot-password", post(handlers::forgot_password))
        .route("/reset-password", post(handlers::reset_password))
        .route_layer(axum::middleware::from_fn_with_state(state, rate_limit::rate_limit));
}
