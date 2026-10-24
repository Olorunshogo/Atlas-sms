//! The online storefront end to end: catalogue, carts and wishlists, checkout and payments, pay on delivery,
//! promotions, loyalty, social sign-in, recommendations, shipping, and staff order handling.

mod common;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use atlas_backend::integrations::{mock_signature, Integrations};
use atlas_backend::shop::{COURIER_SIGNATURE_HEADER, PAYMENT_SIGNATURE_HEADER};
use atlas_backend::{api_router, AppState, EmailError, EmailProvider, RateLimiter};
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::Router;
use common::*;
use http_body_util::BodyExt;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const WEBHOOK_SECRET: &[u8] = b"test-webhook-secret";

// === Harness

#[derive(Clone, Default)]
struct Mailbox {
    sent: Arc<Mutex<Vec<(String, String, String)>>>,
}

#[async_trait::async_trait]
impl EmailProvider for Mailbox {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError> {
        self.sent.lock().unwrap().push((to.to_string(), subject.to_string(), body.to_string()));
        return Ok(());
    }
}

impl Mailbox {
    fn subjects(&self) -> Vec<String> {
        return self.sent.lock().unwrap().iter().map(|(_, s, _)| s.clone()).collect();
    }
}

fn router_with(pool: PgPool, mailbox: Mailbox) -> Router {
    return api_router(AppState {
        pool,
        email: Arc::new(mailbox),
        jwt_secret: Arc::new(JWT_SECRET.to_vec()),
        rate_limiter: RateLimiter::with_limits(1000, std::time::Duration::from_secs(60)),
        integrations: Integrations::mock(WEBHOOK_SECRET),
    });
}

/// A request to the shop API with an optional shopper token and guest cart token.
async fn shop(router: &Router, method: &str, path: &str, shopper: Option<&str>, cart: Option<&str>, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path).header("content-type", "application/json");
    if let Some(token) = shopper {
        builder = builder.header("authorization", format!("Bearer {}", token));
    }
    if let Some(token) = cart {
        builder = builder.header("x-cart-token", token);
    }
    let body = body.map(|b| Body::from(serde_json::to_vec(&b).unwrap())).unwrap_or_else(Body::empty);
    let mut request = builder.body(body).unwrap();
    request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap_or(Value::String(String::from_utf8_lossy(&bytes).to_string())) };
    return (status, json);
}

async fn webhook(router: &Router, path: &str, header: &str, body: Value, sign: bool) -> StatusCode {
    let bytes = serde_json::to_vec(&body).unwrap();
    let signature = if sign { mock_signature(WEBHOOK_SECRET, &bytes) } else { "nope".to_string() };
    let mut request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .header(header, signature)
        .body(Body::from(bytes))
        .unwrap();
    request.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    return router.clone().oneshot(request).await.unwrap().status();
}

async fn pay(router: &Router, gateway_ref: &str, status: &str) -> StatusCode {
    return webhook(router, "/api/v1/shop-webhooks/payments", PAYMENT_SIGNATURE_HEADER, json!({ "gateway_ref": gateway_ref, "status": status }), true).await;
}

fn qty(value: &Value) -> Decimal {
    return value.as_str().unwrap().parse().unwrap();
}

struct Shop {
    router: Router,
    pool: PgPool,
    mailbox: Mailbox,
    owner: String,
    tenant_id: Uuid,
    store: Uuid,
    unit: Uuid,
    rice: Uuid,
    beans: Uuid,
    zone: Uuid,
    base: String,
}

/// A store selling Rice (1,000) and Beans (500) online, 20 of each, with one delivery zone: 1,500 flat + 100 per kg.
async fn open_shop(pool: &PgPool, email: &str) -> Shop {
    let mailbox = Mailbox::default();
    let router = router_with(pool.clone(), mailbox.clone());
    let (owner, tenant_id) = signup_owner(&router, pool, email).await;
    let store = create_store(&router, &owner, "web-shop").await;
    let unit = seed_unit(pool, tenant_id, "Each", "ea").await;
    let rice = sellable(&router, &owner, store, unit, "Rice 5kg", "RICE-W", 1_000, "20").await;
    let beans = sellable(&router, &owner, store, unit, "Beans 1kg", "BEANS-W", 500, "20").await;
    patch(&router, &format!("/api/v1/products/{}", rice), &as_user(&owner), json!({ "attributes": { "weight_kg": 5 } })).await;
    let handle = format!("shop-{}", &Uuid::now_v7().simple().to_string()[24..]);
    let (status, body) = call(
        &router,
        "PUT",
        &format!("/api/v1/storefronts/{}", store),
        Some(&as_user(&owner)),
        Some(json!({ "handle": handle, "is_enabled": true, "pay_on_delivery_max_minor": 50_000, "loyalty_minor_per_point": 100 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let (status, body) = post(
        &router,
        &format!("/api/v1/storefronts/{}/shipping-zones", store),
        &as_user(&owner),
        json!({ "name": "Lagos", "regions": ["Lagos"], "base_fee_minor": 1_500, "per_kg_fee_minor": 100, "allows_pay_on_delivery": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let zone = Uuid::parse_str(&id_of(&body)).unwrap();
    return Shop { router, pool: pool.clone(), mailbox, owner, tenant_id, store, unit, rice, beans, zone, base: format!("/api/v1/shop/{}", handle) };
}

impl Shop {
    fn path(&self, rest: &str) -> String {
        return format!("{}{}", self.base, rest);
    }

    async fn add(&self, shopper: Option<&str>, cart: Option<&str>, product: Uuid, quantity: &str) -> Value {
        let (status, body) = shop(&self.router, "POST", &self.path("/cart/items"), shopper, cart, Some(json!({ "product_id": product, "quantity": quantity }))).await;
        assert_eq!(status, StatusCode::OK, "{}", body);
        return body;
    }

    async fn quote(&self, shopper: Option<&str>, cart: Option<&str>, extra: Value) -> Value {
        let mut request = json!({ "shipping_zone_id": self.zone });
        request.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let (status, body) = shop(&self.router, "POST", &self.path("/checkout/quote"), shopper, cart, Some(request)).await;
        assert_eq!(status, StatusCode::OK, "{}", body);
        return body["data"].clone();
    }

    async fn checkout(&self, shopper: Option<&str>, cart: Option<&str>, method: &str, expected: i64, extra: Value) -> (StatusCode, Value) {
        let mut request = json!({
            "idempotency_key": Uuid::now_v7().to_string(),
            "email": "guest@example.com",
            "phone": "+2348030000000",
            "full_name": "Guest Buyer",
            "delivery_address": { "line1": "4 Marina", "city": "Lagos", "state": "Lagos" },
            "shipping_zone_id": self.zone,
            "payment_method": method,
            "expected_total_minor": expected,
        });
        request.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        return shop(&self.router, "POST", &self.path("/checkout"), shopper, cart, Some(request)).await;
    }

    async fn shopper(&self, email: &str) -> String {
        let (status, body) = shop(
            &self.router,
            "POST",
            &self.path("/auth/signup"),
            None,
            None,
            Some(json!({ "email": email, "password": "long-enough-password", "full_name": "Ada Shopper", "phone": "08031234567" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{}", body);
        return body["data"]["token"].as_str().unwrap().to_string();
    }

    async fn available(&self, product: Uuid) -> Decimal {
        let (_, body) = shop(&self.router, "GET", &self.path(&format!("/products/{}", product)), None, None, None).await;
        return qty(&body["data"]["available"]);
    }

    async fn on_hand(&self, product: Uuid) -> Decimal {
        return on_hand(&self.router, &self.owner, self.store, product).await.parse().unwrap();
    }

    async fn promotion(&self, body: Value) -> String {
        let (status, created) = post(&self.router, "/api/v1/promotions", &as_user(&self.owner), body).await;
        assert_eq!(status, StatusCode::OK, "{}", created);
        return id_of(&created);
    }

    async fn staff(&self, order_id: &str, action: &str, body: Value) -> (StatusCode, Value) {
        return post(&self.router, &format!("/api/v1/shop-orders/{}/{}", order_id, action), &as_user(&self.owner), body).await;
    }
}

// === Catalogue

#[sqlx::test]
async fn the_catalogue_shows_priced_listed_products_and_hides_a_disabled_shop(pool: PgPool) {
    let s = open_shop(&pool, "catalog-owner@example.com").await;
    let unlisted = create_product(&s.router, &s.owner, s.unit, "Hidden Oil", "OIL-H").await;
    let _ = unlisted;

    let (status, body) = shop(&s.router, "GET", &s.base, None, None, None).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["shipping_zones"][0]["base_fee_minor"], 1_500);
    assert_eq!(body["data"]["loyalty_enabled"], true);

    let (_, body) = shop(&s.router, "GET", &s.path("/products?sort=price_asc"), None, None, None).await;
    let names: Vec<&str> = body["data"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Beans 1kg", "Rice 5kg"]);
    assert_eq!(body["data"][1]["price_minor"], 1_000);
    assert_eq!(qty(&body["data"][1]["available"]), "20".parse().unwrap());

    let (_, body) = shop(&s.router, "GET", &s.path("/products?q=rice"), None, None, None).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    let (status, _) = shop(&s.router, "GET", &s.path(&format!("/products/{}", Uuid::now_v7())), None, None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    call(&s.router, "PUT", &format!("/api/v1/storefronts/{}", s.store), Some(&as_user(&s.owner)), Some(json!({ "handle": s.base.rsplit('/').next().unwrap(), "is_enabled": false }))).await;
    let (status, _) = shop(&s.router, "GET", &s.path("/products"), None, None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// === Cart and wishlist

#[sqlx::test]
async fn a_guest_cart_flags_price_changes_and_merges_into_the_account_on_signup(pool: PgPool) {
    let s = open_shop(&pool, "cart-owner@example.com").await;
    let body = s.add(None, None, s.rice, "2").await;
    let token = body["data"]["cart_token"].as_str().unwrap().to_string();
    s.add(None, Some(&token), s.beans, "1").await;
    let (_, body) = shop(&s.router, "PATCH", &s.path(&format!("/cart/items/{}", s.beans)), None, Some(&token), Some(json!({ "quantity": "3" }))).await;
    assert_eq!(body["data"]["subtotal_minor"], 2 * 1_000 + 3 * 500);

    price_rule(&s.router, &s.owner, s.rice, Some(s.store), 1_100).await;
    let (_, body) = shop(&s.router, "GET", &s.path("/cart"), None, Some(&token), None).await;
    let rice = body["data"]["items"].as_array().unwrap().iter().find(|i| i["product_id"] == s.rice.to_string()).unwrap().clone();
    assert_eq!(rice["price_changed"], true);
    assert_eq!(rice["unit_price_minor"], 1_100);
    assert_eq!(rice["added_unit_price_minor"], 1_000);

    let (status, _) = shop(&s.router, "PUT", &s.path(&format!("/wishlist/{}", s.rice)), None, Some(&token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = shop(
        &s.router,
        "POST",
        &s.path("/auth/signup"),
        None,
        Some(&token),
        Some(json!({ "email": "merge@example.com", "password": "long-enough-password", "full_name": "Merge Me" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let shopper = body["data"]["token"].as_str().unwrap().to_string();
    let (_, body) = shop(&s.router, "GET", &s.path("/cart"), Some(&shopper), None, None).await;
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 2);
    let (_, body) = shop(&s.router, "GET", &s.path("/cart"), None, Some(&token), None).await;
    assert!(body["data"]["items"].as_array().unwrap().is_empty(), "the guest cart is gone");

    let (_, body) = shop(&s.router, "POST", &s.path(&format!("/cart/items/{}/move-to-wishlist", s.beans)), Some(&shopper), None, None).await;
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 1);
    let (_, body) = shop(&s.router, "GET", &s.path("/wishlist"), Some(&shopper), None, None).await;
    assert_eq!(body["data"][0]["name"], "Beans 1kg");
    let (_, body) = shop(&s.router, "POST", &s.path(&format!("/wishlist/{}/move-to-cart", s.beans)), Some(&shopper), None, None).await;
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 2);

    // The same login on another device sees the same cart
    let (_, body) = shop(&s.router, "POST", &s.path("/auth/login"), None, None, Some(json!({ "email": "merge@example.com", "password": "long-enough-password" }))).await;
    let second = body["data"]["token"].as_str().unwrap().to_string();
    let (_, body) = shop(&s.router, "GET", &s.path("/cart"), Some(&second), None, None).await;
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 2);
}

// === Checkout and payment

#[sqlx::test]
async fn a_guest_checkout_reserves_stock_and_only_the_signed_webhook_marks_it_paid(pool: PgPool) {
    let s = open_shop(&pool, "checkout-owner@example.com").await;
    let token = s.add(None, None, s.rice, "2").await["data"]["cart_token"].as_str().unwrap().to_string();

    let quote = s.quote(None, Some(&token), json!({})).await;
    assert_eq!(quote["subtotal_minor"], 2_000);
    // 1,500 + 10kg x 100
    assert_eq!(quote["shipping_minor"], 2_500);
    assert_eq!(quote["total_minor"], 4_500);

    let (status, body) = s.checkout(None, Some(&token), "card", 4_000, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "TOTAL_CHANGED");
    assert_eq!(body["data"]["quote"]["total_minor"], 4_500);

    let key = Uuid::now_v7().to_string();
    let (status, body) = s.checkout(None, Some(&token), "card", 4_500, json!({ "idempotency_key": key })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let order_id = body["data"]["order"]["id"].as_str().unwrap().to_string();
    let access = body["data"]["access_token"].as_str().unwrap().to_string();
    let gateway_ref = body["data"]["order"]["payments"][0]["gateway_ref"].as_str().unwrap().to_string();
    assert_eq!(body["data"]["order"]["status"], "pending_payment");
    assert!(body["data"]["payment"]["redirect_url"].as_str().unwrap().contains(&gateway_ref));
    assert!(body["data"]["order"]["invoice_number"].as_str().unwrap().starts_with("INV-"));
    assert!(s.mailbox.subjects().iter().any(|m| m.starts_with("Your invoice")));

    // Reserved, not yet sold
    assert_eq!(s.available(s.rice).await, "18".parse().unwrap());
    assert_eq!(s.on_hand(s.rice).await, "20".parse().unwrap());

    let (_, again) = s.checkout(None, Some(&token), "card", 4_500, json!({ "idempotency_key": key })).await;
    assert_eq!(again["data"]["order"]["id"], order_id);

    assert_eq!(webhook(&s.router, "/api/v1/shop-webhooks/payments", PAYMENT_SIGNATURE_HEADER, json!({ "gateway_ref": gateway_ref, "status": "succeeded" }), false).await, StatusCode::UNAUTHORIZED);
    let (_, body) = shop(&s.router, "GET", &s.path(&format!("/orders/{}?access_token={}", order_id, access)), None, None, None).await;
    assert_eq!(body["data"]["status"], "pending_payment");

    assert_eq!(pay(&s.router, &gateway_ref, "succeeded").await, StatusCode::OK);
    assert_eq!(pay(&s.router, &gateway_ref, "succeeded").await, StatusCode::OK, "a repeat is harmless");
    let (_, body) = shop(&s.router, "GET", &s.path(&format!("/orders/{}?access_token={}", order_id, access)), None, None, None).await;
    assert_eq!(body["data"]["status"], "paid");
    assert!(body["data"]["receipt_number"].as_str().is_some());
    assert_eq!(s.on_hand(s.rice).await, "18".parse().unwrap());
    assert_eq!(s.available(s.rice).await, "18".parse().unwrap());

    let (status, _) = shop(&s.router, "GET", &s.path(&format!("/orders/{}", order_id)), None, None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "a guest order needs its link token");
    let (status, text) = shop(&s.router, "GET", &s.path(&format!("/orders/{}/invoice?access_token={}&format=text", order_id, access)), None, None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(text.as_str().unwrap().contains("RECEIPT"));
    assert!(s.mailbox.subjects().iter().any(|m| m.starts_with("Payment received")));
}

#[sqlx::test]
async fn a_failed_payment_can_be_retried_and_an_unpaid_order_expires_and_frees_its_stock(pool: PgPool) {
    let s = open_shop(&pool, "retry-owner@example.com").await;
    let token = s.add(None, None, s.beans, "4").await["data"]["cart_token"].as_str().unwrap().to_string();
    let total = s.quote(None, Some(&token), json!({})).await["total_minor"].as_i64().unwrap();
    let (_, body) = s.checkout(None, Some(&token), "card", total, json!({})).await;
    let order_id = body["data"]["order"]["id"].as_str().unwrap().to_string();
    let access = body["data"]["access_token"].as_str().unwrap().to_string();
    let first_ref = body["data"]["order"]["payments"][0]["gateway_ref"].as_str().unwrap().to_string();

    assert_eq!(pay(&s.router, &first_ref, "failed").await, StatusCode::OK);
    let (status, body) = shop(
        &s.router,
        "POST",
        &s.path(&format!("/orders/{}/payments", order_id)),
        None,
        None,
        Some(json!({ "method": "bank_transfer", "idempotency_key": "retry-1", "access_token": access })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert!(body["data"]["instructions"].as_str().unwrap().contains("Mock Bank"));

    sqlx::query("UPDATE atlas.shop_orders SET payment_expires_at = now() - interval '1 minute' WHERE id = $1::uuid")
        .bind(&order_id)
        .execute(&s.pool)
        .await
        .unwrap();
    let (_, body) = shop(&s.router, "GET", &s.path(&format!("/orders/{}?access_token={}", order_id, access)), None, None, None).await;
    assert_eq!(body["data"]["status"], "expired");
    let statuses: Vec<&str> = body["data"]["events"].as_array().unwrap().iter().map(|e| e["status"].as_str().unwrap()).collect();
    assert_eq!(statuses, vec!["pending_payment", "expired"]);
    assert_eq!(s.available(s.beans).await, "20".parse().unwrap());
}

#[sqlx::test]
async fn pay_on_delivery_is_limited_then_ships_delivers_and_is_collected(pool: PgPool) {
    let s = open_shop(&pool, "pod-owner@example.com").await;
    let guest = s.add(None, None, s.beans, "1").await["data"]["cart_token"].as_str().unwrap().to_string();
    let total = s.quote(None, Some(&guest), json!({})).await["total_minor"].as_i64().unwrap();
    let (status, body) = s.checkout(None, Some(&guest), "pay_on_delivery", total, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "PAY_ON_DELIVERY_UNAVAILABLE");

    let shopper = s.shopper("pod@example.com").await;
    s.add(Some(&shopper), None, s.beans, "2").await;
    let total = s.quote(Some(&shopper), None, json!({})).await["total_minor"].as_i64().unwrap();
    let (status, body) = s.checkout(Some(&shopper), None, "pay_on_delivery", total, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let order_id = body["data"]["order"]["id"].as_str().unwrap().to_string();
    assert!(body["data"].get("access_token").is_none());

    let (status, body) = s.staff(&order_id, "ship", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let tracking_number = body["data"]["shipment"]["tracking_number"].as_str().unwrap().to_string();
    assert!(body["data"]["shipment"]["label_url"].as_str().unwrap().ends_with(".pdf"));
    assert!(s.mailbox.subjects().iter().any(|m| m.starts_with("Your order has shipped")));

    let courier = |status: &str| json!({ "tracking_number": tracking_number, "status": status, "note": "Lekki hub" });
    assert_eq!(webhook(&s.router, "/api/v1/shop-webhooks/couriers", COURIER_SIGNATURE_HEADER, courier("in_transit"), false).await, StatusCode::UNAUTHORIZED);
    assert_eq!(webhook(&s.router, "/api/v1/shop-webhooks/couriers", COURIER_SIGNATURE_HEADER, courier("in_transit"), true).await, StatusCode::OK);
    assert_eq!(webhook(&s.router, "/api/v1/shop-webhooks/couriers", COURIER_SIGNATURE_HEADER, courier("delivered"), true).await, StatusCode::OK);
    let (_, track) = shop(&s.router, "GET", &s.path(&format!("/orders/{}/tracking", order_id)), Some(&shopper), None, None).await;
    assert_eq!(track["data"]["order_status"], "delivered");
    let events: Vec<&str> = track["data"]["events"].as_array().unwrap().iter().map(|e| e["status"].as_str().unwrap()).collect();
    assert_eq!(events, vec!["created", "in_transit", "delivered"]);
    assert_eq!(s.on_hand(s.beans).await, "20".parse().unwrap(), "stock is held, not sold, until collected");

    let (status, body) = s.staff(&order_id, "collect-payment", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "delivered");
    assert!(body["data"]["paid_at"].is_string());
    assert_eq!(s.on_hand(s.beans).await, "18".parse().unwrap());

    // 2 x 500 earns 10 points, usable after the return window
    let (_, account) = shop(&s.router, "GET", &s.path("/account"), Some(&shopper), None, None).await;
    assert_eq!(account["data"]["pending_loyalty_points"], 10);
    assert_eq!(account["data"]["loyalty_points"], 0);
}

// === Promotions

#[sqlx::test]
async fn coupons_stack_only_when_both_sides_allow_and_the_best_deal_wins(pool: PgPool) {
    let s = open_shop(&pool, "promo-owner@example.com").await;
    let token = s.add(None, None, s.rice, "4").await["data"]["cart_token"].as_str().unwrap().to_string();
    let store = s.store;

    s.promotion(json!({ "store_id": store, "name": "10% off over 3,000", "kind": "automatic", "discount": "percentage", "discount_value": 1000, "min_spend_minor": 3000 })).await;
    s.promotion(json!({ "store_id": store, "name": "SAVE500", "kind": "coupon", "coupon_code": "save500", "discount": "fixed_amount", "discount_value": 500 })).await;

    // Not stackable: the better one alone (10% of 4,000 = 400 vs the 500 coupon)
    let quote = s.quote(None, Some(&token), json!({ "coupon_code": "SAVE500" })).await;
    assert_eq!(quote["discount_minor"], 500);
    assert_eq!(quote["promotions"].as_array().unwrap().len(), 1);
    assert!(quote["coupon_error"].is_null());

    let quote = s.quote(None, Some(&token), json!({ "coupon_code": "NOPE" })).await;
    assert!(quote["coupon_error"].is_string());
    assert_eq!(quote["discount_minor"], 400);
    let total = quote["total_minor"].as_i64().unwrap();
    let (status, body) = s.checkout(None, Some(&token), "card", total, json!({ "coupon_code": "NOPE" })).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "COUPON_NOT_APPLICABLE");

    // Both stackable: both apply
    s.promotion(json!({ "store_id": store, "name": "Stack 5%", "kind": "automatic", "discount": "percentage", "discount_value": 500, "stackable": true })).await;
    s.promotion(json!({ "store_id": store, "name": "STACK300", "kind": "coupon", "coupon_code": "STACK300", "discount": "fixed_amount", "discount_value": 300, "stackable": true })).await;
    let quote = s.quote(None, Some(&token), json!({ "coupon_code": "STACK300" })).await;
    assert_eq!(quote["discount_minor"], 200 + 300);

    let free = s.promotion(json!({ "store_id": store, "name": "Free delivery", "kind": "coupon", "coupon_code": "SHIPFREE", "discount": "free_shipping", "per_customer_limit": 1 })).await;
    let quote = s.quote(None, Some(&token), json!({ "coupon_code": "SHIPFREE", "email": "promo-guest@example.com" })).await;
    assert_eq!(quote["shipping_minor"], 0);
    let (status, body) = s.checkout(None, Some(&token), "card", quote["total_minor"].as_i64().unwrap(), json!({ "coupon_code": "SHIPFREE", "email": "promo-guest@example.com" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let (_, detail) = get(&s.router, &format!("/api/v1/promotions/{}", free), &as_user(&s.owner)).await;
    assert_eq!(detail["data"]["used_count"], 1);

    // The per-customer limit is spent for that email
    let token = s.add(None, None, s.rice, "1").await["data"]["cart_token"].as_str().unwrap().to_string();
    let quote = s.quote(None, Some(&token), json!({ "coupon_code": "SHIPFREE", "email": "promo-guest@example.com" })).await;
    assert!(quote["coupon_error"].is_string());
}

#[sqlx::test]
async fn a_flash_sale_stops_at_its_cap_and_a_bundle_needs_every_item(pool: PgPool) {
    let s = open_shop(&pool, "flash-owner@example.com").await;
    s.promotion(json!({ "store_id": s.store, "name": "Rice flash", "kind": "flash_sale", "discount": "fixed_amount", "flash_price_minor": 700, "flash_stock_cap": "3", "product_ids": [s.rice] })).await;
    s.promotion(json!({ "store_id": s.store, "name": "Rice and beans", "kind": "bundle", "discount": "fixed_amount", "discount_value": 250, "bundle_product_ids": [s.rice, s.beans], "stackable": true })).await;

    let (_, listing) = shop(&s.router, "GET", &s.path(&format!("/products/{}", s.rice)), None, None, None).await;
    assert_eq!(listing["data"]["sale_price_minor"], 700);

    // 5 rice: 3 at the flash price (saves 900); flash lines don't take the bundle and beans aren't there anyway
    let token = s.add(None, None, s.rice, "5").await["data"]["cart_token"].as_str().unwrap().to_string();
    let quote = s.quote(None, Some(&token), json!({})).await;
    assert_eq!(quote["discount_minor"], 900);
    let (status, body) = s.checkout(None, Some(&token), "card", quote["total_minor"].as_i64().unwrap(), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);

    // The cap is used up: no more flash price; with beans in the cart the bundle now applies to the beans
    let token = s.add(None, None, s.rice, "1").await["data"]["cart_token"].as_str().unwrap().to_string();
    s.add(None, Some(&token), s.beans, "1").await;
    let quote = s.quote(None, Some(&token), json!({})).await;
    let kinds: Vec<&str> = quote["promotions"].as_array().unwrap().iter().map(|p| p["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, vec!["bundle"]);
    assert_eq!(quote["discount_minor"], 250);
}

// === Loyalty

#[sqlx::test]
async fn points_redeem_up_to_the_cap_and_cancelling_gives_them_back(pool: PgPool) {
    let s = open_shop(&pool, "loyalty-owner@example.com").await;
    let shopper = s.shopper("loyal@example.com").await;
    let (_, account) = shop(&s.router, "GET", &s.path("/account"), Some(&shopper), None, None).await;
    let shopper_id = account["data"]["id"].as_str().unwrap().to_string();
    sqlx::query("INSERT INTO atlas.loyalty_entries (id, tenant_id, shopper_id, kind, points, available_at) VALUES ($1, $2, $3::uuid, 'earn', 50, now())")
        .bind(Uuid::now_v7())
        .bind(s.tenant_id)
        .bind(&shopper_id)
        .execute(&s.pool)
        .await
        .unwrap();

    s.add(Some(&shopper), None, s.rice, "2").await;
    // Cap: 20% of 2,000 = 400, at 100 a point = 4 points
    let (status, body) = shop(&s.router, "POST", &s.path("/checkout/quote"), Some(&shopper), None, Some(json!({ "shipping_zone_id": s.zone, "points": 5 }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{}", body);
    assert_eq!(body["code"], "LOYALTY_REDEMPTION_INVALID");
    let quote = s.quote(Some(&shopper), None, json!({ "points": 4 })).await;
    assert_eq!(quote["points_value_minor"], 400);
    assert_eq!(quote["total_minor"], 2_000 - 400 + 2_500);

    let (status, body) = s.checkout(Some(&shopper), None, "card", quote["total_minor"].as_i64().unwrap(), json!({ "points": 4 })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let order_id = body["data"]["order"]["id"].as_str().unwrap().to_string();
    let (_, account) = shop(&s.router, "GET", &s.path("/account"), Some(&shopper), None, None).await;
    assert_eq!(account["data"]["loyalty_points"], 46);

    let (status, body) = s.staff(&order_id, "cancel", json!({ "reason": "Customer asked" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "cancelled");
    let (_, account) = shop(&s.router, "GET", &s.path("/account"), Some(&shopper), None, None).await;
    assert_eq!(account["data"]["loyalty_points"], 50);
    assert_eq!(s.available(s.rice).await, "20".parse().unwrap());
}

// === Social sign-in and token separation

#[sqlx::test]
async fn social_sign_in_links_by_email_and_shopper_and_staff_tokens_never_cross(pool: PgPool) {
    let s = open_shop(&pool, "social-owner@example.com").await;
    let password_token = s.shopper("sam@example.com").await;

    let (status, body) = shop(&s.router, "POST", &s.path("/auth/social"), None, None, Some(json!({ "provider": "google", "token": "mock:g-123:sam@example.com:Sam" }))).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let social = body["data"]["token"].as_str().unwrap().to_string();
    let (_, a) = shop(&s.router, "GET", &s.path("/account"), Some(&password_token), None, None).await;
    let (_, b) = shop(&s.router, "GET", &s.path("/account"), Some(&social), None, None).await;
    assert_eq!(a["data"]["id"], b["data"]["id"], "linked to the same account");

    let (status, body) = shop(&s.router, "POST", &s.path("/auth/social"), None, None, Some(json!({ "provider": "facebook", "token": "mock:f-9:new@example.com:New Person" }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["shopper"]["email"], "new@example.com");
    let (status, _) = shop(&s.router, "POST", &s.path("/auth/social"), None, None, Some(json!({ "provider": "google", "token": "forged" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = get(&s.router, "/api/v1/stores", &as_user(&social)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a shopper token is not a staff token");
    let (status, _) = shop(&s.router, "GET", &s.path("/account"), Some(&s.owner), None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "a staff token is not a shopper token");
}

// === Recommendations

#[sqlx::test]
async fn recommendations_use_shared_baskets_categories_and_the_viewers_own_trail(pool: PgPool) {
    let s = open_shop(&pool, "reco-owner@example.com").await;
    let (_, category) = post(&s.router, "/api/v1/product-categories", &as_user(&s.owner), json!({ "name": "Grains" })).await;
    let category_id = id_of(&category);
    let millet = sellable(&s.router, &s.owner, s.store, s.unit, "Millet", "MILLET-W", 800, "10").await;
    for product in [s.rice, millet] {
        patch(&s.router, &format!("/api/v1/products/{}", product), &as_user(&s.owner), json!({ "category_id": category_id })).await;
    }

    // Rice and beans bought together once at the till
    let shift = open_shift(&s.router, &s.owner, s.store).await;
    let cash = method_id(&s.router, &s.owner, "cash").await;
    let (status, body) = post(
        &s.router,
        "/api/v1/sales",
        &as_user(&s.owner),
        sale_request(s.store, shift, vec![cart_line(s.rice, s.unit, "1"), cart_line(s.beans, s.unit, "1")], vec![common::pay(cash, 1_500)]),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);

    let token = s.add(None, None, s.beans, "1").await["data"]["cart_token"].as_str().unwrap().to_string();
    shop(&s.router, "GET", &s.path(&format!("/products/{}", millet)), None, Some(&token), None).await;
    let (status, body) = shop(&s.router, "GET", &s.path(&format!("/products/{}/recommendations", s.rice)), None, Some(&token), None).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let ids = |key: &str| -> Vec<String> { body["data"][key].as_array().unwrap().iter().map(|p| p["id"].as_str().unwrap().to_string()).collect() };
    assert_eq!(ids("related"), vec![millet.to_string()]);
    assert_eq!(ids("frequently_bought_together"), vec![s.beans.to_string()]);
    assert_eq!(ids("recently_viewed"), vec![millet.to_string()]);
}

// === Staff order handling

#[sqlx::test]
async fn staff_move_a_paid_order_through_and_a_cancelled_paid_order_is_refunded_and_restocked(pool: PgPool) {
    let s = open_shop(&pool, "staff-orders@example.com").await;
    let place = |quantity: &'static str| {
        let s = &s;
        async move {
            let token = s.add(None, None, s.beans, quantity).await["data"]["cart_token"].as_str().unwrap().to_string();
            let total = s.quote(None, Some(&token), json!({})).await["total_minor"].as_i64().unwrap();
            let (_, body) = s.checkout(None, Some(&token), "card", total, json!({})).await;
            let order_id = body["data"]["order"]["id"].as_str().unwrap().to_string();
            let gateway_ref = body["data"]["order"]["payments"][0]["gateway_ref"].as_str().unwrap().to_string();
            assert_eq!(pay(&s.router, &gateway_ref, "succeeded").await, StatusCode::OK);
            order_id
        }
    };

    let first = place("3").await;
    let (_, cashier) = seed_staff(&s.pool, s.tenant_id, s.store, "cashier").await;
    let (status, _) = get(&s.router, &format!("/api/v1/shop-orders?store_id={}", s.store), &at_store(&cashier, &s.store.to_string())).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = s.staff(&first, "process", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    s.staff(&first, "ship", json!({})).await;
    let (status, body) = s.staff(&first, "deliver", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let statuses: Vec<&str> = body["data"]["events"].as_array().unwrap().iter().map(|e| e["status"].as_str().unwrap()).collect();
    assert_eq!(statuses, vec!["pending_payment", "paid", "processing", "shipped", "delivered"]);
    let (status, _) = s.staff(&first, "cancel", json!({ "reason": "Too late" })).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let second = place("2").await;
    assert_eq!(s.on_hand(s.beans).await, "15".parse().unwrap());
    let (status, body) = s.staff(&second, "cancel", json!({ "reason": "Out of delivery range" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["payments"][0]["status"], "refunded");
    assert_eq!(s.on_hand(s.beans).await, "17".parse().unwrap());

    let (_, list) = get(&s.router, &format!("/api/v1/shop-orders?store_id={}&status=cancelled", s.store), &as_user(&s.owner)).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 1);
}
