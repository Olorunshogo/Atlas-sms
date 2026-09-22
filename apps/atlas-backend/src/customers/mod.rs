mod handlers;
mod models;
mod service;

use axum::routing::{get, patch};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_customers).post(handlers::create_customer))
        .route("/{id}", patch(handlers::update_customer));
}
