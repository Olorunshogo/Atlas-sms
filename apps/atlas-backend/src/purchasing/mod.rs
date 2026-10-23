mod handlers;
mod models;
mod service;

use axum::routing::{get, patch, post};
use axum::Router;

use crate::state::AppState;

pub fn suppliers_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_suppliers).post(handlers::create_supplier))
        .route("/{id}", patch(handlers::update_supplier));
}

pub fn purchase_orders_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_purchase_orders).post(handlers::create_purchase_order))
        .route("/{id}/lines", get(handlers::list_purchase_order_lines))
        .route("/{id}/receive", post(handlers::receive_goods));
}
