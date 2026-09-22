mod handlers;
mod models;
pub mod service;

pub use models::{CreateSaleRequest, Sale};

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn shifts_router() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::open_shift))
        .route("/{id}/close", post(handlers::close_shift))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list).post(handlers::create))
        .route("/{id}/lines", get(handlers::lines))
}
