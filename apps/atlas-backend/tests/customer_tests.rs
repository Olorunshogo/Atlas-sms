//! FR-19 customer records: create, view, edit, search, privacy masking, duplicate protection and erasure.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test]
async fn staff_create_search_edit_and_erase_customers(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "customers-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, store, "cashier").await;
    let store_id = store.to_string();
    let at_till = at_store(&cashier, &store_id);

    let (status, body) = post(
        &router,
        "/api/v1/customers",
        &at_till,
        json!({ "full_name": "Ada Obi", "phone": "0803 123 4567", "email": "ada@example.com", "address": "12 Allen Ave", "marketing_consent": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let customer_id = id_of(&body);

    // Same number in another format is the same person.
    let (status, body) = post(&router, "/api/v1/customers", &at_till, json!({ "full_name": "Ada O.", "phone": "+234 803 123 4567" })).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "CUSTOMER_PHONE_EXISTS");

    // Search by normalised phone; a cashier sees masked contact details.
    let (status, body) = get(&router, "/api/v1/customers?q=08031234567", &at_till).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let found = &body["data"]["items"][0];
    assert_eq!(found["id"], customer_id);
    assert_eq!(found["phone"], "0803******567");
    assert_eq!(found["email"], "a***@example.com");
    assert_eq!(found["contact_masked"], true);

    // The owner (customer.history) sees everything.
    let (status, body) = get(&router, &format!("/api/v1/customers/{}", customer_id), &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["phone"], "0803 123 4567");
    assert_eq!(body["data"]["address"], "12 Allen Ave");

    let (status, body) = patch(
        &router,
        &format!("/api/v1/customers/{}", customer_id),
        &at_till,
        json!({ "email": null, "notes": "Prefers transfer" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["email"], serde_json::Value::Null);
    assert_eq!(body["data"]["full_name"], "Ada Obi");

    let (status, _) = delete(&router, &format!("/api/v1/customers/{}", customer_id), &at_till).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = get(&router, &format!("/api/v1/customers/{}", customer_id), &as_user(&owner)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let erased: (String, Option<String>) = sqlx::query_as("SELECT full_name, phone FROM atlas.customers WHERE id = $1::uuid")
        .bind(&customer_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(erased, ("Deleted customer".to_string(), None));
}

#[sqlx::test]
async fn purchase_history_needs_customer_history(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "history-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, store, "cashier").await;
    let (status, body) = post(&router, "/api/v1/customers", &as_user(&owner), json!({ "full_name": "Bola" })).await;
    assert_eq!(status, StatusCode::OK);
    let customer_id = id_of(&body);

    let store_id = store.to_string();
    let (status, _) = get(&router, &format!("/api/v1/customers/{}/sales", customer_id), &at_store(&cashier, &store_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = get(&router, &format!("/api/v1/customers/{}/sales", customer_id), &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn fr19_history_for_manager_and_walk_in_sale_for_cashier(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "fr19-owner@example.com").await;
    let lagos = create_store(&router, &owner, "lagos").await;
    let abuja = create_store(&router, &owner, "abuja").await;
    let each = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = sellable(&router, &owner, lagos, each, "Soap", "SOAP", 500, "100").await;
    list_at(&router, &owner, soap, abuja).await;
    stock_up(&router, &owner, abuja, soap, "100").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, lagos, "cashier").await;
    let (_, manager) = seed_staff(&pool, tenant_id, lagos, "store_manager").await;
    let cash = method_id(&router, &owner, "cash").await;
    let lagos_shift = open_shift(&router, &cashier, lagos).await;
    let abuja_shift = open_shift(&router, &owner, abuja).await;

    // A walk-in sale: no customer profile needed.
    let walk_in = sale_request(lagos, lagos_shift, vec![cart_line(soap, each, "1")], vec![pay(cash, 500)]);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&cashier), walk_in).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["customer_id"], serde_json::Value::Null);

    let lagos_id = lagos.to_string();
    let (status, body) = post(&router, "/api/v1/customers", &at_store(&cashier, &lagos_id), json!({ "full_name": "Chidi" })).await;
    assert_eq!(status, StatusCode::OK);
    let customer_id = id_of(&body);

    for quantity in ["1", "2"] {
        let mut request = sale_request(lagos, lagos_shift, vec![cart_line(soap, each, quantity)], vec![]);
        request["customer_id"] = json!(customer_id);
        let amount = if quantity == "1" { 500 } else { 1_000 };
        request["payments"] = json!([pay(cash, amount)]);
        let (status, body) = post(&router, "/api/v1/sales", &as_user(&cashier), request).await;
        assert_eq!(status, StatusCode::OK, "{}", body);
    }
    // A sale at a store the Lagos manager has no grant at stays out of their view.
    let mut abuja_sale = sale_request(abuja, abuja_shift, vec![cart_line(soap, each, "1")], vec![pay(cash, 500)]);
    abuja_sale["customer_id"] = json!(customer_id);
    let (status, body) = post(&router, "/api/v1/sales", &as_user(&owner), abuja_sale).await;
    assert_eq!(status, StatusCode::OK, "{}", body);

    let path = format!("/api/v1/customers/{}/sales", customer_id);
    let (status, body) = get(&router, &path, &at_store(&manager, &lagos_id)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let history = body["data"]["items"].as_array().unwrap();
    assert_eq!(history.len(), 2);
    assert!(history.iter().all(|entry| entry["store_id"] == lagos_id));
    assert_eq!(history[0]["total_amount_minor"], 1_000);

    let (status, body) = get(&router, &path, &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 3);

    let (status, _) = get(&router, &path, &at_store(&cashier, &lagos_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
