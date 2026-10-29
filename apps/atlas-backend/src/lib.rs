// Format strings use positional args, not inline captures, throughout this
// crate - see the coding-assistant skill's locked rule.
#![allow(clippy::uninlined_format_args)]

use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use sqlx::PgPool;

mod approvals;
mod audit;
mod audit_log;
mod auth;
mod catalog;
mod crypto;
mod csv;
mod customers;
mod email;
mod error;
mod expenses;
mod extractors;
mod imports;
pub mod integrations;
mod inventory;
mod normalize;
mod notifications;
mod operations;
mod payments;
mod pdf;
mod pricing;
mod products;
mod purchasing;
mod rate_limit;
mod reports;
mod response;
mod returns;
pub mod roles;
mod sales;
pub mod shop;
mod state;
mod stock;
mod stores;
mod sync;
mod tenants;
mod validate;

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
    let webhook_secret = std::env::var("ATLAS_WEBHOOK_SECRET").unwrap_or_else(|_| format!("{}:webhook", jwt_secret));

    return AppState {
        pool,
        email: Arc::new(LogEmailProvider),
        jwt_secret: Arc::new(jwt_secret.into_bytes()),
        rate_limiter: rate_limit::RateLimiter::new(),
        integrations: integrations::Integrations::mock(webhook_secret.as_bytes()),
    };
}

/// One pass of the notification email worker (FR-26); `main` runs it on an interval, tests call it directly.
pub async fn deliver_notifications(state: &AppState) -> Result<notifications::delivery::DeliveryRun, error::ApiError> {
    return notifications::delivery::deliver_pending(&state.pool, state.email.as_ref()).await;
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
        .nest(&format!("{}/stores", API_PREFIX), stores::router().merge(catalog::router()))
        .nest(&format!("{}/tax-rates", API_PREFIX), pricing::tax_rates_router())
        .nest(&format!("{}/customers", API_PREFIX), customers::router())
        .nest(&format!("{}/customer-groups", API_PREFIX), customers::groups_router())
        .nest(&format!("{}/units", API_PREFIX), products::units_router())
        .nest(&format!("{}/payments", API_PREFIX), payments::router())
        .nest(&format!("{}/payment-methods", API_PREFIX), payments::methods_router())
        .nest(&format!("{}/suppliers", API_PREFIX), purchasing::suppliers_router())
        .nest(&format!("{}/purchase-orders", API_PREFIX), purchasing::purchase_orders_router())
        .nest(&format!("{}/purchase-returns", API_PREFIX), purchasing::purchase_returns_router())
        .nest(&format!("{}/expense-categories", API_PREFIX), expenses::categories_router())
        .nest(&format!("{}/expenses", API_PREFIX), expenses::router())
        .nest(&format!("{}/notifications", API_PREFIX), notifications::router())
        .nest(&format!("{}/approval-policies", API_PREFIX), approvals::policies_router())
        .nest(&format!("{}/approval-requests", API_PREFIX), approvals::requests_router())
        .nest(&format!("{}/audit-events", API_PREFIX), audit_log::router())
        .nest(&format!("{}/operations", API_PREFIX), operations::router())
        .nest(&format!("{}/imports", API_PREFIX), imports::imports_router())
        .nest(&format!("{}/exports", API_PREFIX), imports::exports_router())
        .nest(&format!("{}/roles", API_PREFIX), roles::router())
        .nest(&format!("{}/product-types", API_PREFIX), products::product_types_router())
        .nest(&format!("{}/product-categories", API_PREFIX), products::categories_router())
        .nest(&format!("{}/products", API_PREFIX), products::router())
        .nest(&format!("{}/stock", API_PREFIX), stock::router())
        .nest(&format!("{}/shifts", API_PREFIX), sales::shifts_router())
        .nest(&format!("{}/sales", API_PREFIX), sales::router().merge(returns::sale_routes()))
        .nest(&format!("{}/returns", API_PREFIX), returns::router())
        .nest(&format!("{}/sync", API_PREFIX), sync::router())
        .nest(&format!("{}/reports", API_PREFIX), reports::router())
        .nest(&format!("{}/shop", API_PREFIX), shop::shop_router())
        .nest(&format!("{}/storefronts", API_PREFIX), shop::storefronts_router())
        .nest(&format!("{}/promotions", API_PREFIX), shop::promotions_router())
        .nest(&format!("{}/shop-orders", API_PREFIX), shop::orders_router())
        .nest(&format!("{}/shop-webhooks", API_PREFIX), shop::webhooks_router())
        .with_state(state)
        .layer(axum::middleware::from_fn(normalize::json_errors));
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    /// The source text of `pub fn name()` in a module's `mod.rs` (or its single `module.rs`), up to the next `pub fn`.
    fn router_body(module: &str, name: &str) -> String {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let nested = src.join(module).join("mod.rs");
        let path = if nested.exists() { nested } else { src.join(format!("{}.rs", module)) };
        let source = std::fs::read_to_string(&path).unwrap();
        let start = source
            .find(&format!("pub fn {}(", name))
            .unwrap_or_else(|| panic!("no {}::{}", module, name));
        let rest = &source[start + 1..];
        let end = rest.find("pub fn ").map(|i| i + 1).unwrap_or(rest.len());
        return source[start..start + end].to_string();
    }

    /// Every `METHOD /path` the API mounts, read from `api_router` and each module's router function.
    fn mounted_routes() -> BTreeSet<String> {
        let lib = include_str!("lib.rs");
        let start = lib.find("pub fn api_router").unwrap();
        let body = &lib[start..start + lib[start..].find("#[cfg(test)]").unwrap()];
        let mut routes = BTreeSet::new();
        for line in body.lines().filter(|l| l.contains(".nest(")) {
            let prefix = line.split("\"{}").nth(1).unwrap().split('"').next().unwrap();
            let calls = line.split(API_PREFIX_ARG).nth(1).unwrap();
            for call in calls.split(&['(', ')', '.', ' ', ','][..]).filter(|c| c.contains("::")) {
                let (module, name) = call.split_once("::").unwrap();
                for route in router_body(module, name).split(".route(").skip(1) {
                    let path = route.split('"').nth(1).unwrap();
                    let full = if path == "/" { prefix.to_string() } else { format!("{}{}", prefix, path) };
                    let handlers = route.split("))").next().unwrap_or(route);
                    for method in ["get", "post", "patch", "put", "delete"] {
                        if handlers.contains(&format!("{}(", method)) {
                            routes.insert(format!("{} {}", method.to_uppercase(), full));
                        }
                    }
                }
            }
        }
        return routes;
    }

    const API_PREFIX_ARG: &str = "API_PREFIX),";

    /// Fails when a route is mounted but `docs/atlas/api-reference.md` never mentions it as `METHOD /path`.
    #[test]
    fn every_route_is_in_the_api_reference() {
        let reference = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/atlas/api-reference.md"),
        )
        .unwrap();
        let routes = mounted_routes();
        assert!(routes.len() > 60, "route discovery found only {:?}", routes);
        let missing: Vec<&String> = routes.iter().filter(|r| !reference.contains(&format!("`{}", r))).collect();
        assert!(missing.is_empty(), "routes missing from docs/atlas/api-reference.md: {:#?}", missing);
    }
}
