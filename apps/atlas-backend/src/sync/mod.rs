mod handlers;

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/sales", post(handlers::sync_sale))
}
