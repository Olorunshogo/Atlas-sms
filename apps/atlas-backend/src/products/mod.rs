mod handlers;
mod models;
mod service;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn product_types_router() -> Router<AppState> {
    return Router::new().route(
        "/",
        get(handlers::list_product_types).post(handlers::create_product_type),
    );
}

pub fn router() -> Router<AppState> {
    return Router::new().route(
        "/",
        get(handlers::list_products).post(handlers::create_product),
    );
}
