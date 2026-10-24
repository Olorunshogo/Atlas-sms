/*!
 * Read-only reports: payment reconciliation against sales, the per-shift cash
 * summary a till close is checked against, and the FR-25 business reports.
 */
pub mod business;
mod handlers;
pub mod models;
pub mod service;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/payments/reconciliation", get(handlers::reconciliation))
        .route("/summary", get(business::summary))
        .route("/sales-by-product", get(business::sales_by_product))
        .route("/inventory-valuation", get(business::inventory_valuation))
        .route("/purchases-by-supplier", get(business::purchases_by_supplier))
        .route("/expenses-by-category", get(business::expenses_by_category));
}
