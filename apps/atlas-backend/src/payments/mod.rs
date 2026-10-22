mod handlers;
mod models;
pub mod service;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_payments).post(handlers::create_payment))
        .route("/{id}/refund", post(handlers::request_refund))
        .route("/refunds/{id}/confirm", post(handlers::confirm_refund));
}
