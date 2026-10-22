mod handlers;
mod models;
pub mod service;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

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
