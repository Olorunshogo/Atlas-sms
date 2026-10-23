mod handlers;
mod models;
pub mod service;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/on-hand", get(handlers::on_hand))
        .route(
            "/inventory",
            get(handlers::inventory).post(handlers::initial_stock),
        )
        .route("/products/{product_id}", get(handlers::product_inventory))
        .route("/movements", get(handlers::movements))
        .route("/restocks", post(handlers::restock))
        .route("/adjustments", post(handlers::adjust))
        .route("/transfers", post(handlers::transfer))
}
