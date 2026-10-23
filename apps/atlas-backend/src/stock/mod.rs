mod handlers;
mod models;
mod service;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/on-hand", get(handlers::get_stock_on_hand))
        .route("/inventory", get(handlers::get_inventory))
        .route("/products/{product_id}", get(handlers::get_product_inventory))
        .route("/movements", get(handlers::list_movements))
        .route("/adjustments", post(handlers::adjust_stock))
        .route("/transfers", post(handlers::transfer_stock));
}
