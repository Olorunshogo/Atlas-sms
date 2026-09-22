use axum::http::HeaderMap;

// === Client type

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientType {
    /// `atlas-marketing`'s Nitro proxy - gets HttpOnly cookies, never
    /// tokens in the response body.
    Web,
    /// `atlas-pos` (Capacitor) - no server of its own to hold a cookie
    /// for it, so it gets tokens in the body instead. Its own client
    /// keeps the refresh token in Capacitor secure storage and the access
    /// token in memory only, never `localStorage`.
    App,
}

/**
 * Read from `X-Client-Type`, defaulting to `Web` when absent or
 * unrecognized - a response-shape hint only, never a permission boundary.
 * Neither mode may ever grant more than the other; the worst outcome of a
 * forged or missing value is a client receiving a token shape it can't use.
 */
pub fn client_type(headers: &HeaderMap) -> ClientType {
    match headers.get("x-client-type").and_then(|value| value.to_str().ok()) {
        Some("app") => ClientType::App,
        _ => ClientType::Web,
    }
}

pub const ACCESS_TOKEN_COOKIE: &str = "access_token";
pub const REFRESH_TOKEN_COOKIE: &str = "refresh_token";

/**
 * `HttpOnly`, `Secure`, `SameSite=Lax`, `Path=/` - client-side JS never
 * reads these, and they ride along on the proxy's own same-site requests
 * to `atlas-backend`. Host-only (no `Domain=`) deliberately: Atlas has no
 * per-tenant subdomain requiring a shared cookie domain, unlike Klassivox's
 * school-subdomain isolation.
 */
pub fn build_cookie(name: &str, value: &str, max_age_seconds: i64) -> String {
    format!("{name}={value}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={max_age_seconds}")
}
