use std::sync::Arc;

use sqlx::PgPool;

use crate::email::EmailProvider;
use crate::rate_limit::RateLimiter;

// === AppState

/**
 * App-wide shared state.
 * Cloned per request (cheap: `PgPool` and `Arc` are both reference-counted handles, not the resources themselves).
 */
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub email: Arc<dyn EmailProvider>,
    pub jwt_secret: Arc<Vec<u8>>,
    pub rate_limiter: RateLimiter,
}
