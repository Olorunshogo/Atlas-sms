mod admin;
mod categories;
mod handlers;
mod models;
mod price_history;
mod selling;
mod service;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::state::AppState;

pub fn product_types_router() -> Router<AppState> {
    return Router::new().route(
        "/",
        get(handlers::list_product_types).post(handlers::create_product_type),
    );
}

pub fn categories_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(categories::list_categories).post(categories::create_category))
        .route("/{id}", patch(categories::update_category))
        .route("/{id}/archive", post(categories::archive_category))
        .route("/{id}/restore", post(categories::restore_category));
}

pub fn units_router() -> Router<AppState> {
    return Router::new().route("/", get(admin::list_units).post(admin::create_unit));
}

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_products).post(handlers::create_product))
        .route("/{id}", get(admin::get_product).patch(selling::update_product))
        .route("/{id}/listings/{store_id}", put(selling::upsert_listing))
        .route("/{id}/sale-units", get(admin::list_sale_units).post(selling::create_sale_unit))
        .route("/{id}/price-rules", get(admin::list_price_rules).post(selling::create_price_rule))
        .route("/{id}/price-rules/{rule_id}", delete(admin::delete_price_rule))
        .route("/{id}/price-history", get(price_history::list_price_history))
        .route("/{id}/barcodes", post(selling::create_barcode));
}
