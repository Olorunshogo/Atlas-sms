mod common;

use axum::http::{Method, StatusCode};
use chrono::{DateTime, Utc};
use common::{bare_world, d, send, store_cashier, store_user, world, World, ADMIN_ROLE_ID};
use serde_json::{json, Value};
use uuid::Uuid;

fn code(body: &Value) -> &str {
    body["code"].as_str().unwrap_or_default()
}

async fn store_updated_at(w: &World, store: Uuid) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT updated_at FROM atlas.stores WHERE id = $1")
        .bind(store)
        .fetch_one(&w.admin)
        .await
        .unwrap()
}

async fn product_updated_at(w: &World, product: Uuid) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT updated_at FROM atlas.products WHERE id = $1")
        .bind(product)
        .fetch_one(&w.admin)
        .await
        .unwrap()
}

// === Stores

#[tokio::test]
async fn a_store_can_be_renamed_moved_and_fetched() {
    let w = world().await;
    let store = w.stores[0];
    let uri = format!("/api/v1/stores/{store}");
    let before = store_updated_at(&w, store).await;

    let (status, body) = w
        .patch(
            &uri,
            json!({ "name": "  Lekki Branch  ", "address": "12 Admiralty Way" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["name"], "Lekki Branch");
    assert_eq!(body["data"]["address"], "12 Admiralty Way");
    // Untouched fields stay as they were.
    assert_eq!(body["data"]["slug"], "location-1");
    assert_eq!(body["data"]["is_active"], true);
    assert_eq!(body["data"]["currency"], "NGN");
    assert!(store_updated_at(&w, store).await > before);

    let (status, fetched) = w.get(&uri).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["data"]["name"], "Lekki Branch");

    // Changing the slug, and "changing" it to itself, both work.
    let (status, body) = w.patch(&uri, json!({ "slug": "lekki" })).await;
    assert_eq!(
        (status, body["data"]["slug"].as_str()),
        (StatusCode::OK, Some("lekki"))
    );
    let (status, _) = w.patch(&uri, json!({ "slug": "lekki" })).await;
    assert_eq!(status, StatusCode::OK);

    // null clears the address; a blank one does too; absent leaves it alone.
    let (_, body) = w.patch(&uri, json!({ "address": null })).await;
    assert!(body["data"]["address"].is_null());
    w.patch(&uri, json!({ "address": "Somewhere" })).await;
    let (_, body) = w.patch(&uri, json!({ "address": "   " })).await;
    assert!(body["data"]["address"].is_null());
    w.patch(&uri, json!({ "address": "Somewhere" })).await;
    let (_, body) = w.patch(&uri, json!({ "name": "Renamed" })).await;
    assert_eq!(body["data"]["address"], "Somewhere");

    // The id and the stock held there are unaffected by edits.
    w.seed_stock(store, w.rice, 7).await;
    w.patch(&uri, json!({ "name": "Again" })).await;
    assert_eq!(w.on_hand(0, w.rice).await, d(7));
    let (_, list) = w.get("/api/v1/stores").await;
    assert_eq!(list["data"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn invalid_store_edits_are_rejected_and_change_nothing() {
    let w = world().await;
    let store = w.stores[0];
    let uri = format!("/api/v1/stores/{store}");
    let (_, original) = w.get(&uri).await;

    for (body, expected, expected_code) in [
        (json!({}), StatusCode::BAD_REQUEST, "VALIDATION_FAILED"),
        (
            json!({ "name": "  " }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        (
            json!({ "name": "x".repeat(101) }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        (
            json!({ "slug": "Not Valid" }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        (
            json!({ "address": "x".repeat(256) }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        // location-2 belongs to another store in this tenant.
        (
            json!({ "slug": "location-2" }),
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
        ),
        // A valid field next to a bad one must not be half-applied.
        (
            json!({ "name": "Half Applied", "slug": "BAD" }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
    ] {
        let (status, response) = w.patch(&uri, body.clone()).await;
        assert_eq!(status, expected, "{body} -> {response}");
        assert_eq!(code(&response), expected_code, "{body}");
    }

    // Fields that must not be editable are refused, not silently ignored.
    for field in ["currency", "tenant_id", "id", "timezone"] {
        let (status, _) = w.patch(&uri, json!({ field: "x" })).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{field}");
    }

    let (_, after) = w.get(&uri).await;
    assert_eq!(after["data"], original["data"]);

    let (status, _) = w
        .patch(
            &format!("/api/v1/stores/{}", Uuid::now_v7()),
            json!({ "name": "Ghost" }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = w.get(&format!("/api/v1/stores/{}", Uuid::now_v7())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn another_tenants_store_cannot_be_read_or_edited() {
    let w = world().await;
    let other = world().await;
    let uri = format!("/api/v1/stores/{}", other.stores[0]);

    assert_eq!(w.get(&uri).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        w.patch(&uri, json!({ "name": "Hijacked" })).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        w.patch(&uri, json!({ "is_active": false })).await.0,
        StatusCode::NOT_FOUND
    );

    let (_, theirs) = other.get(&uri).await;
    assert_eq!(theirs["data"]["name"], "Location One");
    assert_eq!(theirs["data"]["is_active"], true);
}

#[tokio::test]
async fn deactivating_a_store_stops_trading_but_keeps_its_stock() {
    let w = world().await;
    w.seed_rice().await;
    let store = w.stores[2];
    let uri = format!("/api/v1/stores/{store}");

    // A till is still open, so the store cannot be closed yet.
    let (status, body) = w.patch(&uri, json!({ "is_active": false })).await;
    assert_eq!((status, code(&body)), (StatusCode::CONFLICT, "SHIFT_OPEN"));
    assert_eq!(w.get(&uri).await.1["data"]["is_active"], true);

    let (status, _) = w
        .post(
            &format!("/api/v1/shifts/{}/close", w.shifts[2]),
            json!({ "closing_cash_amount_minor": 0 }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = w.patch(&uri, json!({ "is_active": false })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["is_active"], false);

    // It still exists and is still listed, flagged inactive, with its stock.
    let (_, list) = w.get("/api/v1/stores").await;
    let listed = list["data"].as_array().unwrap();
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[2]["is_active"], false);
    assert_eq!(w.on_hand(2, w.rice).await, d(20));
    assert_eq!(w.total(w.rice).await, d(100));

    let (status, body) = w.sell(2, w.rice, 1).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "STORE_INACTIVE")
    );
    let (status, body) = w
        .post("/api/v1/stock/transfers", w.transfer_body(0, 2, 1))
        .await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "STORE_INACTIVE")
    );

    // Other stores are unaffected.
    assert_eq!(w.sell(0, w.rice, 1).await.0, StatusCode::OK);

    // Reactivate and trade again; renaming an inactive store also works.
    assert_eq!(
        w.patch(&uri, json!({ "name": "Closed for now" })).await.0,
        StatusCode::OK
    );
    let (status, _) = w.patch(&uri, json!({ "is_active": true })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(w.sell(2, w.rice, 1).await.0, StatusCode::OK);
    assert_eq!(w.on_hand(2, w.rice).await, d(19));
}

#[tokio::test]
async fn store_edits_follow_store_scoped_permissions() {
    let w = world().await;
    let own_admin = store_user(&w, w.stores[0], ADMIN_ROLE_ID).await;
    let cashier = store_cashier(&w, w.stores[0]).await;
    let patch = |token: String, store: Uuid| {
        let app = w.app.clone();
        async move {
            send(
                &app,
                Method::PATCH,
                &format!("/api/v1/stores/{store}"),
                &token,
                Some(json!({ "address": "New street" })),
            )
            .await
            .0
        }
    };

    // An admin of Location One edits Location One, and only that.
    assert_eq!(patch(own_admin.clone(), w.stores[0]).await, StatusCode::OK);
    assert_eq!(patch(own_admin, w.stores[1]).await, StatusCode::FORBIDDEN);
    assert_eq!(
        patch(cashier.clone(), w.stores[0]).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(patch(w.token.clone(), w.stores[1]).await, StatusCode::OK);

    // Anyone in the tenant can read a store.
    let (status, _) = send(
        &w.app,
        Method::GET,
        &format!("/api/v1/stores/{}", w.stores[1]),
        &cashier,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, one) = w.get(&format!("/api/v1/stores/{}", w.stores[2])).await;
    assert!(one["data"]["address"].is_null());
}

#[tokio::test]
async fn a_store_can_be_given_an_address_when_it_is_created() {
    let w = bare_world().await;
    let (status, body) = w
        .post(
            "/api/v1/stores",
            json!({ "name": "Main", "slug": "main", "address": "  1 High St " }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["address"], "1 High St");
    let (_, body) = w
        .post("/api/v1/stores", json!({ "name": "Bare", "slug": "bare" }))
        .await;
    assert!(body["data"]["address"].is_null());
}

// === Products

#[tokio::test]
async fn a_product_can_be_edited_and_fetched() {
    let w = world().await;
    let uri = format!("/api/v1/products/{}", w.rice);
    let (_, original) = w.get(&uri).await;
    let before = product_updated_at(&w, w.rice).await;

    let (status, body) = w
        .patch(
            &uri,
            json!({ "name": "  Premium Rice  ", "sku": "RICE-PREM", "attributes": { "grade": "A" } }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["name"], "Premium Rice");
    assert_eq!(body["data"]["sku"], "RICE-PREM");
    assert_eq!(body["data"]["attributes"], json!({ "grade": "A" }));
    assert!(product_updated_at(&w, w.rice).await > before);

    // Identity-defining fields did not move.
    assert_eq!(body["data"]["id"], original["data"]["id"]);
    assert_eq!(
        body["data"]["product_type_id"],
        original["data"]["product_type_id"]
    );
    assert_eq!(
        body["data"]["base_unit_id"],
        original["data"]["base_unit_id"]
    );
    assert_eq!(body["data"]["is_active"], true);

    // Absent leaves the SKU, null clears it, and the SKU can be set again.
    let (_, body) = w.patch(&uri, json!({ "name": "Rice" })).await;
    assert_eq!(body["data"]["sku"], "RICE-PREM");
    let (_, body) = w.patch(&uri, json!({ "sku": null })).await;
    assert!(body["data"]["sku"].is_null());
    let (_, body) = w.patch(&uri, json!({ "sku": "RICE-2" })).await;
    assert_eq!(body["data"]["sku"], "RICE-2");

    let (_, fetched) = w.get(&uri).await;
    assert_eq!(fetched["data"]["name"], "Rice");
    let (_, list) = w.get("/api/v1/products").await;
    assert_eq!(list["data"][0]["sku"], "RICE-2");

    // Renaming leaves stock and the price alone.
    w.seed_stock(w.stores[0], w.rice, 5).await;
    w.patch(&uri, json!({ "name": "Renamed again" })).await;
    assert_eq!(w.on_hand(0, w.rice).await, d(5));
    let (_, sale) = w.sell(0, w.rice, 1).await;
    assert_eq!(sale["data"]["total_amount_minor"], 150_000);
}

#[tokio::test]
async fn invalid_product_edits_are_rejected_and_change_nothing() {
    let w = world().await;
    let uri = format!("/api/v1/products/{}", w.rice);
    let product_type: Uuid =
        sqlx::query_scalar("SELECT product_type_id FROM atlas.products WHERE id = $1")
            .bind(w.rice)
            .fetch_one(&w.admin)
            .await
            .unwrap();
    // A second product that already owns a SKU.
    let beans = common::new_product(
        &w.admin,
        w.tenant_id,
        product_type,
        w.piece_unit,
        "Beans",
        1,
    )
    .await;
    w.patch(
        &format!("/api/v1/products/{beans}"),
        json!({ "sku": "BEANS-1" }),
    )
    .await;
    let (_, original) = w.get(&uri).await;

    for (body, expected, expected_code) in [
        (json!({}), StatusCode::BAD_REQUEST, "VALIDATION_FAILED"),
        (
            json!({ "name": "" }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        (
            json!({ "name": "x".repeat(201) }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        (
            json!({ "sku": "   " }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        (
            json!({ "attributes": ["no"] }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        (
            json!({ "sku": "BEANS-1" }),
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
        ),
        (
            json!({ "name": "Half Applied", "sku": "BEANS-1" }),
            StatusCode::CONFLICT,
            "ALREADY_EXISTS",
        ),
    ] {
        let (status, response) = w.patch(&uri, body.clone()).await;
        assert_eq!(status, expected, "{body} -> {response}");
        assert_eq!(code(&response), expected_code, "{body}");
    }

    // Changing what a product fundamentally is must be refused outright.
    for field in ["product_type_id", "base_unit_id", "tenant_id", "id"] {
        let (status, _) = w.patch(&uri, json!({ field: Uuid::now_v7() })).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{field}");
    }

    let (_, after) = w.get(&uri).await;
    assert_eq!(after["data"], original["data"]);

    let ghost = format!("/api/v1/products/{}", Uuid::now_v7());
    assert_eq!(
        w.patch(&ghost, json!({ "name": "Ghost" })).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(w.get(&ghost).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn another_tenants_product_cannot_be_read_or_edited() {
    let w = world().await;
    let other = world().await;
    let uri = format!("/api/v1/products/{}", other.rice);

    assert_eq!(w.get(&uri).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        w.patch(&uri, json!({ "name": "Hijacked" })).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        w.patch(&uri, json!({ "is_active": false })).await.0,
        StatusCode::NOT_FOUND
    );

    let (_, theirs) = other.get(&uri).await;
    assert_eq!(theirs["data"]["name"], "Rice");
    assert_eq!(theirs["data"]["is_active"], true);
}

#[tokio::test]
async fn deactivating_a_product_stops_trading_but_keeps_everything_else() {
    let w = world().await;
    w.seed_rice().await;
    let (_, sold) = w.sell(0, w.rice, 2).await;
    let sale_id = sold["data"]["id"].as_str().unwrap().to_string();
    let uri = format!("/api/v1/products/{}", w.rice);

    let (status, body) = w.patch(&uri, json!({ "is_active": false })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["is_active"], false);

    // Still listed (flagged), stock and history intact, price rule untouched.
    let (_, list) = w.get("/api/v1/products").await;
    assert_eq!(list["data"][0]["is_active"], false);
    assert_eq!(w.total(w.rice).await, d(98));
    assert_eq!(w.count("price_rules").await, 1);
    let (_, lines) = w.get(&format!("/api/v1/sales/{sale_id}/lines")).await;
    assert_eq!(lines["data"][0]["unit_price_amount_minor"], 150_000);

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
    assert_eq!(w.total(w.rice).await, d(98));

    // Write-offs still work, and reactivating restores normal trading.
    let (status, _) = w
        .post(
            "/api/v1/stock/adjustments",
            json!({ "store_id": w.stores[0], "product_id": w.rice, "quantity": -3, "reason": "Spoiled" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = w.patch(&uri, json!({ "is_active": true })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(w.sell(0, w.rice, 1).await.0, StatusCode::OK);
    assert_eq!(w.on_hand(0, w.rice).await, d(44));
}

#[tokio::test]
async fn product_edits_need_tenant_wide_product_manage() {
    let w = world().await;
    let uri = format!("/api/v1/products/{}", w.rice);
    let cashier = store_cashier(&w, w.stores[0]).await;
    // An admin scoped to one store holds product.manage there, but the catalogue is tenant-level.
    let store_admin = store_user(&w, w.stores[0], ADMIN_ROLE_ID).await;

    for token in [&cashier, &store_admin] {
        let (status, _) = send(
            &w.app,
            Method::PATCH,
            &uri,
            token,
            Some(json!({ "name": "Nope" })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
    let (status, _) = send(&w.app, Method::GET, &uri, &cashier, None).await;
    assert_eq!(status, StatusCode::OK);

    let (_, product) = w.get(&uri).await;
    assert_eq!(product["data"]["name"], "Rice");
}
