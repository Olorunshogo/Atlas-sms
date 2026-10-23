mod groups;
mod handlers;
mod models;
pub mod service;

use axum::routing::{get, patch};
use axum::Router;

use crate::state::AppState;

pub fn groups_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(groups::list_groups).post(groups::create_group))
        .route("/{id}", patch(groups::rename_group).delete(groups::delete_group));
}

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_customers).post(handlers::create_customer))
        .route(
            "/{id}",
            get(handlers::get_customer)
                .patch(handlers::update_customer)
                .delete(handlers::delete_customer),
        )
        .route("/{id}/sales", get(handlers::purchase_history));
}
