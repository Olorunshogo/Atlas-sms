//! Catalogue and store administration: units, product reads and edits, sale units, price rules, store edits, customer groups, and the tenant-ownership and validation rules around them.

mod common;

use axum::http::StatusCode;
use common::*;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn code(body: &Value) -> &str {
    return body["code"].as_str().unwrap_or_default();
}

fn qty(value: &str) -> Decimal {
    return value.parse().unwrap();
}

async fn api_unit(router: &axum::Router, token: &str, name: &str) -> Uuid {
    let (status, body) = post(router, "/api/v1/units", &as_user(token), json!({ "name": name, "abbreviation": "u" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return Uuid::parse_str(&id_of(&body)).unwrap();
}

async fn product_type_of(pool: &PgPool, product_id: Uuid) -> Uuid {
    return sqlx::query_scalar("SELECT product_type_id FROM atlas.products WHERE id = $1")
        .bind(product_id)
        .fetch_one(pool)
        .await
        .unwrap();
}

// === Units

#[sqlx::test]
async fn units_can_be_created_listed_and_validated(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "units-owner@example.com").await;

    let (status, unit) = post(&router, "/api/v1/units", &as_user(&owner), json!({ "name": "  Kilogram ", "abbreviation": "kg" })).await;
    assert_eq!(status, StatusCode::OK, "{}", unit);
    assert_eq!(unit["data"]["name"], "Kilogram");
    api_unit(&router, &owner, "Carton").await;

    let (_, list) = get(&router, "/api/v1/units", &as_user(&owner)).await;
    let names: Vec<&str> = list["data"].as_array().unwrap().iter().map(|u| u["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Carton", "Kilogram"]);

    for body in [
        json!({ "name": "", "abbreviation": "x" }),
        json!({ "name": "Litre", "abbreviation": "   " }),
        json!({ "name": "Litre", "abbreviation": "much-too-long" }),
        json!({ "name": "x".repeat(51), "abbreviation": "x" }),
    ] {
        let (status, response) = post(&router, "/api/v1/units", &as_user(&owner), body.clone()).await;
        assert_eq!((status, code(&response)), (StatusCode::BAD_REQUEST, "VALIDATION_FAILED"), "{}", body);
    }
    let (status, response) = post(&router, "/api/v1/units", &as_user(&owner), json!({ "name": "Kilogram", "abbreviation": "k" })).await;
    assert_eq!((status, code(&response)), (StatusCode::CONFLICT, "ALREADY_EXISTS"));

    // Another business's units are invisible, and the same name is fine there.
    let (other, _) = signup_owner(&router, &pool, "units-other@example.com").await;
    let (_, theirs) = get(&router, "/api/v1/units", &as_user(&other)).await;
    assert!(theirs["data"].as_array().unwrap().is_empty());
    api_unit(&router, &other, "Kilogram").await;
}

#[sqlx::test]
async fn catalogue_writes_need_product_manage(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "perm-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let product = create_product(&router, &owner, unit, "Soap", "SOAP").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, store, "cashier").await;
    let store_id = store.to_string();
    let caller = at_store(&cashier, &store_id);

    assert_eq!(post(&router, "/api/v1/units", &caller, json!({ "name": "x", "abbreviation": "x" })).await.0, StatusCode::FORBIDDEN);
    assert_eq!(post(&router, "/api/v1/customer-groups", &caller, json!({ "name": "VIP" })).await.0, StatusCode::FORBIDDEN);
    assert_eq!(get(&router, &format!("/api/v1/products/{}/price-rules", product), &caller).await.0, StatusCode::FORBIDDEN);
    assert_eq!(
        delete(&router, &format!("/api/v1/products/{}/price-rules/{}", product, Uuid::now_v7()), &caller).await.0,
        StatusCode::FORBIDDEN
    );
    // Reading the catalogue stays open to staff.
    assert_eq!(get(&router, "/api/v1/units", &caller).await.0, StatusCode::OK);
    assert_eq!(get(&router, &format!("/api/v1/products/{}", product), &caller).await.0, StatusCode::OK);
    assert_eq!(get(&router, &format!("/api/v1/products/{}/sale-units", product), &caller).await.0, StatusCode::OK);
    assert_eq!(get(&router, "/api/v1/customer-groups", &caller).await.0, StatusCode::OK);
}

// === Products

#[sqlx::test]
async fn products_validate_input_and_refuse_other_tenants_references(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "prod-owner@example.com").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let rice = create_product(&router, &owner, unit, "Rice", "RICE-1").await;
    let product_type = product_type_of(&pool, rice).await;
    let base = json!({ "product_type_id": product_type, "base_unit_id": unit, "name": "Thing" });
    let with = |key: &str, value: Value| {
        let mut body = base.clone();
        body[key] = value;
        return body;
    };

    for (body, expected) in [
        (with("name", json!("  ")), StatusCode::BAD_REQUEST),
        (with("name", json!("x".repeat(201))), StatusCode::BAD_REQUEST),
        (with("sku", json!("  ")), StatusCode::BAD_REQUEST),
        (with("attributes", json!(["not", "an", "object"])), StatusCode::BAD_REQUEST),
        (with("product_type_id", json!(Uuid::now_v7())), StatusCode::NOT_FOUND),
        (with("base_unit_id", json!(Uuid::now_v7())), StatusCode::NOT_FOUND),
        (with("sku", json!("RICE-1")), StatusCode::CONFLICT),
    ] {
        let (status, response) = post(&router, "/api/v1/products", &as_user(&owner), body.clone()).await;
        assert_eq!(status, expected, "{} -> {}", body, response);
    }

    // Another business's product type and unit look nonexistent, not accepted.
    let (other, other_tenant) = signup_owner(&router, &pool, "prod-other@example.com").await;
    let other_unit = seed_unit(&pool, other_tenant, "Each", "ea").await;
    let other_product = create_product(&router, &other, other_unit, "Beans", "B-1").await;
    let other_type = product_type_of(&pool, other_product).await;
    assert_eq!(post(&router, "/api/v1/products", &as_user(&owner), with("product_type_id", json!(other_type))).await.0, StatusCode::NOT_FOUND);
    assert_eq!(post(&router, "/api/v1/products", &as_user(&owner), with("base_unit_id", json!(other_unit))).await.0, StatusCode::NOT_FOUND);

    // Products with no SKU do not collide with each other.
    for name in ["A", "B"] {
        let (status, _) = post(&router, "/api/v1/products", &as_user(&owner), with("name", json!(name))).await;
        assert_eq!(status, StatusCode::OK);
    }
    let (_, list) = get(&router, "/api/v1/products", &as_user(&owner)).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 3, "rejected requests created nothing");
}

#[sqlx::test]
async fn product_types_validate_input(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "type-owner@example.com").await;
    for body in [
        json!({ "name": "" }),
        json!({ "name": "Ok", "identity_mode": "by_magic" }),
        json!({ "name": "Ok", "attribute_schema": { "not": "an array" } }),
    ] {
        let (status, response) = post(&router, "/api/v1/product-types", &as_user(&owner), body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{} -> {}", body, response);
    }
    for mode in ["by_product", "by_batch", "by_serial", "by_both"] {
        let (status, _) = post(&router, "/api/v1/product-types", &as_user(&owner), json!({ "name": mode, "identity_mode": mode })).await;
        assert_eq!(status, StatusCode::OK, "{}", mode);
    }
}

#[sqlx::test]
async fn a_product_can_be_read_and_edited(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "edit-owner@example.com").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let rice = create_product(&router, &owner, unit, "Rice", "RICE-1").await;
    let beans = create_product(&router, &owner, unit, "Beans", "BEANS-1").await;
    let uri = format!("/api/v1/products/{}", rice);
    let (_, original) = get(&router, &uri, &as_user(&owner)).await;

    let (status, body) = patch(
        &router,
        &uri,
        &as_user(&owner),
        json!({ "name": "  Premium Rice ", "sku": "RICE-PREM", "attributes": { "grade": "A" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["name"], "Premium Rice");
    assert_eq!(body["data"]["sku"], "RICE-PREM");
    assert_eq!(body["data"]["attributes"], json!({ "grade": "A" }));
    // What a product fundamentally is does not move.
    assert_eq!(body["data"]["base_unit_id"], original["data"]["base_unit_id"]);
    assert_eq!(body["data"]["product_type_id"], original["data"]["product_type_id"]);

    // Absent leaves the SKU alone, null clears it, a value sets it again.
    let (_, body) = patch(&router, &uri, &as_user(&owner), json!({ "name": "Rice" })).await;
    assert_eq!(body["data"]["sku"], "RICE-PREM");
    let (_, body) = patch(&router, &uri, &as_user(&owner), json!({ "sku": null })).await;
    assert!(body["data"]["sku"].is_null());
    let (_, body) = patch(&router, &uri, &as_user(&owner), json!({ "sku": "RICE-2" })).await;
    assert_eq!(body["data"]["sku"], "RICE-2");

    // Bad edits are rejected and apply nothing, even next to a valid field.
    for (body, expected) in [
        (json!({}), StatusCode::BAD_REQUEST),
        (json!({ "name": "  " }), StatusCode::BAD_REQUEST),
        (json!({ "sku": "   " }), StatusCode::BAD_REQUEST),
        (json!({ "attributes": ["no"] }), StatusCode::BAD_REQUEST),
        (json!({ "sku": "BEANS-1" }), StatusCode::CONFLICT),
        (json!({ "name": "Half Applied", "sku": "BEANS-1" }), StatusCode::CONFLICT),
    ] {
        let (status, response) = patch(&router, &uri, &as_user(&owner), body.clone()).await;
        assert_eq!(status, expected, "{} -> {}", body, response);
    }
    let (_, after) = get(&router, &uri, &as_user(&owner)).await;
    assert_eq!(after["data"]["name"], "Rice");
    assert_eq!(after["data"]["sku"], "RICE-2");

    assert_eq!(get(&router, &format!("/api/v1/products/{}", Uuid::now_v7()), &as_user(&owner)).await.0, StatusCode::NOT_FOUND);
    assert_eq!(patch(&router, &format!("/api/v1/products/{}", Uuid::now_v7()), &as_user(&owner), json!({ "name": "Ghost" })).await.0, StatusCode::NOT_FOUND);
    let _ = beans;

    // Another business's product can be neither read nor edited.
    let (other, other_tenant) = signup_owner(&router, &pool, "edit-other@example.com").await;
    let other_unit = seed_unit(&pool, other_tenant, "Each", "ea").await;
    let theirs = create_product(&router, &other, other_unit, "Theirs", "T-1").await;
    let their_uri = format!("/api/v1/products/{}", theirs);
    assert_eq!(get(&router, &their_uri, &as_user(&owner)).await.0, StatusCode::NOT_FOUND);
    assert_eq!(patch(&router, &their_uri, &as_user(&owner), json!({ "name": "Hijacked" })).await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&router, &their_uri, &as_user(&other)).await.1["data"]["name"], "Theirs");
}

#[sqlx::test]
async fn sale_units_are_validated_and_listed(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "su-owner@example.com").await;
    let piece = seed_unit(&pool, tenant_id, "Piece", "pc").await;
    let carton = api_unit(&router, &owner, "Carton").await;
    let rice = create_product(&router, &owner, piece, "Rice", "RICE-1").await;
    let uri = format!("/api/v1/products/{}/sale-units", rice);
    let make = |unit: Uuid, factor: Value| json!({ "unit_id": unit, "factor_to_base": factor });

    for factor in [json!(0), json!(-2)] {
        assert_eq!(post(&router, &uri, &as_user(&owner), make(carton, factor)).await.0, StatusCode::BAD_REQUEST);
    }
    // The base unit is already sellable.
    assert_eq!(post(&router, &uri, &as_user(&owner), make(piece, json!(1))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(post(&router, &uri, &as_user(&owner), make(Uuid::now_v7(), json!(12))).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        post(&router, &format!("/api/v1/products/{}/sale-units", Uuid::now_v7()), &as_user(&owner), make(carton, json!(12))).await.0,
        StatusCode::NOT_FOUND
    );
    // Another business's unit is not usable here.
    let (other, other_tenant) = signup_owner(&router, &pool, "su-other@example.com").await;
    let foreign = seed_unit(&pool, other_tenant, "Pallet", "plt").await;
    let _ = other;
    assert_eq!(post(&router, &uri, &as_user(&owner), make(foreign, json!(100))).await.0, StatusCode::NOT_FOUND);

    assert_eq!(post(&router, &uri, &as_user(&owner), make(carton, json!(12))).await.0, StatusCode::OK);
    let (status, response) = post(&router, &uri, &as_user(&owner), make(carton, json!(6))).await;
    assert_eq!((status, code(&response)), (StatusCode::CONFLICT, "ALREADY_EXISTS"));

    let (_, list) = get(&router, &uri, &as_user(&owner)).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 1);
    assert_eq!(list["data"][0]["unit_id"], carton.to_string());
    assert_eq!(get(&router, &format!("/api/v1/products/{}/sale-units", Uuid::now_v7()), &as_user(&owner)).await.0, StatusCode::NOT_FOUND);
}

// === Price rules

#[sqlx::test]
async fn price_rules_are_validated_listed_and_deleted(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "pr-owner@example.com").await;
    let store = create_store(&router, &owner, "lagos").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let rice = create_product(&router, &owner, unit, "Rice", "RICE-1").await;
    let uri = format!("/api/v1/products/{}/price-rules", rice);
    let now = chrono::Utc::now();

    let bad: Vec<(Value, StatusCode)> = vec![
        (json!({ "unit_price_amount_minor": -1 }), StatusCode::BAD_REQUEST),
        (json!({ "unit_price_amount_minor": 5, "min_quantity": "0" }), StatusCode::BAD_REQUEST),
        (json!({ "unit_price_amount_minor": 5, "min_quantity": "-3" }), StatusCode::BAD_REQUEST),
        (json!({ "unit_price_amount_minor": 5, "starts_at": now, "ends_at": now - chrono::Duration::days(1) }), StatusCode::BAD_REQUEST),
        (json!({ "unit_price_amount_minor": 5, "starts_at": now, "ends_at": now }), StatusCode::BAD_REQUEST),
        (json!({ "unit_price_amount_minor": 5, "store_id": Uuid::now_v7() }), StatusCode::NOT_FOUND),
        (json!({ "unit_price_amount_minor": 5, "customer_group_id": Uuid::now_v7() }), StatusCode::NOT_FOUND),
    ];
    for (body, expected) in bad {
        let (status, response) = post(&router, &uri, &as_user(&owner), body.clone()).await;
        assert_eq!(status, expected, "{} -> {}", body, response);
    }
    assert_eq!(
        post(&router, &format!("/api/v1/products/{}/price-rules", Uuid::now_v7()), &as_user(&owner), json!({ "unit_price_amount_minor": 5 })).await.0,
        StatusCode::NOT_FOUND
    );

    // Free is a valid price; another business's store and product are not valid references.
    assert_eq!(post(&router, &uri, &as_user(&owner), json!({ "unit_price_amount_minor": 0, "store_id": store })).await.0, StatusCode::OK);
    let (other, other_tenant) = signup_owner(&router, &pool, "pr-other@example.com").await;
    let other_store = create_store(&router, &other, "abuja").await;
    let other_unit = seed_unit(&pool, other_tenant, "Each", "ea").await;
    let other_product = create_product(&router, &other, other_unit, "Beans", "B-1").await;
    assert_eq!(post(&router, &uri, &as_user(&owner), json!({ "unit_price_amount_minor": 5, "store_id": other_store })).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        post(&router, &format!("/api/v1/products/{}/price-rules", other_product), &as_user(&owner), json!({ "unit_price_amount_minor": 5 })).await.0,
        StatusCode::NOT_FOUND
    );

    // List and delete.
    let second = price_rule(&router, &owner, rice, None, 150_000).await;
    let (_, list) = get(&router, &uri, &as_user(&owner)).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 2);
    let rule_uri = format!("{}/{}", uri, second);
    assert_eq!(delete(&router, &rule_uri, &as_user(&owner)).await.0, StatusCode::OK);
    assert_eq!(delete(&router, &rule_uri, &as_user(&owner)).await.0, StatusCode::NOT_FOUND);
    // A rule is only deletable through its own product.
    let (_, remaining) = get(&router, &uri, &as_user(&owner)).await;
    let first = remaining["data"][0]["id"].as_str().unwrap().to_string();
    let wrong_product = create_product(&router, &owner, unit, "Beans2", "B-2").await;
    assert_eq!(delete(&router, &format!("/api/v1/products/{}/price-rules/{}", wrong_product, first), &as_user(&owner)).await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&router, &uri, &as_user(&owner)).await.1["data"].as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn a_business_can_be_set_up_and_trade_using_only_the_api(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "e2e-owner@example.com").await;

    // No SQL fixtures: stores, unit, product, price, listing and stock all come from the API.
    let lagos = create_store(&router, &owner, "lagos").await;
    let abuja = create_store(&router, &owner, "abuja").await;
    let piece = api_unit(&router, &owner, "Piece").await;
    let rice = create_product(&router, &owner, piece, "Rice", "RICE-1").await;
    price_rule(&router, &owner, rice, None, 150_000).await;
    list_at(&router, &owner, rice, lagos).await;
    stock_up(&router, &owner, lagos, rice, "50").await;
    stock_up(&router, &owner, abuja, rice, "30").await;
    let shift = open_shift(&router, &owner, lagos).await;
    let cash = method_id(&router, &owner, "cash").await;

    let request = sale_request(lagos, shift, vec![cart_line(rice, piece, "10")], vec![pay(cash, 1_500_000)]);
    let (status, sale) = post(&router, "/api/v1/sales", &as_user(&owner), request).await;
    assert_eq!(status, StatusCode::OK, "{}", sale);
    assert_eq!(sale["data"]["total_amount_minor"], 1_500_000);

    // Only the selling store changed.
    assert_eq!(qty(&on_hand(&router, &owner, lagos, rice).await), qty("40"));
    assert_eq!(qty(&on_hand(&router, &owner, abuja, rice).await), qty("30"));
}

// === Stores

#[sqlx::test]
async fn a_store_can_be_read_and_edited(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "store-owner@example.com").await;
    let lagos = create_store(&router, &owner, "lagos").await;
    let uri = format!("/api/v1/stores/{}", lagos);

    let (status, body) = patch(&router, &uri, &as_user(&owner), json!({ "name": "  Lekki Branch ", "address": "12 Admiralty Way" })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["name"], "Lekki Branch");
    assert_eq!(body["data"]["address"], "12 Admiralty Way");
    assert_eq!(body["data"]["slug"], "lagos");
    assert_eq!(body["data"]["currency"], "NGN");
    assert_eq!(body["data"]["is_active"], true);
    assert_eq!(get(&router, &uri, &as_user(&owner)).await.1["data"]["name"], "Lekki Branch");

    // The slug can change, "changing" it to itself works, null and blank clear the address.
    assert_eq!(patch(&router, &uri, &as_user(&owner), json!({ "slug": "lekki" })).await.1["data"]["slug"], "lekki");
    assert_eq!(patch(&router, &uri, &as_user(&owner), json!({ "slug": "lekki" })).await.0, StatusCode::OK);
    assert!(patch(&router, &uri, &as_user(&owner), json!({ "address": null })).await.1["data"]["address"].is_null());
    patch(&router, &uri, &as_user(&owner), json!({ "address": "Somewhere" })).await;
    assert!(patch(&router, &uri, &as_user(&owner), json!({ "address": "   " })).await.1["data"]["address"].is_null());
    patch(&router, &uri, &as_user(&owner), json!({ "address": "Somewhere" })).await;
    assert_eq!(patch(&router, &uri, &as_user(&owner), json!({ "name": "Renamed" })).await.1["data"]["address"], "Somewhere");

    // Edits are audited with before and after.
    let audited: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM atlas.audit_events WHERE entity_id = $1 AND action = 'store.update'")
        .bind(lagos)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(audited >= 5, "audit rows: {}", audited);

    // Stock held at the store is untouched by editing it.
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = create_product(&router, &owner, unit, "Soap", "SOAP").await;
    stock_up(&router, &owner, lagos, soap, "7").await;
    patch(&router, &uri, &as_user(&owner), json!({ "name": "Again" })).await;
    assert_eq!(qty(&on_hand(&router, &owner, lagos, soap).await), qty("7"));

    let (status, created) = post(&router, "/api/v1/stores", &as_user(&owner), json!({ "name": "Main", "slug": "main", "address": " 1 High St " })).await;
    assert_eq!(status, StatusCode::OK, "{}", created);
    assert_eq!(created["data"]["address"], "1 High St");
}

#[sqlx::test]
async fn store_input_is_validated_and_conflicts_are_409(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "store-valid@example.com").await;
    let lagos = create_store(&router, &owner, "lagos").await;
    create_store(&router, &owner, "abuja").await;
    let uri = format!("/api/v1/stores/{}", lagos);
    let (_, original) = get(&router, &uri, &as_user(&owner)).await;

    for body in [
        json!({ "name": "", "slug": "x" }),
        json!({ "name": "   ", "slug": "x" }),
        json!({ "name": "Ok", "slug": "Has Space" }),
        json!({ "name": "Ok", "slug": "UPPER" }),
        json!({ "name": "Ok", "slug": "-lead" }),
        json!({ "name": "Ok", "slug": "double--hyphen" }),
        json!({ "name": "x".repeat(101), "slug": "ok" }),
    ] {
        let (status, response) = post(&router, "/api/v1/stores", &as_user(&owner), body.clone()).await;
        assert_eq!((status, code(&response)), (StatusCode::BAD_REQUEST, "VALIDATION_FAILED"), "{}", body);
    }
    let (status, response) = post(&router, "/api/v1/stores", &as_user(&owner), json!({ "name": "Dup", "slug": "lagos" })).await;
    assert_eq!((status, code(&response)), (StatusCode::CONFLICT, "ALREADY_EXISTS"));
    // Database internals must not leak through the message.
    assert!(!response.to_string().contains("stores_tenant_id_slug_key"), "{}", response);

    for (body, expected) in [
        (json!({}), StatusCode::BAD_REQUEST),
        (json!({ "name": "  " }), StatusCode::BAD_REQUEST),
        (json!({ "slug": "Not Valid" }), StatusCode::BAD_REQUEST),
        (json!({ "address": "x".repeat(256) }), StatusCode::BAD_REQUEST),
        (json!({ "slug": "abuja" }), StatusCode::CONFLICT),
        (json!({ "name": "Half Applied", "slug": "BAD" }), StatusCode::BAD_REQUEST),
        // Fixed once created, so refused rather than silently ignored.
        (json!({ "currency": "USD" }), StatusCode::UNPROCESSABLE_ENTITY),
        (json!({ "tenant_id": Uuid::now_v7() }), StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let (status, response) = patch(&router, &uri, &as_user(&owner), body.clone()).await;
        assert_eq!(status, expected, "{} -> {}", body, response);
    }
    assert_eq!(get(&router, &uri, &as_user(&owner)).await.1["data"], original["data"], "nothing was half applied");

    assert_eq!(get(&router, &format!("/api/v1/stores/{}", Uuid::now_v7()), &as_user(&owner)).await.0, StatusCode::NOT_FOUND);
    assert_eq!(patch(&router, &format!("/api/v1/stores/{}", Uuid::now_v7()), &as_user(&owner), json!({ "name": "Ghost" })).await.0, StatusCode::NOT_FOUND);

    // The same slug is fine in another business; their stores are invisible here.
    let (other, _) = signup_owner(&router, &pool, "store-valid-other@example.com").await;
    let theirs = create_store(&router, &other, "lagos").await;
    let their_uri = format!("/api/v1/stores/{}", theirs);
    assert_eq!(get(&router, &their_uri, &as_user(&owner)).await.0, StatusCode::NOT_FOUND);
    assert_eq!(patch(&router, &their_uri, &as_user(&owner), json!({ "name": "Hijacked" })).await.0, StatusCode::NOT_FOUND);
    assert_eq!(patch(&router, &their_uri, &as_user(&owner), json!({ "is_active": false })).await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&router, &their_uri, &as_user(&other)).await.1["data"]["is_active"], true);
}

#[sqlx::test]
async fn deactivating_a_store_needs_a_closed_shift_and_stops_trading(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "deact-owner@example.com").await;
    let lagos = create_store(&router, &owner, "lagos").await;
    let abuja = create_store(&router, &owner, "abuja").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let rice = sellable(&router, &owner, lagos, unit, "Rice", "RICE-1", 1_000, "100").await;
    list_at(&router, &owner, rice, abuja).await;
    stock_up(&router, &owner, abuja, rice, "10").await;
    let shift = open_shift(&router, &owner, lagos).await;
    let uri = format!("/api/v1/stores/{}", lagos);

    let (status, body) = patch(&router, &uri, &as_user(&owner), json!({ "is_active": false })).await;
    assert_eq!((status, code(&body)), (StatusCode::CONFLICT, "SHIFT_OPEN"));
    assert_eq!(get(&router, &uri, &as_user(&owner)).await.1["data"]["is_active"], true);

    let (status, _) = post(&router, &format!("/api/v1/shifts/{}/close", shift), &as_user(&owner), json!({ "closing_cash_amount_minor": 0 })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(patch(&router, &uri, &as_user(&owner), json!({ "is_active": false })).await.0, StatusCode::OK);

    // Still listed, flagged inactive, stock intact.
    let (_, list) = get(&router, "/api/v1/stores", &as_user(&owner)).await;
    assert_eq!(list["data"].as_array().unwrap().len(), 2);
    assert_eq!(qty(&on_hand(&router, &owner, lagos, rice).await), qty("100"));

    // Transfers in or out are refused; a write-off of the leftover still works.
    for (from, to) in [(lagos, abuja), (abuja, lagos)] {
        let (status, body) = post(
            &router,
            "/api/v1/stock/transfers",
            &as_user(&owner),
            json!({ "from_store_id": from, "to_store_id": to, "product_id": rice, "quantity": "1", "reason": "r" }),
        )
        .await;
        assert_eq!((status, code(&body)), (StatusCode::CONFLICT, "STORE_INACTIVE"));
    }
    let (status, _) = post(
        &router,
        "/api/v1/stock/adjustments",
        &as_user(&owner),
        json!({ "store_id": lagos, "product_id": rice, "quantity": "-100", "reason": "Store closed" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(qty(&on_hand(&router, &owner, lagos, rice).await), qty("0"));

    // Renaming an inactive store works; reactivating restores trading.
    assert_eq!(patch(&router, &uri, &as_user(&owner), json!({ "name": "Closed for now" })).await.0, StatusCode::OK);
    assert_eq!(patch(&router, &uri, &as_user(&owner), json!({ "is_active": true })).await.0, StatusCode::OK);
    stock_up(&router, &owner, lagos, rice, "5").await;
    let (status, _) = post(
        &router,
        "/api/v1/stock/transfers",
        &as_user(&owner),
        json!({ "from_store_id": lagos, "to_store_id": abuja, "product_id": rice, "quantity": "2", "reason": "r" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn store_edits_follow_store_scoped_permissions(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "scope-owner@example.com").await;
    let lagos = create_store(&router, &owner, "lagos").await;
    let abuja = create_store(&router, &owner, "abuja").await;
    let (_, lagos_admin) = seed_staff(&pool, tenant_id, lagos, "admin").await;
    let (_, lagos_cashier) = seed_staff(&pool, tenant_id, lagos, "cashier").await;
    let edit = |token: &str, store: Uuid| {
        let router = router.clone();
        let token = token.to_string();
        async move { patch(&router, &format!("/api/v1/stores/{}", store), &as_user(&token), json!({ "address": "New street" })).await.0 }
    };

    assert_eq!(edit(&lagos_admin, lagos).await, StatusCode::OK);
    assert_eq!(edit(&lagos_admin, abuja).await, StatusCode::FORBIDDEN);
    assert_eq!(edit(&lagos_cashier, lagos).await, StatusCode::FORBIDDEN);
    assert_eq!(edit(&owner, abuja).await, StatusCode::OK);
    // Anyone in the business can read a store.
    assert_eq!(get(&router, &format!("/api/v1/stores/{}", abuja), &as_user(&lagos_cashier)).await.0, StatusCode::OK);
}

// === Customer groups

#[sqlx::test]
async fn customer_groups_can_be_managed_but_not_while_in_use(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "grp-owner@example.com").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let rice = create_product(&router, &owner, unit, "Rice", "RICE-1").await;

    let (status, group) = post(&router, "/api/v1/customer-groups", &as_user(&owner), json!({ "name": "  Wholesale " })).await;
    assert_eq!(status, StatusCode::OK, "{}", group);
    assert_eq!(group["data"]["name"], "Wholesale");
    let group_id = id_of(&group);
    post(&router, "/api/v1/customer-groups", &as_user(&owner), json!({ "name": "Staff" })).await;
    let (_, list) = get(&router, "/api/v1/customer-groups", &as_user(&owner)).await;
    let names: Vec<&str> = list["data"].as_array().unwrap().iter().map(|g| g["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Staff", "Wholesale"]);

    for name in ["", "   ", &"x".repeat(101)] {
        assert_eq!(post(&router, "/api/v1/customer-groups", &as_user(&owner), json!({ "name": name })).await.0, StatusCode::BAD_REQUEST);
    }
    let (status, response) = post(&router, "/api/v1/customer-groups", &as_user(&owner), json!({ "name": "Wholesale" })).await;
    assert_eq!((status, code(&response)), (StatusCode::CONFLICT, "ALREADY_EXISTS"));

    let uri = format!("/api/v1/customer-groups/{}", group_id);
    assert_eq!(patch(&router, &uri, &as_user(&owner), json!({ "name": "Trade" })).await.1["data"]["name"], "Trade");
    assert_eq!(patch(&router, &uri, &as_user(&owner), json!({ "name": "Staff" })).await.0, StatusCode::CONFLICT);

    // A price rule pointing at the group blocks deleting it (the cascade would silently drop the rule).
    let (_, rule) = post(
        &router,
        &format!("/api/v1/products/{}/price-rules", rice),
        &as_user(&owner),
        json!({ "unit_price_amount_minor": 100, "customer_group_id": group_id }),
    )
    .await;
    let (status, body) = delete(&router, &uri, &as_user(&owner)).await;
    assert_eq!((status, code(&body)), (StatusCode::CONFLICT, "GROUP_IN_USE"));
    delete(&router, &format!("/api/v1/products/{}/price-rules/{}", rice, id_of(&rule)), &as_user(&owner)).await;

    // So does a customer in it.
    let (_, customer) = post(&router, "/api/v1/customers", &as_user(&owner), json!({ "full_name": "Bulk Buyer", "customer_group_id": group_id })).await;
    let (status, body) = delete(&router, &uri, &as_user(&owner)).await;
    assert_eq!((status, code(&body)), (StatusCode::CONFLICT, "GROUP_IN_USE"));
    patch(&router, &format!("/api/v1/customers/{}", id_of(&customer)), &as_user(&owner), json!({ "customer_group_id": null })).await;
    assert_eq!(delete(&router, &uri, &as_user(&owner)).await.0, StatusCode::OK);
    assert_eq!(delete(&router, &uri, &as_user(&owner)).await.0, StatusCode::NOT_FOUND);

    // Another business's groups are invisible and unusable.
    let (other, _) = signup_owner(&router, &pool, "grp-other@example.com").await;
    let (_, theirs) = post(&router, "/api/v1/customer-groups", &as_user(&other), json!({ "name": "Theirs" })).await;
    let their_uri = format!("/api/v1/customer-groups/{}", id_of(&theirs));
    assert_eq!(patch(&router, &their_uri, &as_user(&owner), json!({ "name": "Mine" })).await.0, StatusCode::NOT_FOUND);
    assert_eq!(delete(&router, &their_uri, &as_user(&owner)).await.0, StatusCode::NOT_FOUND);
    let (status, _) = post(&router, "/api/v1/customers", &as_user(&owner), json!({ "full_name": "A", "customer_group_id": id_of(&theirs) })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        post(&router, &format!("/api/v1/products/{}/price-rules", rice), &as_user(&owner), json!({ "unit_price_amount_minor": 1, "customer_group_id": id_of(&theirs) })).await.0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn a_cashier_cannot_move_customers_between_pricing_groups(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "tier-owner@example.com").await;
    let lagos = create_store(&router, &owner, "lagos").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let rice = sellable(&router, &owner, lagos, unit, "Rice", "RICE-1", 150_000, "100").await;
    let (_, group) = post(&router, "/api/v1/customer-groups", &as_user(&owner), json!({ "name": "Wholesale" })).await;
    let group_id = id_of(&group);
    post(
        &router,
        &format!("/api/v1/products/{}/price-rules", rice),
        &as_user(&owner),
        json!({ "unit_price_amount_minor": 120_000, "customer_group_id": group_id }),
    )
    .await;
    let (_, trade) = post(&router, "/api/v1/customers", &as_user(&owner), json!({ "full_name": "Trade Buyer", "customer_group_id": group_id })).await;
    let (_, cashier) = seed_staff(&pool, tenant_id, lagos, "cashier").await;
    let store_id = lagos.to_string();
    let caller = at_store(&cashier, &store_id);

    // They can register customers at their till...
    let (status, walk_in) = post(&router, "/api/v1/customers", &caller, json!({ "full_name": "Walk In", "phone": "08012345678" })).await;
    assert_eq!(status, StatusCode::OK, "{}", walk_in);
    let walk_in_uri = format!("/api/v1/customers/{}", id_of(&walk_in));
    assert_eq!(patch(&router, &walk_in_uri, &caller, json!({ "full_name": "Walk In Renamed" })).await.0, StatusCode::OK);

    // ...but not choose a pricing tier for anyone, themselves included.
    assert_eq!(post(&router, "/api/v1/customers", &caller, json!({ "full_name": "Friend", "customer_group_id": group_id })).await.0, StatusCode::FORBIDDEN);
    assert_eq!(patch(&router, &walk_in_uri, &caller, json!({ "customer_group_id": group_id })).await.0, StatusCode::FORBIDDEN);
    assert_eq!(patch(&router, &format!("/api/v1/customers/{}", id_of(&trade)), &caller, json!({ "customer_group_id": null })).await.0, StatusCode::FORBIDDEN);
    let (_, still) = get(&router, &format!("/api/v1/customers/{}", id_of(&trade)), &as_user(&owner)).await;
    assert_eq!(still["data"]["customer_group_id"], group_id);

    // And the group really does set the price, end to end through the API.
    let shift = open_shift(&router, &cashier, lagos).await;
    let cash = method_id(&router, &owner, "cash").await;
    let price_for = |customer: Option<String>, amount: i64| {
        let router = router.clone();
        let (cashier, store_id) = (cashier.clone(), store_id.clone());
        async move {
            let mut request = sale_request(lagos, shift, vec![cart_line(rice, unit, "1")], vec![pay(cash, amount)]);
            request["customer_id"] = json!(customer);
            let (status, body) = post(&router, "/api/v1/sales", &at_store(&cashier, &store_id), request).await;
            assert_eq!(status, StatusCode::OK, "{}", body);
            return body["data"]["total_amount_minor"].as_i64().unwrap();
        }
    };
    assert_eq!(price_for(Some(id_of(&trade)), 120_000).await, 120_000);
    assert_eq!(price_for(Some(id_of(&walk_in)), 150_000).await, 150_000);
    assert_eq!(price_for(None, 150_000).await, 150_000);
}

// === Duplicates and tenant ownership of stock writes

#[sqlx::test]
async fn duplicates_are_conflicts_not_server_errors(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "dup-owner@example.com").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let rice = create_product(&router, &owner, unit, "Rice", "RICE-1").await;
    let product_type = product_type_of(&pool, rice).await;

    let (status, body) = post(
        &router,
        "/api/v1/products",
        &as_user(&owner),
        json!({ "product_type_id": product_type, "base_unit_id": unit, "name": "Rice 2", "sku": "RICE-1" }),
    )
    .await;
    assert_eq!((status, code(&body)), (StatusCode::CONFLICT, "ALREADY_EXISTS"));
    assert!(body["message"].as_str().unwrap().len() < 80, "generic message, no SQL: {}", body);

    // A missing reference is the caller's error, not a 500.
    let (status, body) = delete(&router, &format!("/api/v1/products/{}/price-rules/{}", rice, Uuid::now_v7()), &as_user(&owner)).await;
    assert_eq!((status, code(&body)), (StatusCode::NOT_FOUND, ""));
}

#[sqlx::test]
async fn stock_writes_refuse_another_tenants_stores_and_products(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "xt-owner@example.com").await;
    let lagos = create_store(&router, &owner, "lagos").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let rice = create_product(&router, &owner, unit, "Rice", "RICE-1").await;
    stock_up(&router, &owner, lagos, rice, "10").await;

    let (other, other_tenant) = signup_owner(&router, &pool, "xt-other@example.com").await;
    let their_store = create_store(&router, &other, "abuja").await;
    let their_unit = seed_unit(&pool, other_tenant, "Each", "ea").await;
    let their_product = create_product(&router, &other, their_unit, "Beans", "B-1").await;
    let ledger_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM atlas.stock_movements").fetch_one(&pool).await.unwrap();

    // The owner's tenant-wide grant would otherwise let them name any store id.
    for body in [
        json!({ "store_id": their_store, "product_id": rice, "quantity": "5", "reason": "x" }),
        json!({ "store_id": lagos, "product_id": their_product, "quantity": "5", "reason": "x" }),
    ] {
        let (status, response) = post(&router, "/api/v1/stock/adjustments", &as_user(&owner), body.clone()).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{} -> {}", body, response);
    }
    for (from, to, product) in [(lagos, their_store, rice), (their_store, lagos, rice), (lagos, lagos, their_product)] {
        let (status, _) = post(
            &router,
            "/api/v1/stock/transfers",
            &as_user(&owner),
            json!({ "from_store_id": from, "to_store_id": to, "product_id": product, "quantity": "1", "reason": "x" }),
        )
        .await;
        assert_ne!(status, StatusCode::OK);
    }

    let ledger_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM atlas.stock_movements").fetch_one(&pool).await.unwrap();
    assert_eq!(ledger_after, ledger_before, "no ledger row was written for a foreign id");
    assert_eq!(qty(&on_hand(&router, &owner, lagos, rice).await), qty("10"));
}

#[sqlx::test]
async fn inactive_products_cannot_be_transferred_but_can_be_written_off(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "inact-owner@example.com").await;
    let lagos = create_store(&router, &owner, "lagos").await;
    let abuja = create_store(&router, &owner, "abuja").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let rice = create_product(&router, &owner, unit, "Rice", "RICE-1").await;
    stock_up(&router, &owner, lagos, rice, "10").await;
    assert_eq!(patch(&router, &format!("/api/v1/products/{}", rice), &as_user(&owner), json!({ "is_active": false })).await.0, StatusCode::OK);

    let (status, body) = post(
        &router,
        "/api/v1/stock/transfers",
        &as_user(&owner),
        json!({ "from_store_id": lagos, "to_store_id": abuja, "product_id": rice, "quantity": "1", "reason": "r" }),
    )
    .await;
    assert_eq!((status, code(&body)), (StatusCode::CONFLICT, "PRODUCT_INACTIVE"));
    let (status, _) = post(
        &router,
        "/api/v1/stock/adjustments",
        &as_user(&owner),
        json!({ "store_id": lagos, "product_id": rice, "quantity": "-10", "reason": "Discontinued" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(qty(&on_hand(&router, &owner, lagos, rice).await), qty("0"));
}
