// Format strings use positional args, not inline captures, throughout this
// crate - see the coding-assistant skill's locked rule.
#![allow(clippy::uninlined_format_args)]

use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use sqlx::PgPool;

mod audit;
mod auth;
mod crypto;
mod customers;
mod email;
mod error;
mod extractors;
mod inventory;
mod payments;
mod products;
mod purchasing;
mod rate_limit;
mod response;
pub mod roles;
mod sales;
mod state;
mod stock;
mod stores;
mod sync;
mod tenants;

pub use email::{EmailError, EmailProvider, LogEmailProvider};
pub use rate_limit::RateLimiter;
pub use state::AppState;

/// Health check only, no state - kept exactly as it was so `tests/health_check.rs` (which calls `app()` with no arguments) keeps working unmodified.
pub fn app() -> Router {
    return Router::new().route("/health", get(health));
}

async fn health() -> &'static str {
    return "ok";
}

/**
 * Builds `AppState` from environment: `ATLAS_DATABASE_URL` (connected as `atlas_api`, never the superuser - RLS is bypassed entirely for a superuser connection, see the `enable_rls` migration) and `ATLAS_JWT_SECRET` (signs/verifies the access token issued by `auth::login`).
 *
 * Prefixed, unlike atlas-backend's plain `DATABASE_URL`/`PORT`: Nx loads the one root `.env` into every task's environment, so an unprefixed name here would collide with atlas-backend's own same-named variable the moment both are set in that one file.
 */
pub async fn build_state() -> AppState {
    let database_url = std::env::var("ATLAS_DATABASE_URL").expect("ATLAS_DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let jwt_secret = std::env::var("ATLAS_JWT_SECRET").expect("ATLAS_JWT_SECRET must be set");

    return AppState {
        pool,
        email: Arc::new(LogEmailProvider),
        jwt_secret: Arc::new(jwt_secret.into_bytes()),
        rate_limiter: rate_limit::RateLimiter::new(),
    };
}

/// Single source of truth for the versioned path segment every route below nests under - bumping to `/api/v2` is a one-line change here, not nine hand-edits that can drift out of sync with each other.
pub const API_PREFIX: &str = "/api/v1";

/**
 * The versioned API surface.
 * Kept separate from `app()` because it needs `AppState` (a database pool) and `app()`'s zero-argument shape is relied on by the health check test.
 */
pub fn api_router(state: AppState) -> Router {
    return Router::new()
        .nest(&format!("{}/auth", API_PREFIX), auth::router(state.clone()))
        .nest(&format!("{}/tenants", API_PREFIX), tenants::router())
        .nest(&format!("{}/stores", API_PREFIX), stores::router())
        .nest(&format!("{}/customers", API_PREFIX), customers::router())
        .nest(&format!("{}/payments", API_PREFIX), payments::router())
        .nest(&format!("{}/suppliers", API_PREFIX), purchasing::suppliers_router())
        .nest(&format!("{}/purchase-orders", API_PREFIX), purchasing::purchase_orders_router())
        .nest(&format!("{}/roles", API_PREFIX), roles::router())
        .nest(&format!("{}/product-types", API_PREFIX), products::product_types_router())
        .nest(&format!("{}/products", API_PREFIX), products::router())
        .nest(&format!("{}/stock", API_PREFIX), stock::router())
        .nest(&format!("{}/shifts", API_PREFIX), sales::shifts_router())
        .nest(&format!("{}/sales", API_PREFIX), sales::router())
        .nest(&format!("{}/sync", API_PREFIX), sync::router())
        .with_state(state);
}
