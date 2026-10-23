mod handlers;
mod models;
mod service;

use axum::routing::{get, patch, post};
use axum::Router;

use crate::state::AppState;

pub fn categories_router() -> Router<AppState> {
    return Router::new().route("/", get(handlers::list_categories).post(handlers::create_category));
}

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_expenses).post(handlers::create_expense))
        .route("/{id}", patch(handlers::edit_expense))
        .route("/{id}/submit", post(handlers::submit_expense))
        .route("/{id}/approve", post(handlers::approve_expense))
        .route("/{id}/reject", post(handlers::reject_expense))
        .route("/{id}/pay", post(handlers::mark_expense_paid));
}
