mod common;

use axum::http::{Method, StatusCode};
use common::{d, send, store_cashier, store_user, world, World, MANAGER_ROLE_ID};
use serde_json::{json, Value};
use uuid::Uuid;

fn code(body: &Value) -> &str {
    body["code"].as_str().unwrap_or_default()
}

fn text(value: &Value) -> String {
    value.as_str().unwrap().to_string()
}

/** Sells `quantity` rice at store `store_index`; returns (sale_id, first line id). */
async fn sell_rice(w: &World, store_index: usize, quantity: i64) -> (String, String) {
    let (status, body) = w.sell(store_index, w.rice, quantity).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    (
        text(&body["data"]["id"]),
        text(&body["data"]["lines"][0]["id"]),
    )
}

async fn return_lines(w: &World, sale: &str, lines: &[(&str, i64)]) -> (StatusCode, Value) {
    w.post(
        &format!("/api/v1/sales/{sale}/returns"),
        json!({
            "reason": "Customer changed their mind",
            "lines": lines.iter().map(|(line, quantity)| json!({
                "sale_line_id": line, "quantity": quantity,
            })).collect::<Vec<_>>(),
        }),
    )
    .await
}

async fn void(w: &World, sale: &str) -> (StatusCode, Value) {
    w.post(
        &format!("/api/v1/sales/{sale}/void"),
        json!({ "reason": "Rung up by mistake" }),
    )
    .await
}

// === Voids

#[tokio::test]
async fn voiding_a_sale_restores_exactly_what_it_took() {
    let w = world().await;
    w.seed_rice().await; // 50 / 30 / 20
    let (sale, _) = sell_rice(&w, 0, 10).await;
    assert_eq!(w.on_hand(0, w.rice).await, d(40));

    let (status, body) = void(&w, &sale).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["sale"]["status"], "voided");
    assert!(body["data"]["sale"]["voided_at"].is_string());
    assert_eq!(body["data"]["refund_amount_minor"], 1_500_000);

    // Only the selling store is touched, and it is back where it started.
    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    assert_eq!(w.on_hand(1, w.rice).await, d(30));
    assert_eq!(w.on_hand(2, w.rice).await, d(20));
    assert_eq!(w.total(w.rice).await, d(100));

    // The original sale rows stay; the void adds reversing rows beside them.
    assert_eq!(w.count_movements("sale").await, 1);
    assert_eq!(w.count_movements("sale_return").await, 1);
    let reason: String = sqlx::query_scalar(
        "SELECT reason FROM atlas.stock_movements WHERE tenant_id = $1 AND movement_type = 'sale_return'",
    )
    .bind(w.tenant_id)
    .fetch_one(&w.admin)
    .await
    .unwrap();
    assert_eq!(reason, "Void: Rung up by mistake");
    assert_eq!(w.count("sale_lines").await, 1);

    let (_, fetched) = w.get(&format!("/api/v1/sales/{sale}")).await;
    assert_eq!(fetched["data"]["status"], "voided");
    assert_eq!(fetched["data"]["lines"].as_array().unwrap().len(), 1);
    w.assert_ledger_matches_balances().await;
}

#[tokio::test]
async fn voiding_a_multi_line_sale_restores_every_line() {
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
    w.seed_stock(w.stores[0], beans, 8).await;

    let (status, sold) = w.sell_lines(0, &[(w.rice, 5), (beans, 3)]).await;
    assert_eq!(status, StatusCode::OK, "{sold}");
    assert_eq!(w.on_hand(0, beans).await, d(5));

    let (status, _) = void(&w, &text(&sold["data"]["id"])).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    assert_eq!(w.on_hand(0, beans).await, d(8));
    w.assert_ledger_matches_balances().await;
}

#[tokio::test]
async fn a_sale_can_only_be_voided_once_and_a_voided_sale_stays_closed() {
    let w = world().await;
    w.seed_rice().await;
    let (sale, line) = sell_rice(&w, 0, 10).await;
    let sale_body = w.sale_body(0, &[(w.rice, 10)]);
    assert_eq!(void(&w, &sale).await.0, StatusCode::OK);

    let (status, body) = void(&w, &sale).await;
    assert_eq!((status, code(&body)), (StatusCode::CONFLICT, "SALE_VOIDED"));
    let (status, body) = return_lines(&w, &sale, &[(&line, 1)]).await;
    assert_eq!((status, code(&body)), (StatusCode::CONFLICT, "SALE_VOIDED"));
    assert_eq!(w.on_hand(0, w.rice).await, d(50), "not credited twice");

    // Re-syncing a voided sale from an offline till must not resurrect or re-deduct it.
    let mut resync = sale_body;
    resync["id"] = json!(sale);
    let (status, body) = w.post("/api/v1/sales", resync).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["status"], "voided");
    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    assert_eq!(w.count("sales").await, 1);
}

#[tokio::test]
async fn a_sale_with_returns_cannot_be_voided() {
    let w = world().await;
    w.seed_rice().await;
    let (sale, line) = sell_rice(&w, 0, 10).await;
    assert_eq!(
        return_lines(&w, &sale, &[(&line, 2)]).await.0,
        StatusCode::OK
    );
    assert_eq!(w.on_hand(0, w.rice).await, d(42));

    let (status, body) = void(&w, &sale).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "SALE_HAS_RETURNS")
    );

    // The refused void changed nothing.
    assert_eq!(w.on_hand(0, w.rice).await, d(42));
    let (_, fetched) = w.get(&format!("/api/v1/sales/{sale}")).await;
    assert_eq!(fetched["data"]["status"], "completed");
    assert_eq!(w.count_movements("sale_return").await, 1);
    w.assert_ledger_matches_balances().await;
}

#[tokio::test]
async fn void_input_is_validated() {
    let w = world().await;
    w.seed_rice().await;
    let (sale, _) = sell_rice(&w, 0, 1).await;

    for body in [json!({ "reason": "" }), json!({ "reason": "   " })] {
        let (status, response) = w.post(&format!("/api/v1/sales/{sale}/void"), body).await;
        assert_eq!(
            (status, code(&response)),
            (StatusCode::BAD_REQUEST, "REASON_REQUIRED")
        );
    }
    let (status, _) = w
        .post(&format!("/api/v1/sales/{sale}/void"), json!({}))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = void(&w, &Uuid::now_v7().to_string()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Another tenant's sale is invisible.
    let other = world().await;
    other.seed_rice().await;
    let (theirs, _) = sell_rice(&other, 0, 1).await;
    assert_eq!(void(&w, &theirs).await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        w.get(&format!("/api/v1/sales/{theirs}")).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, still) = other.get(&format!("/api/v1/sales/{theirs}")).await;
    assert_eq!(still["data"]["status"], "completed");

    // Only a completed sale can be voided or returned.
    sqlx::query("UPDATE atlas.sales SET status = 'held' WHERE id = $1")
        .bind(sale.parse::<Uuid>().unwrap())
        .execute(&w.admin)
        .await
        .unwrap();
    let (status, body) = void(&w, &sale).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "SALE_NOT_COMPLETED")
    );
    assert_eq!(w.on_hand(0, w.rice).await, d(49));
}

#[tokio::test]
async fn the_database_refuses_an_inconsistent_void() {
    let w = world().await;
    w.seed_rice().await;
    let (sale, _) = sell_rice(&w, 0, 1).await;
    let result = sqlx::query("UPDATE atlas.sales SET status = 'voided' WHERE id = $1")
        .bind(sale.parse::<Uuid>().unwrap())
        .execute(&w.admin)
        .await;
    assert!(result.is_err(), "voided needs who, when and why");
}

// === Returns

#[tokio::test]
async fn partial_returns_refund_and_restock_proportionally() {
    let w = world().await;
    w.seed_rice().await;
    let (sale, line) = sell_rice(&w, 0, 10).await; // 1_500_000 total, stock 40

    let (status, first) = return_lines(&w, &sale, &[(&line, 3)]).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["data"]["refund_amount_minor"], 450_000);
    assert_eq!(w.on_hand(0, w.rice).await, d(43));

    let (status, second) = return_lines(&w, &sale, &[(&line, 7)]).await;
    assert_eq!(status, StatusCode::OK, "{second}");
    assert_eq!(second["data"]["refund_amount_minor"], 1_050_000);
    assert_eq!(w.on_hand(0, w.rice).await, d(50));

    // Everything sold has been returned; nothing more can be.
    let (status, body) = return_lines(&w, &sale, &[(&line, 1)]).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "RETURN_EXCEEDS_SALE")
    );
    assert_eq!(w.on_hand(0, w.rice).await, d(50));

    let (_, list) = w.get(&format!("/api/v1/sales/{sale}/returns")).await;
    let rows = list["data"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    let refunded: i64 = rows
        .iter()
        .map(|row| row["refund_amount_minor"].as_i64().unwrap())
        .sum();
    assert_eq!(refunded, 1_500_000);
    assert_eq!(rows[0]["returned_to_stock"], true);

    // The sale itself stays completed with its original price.
    let (_, fetched) = w.get(&format!("/api/v1/sales/{sale}")).await;
    assert_eq!(fetched["data"]["status"], "completed");
    assert_eq!(fetched["data"]["total_amount_minor"], 1_500_000);
    assert_eq!(w.count_movements("sale_return").await, 2);
    w.assert_ledger_matches_balances().await;
}

#[tokio::test]
async fn sale_level_discount_and_tax_are_refunded_proportionally() {
    let w = world().await;
    w.seed_rice().await;

    // Discount: 10 x 150000 - 100000 = 1_400_000.
    let mut discounted = w.sale_body(0, &[(w.rice, 10)]);
    discounted["discount_amount_minor"] = json!(100_000);
    let (_, sold) = w.post("/api/v1/sales", discounted).await;
    assert_eq!(sold["data"]["total_amount_minor"], 1_400_000);
    let (sale, line) = (
        text(&sold["data"]["id"]),
        text(&sold["data"]["lines"][0]["id"]),
    );
    let (_, half) = return_lines(&w, &sale, &[(&line, 5)]).await;
    assert_eq!(half["data"]["refund_amount_minor"], 700_000);
    let (_, rest) = return_lines(&w, &sale, &[(&line, 5)]).await;
    assert_eq!(rest["data"]["refund_amount_minor"], 700_000);

    // Tax: 10 x 150000 + 150000 = 1_650_000, refunded in full on a full return.
    let mut taxed = w.sale_body(0, &[(w.rice, 10)]);
    taxed["tax_amount_minor"] = json!(150_000);
    let (_, sold) = w.post("/api/v1/sales", taxed).await;
    assert_eq!(sold["data"]["total_amount_minor"], 1_650_000);
    let (sale, line) = (
        text(&sold["data"]["id"]),
        text(&sold["data"]["lines"][0]["id"]),
    );
    let (_, all) = return_lines(&w, &sale, &[(&line, 10)]).await;
    assert_eq!(all["data"]["refund_amount_minor"], 1_650_000);
    assert_eq!(w.on_hand(0, w.rice).await, d(50));
}

#[tokio::test]
async fn split_returns_never_refund_more_or_less_than_the_total() {
    let w = world().await;
    let product_type: Uuid =
        sqlx::query_scalar("SELECT product_type_id FROM atlas.products WHERE id = $1")
            .bind(w.rice)
            .fetch_one(&w.admin)
            .await
            .unwrap();
    let cheap = common::new_product(
        &w.admin,
        w.tenant_id,
        product_type,
        w.piece_unit,
        "Cheap",
        100,
    )
    .await;
    w.seed_stock(w.stores[0], cheap, 10).await;

    // 3 x 100 - 1 = 299: it does not divide into three equal minor-unit refunds.
    let mut body = w.sale_body(0, &[(cheap, 3)]);
    body["discount_amount_minor"] = json!(1);
    let (_, sold) = w.post("/api/v1/sales", body).await;
    assert_eq!(sold["data"]["total_amount_minor"], 299);
    let (sale, line) = (
        text(&sold["data"]["id"]),
        text(&sold["data"]["lines"][0]["id"]),
    );

    let mut refunds = Vec::new();
    for _ in 0..3 {
        let (status, receipt) = return_lines(&w, &sale, &[(&line, 1)]).await;
        assert_eq!(status, StatusCode::OK, "{receipt}");
        refunds.push(receipt["data"]["refund_amount_minor"].as_i64().unwrap());
    }
    assert_eq!(refunds.iter().sum::<i64>(), 299, "{refunds:?}");
    assert!(
        refunds.iter().all(|r| (99..=100).contains(r)),
        "{refunds:?}"
    );
}

#[tokio::test]
async fn returns_in_a_sale_unit_restore_base_units() {
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
    let (_, sold) = w.post("/api/v1/sales", sale).await;
    let (sale_id, line) = (
        text(&sold["data"]["id"]),
        text(&sold["data"]["lines"][0]["id"]),
    );
    assert_eq!(w.on_hand(0, w.rice).await, d(26)); // 50 - 24

    // The factor changing afterwards must not change what a return restores.
    sqlx::query("UPDATE atlas.product_sale_units SET factor_to_base = 99 WHERE product_id = $1")
        .bind(w.rice)
        .execute(&w.admin)
        .await
        .unwrap();

    let (status, first) = return_lines(&w, &sale_id, &[(&line, 1)]).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["data"]["refund_amount_minor"], 12 * 150_000);
    assert_eq!(w.on_hand(0, w.rice).await, d(38));
    let (_, last) = return_lines(&w, &sale_id, &[(&line, 1)]).await;
    assert_eq!(last["data"]["refund_amount_minor"], 12 * 150_000);
    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    w.assert_ledger_matches_balances().await;
}

#[tokio::test]
async fn damaged_goods_can_be_refunded_without_restocking() {
    let w = world().await;
    w.seed_rice().await;
    let (sale, line) = sell_rice(&w, 0, 10).await;

    let (status, body) = w
        .post(
            &format!("/api/v1/sales/{sale}/returns"),
            json!({ "reason": "Water damaged", "return_to_stock": false,
                    "lines": [{ "sale_line_id": line, "quantity": 4 }] }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"]["refund_amount_minor"], 600_000);
    assert_eq!(body["data"]["lines"][0]["returned_to_stock"], false);

    assert_eq!(
        w.on_hand(0, w.rice).await,
        d(40),
        "damaged goods are not resold"
    );
    assert_eq!(w.count_movements("sale_return").await, 0);
    // It still counts as returned: it cannot be returned again, nor voided around.
    let (status, body) = return_lines(&w, &sale, &[(&line, 7)]).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "RETURN_EXCEEDS_SALE")
    );
    assert_eq!(void(&w, &sale).await.0, StatusCode::CONFLICT);
    w.assert_ledger_matches_balances().await;
}

#[tokio::test]
async fn invalid_returns_change_nothing() {
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
    w.seed_stock(w.stores[0], beans, 8).await;
    let (_, sold) = w.sell_lines(0, &[(w.rice, 10), (beans, 3)]).await;
    let sale = text(&sold["data"]["id"]);
    let lines = sold["data"]["lines"].as_array().unwrap();
    let rice_line = text(
        &lines
            .iter()
            .find(|l| l["product_id"] == json!(w.rice))
            .unwrap()["id"],
    );
    let beans_line = text(
        &lines
            .iter()
            .find(|l| l["product_id"] == json!(beans))
            .unwrap()["id"],
    );
    let (other_sale, other_line) = sell_rice(&w, 1, 2).await;
    let after_sales = (w.on_hand(0, w.rice).await, w.on_hand(0, beans).await);
    let uri = format!("/api/v1/sales/{sale}/returns");

    let cases: Vec<(Value, StatusCode, &str)> = vec![
        (
            json!({ "reason": "x", "lines": [] }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        (
            json!({ "reason": "  ", "lines": [{ "sale_line_id": rice_line, "quantity": 1 }] }),
            StatusCode::BAD_REQUEST,
            "REASON_REQUIRED",
        ),
        (
            json!({ "reason": "x", "lines": [{ "sale_line_id": rice_line, "quantity": 0 }] }),
            StatusCode::BAD_REQUEST,
            "INVALID_QUANTITY",
        ),
        (
            json!({ "reason": "x", "lines": [{ "sale_line_id": rice_line, "quantity": -1 }] }),
            StatusCode::BAD_REQUEST,
            "INVALID_QUANTITY",
        ),
        (
            json!({ "reason": "x", "lines": [{ "sale_line_id": rice_line, "quantity": 1 }, { "sale_line_id": rice_line, "quantity": 1 }] }),
            StatusCode::BAD_REQUEST,
            "VALIDATION_FAILED",
        ),
        (
            json!({ "reason": "x", "lines": [{ "sale_line_id": rice_line, "quantity": 11 }] }),
            StatusCode::CONFLICT,
            "RETURN_EXCEEDS_SALE",
        ),
        // Valid rice line next to an over-returned beans line: all or nothing.
        (
            json!({ "reason": "x", "lines": [{ "sale_line_id": rice_line, "quantity": 5 }, { "sale_line_id": beans_line, "quantity": 4 }] }),
            StatusCode::CONFLICT,
            "RETURN_EXCEEDS_SALE",
        ),
    ];
    for (body, expected, expected_code) in cases {
        let (status, response) = w.post(&uri, body.clone()).await;
        assert_eq!(status, expected, "{body} -> {response}");
        assert_eq!(code(&response), expected_code, "{body}");
    }

    // A line that belongs to a different sale, and a line that does not exist.
    for bad_line in [other_line.clone(), Uuid::now_v7().to_string()] {
        let (status, _) = return_lines(&w, &sale, &[(&bad_line, 1)]).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{bad_line}");
    }
    let (status, _) = return_lines(&w, &Uuid::now_v7().to_string(), &[(&rice_line, 1)]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let _ = other_sale;

    assert_eq!(
        (w.on_hand(0, w.rice).await, w.on_hand(0, beans).await),
        after_sales
    );
    assert_eq!(w.count("sale_returns").await, 0);
    assert_eq!(w.count_movements("sale_return").await, 0);
}

#[tokio::test]
async fn a_return_can_span_several_lines() {
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
    w.seed_stock(w.stores[0], beans, 8).await;
    let (_, sold) = w.sell_lines(0, &[(w.rice, 10), (beans, 3)]).await;
    let sale = text(&sold["data"]["id"]);
    let lines = sold["data"]["lines"].as_array().unwrap();
    let pick = |product: Uuid| {
        text(
            &lines
                .iter()
                .find(|l| l["product_id"] == json!(product))
                .unwrap()["id"],
        )
    };

    let (status, receipt) = return_lines(&w, &sale, &[(&pick(w.rice), 4), (&pick(beans), 3)]).await;
    assert_eq!(status, StatusCode::OK, "{receipt}");
    assert_eq!(
        receipt["data"]["refund_amount_minor"],
        4 * 150_000 + 3 * 90_000
    );
    assert_eq!(receipt["data"]["lines"].as_array().unwrap().len(), 2);
    // One return, one shared id across its lines and its ledger rows.
    let ids: Vec<&Value> = receipt["data"]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| &l["return_id"])
        .collect();
    assert_eq!(ids[0], ids[1]);
    assert_eq!(ids[0], &receipt["data"]["return_id"]);
    assert_eq!(w.on_hand(0, w.rice).await, d(44));
    assert_eq!(w.on_hand(0, beans).await, d(8));
    w.assert_ledger_matches_balances().await;
}

// === Permissions

#[tokio::test]
async fn voids_and_returns_follow_store_scoped_permissions() {
    let w = world().await;
    w.seed_rice().await;
    let (sale_here, line_here) = sell_rice(&w, 0, 4).await;
    let (sale_there, line_there) = sell_rice(&w, 1, 4).await;
    let cashier = store_cashier(&w, w.stores[0]).await;
    let manager = store_user(&w, w.stores[0], MANAGER_ROLE_ID).await;
    let body_return =
        |line: &str| json!({ "reason": "r", "lines": [{ "sale_line_id": line, "quantity": 1 }] });

    // A cashier can ring sales up but not undo them.
    let (status, _) = send(
        &w.app,
        Method::POST,
        &format!("/api/v1/sales/{sale_here}/void"),
        &cashier,
        Some(json!({ "reason": "r" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = send(
        &w.app,
        Method::POST,
        &format!("/api/v1/sales/{sale_here}/returns"),
        &cashier,
        Some(body_return(&line_here)),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // A manager can, at their own store only.
    let (status, _) = send(
        &w.app,
        Method::POST,
        &format!("/api/v1/sales/{sale_there}/void"),
        &manager,
        Some(json!({ "reason": "r" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = send(
        &w.app,
        Method::POST,
        &format!("/api/v1/sales/{sale_there}/returns"),
        &manager,
        Some(body_return(&line_there)),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = send(
        &w.app,
        Method::POST,
        &format!("/api/v1/sales/{sale_here}/returns"),
        &manager,
        Some(body_return(&line_here)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(
        &w.app,
        Method::POST,
        &format!("/api/v1/sales/{sale_there}/void"),
        &w.token,
        Some(json!({ "reason": "owner" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // Refused attempts moved no stock: only the manager's 1 unit and the owner's void.
    assert_eq!(w.on_hand(0, w.rice).await, d(47));
    assert_eq!(w.on_hand(1, w.rice).await, d(30));
}

// === Concurrency

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_returns_cannot_return_more_than_was_sold() {
    let w = world().await;
    w.seed_rice().await;
    let (sale, line) = sell_rice(&w, 0, 10).await; // stock 40

    // Six returns of 2 against 10 sold: exactly five may succeed.
    let mut tasks = Vec::new();
    for _ in 0..6 {
        let (app, token, uri) = (
            w.app.clone(),
            w.token.clone(),
            format!("/api/v1/sales/{sale}/returns"),
        );
        let body = json!({ "reason": "r", "lines": [{ "sale_line_id": line, "quantity": 2 }] });
        tasks.push(tokio::spawn(async move {
            send(&app, Method::POST, &uri, &token, Some(body)).await
        }));
    }
    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await.unwrap());
    }
    let ok = results.iter().filter(|(s, _)| *s == StatusCode::OK).count();
    for (status, body) in results.iter().filter(|(s, _)| *s != StatusCode::OK) {
        assert_eq!(
            (*status, code(body)),
            (StatusCode::CONFLICT, "RETURN_EXCEEDS_SALE")
        );
    }
    assert_eq!(ok, 5);
    assert_eq!(w.on_hand(0, w.rice).await, d(50));
    let refunded: i64 = sqlx::query_scalar(
        "SELECT SUM(refund_amount_minor)::bigint FROM atlas.sale_returns WHERE tenant_id = $1",
    )
    .bind(w.tenant_id)
    .fetch_one(&w.admin)
    .await
    .unwrap();
    assert_eq!(refunded, 1_500_000);
    w.assert_ledger_matches_balances().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_void_racing_a_return_never_double_credits_stock() {
    let w = world().await;
    w.seed_rice().await;

    for round in 0..6 {
        let (sale, line) = sell_rice(&w, 0, 10).await;
        let (app, token) = (w.app.clone(), w.token.clone());
        let void_task = {
            let (app, token, uri) = (
                app.clone(),
                token.clone(),
                format!("/api/v1/sales/{sale}/void"),
            );
            tokio::spawn(async move {
                send(
                    &app,
                    Method::POST,
                    &uri,
                    &token,
                    Some(json!({ "reason": "race" })),
                )
                .await
            })
        };
        let return_task = {
            let uri = format!("/api/v1/sales/{sale}/returns");
            let body =
                json!({ "reason": "race", "lines": [{ "sale_line_id": line, "quantity": 10 }] });
            tokio::spawn(async move { send(&app, Method::POST, &uri, &token, Some(body)).await })
        };
        let (void_status, void_body) = void_task.await.unwrap();
        let (return_status, return_body) = return_task.await.unwrap();

        // Exactly one wins, whichever order they ran in.
        let wins = [void_status, return_status]
            .iter()
            .filter(|s| **s == StatusCode::OK)
            .count();
        assert_eq!(
            wins, 1,
            "round {round}: void {void_status} {void_body} / return {return_status} {return_body}"
        );
        assert_eq!(
            w.on_hand(0, w.rice).await,
            d(50),
            "round {round}: credited exactly once"
        );
    }
    w.assert_ledger_matches_balances().await;
}
