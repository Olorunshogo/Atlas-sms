//! Cross-store stock reads: per-product totals, the low-stock filter, store-scoped visibility and the movement audit trail.

mod common;

use axum::http::StatusCode;
use common::*;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn qty(value: &Value) -> Decimal {
    return value.as_str().unwrap().parse().unwrap();
}

fn names(body: &Value) -> Vec<String> {
    return body["data"].as_array().unwrap().iter().map(|row| row["product_name"].as_str().unwrap().to_string()).collect();
}

/// The acceptance example: rice at 20 / 15 / 8 across three branches.
async fn three_branches(router: &axum::Router, pool: &PgPool, email: &str) -> (String, Uuid, [Uuid; 3], Uuid) {
    let (owner, tenant_id) = signup_owner(router, pool, email).await;
    let lagos = create_store(router, &owner, "lagos").await;
    let abuja = create_store(router, &owner, "abuja").await;
    let kaduna = create_store(router, &owner, "kaduna").await;
    let unit = seed_unit(pool, tenant_id, "Bag", "bag").await;
    let rice = create_product(router, &owner, unit, "Rice", "RICE-1").await;
    for (store, quantity) in [(lagos, "20"), (abuja, "15"), (kaduna, "8")] {
        stock_up(router, &owner, store, rice, quantity).await;
    }
    return (owner, tenant_id, [lagos, abuja, kaduna], rice);
}

#[sqlx::test]
async fn a_product_shows_its_stock_per_store_and_a_derived_total(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id, [lagos, abuja, kaduna], rice) = three_branches(&router, &pool, "read-owner@example.com").await;

    let (status, body) = get(&router, &format!("/api/v1/stock/products/{}", rice), &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let data = &body["data"];
    assert_eq!(data["product_name"], "Rice");
    assert_eq!(qty(&data["total_quantity"]), "43".parse().unwrap());
    let locations = data["locations"].as_array().unwrap();
    assert_eq!(locations.len(), 3);
    let by_store: Vec<(String, Decimal)> = locations.iter().map(|l| (l["store_name"].as_str().unwrap().to_string(), qty(&l["quantity"]))).collect();
    assert_eq!(
        by_store,
        [("Store lagos".to_string(), "20".parse().unwrap()), ("Store abuja".to_string(), "15".parse().unwrap()), ("Store kaduna".to_string(), "8".parse().unwrap())]
    );

    // A sale at one branch changes that branch and the total, and nothing else.
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let _ = unit;
    let base_unit: Uuid = sqlx::query_scalar("SELECT base_unit_id FROM atlas.products WHERE id = $1").bind(rice).fetch_one(&pool).await.unwrap();
    list_at(&router, &owner, rice, lagos).await;
    price_rule(&router, &owner, rice, None, 1_000).await;
    let shift = open_shift(&router, &owner, lagos).await;
    let cash = method_id(&router, &owner, "cash").await;
    let request = sale_request(lagos, shift, vec![cart_line(rice, base_unit, "5")], vec![pay(cash, 5_000)]);
    assert_eq!(post(&router, "/api/v1/sales", &as_user(&owner), request).await.0, StatusCode::OK);

    let (_, body) = get(&router, &format!("/api/v1/stock/products/{}", rice), &as_user(&owner)).await;
    let quantities: Vec<Decimal> = body["data"]["locations"].as_array().unwrap().iter().map(|l| qty(&l["quantity"])).collect();
    assert_eq!(quantities, ["15", "15", "8"].map(|q| q.parse::<Decimal>().unwrap()));
    assert_eq!(qty(&body["data"]["total_quantity"]), "38".parse().unwrap());
    let _ = (abuja, kaduna);
}

#[sqlx::test]
async fn the_inventory_list_filters_by_store_product_and_low_stock(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id, [lagos, abuja, kaduna], rice) = three_branches(&router, &pool, "filter-owner@example.com").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let beans = create_product(&router, &owner, unit, "Beans", "BEANS-1").await;
    stock_up(&router, &owner, lagos, beans, "4").await;

    let (_, all) = get(&router, "/api/v1/stock/inventory", &as_user(&owner)).await;
    assert_eq!(names(&all), ["Beans", "Rice"], "ordered by name");

    let (_, one_store) = get(&router, &format!("/api/v1/stock/inventory?store_id={}", abuja), &as_user(&owner)).await;
    assert_eq!(names(&one_store), ["Rice"]);
    assert_eq!(one_store["data"][0]["locations"].as_array().unwrap().len(), 1);

    let (_, one_product) = get(&router, &format!("/api/v1/stock/inventory?product_id={}", beans), &as_user(&owner)).await;
    assert_eq!(names(&one_product), ["Beans"]);

    // Only locations at or below their listing's threshold are low.
    for store in [lagos, abuja, kaduna] {
        let (status, _) = call(
            &router,
            "PUT",
            &format!("/api/v1/products/{}/listings/{}", rice, store),
            Some(&as_user(&owner)),
            Some(json!({ "cost_amount_minor": null, "low_stock_threshold": "10", "is_active": true })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }
    let (_, low) = get(&router, "/api/v1/stock/inventory?low_stock=true", &as_user(&owner)).await;
    let rice_row = low["data"].as_array().unwrap().iter().find(|p| p["product_name"] == "Rice").unwrap();
    let locations = rice_row["locations"].as_array().unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0]["store_name"], "Store kaduna");
    assert_eq!(qty(&rice_row["total_quantity"]), "8".parse().unwrap(), "the total covers only what is listed");
}

#[sqlx::test]
async fn an_unstocked_product_has_a_zero_total_and_an_unknown_one_is_404(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "empty-owner@example.com").await;
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let soap = create_product(&router, &owner, unit, "Soap", "SOAP").await;

    let (status, body) = get(&router, &format!("/api/v1/stock/products/{}", soap), &as_user(&owner)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(qty(&body["data"]["total_quantity"]), Decimal::ZERO);
    assert!(body["data"]["locations"].as_array().unwrap().is_empty());
    assert_eq!(get(&router, &format!("/api/v1/stock/products/{}", Uuid::now_v7()), &as_user(&owner)).await.0, StatusCode::NOT_FOUND);

    // Another business's stock never shows up, and their product id is simply unknown.
    let (other, other_tenant) = signup_owner(&router, &pool, "empty-other@example.com").await;
    let other_store = create_store(&router, &other, "abuja").await;
    let other_unit = seed_unit(&pool, other_tenant, "Each", "ea").await;
    let theirs = create_product(&router, &other, other_unit, "Beans", "B-1").await;
    stock_up(&router, &other, other_store, theirs, "99").await;
    assert_eq!(get(&router, &format!("/api/v1/stock/products/{}", theirs), &as_user(&owner)).await.0, StatusCode::NOT_FOUND);
    let (_, mine) = get(&router, "/api/v1/stock/inventory", &as_user(&owner)).await;
    assert!(mine["data"].as_array().unwrap().is_empty());
    let (_, trail) = get(&router, "/api/v1/stock/movements", &as_user(&owner)).await;
    assert!(trail["data"].as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn store_scoped_staff_only_see_their_own_store(pool: PgPool) {
    let router = build_router(pool.clone());
    let (_owner, tenant_id, [lagos, abuja, _kaduna], rice) = three_branches(&router, &pool, "scope-read-owner@example.com").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, lagos, "cashier").await;

    let (_, product) = get(&router, &format!("/api/v1/stock/products/{}", rice), &as_user(&cashier)).await;
    assert_eq!(product["data"]["locations"].as_array().unwrap().len(), 1);
    assert_eq!(product["data"]["locations"][0]["store_name"], "Store lagos");
    assert_eq!(qty(&product["data"]["total_quantity"]), "20".parse().unwrap(), "the total is over the stores they can see");

    let (_, inventory) = get(&router, "/api/v1/stock/inventory", &as_user(&cashier)).await;
    assert_eq!(inventory["data"][0]["locations"].as_array().unwrap().len(), 1);

    let (_, trail) = get(&router, "/api/v1/stock/movements", &as_user(&cashier)).await;
    let stores: Vec<&str> = trail["data"].as_array().unwrap().iter().map(|m| m["store_id"].as_str().unwrap()).collect();
    assert!(!stores.is_empty() && stores.iter().all(|s| *s == lagos.to_string()), "{:?}", stores);

    // Naming a store they cannot view is a 403, not an empty list.
    assert_eq!(get(&router, &format!("/api/v1/stock/inventory?store_id={}", abuja), &as_user(&cashier)).await.0, StatusCode::FORBIDDEN);
    assert_eq!(get(&router, &format!("/api/v1/stock/movements?store_id={}", abuja), &as_user(&cashier)).await.0, StatusCode::FORBIDDEN);

    // Someone with no stock.view anywhere gets a 403 as well.
    let nobody = issue_token(Uuid::now_v7(), tenant_id);
    for path in ["/api/v1/stock/inventory".to_string(), format!("/api/v1/stock/products/{}", rice), "/api/v1/stock/movements".to_string()] {
        assert_eq!(get(&router, &path, &as_user(&nobody)).await.0, StatusCode::FORBIDDEN, "{}", path);
    }
}

#[sqlx::test]
async fn the_movement_trail_records_who_what_and_why_newest_first(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id, [lagos, abuja, _kaduna], rice) = three_branches(&router, &pool, "trail-owner@example.com").await;
    let (status, body) = post(
        &router,
        "/api/v1/stock/transfers",
        &as_user(&owner),
        json!({ "from_store_id": lagos, "to_store_id": abuja, "product_id": rice, "quantity": "5", "reason": "Rebalancing" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let transfer_id = body["data"]["id"].as_str().unwrap();
    // Staged (FR-22): a request alone posts no movement - approve, then
    // dispatch and receive are what actually move stock.
    post(&router, &format!("/api/v1/stock/transfers/{transfer_id}/approve"), &as_user(&owner), json!({})).await;
    post(&router, &format!("/api/v1/stock/transfers/{transfer_id}/dispatch"), &as_user(&owner), json!({})).await;
    post(&router, &format!("/api/v1/stock/transfers/{transfer_id}/receive"), &as_user(&owner), json!({})).await;
    let owner_id: Uuid = sqlx::query_scalar("SELECT created_by_user_id FROM atlas.stock_movements ORDER BY created_at LIMIT 1").fetch_one(&pool).await.unwrap();

    let (_, trail) = get(&router, "/api/v1/stock/movements", &as_user(&owner)).await;
    let rows = trail["data"].as_array().unwrap();
    assert_eq!(rows.len(), 5, "three opening counts and the two transfer rows");
    let types: Vec<&str> = rows.iter().map(|r| r["movement_type"].as_str().unwrap()).collect();
    assert_eq!(&types[..2], ["transfer_in", "transfer_out"], "newest first");
    assert_eq!(rows[0]["reference_id"], rows[1]["reference_id"], "both legs share one reference");
    assert_eq!(rows[0]["reason"], "Rebalancing");
    assert_eq!(rows[0]["created_by_user_id"], owner_id.to_string());
    assert_eq!(qty(&rows[1]["quantity"]), "-5".parse().unwrap());
    assert!(rows[0]["created_at"].is_string());

    let (_, one_store) = get(&router, &format!("/api/v1/stock/movements?store_id={}", abuja), &as_user(&owner)).await;
    assert_eq!(one_store["data"].as_array().unwrap().len(), 2);
    let unit = seed_unit(&pool, tenant_id, "Each", "ea").await;
    let beans = create_product(&router, &owner, unit, "Beans", "BEANS-1").await;
    stock_up(&router, &owner, lagos, beans, "3").await;
    let (_, one_product) = get(&router, &format!("/api/v1/stock/movements?product_id={}", beans), &as_user(&owner)).await;
    assert_eq!(one_product["data"].as_array().unwrap().len(), 1);

    // The limit is clamped to something sensible rather than rejected.
    let (_, limited) = get(&router, "/api/v1/stock/movements?limit=2", &as_user(&owner)).await;
    assert_eq!(limited["data"].as_array().unwrap().len(), 2);
    let (_, floor) = get(&router, "/api/v1/stock/movements?limit=0", &as_user(&owner)).await;
    assert_eq!(floor["data"].as_array().unwrap().len(), 1);
    let (_, ceiling) = get(&router, "/api/v1/stock/movements?limit=100000", &as_user(&owner)).await;
    assert_eq!(ceiling["data"].as_array().unwrap().len(), 6);
}
