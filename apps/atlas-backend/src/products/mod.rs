mod handlers;
mod models;
mod selling;
mod service;

use axum::routing::{get, patch, post, put};
use axum::Router;

use crate::state::AppState;

pub fn product_types_router() -> Router<AppState> {
    return Router::new().route(
        "/",
        get(handlers::list_product_types).post(handlers::create_product_type),
    );
}

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_products).post(handlers::create_product))
        .route("/{id}", patch(selling::update_product))
        .route("/{id}/listings/{store_id}", put(selling::upsert_listing))
        .route("/{id}/sale-units", post(selling::create_sale_unit))
        .route("/{id}/price-rules", post(selling::create_price_rule))
        .route("/{id}/barcodes", post(selling::create_barcode));
}
