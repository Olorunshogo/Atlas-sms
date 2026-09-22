mod handlers;
mod models;
mod service;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/me", get(handlers::me))
}
