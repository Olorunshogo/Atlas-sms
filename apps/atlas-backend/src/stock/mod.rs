mod handlers;
mod models;
mod service;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/on-hand", get(handlers::on_hand))
        .route("/adjustments", post(handlers::adjust))
        .route("/transfers", post(handlers::transfer))
}
