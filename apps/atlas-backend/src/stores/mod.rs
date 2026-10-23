mod handlers;
mod models;
mod service;
mod settings;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_stores).post(handlers::create_store))
        .route("/{id}", get(handlers::get_store).patch(handlers::update_store))
        .route("/{id}/settings", get(settings::get_settings).patch(settings::update_settings));
}
