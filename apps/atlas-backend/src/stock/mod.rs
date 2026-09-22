mod handlers;
mod models;
mod service;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/on-hand", get(handlers::get_stock_on_hand))
        .route("/adjustments", post(handlers::adjust_stock))
        .route("/transfers", post(handlers::transfer_stock));
}
