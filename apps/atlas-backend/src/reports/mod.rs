/*!
 * Read-only money reports: payment reconciliation against sales, and the
 * per-shift cash summary a till close is checked against.
 */
mod handlers;
pub mod models;
pub mod service;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    return Router::new().route("/payments/reconciliation", get(handlers::reconciliation));
}
