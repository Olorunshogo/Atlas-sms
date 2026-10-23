/*!
 * What a till needs to build a cart: product search, barcode lookup, a
 * server-priced quote, and a full snapshot for offline use. Lives under
 * `/api/v1/stores/{id}/catalog` because every answer is store-specific -
 * what's listed, at what price, with how much on hand.
 */
mod handlers;
mod models;
mod service;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

/// Merged into the stores router, so the store segment shares its `{id}` name with `/stores/{id}/settings`.
pub fn router() -> Router<AppState> {
    return Router::new()
        .route("/{id}/catalog", get(handlers::search))
        .route("/{id}/catalog/barcode/{code}", get(handlers::by_barcode))
        .route("/{id}/catalog/quote", post(handlers::quote))
        .route("/{id}/catalog/snapshot", get(handlers::snapshot));
}
