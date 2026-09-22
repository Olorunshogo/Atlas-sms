mod handlers;
mod models;
mod service;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new().route("/", get(handlers::list_stores).post(handlers::create_store));
}
