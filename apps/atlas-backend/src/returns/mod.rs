/*!
 * Sales returns and refunds (FR-18): locating the original sale, returning
 * some or all of its items within the store's rules, classifying each item
 * by condition, and paying the refund back out.
 */
mod handlers;
pub mod models;
mod receipt;
pub mod service;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_returns))
        .route("/{id}", get(handlers::get_return))
        .route("/{id}/approve", post(handlers::approve_return))
        .route("/{id}/reject", post(handlers::reject_return))
        .route("/{id}/receipt", get(handlers::return_receipt));
}

/// Merged into the sales router: returns start from the sale they're against.
pub fn sale_routes() -> Router<AppState> {
    return Router::new().route("/{id}/returns", get(handlers::list_for_sale).post(handlers::create_return));
}
