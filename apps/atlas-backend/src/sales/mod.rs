pub mod checkout;
mod handlers;
pub mod models;
pub mod query;
pub mod receipts;
pub mod service;
pub mod settlement;

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
        .route("/by-receipt/{receipt_number}", get(handlers::get_by_receipt))
        .route("/{id}", get(handlers::get_sale))
        .route("/{id}/lines", get(handlers::list_sale_lines))
        .route("/{id}/receipt", get(handlers::get_receipt))
        .route("/{id}/receipt/prints", post(handlers::print_receipt))
        .route("/{id}/void", post(handlers::void_sale))
        .route("/{id}/payments", post(handlers::add_payment));
}
