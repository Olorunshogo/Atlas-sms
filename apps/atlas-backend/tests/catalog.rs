mod common;

use axum::http::{Method, StatusCode};
use chrono::{Duration, Utc};
use common::{bare_world, d, send, store_cashier, world, World};
use serde_json::{json, Value};
use uuid::Uuid;

fn code(body: &Value) -> &str {
    body["code"].as_str().unwrap_or_default()
}

fn id(body: &Value) -> Uuid {
    body["data"]["id"].as_str().unwrap().parse().unwrap()
}

/** Creates a unit, a product type and a product through the API; returns (unit, product). */
async fn api_product(w: &World, name: &str, sku: &str) -> (Uuid, Uuid) {
    let (status, unit) = w
        .post(
            "/api/v1/units",
            json!({ "name": format!("{name}-unit"), "abbreviation": "u" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{unit}");
    let unit_id = id(&unit);

    let (status, product_type) = w
        .post(
            "/api/v1/product-types",
            json!({ "name": format!("{name}-type") }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{product_type}");

    let (status, product) = w
        .post(
            "/api/v1/products",
            json!({
                "product_type_id": id(&product_type),
                "base_unit_id": unit_id,
                "name": name,
                "sku": sku,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{product}");
    (unit_id, id(&product))
}

async fn api_store(w: &World, name: &str, slug: &str) -> Uuid {
    let (status, store) = w
        .post("/api/v1/stores", json!({ "name": name, "slug": slug }))
        .await;
    assert_eq!(status, StatusCode::OK, "{store}");
    id(&store)
}

async fn api_price(w: &World, product: Uuid, price: i64, extra: Value) -> (StatusCode, Value) {
    let mut body = json!({ "product_id": product, "unit_price_amount_minor": price });
    for (key, value) in extra.as_object().unwrap() {
        body[key] = value.clone();
    }
    w.post("/api/v1/price-rules", body).await
}

// === The whole flow, API only

#[tokio::test]
async fn a_business_can_be_set_up_and_trade_using_only_the_api() {
    let w = bare_world().await;

    let one = api_store(&w, "Location One", "location-one").await;
    let two = api_store(&w, "Location Two", "location-two").await;
    let (unit, rice) = api_product(&w, "Rice", "RICE-50KG").await;
    let (status, _) = api_price(&w, rice, 150_000, json!({})).await;
    assert_eq!(status, StatusCode::OK);

    let (status, shift) = w
        .post(
            "/api/v1/shifts",
            json!({ "store_id": one, "opening_cash_amount_minor": 0 }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{shift}");
    let shift_id = id(&shift);

    for (store, quantity) in [(one, 50), (two, 30)] {
        let (status, body) = w
            .post(
                "/api/v1/stock/inventory",
                json!({ "store_id": store, "product_id": rice, "quantity": quantity }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    let (status, sale) = w
        .post(
            "/api/v1/sales",
            json!({
                "id": Uuid::now_v7(),
                "store_id": one,
                "shift_id": shift_id,
                "device_id": "till-1",
                "lines": [{ "product_id": rice, "unit_id": unit, "quantity": 10 }],
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{sale}");
    assert_eq!(sale["data"]["total_amount_minor"], 1_500_000);

    let (_, body) = w.get(&format!("/api/v1/stock/products/{rice}")).await;
    assert_eq!(common::dec(&body["data"]["total_quantity"]), d(70));
    let locations = body["data"]["locations"].as_array().unwrap();
    assert_eq!(common::dec(&locations[0]["quantity"]), d(40));
    assert_eq!(common::dec(&locations[1]["quantity"]), d(30));
}

// === Duplicates

#[tokio::test]
async fn duplicates_are_conflicts_not_server_errors() {
    let w = bare_world().await;
    api_store(&w, "Location One", "location-one").await;
    let (unit, rice) = api_product(&w, "Rice", "RICE-1").await;

    let (status, body) = w
        .post(
            "/api/v1/stores",
            json!({ "name": "Other", "slug": "location-one" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(code(&body), "ALREADY_EXISTS");
    // Database internals (constraint names) must not leak.
    assert!(!body.to_string().contains("stores_tenant_id_slug_key"));

    let product_type: Uuid =
        sqlx::query_scalar("SELECT product_type_id FROM atlas.products WHERE id = $1")
            .bind(rice)
            .fetch_one(&w.admin)
            .await
            .unwrap();
    let (status, body) = w
        .post(
            "/api/v1/products",
            json!({ "product_type_id": product_type, "base_unit_id": unit, "name": "Rice 2", "sku": "RICE-1" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(code(&body), "ALREADY_EXISTS");

    let (status, _) = w
        .post(
            "/api/v1/units",
            json!({ "name": "Rice-unit", "abbreviation": "x" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Two products with no SKU at all are fine: NULLs do not collide.
    for name in ["A", "B"] {
        let (status, _) = w
            .post(
                "/api/v1/products",
                json!({ "product_type_id": product_type, "base_unit_id": unit, "name": name }),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
    }
}

#[tokio::test]
async fn the_same_slug_is_allowed_in_different_tenants() {
    let a = bare_world().await;
    let b = bare_world().await;
    api_store(&a, "Main", "main").await;
    api_store(&b, "Main", "main").await;
}

// === Validation

#[tokio::test]
async fn store_input_is_validated() {
    let w = bare_world().await;
    let cases = [
        json!({ "name": "", "slug": "ok" }),
        json!({ "name": "   ", "slug": "ok" }),
        json!({ "name": "Ok", "slug": "" }),
        json!({ "name": "Ok", "slug": "Has Space" }),
        json!({ "name": "Ok", "slug": "UPPER" }),
        json!({ "name": "Ok", "slug": "-lead" }),
        json!({ "name": "Ok", "slug": "double--hyphen" }),
        json!({ "name": "x".repeat(101), "slug": "ok" }),
    ];
    for body in cases {
        let (status, response) = w.post("/api/v1/stores", body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(code(&response), "VALIDATION_FAILED", "{body}");
    }
    let (_, stores) = w.get("/api/v1/stores").await;
    assert!(stores["data"].as_array().unwrap().is_empty());

    // Surrounding whitespace is trimmed, not stored.
    let (status, store) = w
        .post(
            "/api/v1/stores",
            json!({ "name": "  Padded  ", "slug": "padded" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(store["data"]["name"], "Padded");
}

#[tokio::test]
async fn product_input_is_validated() {
    let w = bare_world().await;
    let (unit, rice) = api_product(&w, "Rice", "RICE-1").await;
    let product_type: Uuid =
        sqlx::query_scalar("SELECT product_type_id FROM atlas.products WHERE id = $1")
            .bind(rice)
            .fetch_one(&w.admin)
            .await
            .unwrap();
    let base = json!({ "product_type_id": product_type, "base_unit_id": unit, "name": "Thing" });
    let with = |key: &str, value: Value| {
        let mut body = base.clone();
        body[key] = value;
        body
    };

    for (body, expected) in [
        (with("name", json!("  ")), StatusCode::BAD_REQUEST),
        (with("sku", json!("  ")), StatusCode::BAD_REQUEST),
        (
            with("attributes", json!(["not", "an", "object"])),
            StatusCode::BAD_REQUEST,
        ),
        (
            with("product_type_id", json!(Uuid::now_v7())),
            StatusCode::NOT_FOUND,
        ),
        (
            with("base_unit_id", json!(Uuid::now_v7())),
            StatusCode::NOT_FOUND,
        ),
    ] {
        let (status, response) = w.post("/api/v1/products", body.clone()).await;
        assert_eq!(status, expected, "{body} -> {response}");
    }

    // Another tenant's type and unit are invisible, not accepted.
    let other = bare_world().await;
    let (other_unit, other_product) = api_product(&other, "Beans", "B-1").await;
    let other_type: Uuid =
        sqlx::query_scalar("SELECT product_type_id FROM atlas.products WHERE id = $1")
            .bind(other_product)
            .fetch_one(&other.admin)
            .await
            .unwrap();
    let (status, _) = w
        .post(
            "/api/v1/products",
            with("product_type_id", json!(other_type)),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = w
        .post("/api/v1/products", with("base_unit_id", json!(other_unit)))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (_, products) = w.get("/api/v1/products").await;
    assert_eq!(products["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn product_type_and_unit_input_is_validated() {
    let w = bare_world().await;
    for body in [
        json!({ "name": "" }),
        json!({ "name": "Ok", "identity_mode": "by_magic" }),
        json!({ "name": "Ok", "attribute_schema": { "not": "an array" } }),
    ] {
        let (status, response) = w.post("/api/v1/product-types", body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} -> {response}");
    }
    for mode in ["by_product", "by_batch", "by_serial", "by_both"] {
        let (status, _) = w
            .post(
                "/api/v1/product-types",
                json!({ "name": mode, "identity_mode": mode }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{mode}");
    }
    for body in [
        json!({ "name": "", "abbreviation": "x" }),
        json!({ "name": "Kilogram", "abbreviation": "  " }),
        json!({ "name": "Kilogram", "abbreviation": "much-too-long" }),
    ] {
        let (status, _) = w.post("/api/v1/units", body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    let (_, units) = w.get("/api/v1/units").await;
    assert!(units["data"].as_array().unwrap().is_empty());
}

// === Sale units

#[tokio::test]
async fn sale_units_are_validated_and_drive_conversion() {
    let w = bare_world().await;
    let store = api_store(&w, "Location One", "location-one").await;
    let (piece, rice) = api_product(&w, "Rice", "RICE-1").await;
    api_price(&w, rice, 1_000, json!({})).await;
    let (_, carton) = w
        .post(
            "/api/v1/units",
            json!({ "name": "carton", "abbreviation": "ctn" }),
        )
        .await;
    let carton = id(&carton);
    let uri = format!("/api/v1/products/{rice}/sale-units");

    for factor in [json!(0), json!(-2)] {
        let (status, _) = w
            .post(&uri, json!({ "unit_id": carton, "factor_to_base": factor }))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{factor}");
    }
    // The base unit is already sellable.
    let (status, _) = w
        .post(&uri, json!({ "unit_id": piece, "factor_to_base": 1 }))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = w
        .post(
            &uri,
            json!({ "unit_id": Uuid::now_v7(), "factor_to_base": 12 }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = w
        .post(
            &format!("/api/v1/products/{}/sale-units", Uuid::now_v7()),
            json!({ "unit_id": carton, "factor_to_base": 12 }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, body) = w
        .post(&uri, json!({ "unit_id": carton, "factor_to_base": 12 }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["unit_name"], "carton");
    let (status, _) = w
        .post(&uri, json!({ "unit_id": carton, "factor_to_base": 6 }))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (_, listed) = w.get(&uri).await;
    assert_eq!(listed["data"].as_array().unwrap().len(), 1);

    // Selling 2 cartons deducts 24 pieces and bills 24 x 1000.
    let (_, shift) = w.post("/api/v1/shifts", json!({ "store_id": store })).await;
    w.post(
        "/api/v1/stock/inventory",
        json!({ "store_id": store, "product_id": rice, "quantity": 100 }),
    )
    .await;
    let (status, sale) = w
        .post(
            "/api/v1/sales",
            json!({
                "id": Uuid::now_v7(), "store_id": store, "shift_id": id(&shift), "device_id": "t",
                "lines": [{ "product_id": rice, "unit_id": carton, "quantity": 2 }],
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{sale}");
    assert_eq!(sale["data"]["total_amount_minor"], 24_000);
    assert_eq!(sale["data"]["lines"][0]["unit_price_amount_minor"], 12_000);
    let (_, body) = w.get(&format!("/api/v1/stock/products/{rice}")).await;
    assert_eq!(common::dec(&body["data"]["total_quantity"]), d(76));
}

// === Price rules

#[tokio::test]
async fn price_rule_input_is_validated() {
    let w = bare_world().await;
    let store = api_store(&w, "Location One", "location-one").await;
    let (_, rice) = api_product(&w, "Rice", "RICE-1").await;
    let now = Utc::now();

    let bad: Vec<(Value, StatusCode)> = vec![
        (
            json!({ "product_id": rice, "unit_price_amount_minor": -1 }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "product_id": rice, "unit_price_amount_minor": 5, "min_quantity": 0 }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "product_id": rice, "unit_price_amount_minor": 5, "min_quantity": -3 }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "product_id": rice, "unit_price_amount_minor": 5,
                    "starts_at": now, "ends_at": now - Duration::days(1) }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "product_id": rice, "unit_price_amount_minor": 5, "starts_at": now, "ends_at": now }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "product_id": Uuid::now_v7(), "unit_price_amount_minor": 5 }),
            StatusCode::NOT_FOUND,
        ),
        (
            json!({ "product_id": rice, "unit_price_amount_minor": 5, "store_id": Uuid::now_v7() }),
            StatusCode::NOT_FOUND,
        ),
        (
            json!({ "product_id": rice, "unit_price_amount_minor": 5, "customer_group_id": Uuid::now_v7() }),
            StatusCode::NOT_FOUND,
        ),
    ];
    for (body, expected) in bad {
        let (status, response) = w.post("/api/v1/price-rules", body.clone()).await;
        assert_eq!(status, expected, "{body} -> {response}");
    }

    // Free is a valid price (a giveaway); another tenant's ids are not valid references.
    let (status, _) = api_price(&w, rice, 0, json!({ "store_id": store })).await;
    assert_eq!(status, StatusCode::OK);
    let other = bare_world().await;
    let (_, other_product) = api_product(&other, "Beans", "B-1").await;
    let other_store = api_store(&other, "Elsewhere", "elsewhere").await;
    let (status, _) = api_price(&w, other_product, 5, json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = api_price(&w, rice, 5, json!({ "store_id": other_store })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (_, rules) = w.get("/api/v1/price-rules").await;
    assert_eq!(rules["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn the_most_specific_active_rule_sets_the_price() {
    let w = bare_world().await;
    let one = api_store(&w, "Location One", "location-one").await;
    let two = api_store(&w, "Location Two", "location-two").await;
    let (unit, rice) = api_product(&w, "Rice", "RICE-1").await;
    for store in [one, two] {
        w.post(
            "/api/v1/stock/inventory",
            json!({ "store_id": store, "product_id": rice, "quantity": 1000 }),
        )
        .await;
    }
    let (_, shift_one) = w.post("/api/v1/shifts", json!({ "store_id": one })).await;
    let (_, shift_two) = w.post("/api/v1/shifts", json!({ "store_id": two })).await;
    let now = Utc::now();

    let price_of = |store: Uuid, shift: &Value, quantity: i64| {
        let body = json!({
            "id": Uuid::now_v7(), "store_id": store, "shift_id": id(shift), "device_id": "t",
            "lines": [{ "product_id": rice, "unit_id": unit, "quantity": quantity }],
        });
        let w = &w;
        async move {
            let (status, sale) = w.post("/api/v1/sales", body).await;
            assert_eq!(status, StatusCode::OK, "{sale}");
            sale["data"]["lines"][0]["unit_price_amount_minor"]
                .as_i64()
                .unwrap()
        }
    };

    // No rule yet: not for sale.
    let (status, body) = w
        .post(
            "/api/v1/sales",
            json!({ "id": Uuid::now_v7(), "store_id": one, "shift_id": id(&shift_one), "device_id": "t",
                    "lines": [{ "product_id": rice, "unit_id": unit, "quantity": 1 }] }),
        )
        .await;
    assert_eq!((status, code(&body)), (StatusCode::BAD_REQUEST, "NO_PRICE"));

    /*
     * The specific rule is created BEFORE the global one on purpose: if
     * "newest wins" were the tiebreak instead of specificity, the global
     * rule would win and this test would fail.
     */
    api_price(&w, rice, 900, json!({ "store_id": one })).await;
    assert_eq!(price_of(one, &shift_one, 1).await, 900);
    // Also before the base rule, so the higher tier must win on min_quantity, not on recency.
    api_price(&w, rice, 800, json!({ "min_quantity": 10 })).await;
    api_price(&w, rice, 1_000, json!({})).await; // everywhere
    assert_eq!(price_of(one, &shift_one, 1).await, 900);
    assert_eq!(price_of(two, &shift_two, 1).await, 1_000);

    // The tier applies from its minimum.
    assert_eq!(price_of(two, &shift_two, 9).await, 1_000);
    assert_eq!(price_of(two, &shift_two, 10).await, 800);

    // Priority breaks a tie between otherwise equal rules.
    api_price(&w, rice, 700, json!({ "store_id": two, "priority": 5 })).await;
    api_price(&w, rice, 650, json!({ "store_id": two, "priority": 1 })).await;
    assert_eq!(price_of(two, &shift_two, 1).await, 700);

    // Expired and not-yet-started rules are ignored.
    api_price(
        &w,
        rice,
        1,
        json!({ "store_id": two, "priority": 99,
        "starts_at": now - Duration::days(10), "ends_at": now - Duration::days(1) }),
    )
    .await;
    api_price(
        &w,
        rice,
        2,
        json!({ "store_id": two, "priority": 99,
        "starts_at": now + Duration::days(1) }),
    )
    .await;
    assert_eq!(price_of(two, &shift_two, 1).await, 700);
}

#[tokio::test]
async fn a_customer_group_rule_applies_to_that_groups_customers() {
    let w = world().await;
    w.seed_stock(w.stores[0], w.rice, 100).await;
    let group = Uuid::now_v7();
    let customer = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.customer_groups (id, tenant_id, name) VALUES ($1, $2, 'Wholesale')",
    )
    .bind(group)
    .bind(w.tenant_id)
    .execute(&w.admin)
    .await
    .unwrap();
    sqlx::query("INSERT INTO atlas.customers (id, tenant_id, full_name, customer_group_id) VALUES ($1, $2, 'Bulk Buyer', $3)")
        .bind(customer)
        .bind(w.tenant_id)
        .bind(group)
        .execute(&w.admin)
        .await
        .unwrap();
    let (status, _) = api_price(&w, w.rice, 120_000, json!({ "customer_group_id": group })).await;
    assert_eq!(status, StatusCode::OK);

    let mut sale = w.sale_body(0, &[(w.rice, 1)]);
    sale["customer_id"] = json!(customer);
    let (_, with_customer) = w.post("/api/v1/sales", sale).await;
    assert_eq!(with_customer["data"]["total_amount_minor"], 120_000);

    let (_, walk_in) = w.sell(0, w.rice, 1).await;
    assert_eq!(walk_in["data"]["total_amount_minor"], 150_000);

    let mut unknown = w.sale_body(0, &[(w.rice, 1)]);
    unknown["customer_id"] = json!(Uuid::now_v7());
    let (status, _) = w.post("/api/v1/sales", unknown).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn deleting_a_price_rule_stops_sales_but_keeps_history() {
    let w = world().await;
    w.seed_stock(w.stores[0], w.rice, 10).await;
    let (_, sold) = w.sell(0, w.rice, 1).await;
    let sale_id = sold["data"]["id"].as_str().unwrap().to_string();

    let (_, rules) = w
        .get(&format!("/api/v1/price-rules?product_id={}", w.rice))
        .await;
    let rule_id = rules["data"][0]["id"].as_str().unwrap().to_string();
    let (status, _) = w.delete(&format!("/api/v1/price-rules/{rule_id}")).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = w.delete(&format!("/api/v1/price-rules/{rule_id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, body) = w.sell(0, w.rice, 1).await;
    assert_eq!((status, code(&body)), (StatusCode::BAD_REQUEST, "NO_PRICE"));

    let (_, lines) = w.get(&format!("/api/v1/sales/{sale_id}/lines")).await;
    assert_eq!(lines["data"][0]["unit_price_amount_minor"], 150_000);
}

#[tokio::test]
async fn catalogue_changes_need_the_product_manage_permission() {
    let w = world().await;
    let cashier = store_cashier(&w, w.stores[0]).await;
    let call = |method: Method, uri: String, body: Option<Value>| {
        let (app, token) = (w.app.clone(), cashier.clone());
        async move { send(&app, method, &uri, &token, body).await.0 }
    };

    assert_eq!(
        call(
            Method::POST,
            "/api/v1/units".into(),
            Some(json!({ "name": "x", "abbreviation": "x" }))
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            Method::POST,
            "/api/v1/price-rules".into(),
            Some(json!({ "product_id": w.rice, "unit_price_amount_minor": 1 }))
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(Method::GET, "/api/v1/price-rules".into(), None).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            Method::DELETE,
            format!("/api/v1/price-rules/{}", Uuid::now_v7()),
            None
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            Method::POST,
            format!("/api/v1/products/{}/sale-units", w.rice),
            Some(json!({ "unit_id": w.piece_unit, "factor_to_base": 2 }))
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            Method::POST,
            "/api/v1/stores".into(),
            Some(json!({ "name": "N", "slug": "n" }))
        )
        .await,
        StatusCode::FORBIDDEN
    );
    // Reading the catalogue is open to staff.
    assert_eq!(
        call(Method::GET, "/api/v1/units".into(), None).await,
        StatusCode::OK
    );
    assert_eq!(
        call(Method::GET, "/api/v1/products".into(), None).await,
        StatusCode::OK
    );
}

// === Inactive stores and products

#[tokio::test]
async fn inactive_stores_and_products_cannot_trade_but_can_be_written_off() {
    let w = world().await;
    w.seed_rice().await; // 50 / 30 / 20
    sqlx::query("UPDATE atlas.stores SET is_active = false WHERE id = $1")
        .bind(w.stores[2])
        .execute(&w.admin)
        .await
        .unwrap();

    // Sale, restock, transfer out, transfer in and initial stock at the closed store.
    let (status, body) = w.sell(2, w.rice, 1).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "STORE_INACTIVE")
    );
    let (status, body) = w
        .post(
            "/api/v1/stock/restocks",
            json!({ "store_id": w.stores[2], "product_id": w.rice, "quantity": 5 }),
        )
        .await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "STORE_INACTIVE")
    );
    let (status, body) = w
        .post("/api/v1/stock/transfers", w.transfer_body(2, 0, 5))
        .await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "STORE_INACTIVE")
    );
    let (status, body) = w
        .post("/api/v1/stock/transfers", w.transfer_body(0, 2, 5))
        .await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "STORE_INACTIVE")
    );
    assert_eq!(w.on_hand(2, w.rice).await, d(20));
    assert_eq!(w.on_hand(0, w.rice).await, d(50));

    // The leftover stock can still be written off.
    let (status, body) = w
        .post(
            "/api/v1/stock/adjustments",
            json!({ "store_id": w.stores[2], "product_id": w.rice, "quantity": -20, "reason": "Store closed" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(w.on_hand(2, w.rice).await, d(0));

    sqlx::query("UPDATE atlas.products SET is_active = false WHERE id = $1")
        .bind(w.rice)
        .execute(&w.admin)
        .await
        .unwrap();
    let (status, body) = w.sell(0, w.rice, 1).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "PRODUCT_INACTIVE")
    );
    let (status, body) = w
        .post(
            "/api/v1/stock/restocks",
            json!({ "store_id": w.stores[0], "product_id": w.rice, "quantity": 5 }),
        )
        .await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "PRODUCT_INACTIVE")
    );
    let (status, body) = w
        .post("/api/v1/stock/transfers", w.transfer_body(0, 1, 5))
        .await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "PRODUCT_INACTIVE")
    );
    assert_eq!(w.count("sales").await, 0);
    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    assert_eq!(w.on_hand(1, w.rice).await, d(30));
}

#[tokio::test]
async fn reactivating_a_store_restores_trading() {
    let w = world().await;
    w.seed_rice().await;
    for active in [false, true] {
        sqlx::query("UPDATE atlas.stores SET is_active = $2 WHERE id = $1")
            .bind(w.stores[0])
            .bind(active)
            .execute(&w.admin)
            .await
            .unwrap();
    }
    assert_eq!(w.sell(0, w.rice, 1).await.0, StatusCode::OK);
}
