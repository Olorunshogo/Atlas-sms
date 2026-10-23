//! End-to-end coverage of the auth lifecycle through the real HTTP router: signup -> verify -> login -> refresh -> logout, plus the negative paths that matter most (wrong password, unverified login, OTP brute-force lockout).
//! Uses `#[sqlx::test]` for an isolated database per test - see `roles_permission_tests.rs` for the `DATABASE_URL` requirement.
//!
//! `x-client-type: app` is used throughout so tokens come back directly in the JSON body instead of `Set-Cookie` headers, which is far simpler to assert against here and exercises the `atlas-pos` code path.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use atlas_backend::{api_router, AppState, EmailError, EmailProvider, RateLimiter};
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

// === Test email provider

/**
 * Captures every "sent" email instead of delivering it, so a test can pull the OTP code back out of the body the same way a real inbox would show it to a user.
 * `Mutex` because `EmailProvider::send` takes `&self`.
 */
#[derive(Clone, Default)]
struct CapturingEmailProvider {
    /// Each entry is (to, subject, body).
    sent: Arc<Mutex<Vec<(String, String, String)>>>,
}

#[async_trait::async_trait]
impl EmailProvider for CapturingEmailProvider {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError> {
        self.sent.lock().unwrap().push((to.to_string(), subject.to_string(), body.to_string()));
        return Ok(());
    }
}

impl CapturingEmailProvider {
    /// Pulls the 6-digit code out of the most recently sent email's body - "Your verification code is 123456. It expires in 10 minutes."
    fn last_otp(&self) -> String {
        let sent = self.sent.lock().unwrap();
        let (_, _, body) = sent.last().expect("an email should have been sent");
        let after = body.split("code is ").nth(1).expect("body contains the code");
        return after.split('.').next().unwrap().trim().to_string();
    }

    /// Pulls the invite token out of the most recently sent invite email's body - "...Use this token to accept: <token>".
    fn last_invite_token(&self) -> String {
        let sent = self.sent.lock().unwrap();
        let (_, _, body) = sent.last().expect("an email should have been sent");
        return body.split("accept: ").nth(1).expect("body contains the token").trim().to_string();
    }
}

// === Fixed seeded system role ids (migration 20260922000009) - same
// constants `auth::handlers::owner_role_id` and the seed data use.
const OWNER_ROLE_ID: &str = "00000000-0000-7000-8000-000000000001";
const CASHIER_ROLE_ID: &str = "00000000-0000-7000-8000-000000000004";

// === Harness

fn build_router(pool: PgPool, email: CapturingEmailProvider) -> Router {
    // Production's default (5/60s) would rate-limit these tests' legitimate
    // multi-request flows, not just abuse - see `RateLimiter::with_limits`'s
    // doc comment.
    return build_router_with_limiter(pool, email, RateLimiter::with_limits(1000, std::time::Duration::from_secs(60)));
}

fn build_router_with_limiter(pool: PgPool, email: CapturingEmailProvider, rate_limiter: RateLimiter) -> Router {
    let state = AppState {
        pool,
        email: Arc::new(email),
        jwt_secret: Arc::new(b"test-only-secret-not-for-production".to_vec()),
        rate_limiter,
    };
    return api_router(state);
}

fn request_with_connect_info(builder: axum::http::request::Builder, body: Value) -> Request<Body> {
    let mut request = builder
        .header("content-type", "application/json")
        .header("x-client-type", "app")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    // `oneshot` doesn't go through `into_make_service_with_connect_info` the
    // way a real server does, so the rate limiter's `ConnectInfo` extractor
    // has nothing to read unless the test inserts it itself - a missing
    // extension is a plain-text axum rejection, not JSON.
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    return request;
}

async fn send(router: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.clone().oneshot(request).await.unwrap();

    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    return (status, json);
}

async fn post(router: &Router, path: &str, body: Value) -> (StatusCode, Value) {
    let request = request_with_connect_info(Request::builder().method("POST").uri(path), body);
    return send(router, request).await;
}

/// Same as `post`, with an `Authorization: Bearer` header - for the routes that need a real signed-in caller (`TenantTx`, or `switch-tenant`'s own direct token decode) rather than a session-bootstrapping one.
async fn authed_post(router: &Router, path: &str, token: &str, body: Value) -> (StatusCode, Value) {
    let request = request_with_connect_info(
        Request::builder().method("POST").uri(path).header("authorization", format!("Bearer {}", token)),
        body,
    );
    return send(router, request).await;
}

async fn authed_get(router: &Router, path: &str, token: &str) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method("GET")
        .uri(path)
        .header("authorization", format!("Bearer {}", token))
        .header("x-client-type", "app")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    return send(router, request).await;
}

async fn signup(router: &Router, address: &str) {
    let (status, _) = post(
        router,
        "/api/v1/auth/signup",
        json!({
            "full_name": "Test User",
            "email": address,
            "phone": "+2340000000000",
            "password": "a-fairly-strong-password",
            "tenant_name": "Test Tenant",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
}

async fn signup_and_verify(router: &Router, email: &CapturingEmailProvider, address: &str) {
    let (status, _) = post(
        router,
        "/api/v1/auth/signup",
        json!({
            "full_name": "Test User",
            "email": address,
            "phone": "+2340000000000",
            "password": "a-fairly-strong-password",
            "tenant_name": "Test Tenant",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let code = email.last_otp();
    let (status, _) = post(router, "/api/v1/auth/verify-email", json!({ "email": address, "code": code })).await;
    assert_eq!(status, StatusCode::OK);
}

// === Happy path

#[sqlx::test]
async fn full_lifecycle_signup_login_refresh_logout(pool: PgPool) {
    let email = CapturingEmailProvider::default();
    let router = build_router(pool, email.clone());
    let address = "lifecycle@example.com";

    signup_and_verify(&router, &email, address).await;

    let (status, body) = post(
        &router,
        "/api/v1/auth/login",
        json!({ "email": address, "password": "a-fairly-strong-password" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let access_token = body["data"]["access_token"].as_str().unwrap().to_string();
    let refresh_token = body["data"]["refresh_token"].as_str().unwrap().to_string();
    assert!(!access_token.is_empty());

    let (status, body) =
        post(&router, "/api/v1/auth/refresh", json!({ "refresh_token": refresh_token })).await;
    assert_eq!(status, StatusCode::OK);
    let rotated_refresh_token = body["data"]["refresh_token"].as_str().unwrap().to_string();
    assert_ne!(rotated_refresh_token, refresh_token, "refresh must rotate the token, not reuse it");

    // The old refresh token is now revoked - a second refresh with it fails.
    let (status, _) = post(&router, "/api/v1/auth/refresh", json!({ "refresh_token": refresh_token })).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Logging out the current (rotated) token, then trying to refresh with it, also fails.
    let (status, _) =
        post(&router, "/api/v1/auth/logout", json!({ "refresh_token": rotated_refresh_token })).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) =
        post(&router, "/api/v1/auth/refresh", json!({ "refresh_token": rotated_refresh_token })).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// === Negative paths

#[sqlx::test]
async fn login_with_the_wrong_password_is_rejected(pool: PgPool) {
    let email = CapturingEmailProvider::default();
    let router = build_router(pool, email.clone());
    let address = "wrongpass@example.com";

    signup_and_verify(&router, &email, address).await;

    let (status, _) =
        post(&router, "/api/v1/auth/login", json!({ "email": address, "password": "not-the-password" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn an_unverified_user_can_log_in_but_tenant_endpoints_stay_closed_until_they_verify(pool: PgPool) {
    let email = CapturingEmailProvider::default();
    let router = build_router(pool, email.clone());
    let address = "unverified@example.com";
    signup(&router, address).await;

    let (status, body) = post(&router, "/api/v1/auth/login", json!({ "email": address, "password": "a-fairly-strong-password" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["user"]["email_verified"], false);
    let token = body["data"]["access_token"].as_str().unwrap().to_string();
    let refresh_token = body["data"]["refresh_token"].as_str().unwrap().to_string();

    let (status, body) = authed_get(&router, "/api/v1/auth/me", &token).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["user"]["email_verified"], false);
    assert_eq!(body["data"]["tenants"].as_array().unwrap().len(), 1);

    for path in ["/api/v1/stores", "/api/v1/tenants/me", "/api/v1/notifications"] {
        let (status, body) = authed_get(&router, path, &token).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{}: {}", path, body);
        assert_eq!(body["code"], "EMAIL_NOT_VERIFIED", "{}", path);
    }

    let code = email.last_otp();
    let (status, _) = post(&router, "/api/v1/auth/verify-email", json!({ "email": address, "code": code })).await;
    assert_eq!(status, StatusCode::OK);

    // The same token works straight away: the gate reads the user, not the token
    let (status, body) = authed_get(&router, "/api/v1/stores", &token).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let (_, body) = authed_get(&router, "/api/v1/auth/me", &token).await;
    assert_eq!(body["data"]["user"]["email_verified"], true);
    let (status, body) = post(&router, "/api/v1/auth/refresh", json!({ "refresh_token": refresh_token })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["user"]["email_verified"], true);
}

#[sqlx::test]
async fn resend_verification_sends_at_most_one_code_a_minute_and_reveals_nothing(pool: PgPool) {
    let email = CapturingEmailProvider::default();
    let router = build_router(pool.clone(), email.clone());
    let address = "resend@example.com";
    signup(&router, address).await;
    let sent_before = email.sent.lock().unwrap().len();

    // Signup just sent one
    let (status, _) = post(&router, "/api/v1/auth/resend-verification", json!({ "email": address })).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    sqlx::query("UPDATE atlas.email_otps SET created_at = now() - interval '2 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    let (status, unknown) = post(&router, "/api/v1/auth/resend-verification", json!({ "email": "nobody@example.com" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(email.sent.lock().unwrap().len(), sent_before);

    let (status, known) = post(&router, "/api/v1/auth/resend-verification", json!({ "email": address })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(known["message"], unknown["message"]);
    assert_eq!(email.sent.lock().unwrap().len(), sent_before + 1);
    let (status, body) = post(&router, "/api/v1/auth/verify-email", json!({ "email": address, "code": email.last_otp() })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);

    // A verified account gets the same answer and no email
    sqlx::query("UPDATE atlas.email_otps SET created_at = now() - interval '2 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    let (status, _) = post(&router, "/api/v1/auth/resend-verification", json!({ "email": address })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(email.sent.lock().unwrap().len(), sent_before + 1);
}

#[sqlx::test]
async fn an_otp_is_locked_out_after_five_wrong_attempts(pool: PgPool) {
    let email = CapturingEmailProvider::default();
    let router = build_router(pool, email.clone());
    let address = "otplockout@example.com";

    let (status, _) = post(
        &router,
        "/api/v1/auth/signup",
        json!({
            "full_name": "Test User",
            "email": address,
            "phone": "+2340000000000",
            "password": "a-fairly-strong-password",
            "tenant_name": "Test Tenant",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let correct_code = email.last_otp();
    // A wrong code, guessed enough times to burn the attempt budget.
    let wrong_code = if correct_code == "000000" { "111111" } else { "000000" };

    for _ in 0..5 {
        let (status, _) =
            post(&router, "/api/v1/auth/verify-email", json!({ "email": address, "code": wrong_code })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    // The code is dead now - even the correct one no longer works.
    let (status, _) =
        post(&router, "/api/v1/auth/verify-email", json!({ "email": address, "code": correct_code })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// === Switch tenant

#[sqlx::test]
async fn switch_tenant_to_a_tenant_with_an_active_grant_succeeds(pool: PgPool) {
    let email = CapturingEmailProvider::default();
    let router = build_router(pool, email.clone());

    // Two separate owners, each with their own tenant from signup.
    signup_and_verify(&router, &email, "multi-tenant-user@example.com").await;
    signup_and_verify(&router, &email, "second-owner@example.com").await;

    // Captured now, before the second grant below makes a bare login (no
    // explicit tenant_id) ambiguous between two active tenants.
    let (_, original_login) = post(
        &router,
        "/api/v1/auth/login",
        json!({ "email": "multi-tenant-user@example.com", "password": "a-fairly-strong-password" }),
    )
    .await;
    let original_tenant_id = original_login["data"]["tenant_id"].as_str().unwrap().to_string();

    let (_, second_owner_login) = post(
        &router,
        "/api/v1/auth/login",
        json!({ "email": "second-owner@example.com", "password": "a-fairly-strong-password" }),
    )
    .await;
    let second_owner_token = second_owner_login["data"]["access_token"].as_str().unwrap().to_string();
    let second_tenant_id = second_owner_login["data"]["tenant_id"].as_str().unwrap().to_string();

    // The second owner invites the first user into their own tenant too -
    // an existing verified user, so this grants directly, no accept step.
    let (status, _) = authed_post(
        &router,
        "/api/v1/roles/invite",
        &second_owner_token,
        json!({ "email": "multi-tenant-user@example.com", "role_id": OWNER_ROLE_ID, "store_id": null }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, first_login) = post(
        &router,
        "/api/v1/auth/login",
        json!({
            "email": "multi-tenant-user@example.com",
            "password": "a-fairly-strong-password",
            "tenant_id": second_tenant_id,
        }),
    )
    .await;
    let first_user_token_for_second_tenant =
        first_login["data"]["access_token"].as_str().unwrap().to_string();

    // Switching to the tenant they hold no grant in yet fails...
    let (status, _) = authed_post(
        &router,
        "/api/v1/auth/switch-tenant",
        &first_user_token_for_second_tenant,
        json!({ "tenant_id": "00000000-0000-7000-8000-000000000000" }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // ...but switching back to their own original tenant (now that they
    // also hold a grant in the second one) succeeds and issues a session
    // scoped to it, without a fresh password.
    let (status, body) = authed_post(
        &router,
        "/api/v1/auth/switch-tenant",
        &first_user_token_for_second_tenant,
        json!({ "tenant_id": original_tenant_id }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["tenant_id"].as_str().unwrap(), original_tenant_id);
    assert!(!body["data"]["access_token"].as_str().unwrap().is_empty());
}

#[sqlx::test]
async fn switch_tenant_to_a_tenant_without_a_grant_is_forbidden(pool: PgPool) {
    let email = CapturingEmailProvider::default();
    let router = build_router(pool, email.clone());
    let address = "no-second-grant@example.com";

    signup_and_verify(&router, &email, address).await;
    let (_, body) =
        post(&router, "/api/v1/auth/login", json!({ "email": address, "password": "a-fairly-strong-password" }))
            .await;
    let access_token = body["data"]["access_token"].as_str().unwrap().to_string();

    let (status, _) = authed_post(
        &router,
        "/api/v1/auth/switch-tenant",
        &access_token,
        json!({ "tenant_id": "00000000-0000-7000-8000-000000000000" }),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// === Accept invite

#[sqlx::test]
async fn accept_invite_creates_a_preverified_user_who_can_log_in(pool: PgPool) {
    let email = CapturingEmailProvider::default();
    let router = build_router(pool, email.clone());
    let owner_address = "inviting-owner@example.com";
    let invitee_address = "brand-new-invitee@example.com";

    signup_and_verify(&router, &email, owner_address).await;
    let (_, login_body) = post(
        &router,
        "/api/v1/auth/login",
        json!({ "email": owner_address, "password": "a-fairly-strong-password" }),
    )
    .await;
    let owner_token = login_body["data"]["access_token"].as_str().unwrap().to_string();

    // No account exists yet for this email - the invite mails a token
    // rather than granting directly.
    let (status, invite_body) = authed_post(
        &router,
        "/api/v1/roles/invite",
        &owner_token,
        json!({ "email": invitee_address, "role_id": CASHIER_ROLE_ID, "store_id": null }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(invite_body["data"]["outcome"], "invited");

    let token = email.last_invite_token();

    let (status, _) = post(
        &router,
        &format!("/api/v1/auth/invites/{}/accept", token),
        json!({ "full_name": "Brand New Invitee", "phone": "+2340000000001", "password": "another-strong-password" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Pre-verified: logs in immediately, no separate OTP step.
    let (status, _) = post(
        &router,
        "/api/v1/auth/login",
        json!({ "email": invitee_address, "password": "another-strong-password" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

// === Rate limiting

#[sqlx::test]
async fn requests_beyond_the_budget_are_rejected_and_only_for_the_offending_ip(pool: PgPool) {
    let email = CapturingEmailProvider::default();
    let router = build_router_with_limiter(pool, email, RateLimiter::with_limits(3, std::time::Duration::from_secs(60)));

    let attempt = json!({ "email": "nobody@example.com", "password": "wrong" });

    for _ in 0..3 {
        let (status, _) = post(&router, "/api/v1/auth/login", attempt.clone()).await;
        // Wrong credentials against a nonexistent user - the request still
        // reaches the handler and is rejected on its own merits, just not
        // yet by the limiter.
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    let (status, _) = post(&router, "/api/v1/auth/login", attempt.clone()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    // A different caller IP is unaffected - this is a per-IP budget, not global.
    let mut other_ip_request = request_with_connect_info(
        Request::builder().method("POST").uri("/api/v1/auth/login"),
        attempt,
    );
    other_ip_request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([10, 0, 0, 1], 0))));
    let (status, _) = send(&router, other_ip_request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
