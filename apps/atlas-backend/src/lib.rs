use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use sqlx::PgPool;

mod auth;
mod crypto;
mod customers;
mod email;
mod error;
mod extractors;
mod http;
mod pricing;
mod products;
mod response;
mod roles;
mod sales;
mod state;
mod stock;
mod stores;
mod sync;
mod tenants;
mod validate;

pub use auth::issue_access_token;
pub use email::LogEmailProvider;
pub use state::AppState;

/**
 * Health check only, no state - kept exactly as it was so
 * `tests/health_check.rs` (which calls `app()` with no arguments) keeps
 * working unmodified.
 */
pub fn app() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> &'static str {
    "ok"
}

/**
 * Builds `AppState` from environment: `ATLAS_DATABASE_URL` (connected as
 * `atlas_api`, never the superuser - RLS is bypassed entirely for a
 * superuser connection, see the `enable_rls` migration) and
 * `ATLAS_JWT_SECRET` (signs/verifies the access token issued by
 * `auth::login`).
 *
 * Prefixed, unlike atlas-backend's plain `DATABASE_URL`/`PORT`: Nx
 * loads the one root `.env` into every task's environment, so an
 * unprefixed name here would collide with atlas-backend's own
 * same-named variable the moment both are set in that one file.
 */
pub async fn build_state() -> AppState {
    let database_url = std::env::var("ATLAS_DATABASE_URL").expect("ATLAS_DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let jwt_secret = std::env::var("ATLAS_JWT_SECRET").expect("ATLAS_JWT_SECRET must be set");

    AppState {
        pool,
        email: Arc::new(LogEmailProvider),
        jwt_secret: Arc::new(jwt_secret.into_bytes()),
    }
}

/**
 * The versioned API surface. Kept separate from `app()` because it needs
 * `AppState` (a database pool) and `app()`'s zero-argument shape is relied
 * on by the health check test.
 */
pub fn api_router(state: AppState) -> Router {
    Router::new()
        .nest("/api/v1/auth", auth::router())
        .nest("/api/v1/tenants", tenants::router())
        .nest("/api/v1/stores", stores::router())
        .nest("/api/v1/roles", roles::router())
        .nest("/api/v1/product-types", products::product_types_router())
        .nest("/api/v1/units", products::units_router())
        .nest("/api/v1/price-rules", pricing::router())
        .nest("/api/v1/customer-groups", customers::groups_router())
        .nest("/api/v1/customers", customers::router())
        .nest("/api/v1/products", products::router())
        .nest("/api/v1/stock", stock::router())
        .nest("/api/v1/shifts", sales::shifts_router())
        .nest("/api/v1/sales", sales::router())
        .nest("/api/v1/sync", sync::router())
        .fallback(http::not_found)
        .method_not_allowed_fallback(http::method_not_allowed)
        .with_state(state)
}
