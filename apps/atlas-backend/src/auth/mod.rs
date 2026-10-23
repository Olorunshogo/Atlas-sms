mod cookies;
mod handlers;
mod jwt;
mod models;
mod otp;
mod password;

pub use jwt::{decode_access_token, issue_access_token, Claims};

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/signup", post(handlers::signup))
        .route("/verify-email", post(handlers::verify_email))
        .route("/login", post(handlers::login))
        .route("/refresh", post(handlers::refresh))
        .route("/invites/{token}/accept", post(handlers::accept_invite))
        .route("/forgot-password", post(handlers::forgot_password))
        .route("/reset-password", post(handlers::reset_password))
}
