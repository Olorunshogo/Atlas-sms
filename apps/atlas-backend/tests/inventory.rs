mod common;

use axum::http::{Method, StatusCode};
use common::{d, dec, send, store_cashier, world};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use uuid::Uuid;

fn code(body: &Value) -> &str {
    body["code"].as_str().unwrap_or_default()
}

// === Test 1: initial inventory

#[tokio::test]
async fn initial_inventory_is_independent_per_store() {
    let w = world().await;
    w.seed_rice().await;

    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    assert_eq!(w.on_hand(1, w.rice).await, d(30));
    assert_eq!(w.on_hand(2, w.rice).await, d(20));
    assert_eq!(w.total(w.rice).await, d(100));

    let (_, body) = w.get(&format!("/api/v1/stock/products/{}", w.rice)).await;
    let locations = body["data"]["locations"].as_array().unwrap();
    assert_eq!(locations.len(), 3);
    assert_eq!(locations[0]["store_name"], "Location One");
    assert_eq!(dec(&locations[0]["quantity"]), d(50));

    // One initial_stock ledger row per store.
    assert_eq!(w.count_movements("initial_stock").await, 3);
}

#[tokio::test]
async fn adding_the_same_product_to_a_store_twice_is_rejected() {
    let w = world().await;
    w.seed_stock(w.stores[0], w.rice, 50).await;

    let (status, body) = w
        .post(
            "/api/v1/stock/inventory",
            json!({ "store_id": w.stores[0], "product_id": w.rice, "quantity": 5 }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(code(&body), "INVENTORY_EXISTS");
    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    assert_eq!(w.count_movements("initial_stock").await, 1);
}

#[tokio::test]
async fn initial_stock_validates_input() {
    let w = world().await;
    let unknown = Uuid::now_v7();

    let (status, _) = w
        .post(
            "/api/v1/stock/inventory",
            json!({ "store_id": w.stores[0], "product_id": w.rice, "quantity": -1 }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = w
        .post(
            "/api/v1/stock/inventory",
            json!({ "store_id": w.stores[0], "product_id": unknown, "quantity": 5 }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    assert_eq!(w.count("stock_levels").await, 0);
}

// === Test 2: branch-specific sale

#[tokio::test]
async fn sale_reduces_only_the_selling_store() {
    let w = world().await;
    w.seed_rice().await;

    let (status, body) = w.sell(0, w.rice, 10).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(w.on_hand(0, w.rice).await, d(40));
    assert_eq!(w.on_hand(1, w.rice).await, d(30));
    assert_eq!(w.on_hand(2, w.rice).await, d(20));
    assert_eq!(w.total(w.rice).await, d(90));

    assert_eq!(w.count_movements("sale").await, 1);
}

#[tokio::test]
async fn sale_price_comes_from_the_server_and_history_survives_a_price_change() {
    let w = world().await;
    w.seed_rice().await;

    // The request sends unit_price_amount_minor = 1; the server must use the price rule (150000).
    let (status, body) = w.sell(0, w.rice, 2).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["total_amount_minor"], 300_000);
    assert_eq!(body["data"]["lines"][0]["unit_price_amount_minor"], 150_000);
    let sale_id = body["data"]["id"].as_str().unwrap().to_string();

    sqlx::query(
        "UPDATE atlas.price_rules SET unit_price_amount_minor = 200000 WHERE product_id = $1",
    )
    .bind(w.rice)
    .execute(&w.admin)
    .await
    .unwrap();

    let (_, lines) = w.get(&format!("/api/v1/sales/{sale_id}/lines")).await;
    assert_eq!(lines["data"][0]["unit_price_amount_minor"], 150_000);
    assert_eq!(lines["data"][0]["line_total_amount_minor"], 300_000);

    let (_, body) = w.sell(0, w.rice, 1).await;
    assert_eq!(body["data"]["total_amount_minor"], 200_000);
}

#[tokio::test]
async fn product_without_a_price_cannot_be_sold() {
    let w = world().await;
    w.seed_rice().await;
    sqlx::query("DELETE FROM atlas.price_rules WHERE product_id = $1")
        .bind(w.rice)
        .execute(&w.admin)
        .await
        .unwrap();

    let (status, body) = w.sell(0, w.rice, 1).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(code(&body), "NO_PRICE");
    assert_eq!(w.on_hand(0, w.rice).await, d(50));
}

#[tokio::test]
async fn sale_in_a_larger_unit_deducts_base_units() {
    let w = world().await;
    w.seed_rice().await;
    let carton = common::new_unit(&w.admin, w.tenant_id, "carton").await;
    sqlx::query("INSERT INTO atlas.product_sale_units (id, tenant_id, product_id, unit_id, factor_to_base) VALUES ($1, $2, $3, $4, 12)")
        .bind(Uuid::now_v7())
        .bind(w.tenant_id)
        .bind(w.rice)
        .bind(carton)
        .execute(&w.admin)
        .await
        .unwrap();

    let mut sale = w.sale_body(0, &[(w.rice, 2)]);
    sale["lines"][0]["unit_id"] = json!(carton);
    let (status, body) = w.post("/api/v1/sales", sale).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // 2 cartons x 12 pieces = 24 pieces, billed at 150000 each.
    assert_eq!(w.on_hand(0, w.rice).await, d(26));
    assert_eq!(body["data"]["total_amount_minor"], 24 * 150_000);

    // A unit the product does not declare is rejected.
    let other = common::new_unit(&w.admin, w.tenant_id, "pallet").await;
    let mut bad = w.sale_body(0, &[(w.rice, 1)]);
    bad["lines"][0]["unit_id"] = json!(other);
    let (status, body) = w.post("/api/v1/sales", bad).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(code(&body), "INVALID_UNIT");
}

#[tokio::test]
async fn repeating_a_sale_id_does_not_deduct_twice() {
    let w = world().await;
    w.seed_rice().await;

    let sale = w.sale_body(0, &[(w.rice, 10)]);
    let (first, _) = w.post("/api/v1/sales", sale.clone()).await;
    let (second, _) = w.post("/api/v1/sales", sale).await;
    assert_eq!((first, second), (StatusCode::OK, StatusCode::OK));

    assert_eq!(w.on_hand(0, w.rice).await, d(40));
    assert_eq!(w.count("sales").await, 1);
    assert_eq!(w.count_movements("sale").await, 1);
}

// === Test 3: insufficient stock

#[tokio::test]
async fn selling_more_than_available_fails_cleanly() {
    let w = world().await;
    w.seed_rice().await;
    let movements_before = w.count("stock_movements").await;

    let (status, body) = w.sell(0, w.rice, 51).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(code(&body), "INSUFFICIENT_STOCK");

    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    assert_eq!(w.total(w.rice).await, d(100));
    assert_eq!(w.count("sales").await, 0);
    assert_eq!(w.count("sale_lines").await, 0);
    assert_eq!(w.count_movements("sale").await, 0);
    assert_eq!(w.count("stock_movements").await, movements_before);
}

#[tokio::test]
async fn selling_a_product_with_no_inventory_record_fails() {
    let w = world().await;
    // Stock exists at store 0 only; store 1 has no row for rice at all.
    w.seed_stock(w.stores[0], w.rice, 50).await;

    let (status, body) = w.sell(1, w.rice, 1).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(code(&body), "INSUFFICIENT_STOCK");
    assert_eq!(w.count("sales").await, 0);
}

#[tokio::test]
async fn invalid_sales_are_rejected() {
    let w = world().await;
    w.seed_rice().await;

    for quantity in [0, -5] {
        let (status, body) = w.sell(0, w.rice, quantity).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "quantity {quantity}");
        assert_eq!(code(&body), "INVALID_QUANTITY");
    }

    let (status, _) = w.sell(0, Uuid::now_v7(), 1).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let mut empty = w.sale_body(0, &[(w.rice, 1)]);
    empty["lines"] = json!([]);
    let (status, _) = w.post("/api/v1/sales", empty).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A shift that belongs to a different store.
    let mut wrong_shift = w.sale_body(0, &[(w.rice, 1)]);
    wrong_shift["shift_id"] = json!(w.shifts[1]);
    let (status, _) = w.post("/api/v1/sales", wrong_shift).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    assert_eq!(w.count("sales").await, 0);
    assert_eq!(w.on_hand(0, w.rice).await, d(50));
}

// === Test 4: stock transfer

#[tokio::test]
async fn transfer_moves_stock_between_stores() {
    let w = world().await;
    w.seed_rice().await;
    assert_eq!(w.sell(0, w.rice, 10).await.0, StatusCode::OK);

    let (status, body) = w
        .post("/api/v1/stock/transfers", w.transfer_body(0, 1, 10))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(w.on_hand(0, w.rice).await, d(30));
    assert_eq!(w.on_hand(1, w.rice).await, d(40));
    assert_eq!(w.on_hand(2, w.rice).await, d(20));
    assert_eq!(w.total(w.rice).await, d(90));

    let out = &body["data"]["out_movement"];
    let inn = &body["data"]["in_movement"];
    assert_eq!(dec(&out["quantity"]), d(-10));
    assert_eq!(dec(&inn["quantity"]), d(10));
    assert_eq!(out["reference_id"], inn["reference_id"]);
}

#[tokio::test]
async fn transfer_to_a_store_without_a_record_creates_it() {
    let w = world().await;
    w.seed_stock(w.stores[0], w.rice, 40).await;

    let (status, body) = w
        .post("/api/v1/stock/transfers", w.transfer_body(0, 2, 15))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(w.on_hand(0, w.rice).await, d(25));
    assert_eq!(w.on_hand(2, w.rice).await, d(15));
    assert_eq!(w.total(w.rice).await, d(40));
}

// === Test 5: transfer validation

#[tokio::test]
async fn invalid_transfers_change_nothing() {
    let w = world().await;
    w.seed_rice().await;
    let movements_before = w.count("stock_movements").await;

    let cases: Vec<(Value, StatusCode, &str)> = vec![
        (
            w.transfer_body(0, 0, 5),
            StatusCode::BAD_REQUEST,
            "SAME_STORE",
        ),
        (
            w.transfer_body(0, 1, 0),
            StatusCode::BAD_REQUEST,
            "INVALID_QUANTITY",
        ),
        (
            w.transfer_body(0, 1, -5),
            StatusCode::BAD_REQUEST,
            "INVALID_QUANTITY",
        ),
        (
            w.transfer_body(0, 1, 51),
            StatusCode::CONFLICT,
            "INSUFFICIENT_STOCK",
        ),
        (
            {
                let mut body = w.transfer_body(0, 1, 5);
                body["reason"] = json!("   ");
                body
            },
            StatusCode::BAD_REQUEST,
            "REASON_REQUIRED",
        ),
    ];
    for (body, expected_status, expected_code) in cases {
        let (status, response) = w.post("/api/v1/stock/transfers", body.clone()).await;
        assert_eq!(status, expected_status, "{body}");
        assert_eq!(code(&response), expected_code, "{body}");
    }

    /*
     * Nonexistent store / product. The owner's tenant-wide grant passes the
     * permission check, so these reach the existence check and are 404s.
     */
    let mut unknown_store = w.transfer_body(0, 1, 5);
    unknown_store["to_store_id"] = json!(Uuid::now_v7());
    let (status, _) = w.post("/api/v1/stock/transfers", unknown_store).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let mut unknown_product = w.transfer_body(0, 1, 5);
    unknown_product["product_id"] = json!(Uuid::now_v7());
    let (status, _) = w.post("/api/v1/stock/transfers", unknown_product).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    assert_eq!(w.on_hand(1, w.rice).await, d(30));
    assert_eq!(w.on_hand(2, w.rice).await, d(20));
    assert_eq!(w.count("stock_movements").await, movements_before);
}

// === Test 6: restock

#[tokio::test]
async fn restock_adds_to_one_store_and_is_recorded() {
    let w = world().await;
    w.seed_stock(w.stores[0], w.rice, 30).await;
    w.seed_stock(w.stores[1], w.rice, 30).await;
    w.seed_stock(w.stores[2], w.rice, 20).await;

    let (status, body) = w
        .post(
            "/api/v1/stock/restocks",
            json!({ "store_id": w.stores[0], "product_id": w.rice, "quantity": 50, "reason": "Delivery note 17" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["movement_type"], "purchase_receipt");

    assert_eq!(w.on_hand(0, w.rice).await, d(80));
    assert_eq!(w.on_hand(1, w.rice).await, d(30));
    assert_eq!(w.on_hand(2, w.rice).await, d(20));
    assert_eq!(w.total(w.rice).await, d(130));
    assert_eq!(w.count_movements("purchase_receipt").await, 1);

    let (status, _) = w
        .post(
            "/api/v1/stock/restocks",
            json!({ "store_id": w.stores[0], "product_id": w.rice, "quantity": 0 }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn adjustments_need_a_reason_and_cannot_go_negative() {
    let w = world().await;
    w.seed_stock(w.stores[0], w.rice, 10).await;
    let adjust = |quantity: i64, reason: &str| json!({ "store_id": w.stores[0], "product_id": w.rice, "quantity": quantity, "reason": reason });

    let (status, body) = w
        .post(
            "/api/v1/stock/adjustments",
            adjust(-3, "Damaged in storage"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(w.on_hand(0, w.rice).await, d(7));

    let (status, body) = w
        .post("/api/v1/stock/adjustments", adjust(-8, "Miscount"))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(code(&body), "INSUFFICIENT_STOCK");

    let (status, body) = w.post("/api/v1/stock/adjustments", adjust(5, "")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(code(&body), "REASON_REQUIRED");

    let (status, _) = w
        .post("/api/v1/stock/adjustments", adjust(0, "Nothing"))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    assert_eq!(w.on_hand(0, w.rice).await, d(7));
    assert_eq!(w.count_movements("adjustment").await, 1);
}

// === Test 7: concurrent sales

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_sales_never_oversell() {
    let w = world().await;
    w.seed_stock(w.stores[0], w.rice, 10).await;

    // 8 sales of 2 = 16 requested against 10 available: exactly 5 may win.
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let app = w.app.clone();
        let token = w.token.clone();
        let body = w.sale_body(0, &[(w.rice, 2)]);
        tasks.push(tokio::spawn(async move {
            send(&app, Method::POST, "/api/v1/sales", &token, Some(body)).await
        }));
    }
    let mut ok = 0;
    let mut rejected = 0;
    for task in tasks {
        let (status, body) = task.await.unwrap();
        match status {
            StatusCode::OK => ok += 1,
            StatusCode::CONFLICT => {
                assert_eq!(code(&body), "INSUFFICIENT_STOCK");
                rejected += 1;
            }
            other => panic!("unexpected status {other}: {body}"),
        }
    }

    assert_eq!((ok, rejected), (5, 3));
    assert_eq!(w.on_hand(0, w.rice).await, d(0));
    assert_eq!(w.count("sales").await, 5);
    assert_eq!(w.count("sale_lines").await, 5);
    assert_eq!(w.count_movements("sale").await, 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn opposing_concurrent_transfers_neither_deadlock_nor_leak_stock() {
    let w = world().await;
    w.seed_stock(w.stores[0], w.rice, 100).await;
    w.seed_stock(w.stores[1], w.rice, 100).await;

    let mut tasks = Vec::new();
    for i in 0..20 {
        let app = w.app.clone();
        let token = w.token.clone();
        let (from, to) = if i % 2 == 0 { (0, 1) } else { (1, 0) };
        let body = w.transfer_body(from, to, 3);
        tasks.push(tokio::spawn(async move {
            send(
                &app,
                Method::POST,
                "/api/v1/stock/transfers",
                &token,
                Some(body),
            )
            .await
        }));
    }
    for task in tasks {
        let (status, body) = task.await.unwrap();
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    // Ten 3-unit transfers each way cancel out.
    assert_eq!(w.on_hand(0, w.rice).await, d(100));
    assert_eq!(w.on_hand(1, w.rice).await, d(100));
    assert_eq!(w.total(w.rice).await, d(200));
}

// === Test 8: transaction rollback

#[tokio::test]
async fn a_failing_line_rolls_back_the_whole_sale() {
    let w = world().await;
    w.seed_rice().await;

    let product_type: Uuid =
        sqlx::query_scalar("SELECT product_type_id FROM atlas.products WHERE id = $1")
            .bind(w.rice)
            .fetch_one(&w.admin)
            .await
            .unwrap();
    let beans = common::new_product(
        &w.admin,
        w.tenant_id,
        product_type,
        w.piece_unit,
        "Beans",
        90_000,
    )
    .await;
    w.seed_stock(w.stores[0], beans, 3).await;
    let movements_before = w.count("stock_movements").await;

    // Rice would succeed, beans (5 > 3) fails: nothing may persist, including rice's deduction.
    let (status, body) = w.sell_lines(0, &[(w.rice, 10), (beans, 5)]).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    assert_eq!(w.on_hand(0, beans).await, d(3));
    assert_eq!(w.count("sales").await, 0);
    assert_eq!(w.count("sale_lines").await, 0);
    assert_eq!(w.count("stock_movements").await, movements_before);

    // The same basket with a valid beans quantity goes through whole.
    let (status, _) = w.sell_lines(0, &[(w.rice, 10), (beans, 3)]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(w.on_hand(0, w.rice).await, d(40));
    assert_eq!(w.on_hand(0, beans).await, d(0));
}

#[tokio::test]
async fn database_refuses_negative_stock_directly() {
    let w = world().await;
    w.seed_stock(w.stores[0], w.rice, 5).await;

    let result = sqlx::query("UPDATE atlas.stock_levels SET quantity = -1 WHERE product_id = $1")
        .bind(w.rice)
        .execute(&w.admin)
        .await;
    assert!(result.is_err(), "CHECK (quantity >= 0) must reject this");
    assert_eq!(w.on_hand(0, w.rice).await, d(5));
}

// === Test 9: consistency

#[tokio::test]
async fn totals_match_branch_balances_and_the_ledger_after_mixed_operations() {
    let w = world().await;
    w.seed_rice().await; // 50 / 30 / 20
    assert_eq!(w.sell(0, w.rice, 10).await.0, StatusCode::OK); // 40 / 30 / 20
    assert_eq!(
        w.post("/api/v1/stock/transfers", w.transfer_body(0, 1, 10))
            .await
            .0,
        StatusCode::OK
    ); // 30 / 40 / 20
    assert_eq!(
        w.post(
            "/api/v1/stock/restocks",
            json!({ "store_id": w.stores[2], "product_id": w.rice, "quantity": 50 })
        )
        .await
        .0,
        StatusCode::OK
    ); // 30 / 40 / 70
    assert_eq!(w.sell(1, w.rice, 5).await.0, StatusCode::OK); // 30 / 35 / 70
    assert_eq!(w.sell(0, w.rice, 99).await.0, StatusCode::CONFLICT); // rejected, no change

    assert_eq!(w.on_hand(0, w.rice).await, d(30));
    assert_eq!(w.on_hand(1, w.rice).await, d(35));
    assert_eq!(w.on_hand(2, w.rice).await, d(70));
    assert_eq!(w.total(w.rice).await, d(135));

    /*
     * Total from the API == sum of balance rows == sum of the ledger, and
     * each store's balance == the sum of that store's ledger rows.
     */
    let balances: Decimal =
        sqlx::query_scalar("SELECT SUM(quantity) FROM atlas.stock_levels WHERE tenant_id = $1")
            .bind(w.tenant_id)
            .fetch_one(&w.admin)
            .await
            .unwrap();
    let ledger: Decimal =
        sqlx::query_scalar("SELECT SUM(quantity) FROM atlas.stock_movements WHERE tenant_id = $1")
            .bind(w.tenant_id)
            .fetch_one(&w.admin)
            .await
            .unwrap();
    assert_eq!(balances, d(135));
    assert_eq!(ledger, d(135));

    let drifted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM atlas.stock_levels sl
         WHERE sl.tenant_id = $1
           AND sl.quantity <> (SELECT COALESCE(SUM(m.quantity), 0) FROM atlas.stock_movements m
                               WHERE m.store_id = sl.store_id AND m.product_id = sl.product_id)",
    )
    .bind(w.tenant_id)
    .fetch_one(&w.admin)
    .await
    .unwrap();
    assert_eq!(drifted, 0);

    // The acceptance example: 20 / 15 / 8 shown independently.
    let other = common::new_product(
        &w.admin,
        w.tenant_id,
        sqlx::query_scalar("SELECT product_type_id FROM atlas.products WHERE id = $1")
            .bind(w.rice)
            .fetch_one(&w.admin)
            .await
            .unwrap(),
        w.piece_unit,
        "Sugar",
        50_000,
    )
    .await;
    for (store, quantity) in w.stores.iter().zip([20, 15, 8]) {
        w.seed_stock(*store, other, quantity).await;
    }
    assert_eq!(w.sell(1, other, 5).await.0, StatusCode::OK);
    assert_eq!(w.on_hand(0, other).await, d(20));
    assert_eq!(w.on_hand(1, other).await, d(10));
    assert_eq!(w.on_hand(2, other).await, d(8));
    assert_eq!(w.total(other).await, d(38));
}

// === Reads

#[tokio::test]
async fn inventory_listing_filters_and_movement_history() {
    let w = world().await;
    w.seed_rice().await;
    sqlx::query(
        "UPDATE atlas.product_store_listings SET low_stock_threshold = 25 WHERE product_id = $1",
    )
    .bind(w.rice)
    .execute(&w.admin)
    .await
    .unwrap();

    let (_, body) = w.get("/api/v1/stock/inventory").await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(dec(&body["data"][0]["total_quantity"]), d(100));

    // Only the store at 20 is at or below the threshold of 25.
    let (_, body) = w.get("/api/v1/stock/inventory?low_stock=true").await;
    let locations = body["data"][0]["locations"].as_array().unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0]["store_name"], "Location Three");

    let (_, body) = w
        .get(&format!(
            "/api/v1/stock/on-hand?store_id={}&low_stock=true",
            w.stores[2]
        ))
        .await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    let (_, body) = w
        .get(&format!(
            "/api/v1/stock/on-hand?store_id={}&low_stock=true",
            w.stores[0]
        ))
        .await;
    assert!(body["data"].as_array().unwrap().is_empty());

    assert_eq!(w.sell(0, w.rice, 4).await.0, StatusCode::OK);
    let (_, body) = w
        .get(&format!(
            "/api/v1/stock/movements?store_id={}&limit=1",
            w.stores[0]
        ))
        .await;
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["movement_type"], "sale");
    assert_eq!(dec(&rows[0]["quantity"]), d(-4));
    assert_eq!(rows[0]["created_by_user_id"], json!(w.owner_id));

    let (status, _) = w
        .get(&format!("/api/v1/stock/products/{}", Uuid::now_v7()))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// === Access control

#[tokio::test]
async fn store_scoped_staff_only_act_on_and_see_their_own_store() {
    let w = world().await;
    w.seed_rice().await;
    let cashier = store_cashier(&w, w.stores[0]).await;

    let (status, body) = send(
        &w.app,
        Method::POST,
        "/api/v1/sales",
        &cashier,
        Some(w.sale_body(0, &[(w.rice, 1)])),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, _) = send(
        &w.app,
        Method::POST,
        "/api/v1/sales",
        &cashier,
        Some(w.sale_body(1, &[(w.rice, 1)])),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(w.on_hand(1, w.rice).await, d(30));

    // A cashier cannot restock, even at their own store.
    let (status, _) = send(
        &w.app,
        Method::POST,
        "/api/v1/stock/restocks",
        &cashier,
        Some(json!({ "store_id": w.stores[0], "product_id": w.rice, "quantity": 5 })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Cross-store views are filtered to the one store they may see.
    let (_, body) = send(
        &w.app,
        Method::GET,
        &format!("/api/v1/stock/products/{}", w.rice),
        &cashier,
        None,
    )
    .await;
    assert_eq!(body["data"]["locations"].as_array().unwrap().len(), 1);
    assert_eq!(dec(&body["data"]["total_quantity"]), d(49));

    let (status, _) = send(
        &w.app,
        Method::GET,
        &format!("/api/v1/stock/on-hand?store_id={}", w.stores[1]),
        &cashier,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn another_tenants_ids_are_not_found() {
    let w = world().await;
    let other = world().await;
    other.seed_stock(other.stores[0], other.rice, 10).await;

    // w's owner naming other's store and product: foreign ids look nonexistent.
    let (status, _) = w
        .post(
            "/api/v1/stock/restocks",
            json!({ "store_id": other.stores[0], "product_id": w.rice, "quantity": 5 }),
        )
        .await;
    assert_ne!(status, StatusCode::OK);
    let (status, _) = w
        .post(
            "/api/v1/stock/inventory",
            json!({ "store_id": w.stores[0], "product_id": other.rice, "quantity": 5 }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(other.on_hand(0, other.rice).await, d(10));
}
