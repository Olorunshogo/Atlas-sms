pub mod delivery;
mod handlers;
mod preferences;
pub mod models;
pub mod service;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub use models::NotificationKind;

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_notifications))
        .route("/unread-count", get(handlers::unread_count))
        .route("/preferences", get(preferences::get_preferences).put(preferences::set_preferences))
        .route("/read-all", post(handlers::mark_all_read))
        .route("/{id}/read", post(handlers::mark_read));
}
