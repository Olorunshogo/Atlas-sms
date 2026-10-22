mod handlers;
pub mod models;
pub mod service;

use axum::routing::{get, patch, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_payments))
        .route("/{id}/status", post(handlers::resolve_payment))
        .route("/{id}/reverse", post(handlers::reverse_payment))
        .route("/{id}/refund", post(handlers::request_refund))
        .route("/refunds/{id}/confirm", post(handlers::confirm_refund));
}

pub fn methods_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_methods).post(handlers::create_method))
        .route("/{id}", patch(handlers::update_method));
}
