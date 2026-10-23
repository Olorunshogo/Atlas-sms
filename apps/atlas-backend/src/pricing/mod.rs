/*!
 * Server-side pricing: `engine` is the pure arithmetic and rounding,
 * `resolver` answers "what does this cost here, for this customer, today"
 * from `price_rules` and tax rates, and `quote` puts both together into a
 * fully priced cart the quote endpoint and checkout share.
 */
pub mod engine;
pub mod models;
pub mod quote;
pub mod resolver;

mod handlers;

use axum::routing::{get, patch};
use axum::Router;

use crate::state::AppState;

pub use models::{Quote, QuoteRequest};

pub fn tax_rates_router() -> Router<AppState> {
    return Router::new()
        .route("/", get(handlers::list_tax_rates).post(handlers::create_tax_rate))
        .route("/{id}", patch(handlers::update_tax_rate));
}
