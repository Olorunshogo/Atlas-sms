pub mod checkout;
mod handlers;
pub mod models;
pub mod service;

pub use models::{CreateSaleRequest, SaleDetail};

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn shifts_router() -> Router<AppState> {
    return Router::new()
        .route("/", post(handlers::open_shift))
        .route("/{id}/close", post(handlers::close_shift));
}

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_sales).post(handlers::create_sale))
        .route("/{id}/lines", get(handlers::list_sale_lines))
        .route("/{id}/void", post(handlers::void_sale));
}
