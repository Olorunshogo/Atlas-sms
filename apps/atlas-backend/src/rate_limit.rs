use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::error::ApiError;
use crate::state::AppState;

// === RateLimiter

const DEFAULT_WINDOW: Duration = Duration::from_secs(60);
const DEFAULT_MAX_ATTEMPTS: usize = 5;

/**
 * In-memory, per-IP sliding-window limiter, applied to the whole auth router (see `rate_limit` below).
 * Resets on process restart and doesn't share state across more than one backend instance - both fine at Atlas's current single-instance deployment scale, worth revisiting (a shared store such as Redis) if that changes.
 */
#[derive(Clone)]
pub struct RateLimiter {
    hits: Arc<Mutex<HashMap<IpAddr, VecDeque<Instant>>>>,
    max_attempts: usize,
    window: Duration,
}

impl Default for RateLimiter {
    fn default() -> Self {
        return RateLimiter::new();
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        return RateLimiter::with_limits(DEFAULT_MAX_ATTEMPTS, DEFAULT_WINDOW);
    }

    /**
     * Same mechanism as `new()`, a configurable budget instead of the production default.
     * For tests: a single test can legitimately need more than `DEFAULT_MAX_ATTEMPTS` real requests against the auth router in a few seconds (a full signup -> verify -> login -> refresh -> logout flow, or five deliberate wrong-OTP guesses plus the calls around them) - `new()`'s tight production limit would rate-limit the test itself, not the behavior under test.
     */
    pub fn with_limits(max_attempts: usize, window: Duration) -> Self {
        return RateLimiter {
            hits: Arc::new(Mutex::new(HashMap::new())),
            max_attempts,
            window,
        };
    }

    fn check(&self, ip: IpAddr) -> Result<(), ApiError> {
        let mut hits = self.hits.lock().expect("rate limiter mutex poisoned");
        let now = Instant::now();
        let window = hits.entry(ip).or_default();

        while window.front().is_some_and(|&first| now.duration_since(first) > self.window) {
            window.pop_front();
        }

        if window.len() >= self.max_attempts {
            return Err(ApiError::TooManyRequests(
                "Too many attempts. Try again in a minute.".to_string(),
            ));
        }

        window.push_back(now);
        return Ok(());
    }
}

/**
 * Axum middleware wired via `route_layer` onto the entire auth router (see `auth::router`'s doc comment for why it's the whole router, not a curated subset).
 */
pub async fn rate_limit(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, ApiError> {
    state.rate_limiter.check(addr.ip())?;
    return Ok(next.run(request).await);
}
