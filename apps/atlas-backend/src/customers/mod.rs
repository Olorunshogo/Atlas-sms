mod handlers;
mod models;
mod service;

use axum::routing::{get, patch};
use axum::Router;

use crate::state::AppState;

pub fn groups_router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list_groups).post(handlers::create_group))
        .route(
            "/{id}",
            patch(handlers::rename_group).delete(handlers::delete_group),
        )
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::list_customers).post(handlers::create_customer),
        )
        .route(
            "/{id}",
            get(handlers::get_customer)
                .patch(handlers::update_customer)
                .delete(handlers::delete_customer),
        )
}
