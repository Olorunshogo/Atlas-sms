mod handlers;
mod models;
pub mod service;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list))
        .route("/invite", post(handlers::invite))
}
