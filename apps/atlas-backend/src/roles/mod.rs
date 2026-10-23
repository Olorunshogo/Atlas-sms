mod handlers;
mod models;
pub mod service;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new().route("/", get(handlers::list_roles)).route("/invite", post(handlers::invite));
}
