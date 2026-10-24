mod handlers;
mod models;
mod service;
mod suppliers;

use axum::routing::{get, post, put};
use axum::Router;

use crate::state::AppState;

pub fn suppliers_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(suppliers::list_suppliers).post(suppliers::create_supplier))
        .route("/{id}", get(suppliers::show_supplier).patch(suppliers::update_supplier))
        .route("/{id}/deactivate", post(suppliers::deactivate_supplier))
        .route("/{id}/reactivate", post(suppliers::reactivate_supplier))
        .route("/{id}/products", put(suppliers::set_supplier_products))
        .route("/{id}/payments", post(suppliers::record_payment))
        .route("/{id}/transactions", get(suppliers::list_transactions));
}

pub fn purchase_orders_router() -> Router<AppState> {
    return Router::new()
        .route(
            "/",
            get(handlers::list_purchase_orders).post(handlers::create_purchase_order),
        )
        .route("/{id}", get(handlers::show_purchase_order).patch(handlers::edit_purchase_order))
        .route("/{id}/lines", get(handlers::list_purchase_order_lines))
        .route("/{id}/receive", post(handlers::receive_goods))
        .route("/{id}/submit", post(handlers::submit_purchase_order))
        .route("/{id}/approve", post(handlers::approve_purchase_order))
        .route("/{id}/cancel", post(handlers::cancel_purchase_order));
}

pub fn purchase_returns_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_purchase_returns).post(handlers::record_purchase_return))
        .route("/{id}", get(handlers::show_purchase_return))
        .route("/{id}/confirm", post(handlers::confirm_purchase_return))
        .route("/{id}/cancel", post(handlers::cancel_purchase_return));
}
