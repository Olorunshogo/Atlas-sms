mod handlers;
mod models;
mod service;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn product_types_router() -> Router<AppState> {
    Router::new().route(
        "/",
        get(handlers::list_product_types).post(handlers::create_product_type),
    )
}

pub fn units_router() -> Router<AppState> {
    Router::new().route("/", get(handlers::list_units).post(handlers::create_unit))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::list_products).post(handlers::create_product),
        )
        .route(
            "/{product_id}",
            get(handlers::get_product).patch(handlers::update_product),
        )
        .route(
            "/{product_id}/sale-units",
            get(handlers::list_sale_units).post(handlers::create_sale_unit),
        )
}
