//! FR-25 to FR-30: reports, notification preferences and email delivery, the audit trail, imports and exports, approval policy, and the operations view.

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use atlas_backend::{api_router, deliver_notifications, AppState, EmailError, EmailProvider, RateLimiter};
use axum::http::StatusCode;
use axum::Router;
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

// === Harness

#[derive(Clone, Default)]
struct Outbox {
    sent: Arc<Mutex<Vec<(String, String)>>>,
    // Flipped after setup, so signup's own verification email still goes out
    failing: Arc<AtomicBool>,
}

impl Outbox {
    fn notification_subjects(&self) -> Vec<String> {
        return self.sent.lock().unwrap().iter().map(|(_, subject)| subject.clone()).filter(|s| !s.starts_with("Verify")).collect();
    }
}

#[async_trait::async_trait]
impl EmailProvider for Outbox {
    async fn send(&self, to: &str, subject: &str, _body: &str) -> Result<(), EmailError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(EmailError("mail server unreachable".to_string()));
        }
        self.sent.lock().unwrap().push((to.to_string(), subject.to_string()));
        return Ok(());
    }
}

fn state_with(pool: PgPool, outbox: Outbox) -> AppState {
    return AppState {
        pool,
        email: Arc::new(outbox),
        jwt_secret: Arc::new(JWT_SECRET.to_vec()),
        rate_limiter: RateLimiter::with_limits(1000, std::time::Duration::from_secs(60)),
        integrations: atlas_backend::integrations::Integrations::mock(b"test-webhook-secret"),
    };
}

struct Shop {
    router: Router,
    owner: String,
    tenant_id: Uuid,
    lagos: Uuid,
    abuja: Uuid,
    unit: Uuid,
}

async fn shop(pool: &PgPool, router: Router, email: &str) -> Shop {
    let (owner, tenant_id) = signup_owner(&router, pool, email).await;
    let lagos = create_store(&router, &owner, "ops-lagos").await;
    let abuja = create_store(&router, &owner, "ops-abuja").await;
    let unit = seed_unit(pool, tenant_id, "Each", "ea").await;
    return Shop { router, owner, tenant_id, lagos, abuja, unit };
}

async fn listing_cost(s: &Shop, product: Uuid, store: Uuid, cost: i64) {
    let (status, body) = call(
        &s.router,
        "PUT",
        &format!("/api/v1/products/{}/listings/{}", product, store),
        Some(&as_user(&s.owner)),
        Some(json!({ "cost_amount_minor": cost, "low_stock_threshold": null, "is_active": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

async fn adjust(s: &Shop, token: &str, store: Uuid, product: Uuid, quantity: &str) -> (StatusCode, Value) {
    return post(
        &s.router,
        "/api/v1/stock/adjustments",
        &at_store(token, &store.to_string()),
        json!({ "store_id": store, "product_id": product, "stock_item_id": null, "quantity": quantity, "reason": "Recount" }),
    )
    .await;
}

// === FR-25 reports

#[sqlx::test]
async fn the_summary_separates_revenue_tax_cost_and_expenses_and_adds_up_across_stores(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "reports-owner@example.com").await;
    let soap = sellable(&s.router, &s.owner, s.lagos, s.unit, "Soap", "SOAP-R", 1_000, "20").await;
    listing_cost(&s, soap, s.lagos, 600).await;
    let shift = open_shift(&s.router, &s.owner, s.lagos).await;
    let cash = method_id(&s.router, &s.owner, "cash").await;
    let (status, body) = post(&s.router, "/api/v1/sales", &as_user(&s.owner), sale_request(s.lagos, shift, vec![cart_line(soap, s.unit, "3")], vec![pay(cash, 3_000)])).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    // Changing the cost later doesn't change what that sale cost
    listing_cost(&s, soap, s.lagos, 900).await;

    let (_, category) = post(&s.router, "/api/v1/expense-categories", &as_user(&s.owner), json!({ "name": "Fuel" })).await;
    let (_, expense) = post(
        &s.router,
        "/api/v1/expenses",
        &as_user(&s.owner),
        json!({ "store_id": s.abuja, "category_id": id_of(&category), "amount_minor": 700, "expense_date": "2026-10-20", "description": "Diesel" }),
    )
    .await;
    for step in ["submit", "approve"] {
        post(&s.router, &format!("/api/v1/expenses/{}/{}", id_of(&expense), step), &as_user(&s.owner), json!({})).await;
    }

    let (status, body) = get(&s.router, "/api/v1/reports/summary", &as_user(&s.owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let totals = &body["data"]["totals"];
    assert_eq!(totals["sales_count"], 1);
    assert_eq!(totals["revenue_minor"], 3_000);
    assert_eq!(totals["payments_collected_minor"], 3_000);
    assert_eq!(totals["cost_of_goods_minor"], 1_800);
    assert_eq!(totals["gross_profit_minor"].as_i64().unwrap(), totals["net_revenue_minor"].as_i64().unwrap() - 1_800);
    assert_eq!(totals["expenses_minor"], 700);
    // 17 left at the new cost of 900
    assert_eq!(totals["inventory_value_minor"], 17 * 900);

    let stores = body["data"]["stores"].as_array().unwrap();
    assert_eq!(stores.len(), 2);
    let sum: i64 = stores.iter().map(|s| s["revenue_minor"].as_i64().unwrap()).sum();
    assert_eq!(sum, 3_000);
    let abuja = stores.iter().find(|row| row["store_id"] == s.abuja.to_string()).unwrap();
    assert_eq!(abuja["expenses_minor"], 700);
    assert_eq!(abuja["revenue_minor"], 0);

    let (_, body) = get(&s.router, &format!("/api/v1/reports/sales-by-product?store_id={}", s.lagos), &as_user(&s.owner)).await;
    assert_eq!(body["data"][0]["sku"], "SOAP-R");
    assert_eq!(body["data"][0]["cost_of_goods_minor"], 1_800);

    let (status, content_type, csv) = call_raw(&s.router, "GET", "/api/v1/reports/sales-by-product?format=csv", Some(&s.owner), None, "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.unwrap().starts_with("text/csv"));
    let csv = String::from_utf8(csv).unwrap();
    assert!(csv.starts_with("product_id,product_name,sku,quantity"));
    assert!(csv.contains("SOAP-R,3,"));
}

#[sqlx::test]
async fn movement_transfer_return_and_payment_reports_reconcile_and_download_as_pdf(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "more-reports-owner@example.com").await;
    let soap = sellable(&s.router, &s.owner, s.lagos, s.unit, "Soap", "SOAP-M", 1_000, "20").await;
    list_at(&s.router, &s.owner, soap, s.abuja).await;
    let shift = open_shift(&s.router, &s.owner, s.lagos).await;
    let cash = method_id(&s.router, &s.owner, "cash").await;
    let card = method_id(&s.router, &s.owner, "card").await;
    let (_, sale) = post(&s.router, "/api/v1/sales", &as_user(&s.owner), sale_request(s.lagos, shift, vec![cart_line(soap, s.unit, "3")], vec![pay(cash, 2_000), pay(card, 1_000)])).await;
    let sale_id = id_of(&sale);
    let line_id = sale["data"]["lines"][0]["id"].as_str().unwrap().to_string();
    let (status, body) = post(
        &s.router,
        &format!("/api/v1/sales/{}/returns", sale_id),
        &as_user(&s.owner),
        json!({ "id": Uuid::now_v7(), "device_id": "till-1", "reason_code": "changed_mind", "lines": [{ "sale_line_id": line_id, "quantity": "1", "condition": "resellable" }], "refund": { "method": "original" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let (_, transfer) = post(
        &s.router,
        "/api/v1/stock/transfers",
        &as_user(&s.owner),
        json!({ "from_store_id": s.lagos, "to_store_id": s.abuja, "product_id": soap, "stock_item_id": null, "quantity": "5", "reason": "Rebalance" }),
    )
    .await;

    let (_, movements) = get(&s.router, &format!("/api/v1/reports/stock-movements?reference_id={}", sale_id), &as_user(&s.owner)).await;
    let rows = movements["data"].as_array().unwrap();
    assert!(rows.iter().all(|m| m["reference_id"] == sale_id.as_str()));
    assert_eq!(rows.iter().filter(|m| m["movement_type"] == "sale").count(), 1);

    let (_, transfers) = get(&s.router, &format!("/api/v1/reports/transfers?store_id={}", s.abuja), &as_user(&s.owner)).await;
    assert_eq!(transfers["data"][0]["transfer_id"], id_of(&transfer));
    assert_eq!(transfers["data"][0]["status"], "requested");

    let (_, returns) = get(&s.router, &format!("/api/v1/reports/returns?product_id={}", soap), &as_user(&s.owner)).await;
    assert_eq!(returns["data"][0]["refund_total_minor"], 1_000);

    // The refund went back to the card first; per-method net adds up to the summary
    let (_, by_method) = get(&s.router, "/api/v1/reports/payments-by-method", &as_user(&s.owner)).await;
    let methods = by_method["data"].as_array().unwrap();
    let card_row = methods.iter().find(|m| m["method"] == "card").unwrap();
    assert_eq!(card_row["collected_minor"], 1_000);
    assert_eq!(card_row["refunded_minor"], 1_000);
    let net: i64 = methods.iter().map(|m| m["net_minor"].as_i64().unwrap()).sum();
    let (_, summary) = get(&s.router, "/api/v1/reports/summary", &as_user(&s.owner)).await;
    let totals = &summary["data"]["totals"];
    assert_eq!(net, totals["payments_collected_minor"].as_i64().unwrap() - totals["refunds_minor"].as_i64().unwrap());

    let (status, content_type, pdf) = call_raw(&s.router, "GET", "/api/v1/reports/summary?format=pdf", Some(&s.owner), None, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.unwrap(), "application/pdf");
    let pdf = String::from_utf8(pdf).unwrap();
    assert!(pdf.starts_with("%PDF-1.4") && pdf.contains("(Summary  -  page 1 of 1)"));
}

#[sqlx::test]
async fn a_store_manager_only_sees_reports_for_their_own_store(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "reports-scope@example.com").await;
    let (_, manager) = seed_staff(&pool, s.tenant_id, s.lagos, "store_manager").await;
    let lagos = s.lagos.to_string();
    let caller = at_store(&manager, &lagos);

    let (status, body) = get(&s.router, "/api/v1/reports/summary", &caller).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let stores = body["data"]["stores"].as_array().unwrap();
    assert_eq!(stores.len(), 1);
    assert_eq!(stores[0]["store_id"], s.lagos.to_string());

    let (status, _) = get(&s.router, &format!("/api/v1/reports/summary?store_id={}", s.abuja), &caller).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, cashier) = seed_staff(&pool, s.tenant_id, s.lagos, "cashier").await;
    let (status, _) = get(&s.router, "/api/v1/reports/inventory-valuation", &at_store(&cashier, &s.lagos.to_string())).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// === FR-26 / FR-30 preferences and email delivery

async fn request_transfer(s: &Shop, product: Uuid) {
    let (status, body) = post(
        &s.router,
        "/api/v1/stock/transfers",
        &as_user(&s.owner),
        json!({ "from_store_id": s.lagos, "to_store_id": s.abuja, "product_id": product, "stock_item_id": null, "quantity": "2", "reason": "Rebalance" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

#[sqlx::test]
async fn preferences_choose_in_app_and_email_and_emails_go_out_through_the_outbox(pool: PgPool) {
    let outbox = Outbox::default();
    let state = state_with(pool.clone(), outbox.clone());
    let s = shop(&pool, api_router(state.clone()), "prefs-owner@example.com").await;
    let soap = create_product(&s.router, &s.owner, s.unit, "Soap", "SOAP-N").await;
    stock_up(&s.router, &s.owner, s.lagos, soap, "10").await;
    let (_, manager) = seed_staff(&pool, s.tenant_id, s.abuja, "store_manager").await;
    let abuja = s.abuja.to_string();
    let manager_caller = at_store(&manager, &abuja);

    let (status, body) = call(
        &s.router,
        "PUT",
        "/api/v1/notifications/preferences",
        Some(&manager_caller),
        Some(json!({ "preferences": [{ "kind": "transfer_requested", "in_app": false, "email": true }] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let saved = body["data"].as_array().unwrap().iter().find(|p| p["kind"] == "transfer_requested").unwrap().clone();
    assert_eq!(saved, json!({ "kind": "transfer_requested", "in_app": false, "email": true }));

    request_transfer(&s, soap).await;
    let (_, inbox) = get(&s.router, "/api/v1/notifications", &manager_caller).await;
    assert!(inbox["data"].as_array().unwrap().is_empty());
    assert!(outbox.notification_subjects().is_empty(), "nothing is sent inside the request");

    let run = deliver_notifications(&state).await.unwrap();
    assert_eq!(run.sent, 1);
    assert_eq!(outbox.notification_subjects(), vec!["Transfer requested".to_string()]);
    assert_eq!(deliver_notifications(&state).await.unwrap().sent, 0);
}

#[sqlx::test]
async fn an_email_that_keeps_failing_is_marked_failed_and_shows_in_operations(pool: PgPool) {
    let outbox = Outbox::default();
    let state = state_with(pool.clone(), outbox.clone());
    let s = shop(&pool, api_router(state.clone()), "delivery-failure@example.com").await;
    let soap = create_product(&s.router, &s.owner, s.unit, "Soap", "SOAP-F").await;
    stock_up(&s.router, &s.owner, s.lagos, soap, "10").await;
    let (_, manager) = seed_staff(&pool, s.tenant_id, s.abuja, "store_manager").await;
    call(
        &s.router,
        "PUT",
        "/api/v1/notifications/preferences",
        Some(&at_store(&manager, &s.abuja.to_string())),
        Some(json!({ "preferences": [{ "kind": "transfer_requested", "in_app": true, "email": true }] })),
    )
    .await;
    request_transfer(&s, soap).await;
    outbox.failing.store(true, Ordering::SeqCst);

    for attempt in 1..=5 {
        sqlx::query("UPDATE atlas.notification_deliveries SET next_attempt_at = now()").execute(&pool).await.unwrap();
        let run = deliver_notifications(&state).await.unwrap();
        if attempt < 5 {
            assert_eq!(run.retrying, 1, "attempt {}", attempt);
        } else {
            assert_eq!(run.failed, 1);
        }
    }
    let status: String = sqlx::query_scalar("SELECT status::text FROM atlas.notification_deliveries").fetch_one(&pool).await.unwrap();
    assert_eq!(status, "failed");

    let (status, body) = get(&s.router, "/api/v1/operations/issues", &as_user(&s.owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let issue = &body["data"][0];
    assert_eq!(issue["kind"], "email_delivery_failed");
    let (status, body) = post(&s.router, &format!("/api/v1/operations/issues/{}/resolve", issue["id"].as_str().unwrap()), &as_user(&s.owner), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["resolved_at"].is_string());
    let (_, body) = get(&s.router, "/api/v1/operations/issues", &as_user(&s.owner)).await;
    assert!(body["data"].as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn a_dropped_notification_is_kept_as_an_operational_issue(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "dropped-notification@example.com").await;
    let soap = create_product(&s.router, &s.owner, s.unit, "Soap", "SOAP-D").await;
    stock_up(&s.router, &s.owner, s.lagos, soap, "10").await;
    seed_staff(&pool, s.tenant_id, s.abuja, "store_manager").await;
    sqlx::query("CREATE FUNCTION atlas.fail_notifications() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'down'; END $$")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER fail_notifications BEFORE INSERT ON atlas.notifications FOR EACH ROW EXECUTE FUNCTION atlas.fail_notifications()")
        .execute(&pool)
        .await
        .unwrap();

    request_transfer(&s, soap).await;
    let (_, body) = get(&s.router, "/api/v1/operations/issues?kind=notification_failed", &as_user(&s.owner)).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}

// === FR-27 audit

#[sqlx::test]
async fn the_audit_trail_is_searchable_by_store_and_action_and_cannot_be_edited(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "audit-owner@example.com").await;
    let soap = create_product(&s.router, &s.owner, s.unit, "Soap", "SOAP-A").await;
    let (status, _) = adjust(&s, &s.owner, s.lagos, soap, "5").await;
    assert_eq!(status, StatusCode::OK);
    adjust(&s, &s.owner, s.abuja, soap, "3").await;

    let (status, body) = get(&s.router, &format!("/api/v1/audit-events?action=stock.&store_id={}", s.lagos), &as_user(&s.owner)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    let entries = body["data"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["action"], "stock.adjust");
    assert_eq!(entries[0]["outcome"], "success");
    assert_eq!(entries[0]["after"]["reason"], "Recount");
    assert!(entries[0]["actor_name"].is_string());

    let (_, body) = get(&s.router, "/api/v1/audit-events?action=auth.login", &as_user(&s.owner)).await;
    assert!(!body["data"].as_array().unwrap().is_empty());

    let (_, cashier) = seed_staff(&pool, s.tenant_id, s.lagos, "cashier").await;
    let (status, _) = get(&s.router, "/api/v1/audit-events", &at_store(&cashier, &s.lagos.to_string())).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let edited = sqlx::query("UPDATE atlas.audit_events SET action = 'tampered'").execute(&pool).await;
    assert!(edited.is_err());
}

// === FR-28 imports and exports

#[sqlx::test]
async fn an_import_with_bad_rows_writes_nothing_and_reports_each_problem(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "import-owner@example.com").await;
    post(&s.router, "/api/v1/product-types", &as_user(&s.owner), json!({ "name": "Grocery" })).await;
    let existing = create_product(&s.router, &s.owner, s.unit, "Old Soap", "OLD-1").await;
    let _ = existing;

    let bad = "name,sku,product_type,base_unit\nRice,RICE-1,Grocery,ea\n,BEANS-1,Grocery,ea\nOil,OLD-1,Grocery,ea\nSalt,SALT-1,Nope,ea\nSugar,RICE-1,Grocery,ea\n";
    let (status, _, body) = call_raw(&s.router, "POST", "/api/v1/imports/products", Some(&s.owner), Some("text/csv"), bad).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["code"], "IMPORT_INVALID");
    let problems: Vec<(u64, String)> = body["data"]["errors"].as_array().unwrap().iter().map(|e| (e["row"].as_u64().unwrap(), e["field"].as_str().unwrap().to_string())).collect();
    assert_eq!(problems, vec![(3, "name".to_string()), (4, "sku".to_string()), (5, "product_type".to_string()), (6, "sku".to_string())]);
    let (_, list) = get(&s.router, "/api/v1/products?q=rice", &as_user(&s.owner)).await;
    assert!(list["data"].as_array().unwrap().is_empty());

    let good = "name,sku,product_type,base_unit,barcode\nRice,RICE-1,Grocery,ea,6000000000001\n\"Beans, brown\",BEANS-1,grocery,Each,\n";
    let (status, _, body) = call_raw(&s.router, "POST", "/api/v1/imports/products?dry_run=true", Some(&s.owner), Some("text/csv"), good).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let (_, list) = get(&s.router, "/api/v1/products?q=rice", &as_user(&s.owner)).await;
    assert!(list["data"].as_array().unwrap().is_empty());

    let (status, _, body) = call_raw(&s.router, "POST", "/api/v1/imports/products", Some(&s.owner), Some("text/csv"), good).await;
    assert_eq!(status, StatusCode::OK);
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["data"]["imported"], 2);
    let (_, list) = get(&s.router, "/api/v1/products?q=6000000000001", &as_user(&s.owner)).await;
    assert_eq!(list["data"][0]["name"], "Rice");

    // Importing it again would duplicate, so it is refused
    let (status, _, _) = call_raw(&s.router, "POST", "/api/v1/imports/products", Some(&s.owner), Some("text/csv"), good).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _, csv) = call_raw(&s.router, "GET", "/api/v1/exports/products", Some(&s.owner), None, "").await;
    assert_eq!(status, StatusCode::OK);
    let csv = String::from_utf8(csv).unwrap();
    assert!(csv.starts_with("name,sku,product_type,base_unit,category,barcode\n"));
    assert!(csv.contains("\"Beans, brown\",BEANS-1,Grocery,ea,,"));
}

#[sqlx::test]
async fn suppliers_and_opening_stock_import_from_their_templates(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "import-stock@example.com").await;
    let soap = create_product(&s.router, &s.owner, s.unit, "Soap", "SOAP-I").await;

    let (status, _, template) = call_raw(&s.router, "GET", "/api/v1/imports/templates/suppliers", Some(&s.owner), None, "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8(template).unwrap().starts_with("name,code,phone,email,address,payment_terms_days"));

    let suppliers = "name,code,payment_terms_days\nAcme,ACM-1,30\nBright,,abc\n";
    let (status, _, body) = call_raw(&s.router, "POST", "/api/v1/imports/suppliers", Some(&s.owner), Some("text/csv"), suppliers).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{}", String::from_utf8_lossy(&body));
    let (status, _, _) = call_raw(&s.router, "POST", "/api/v1/imports/suppliers", Some(&s.owner), Some("text/csv"), "name,code,payment_terms_days\nAcme,ACM-1,30\n").await;
    assert_eq!(status, StatusCode::OK);
    let (_, list) = get(&s.router, "/api/v1/suppliers?q=ACM-1", &as_user(&s.owner)).await;
    assert_eq!(list["data"][0]["payment_terms_days"], 30);

    let stock = "sku,quantity\nSOAP-I,12\nMISSING,4\n";
    let path = format!("/api/v1/imports/opening-stock?store_id={}", s.lagos);
    let (status, _, _) = call_raw(&s.router, "POST", &path, Some(&s.owner), Some("text/csv"), stock).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(on_hand(&s.router, &s.owner, s.lagos, soap).await, "0");
    let (status, _, _) = call_raw(&s.router, "POST", &path, Some(&s.owner), Some("text/csv"), "sku,quantity\nSOAP-I,12\n").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(on_hand(&s.router, &s.owner, s.lagos, soap).await.parse::<f64>().unwrap(), 12.0);
}

// === FR-29 approval policy

#[sqlx::test]
async fn a_change_over_the_policy_is_held_until_someone_with_stock_approve_decides(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "threshold-owner@example.com").await;
    let soap = create_product(&s.router, &s.owner, s.unit, "Soap", "SOAP-T").await;
    stock_up(&s.router, &s.owner, s.lagos, soap, "100").await;
    approval_policy(&s.router, &s.owner, Some(s.lagos), "stock_adjustment", json!({ "limit_quantity": "10" })).await;
    approval_policy(&s.router, &s.owner, Some(s.lagos), "write_off", json!({ "limit_quantity": "5" })).await;
    let (clerk_id, clerk) = seed_staff(&pool, s.tenant_id, s.lagos, "stock_clerk").await;
    let lagos = s.lagos.to_string();
    let at_lagos = at_store(&clerk, &lagos);

    let (status, held) = adjust(&s, &clerk, s.lagos, soap, "-20").await;
    assert_eq!(status, StatusCode::ACCEPTED, "{}", held);
    assert_eq!(held["data"]["status"], "pending");
    assert_eq!(held["data"]["operation"], "stock_adjustment");
    let request_id = id_of(&held);
    assert_eq!(on_hand(&s.router, &s.owner, s.lagos, soap).await, "100", "nothing moves while it waits");
    let (status, _) = adjust(&s, &clerk, s.lagos, soap, "-8").await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = adjust(&s, &s.owner, s.lagos, soap, "-20").await;
    assert_eq!(status, StatusCode::OK, "the owner holds stock.approve");

    let write_off = json!({ "store_id": s.lagos, "product_id": soap, "stock_item_id": null, "quantity": "6", "condition": "damaged", "reason": "Crushed" });
    let (status, held_write_off) = post(&s.router, "/api/v1/stock/write-offs", &at_lagos, write_off).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{}", held_write_off);
    assert_eq!(on_hand(&s.router, &s.owner, s.lagos, soap).await.parse::<f64>().unwrap(), 72.0);

    // The approver sees it waiting; the clerk sees their own two
    let (_, waiting) = get(&s.router, "/api/v1/approval-requests", &as_user(&s.owner)).await;
    assert_eq!(waiting["data"].as_array().unwrap().len(), 2);
    let (_, mine) = get(&s.router, "/api/v1/approval-requests", &at_lagos).await;
    assert_eq!(mine["data"].as_array().unwrap().len(), 2);

    let approve = format!("/api/v1/approval-requests/{}/approve", request_id);
    let (status, _) = post(&s.router, &approve, &at_lagos, json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "a clerk can't approve");
    let (status, approved) = post(&s.router, &approve, &as_user(&s.owner), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", approved);
    assert_eq!(approved["data"]["status"], "approved");
    assert_eq!(approved["data"]["result_entity_type"], "stock_movement");
    assert_eq!(on_hand(&s.router, &s.owner, s.lagos, soap).await.parse::<f64>().unwrap(), 52.0);
    let (status, body) = post(&s.router, &approve, &as_user(&s.owner), json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "APPROVAL_NOT_PENDING");

    // FR-27: the adjustment traces to its author, reason, approval and movement
    let movement_id = approved["data"]["result_id"].as_str().unwrap();
    let (_, trail) = get(&s.router, &format!("/api/v1/audit-events?action=stock.adjust&entity_id={}", movement_id), &as_user(&s.owner)).await;
    let entry = &trail["data"][0];
    assert_eq!(entry["actor_user_id"], clerk_id.to_string());
    assert_eq!(entry["after"]["reason"], "Recount");
    assert_eq!(entry["after"]["approval_request_id"], request_id);
    assert!(entry["after"]["approved_by_user_id"].is_string());

    let reject = format!("/api/v1/approval-requests/{}/reject", id_of(&held_write_off));
    let (status, rejected) = post(&s.router, &reject, &as_user(&s.owner), json!({ "reason": "Still sellable" })).await;
    assert_eq!(status, StatusCode::OK, "{}", rejected);
    assert_eq!(rejected["data"]["decision_note"], "Still sellable");
    assert_eq!(on_hand(&s.router, &s.owner, s.lagos, soap).await.parse::<f64>().unwrap(), 52.0);

    let (_, inbox) = get(&s.router, "/api/v1/notifications", &at_lagos).await;
    let decided = inbox["data"].as_array().unwrap().iter().filter(|n| n["kind"] == "approval_decided").count();
    assert_eq!(decided, 2);
}

#[sqlx::test]
async fn approving_a_held_change_that_no_longer_fits_leaves_it_pending(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "stale-approval@example.com").await;
    let soap = create_product(&s.router, &s.owner, s.unit, "Soap", "SOAP-ST").await;
    stock_up(&s.router, &s.owner, s.lagos, soap, "30").await;
    approval_policy(&s.router, &s.owner, Some(s.lagos), "stock_adjustment", json!({ "limit_quantity": "10" })).await;
    let (_, clerk) = seed_staff(&pool, s.tenant_id, s.lagos, "stock_clerk").await;

    let (status, held) = adjust(&s, &clerk, s.lagos, soap, "-25").await;
    assert_eq!(status, StatusCode::ACCEPTED);
    adjust(&s, &s.owner, s.lagos, soap, "-20").await;

    let (status, body) = post(&s.router, &format!("/api/v1/approval-requests/{}/approve", id_of(&held)), &as_user(&s.owner), json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "PRODUCT_OUT_OF_STOCK");
    let (_, request) = get(&s.router, &format!("/api/v1/approval-requests/{}", id_of(&held)), &as_user(&s.owner)).await;
    assert_eq!(request["data"]["status"], "pending");

    let (status, _) = post(&s.router, &format!("/api/v1/approval-requests/{}/cancel", id_of(&held)), &as_user(&s.owner), json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "only the requester cancels");
    let (status, body) = post(&s.router, &format!("/api/v1/approval-requests/{}/cancel", id_of(&held)), &at_store(&clerk, &s.lagos.to_string()), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "cancelled");
}

/// A business-wide catalogue editor: `product.manage` everywhere, but not `price.approve`. Returns their token.
async fn catalogue_editor(pool: &PgPool, tenant_id: Uuid) -> String {
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.users (id, full_name, email, password_hash, is_email_verified) VALUES ($1, 'Catalogue editor', $2, 'not-a-real-hash', true)")
        .bind(user_id)
        .bind(format!("editor-{}@example.com", user_id))
        .execute(pool)
        .await
        .unwrap();
    let role_id = Uuid::now_v7();
    sqlx::query("INSERT INTO atlas.roles (id, tenant_id, name, permissions) VALUES ($1, $2, 'catalogue_editor', ARRAY['product.manage']::atlas.permission[])")
        .bind(role_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO atlas.role_grants (id, user_id, role_id, scope, tenant_id, status, granted_by_user_id)
         VALUES ($1, $2, $3, 'tenant', $4, 'active', $2)",
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(role_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .unwrap();
    return issue_token(user_id, tenant_id);
}

#[sqlx::test]
async fn fr09_a_big_price_change_waits_for_price_approve(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "price-approval@example.com").await;
    let soap = create_product(&s.router, &s.owner, s.unit, "Soap", "SOAP-P").await;
    price_rule(&s.router, &s.owner, soap, Some(s.lagos), 1_000).await;
    approval_policy(&s.router, &s.owner, None, "price_change", json!({ "limit_basis_points": 2_000 })).await;
    let editor = catalogue_editor(&pool, s.tenant_id).await;
    let at_lagos = as_user(&editor);
    let rule = |amount: i64| json!({ "store_id": s.lagos, "unit_price_amount_minor": amount });
    let path = format!("/api/v1/products/{}/price-rules", soap);

    let (status, body) = post(&s.router, &path, &at_lagos, rule(1_100)).await;
    assert_eq!(status, StatusCode::OK, "10% is within the policy: {}", body);
    let (status, held) = post(&s.router, &path, &at_lagos, rule(1_500)).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{}", held);
    assert_eq!(held["data"]["operation"], "price_change");
    let (_, rules) = get(&s.router, &path, &as_user(&s.owner)).await;
    assert_eq!(rules["data"].as_array().unwrap().len(), 2, "the held price isn't a rule yet");

    let (status, approved) = post(&s.router, &format!("/api/v1/approval-requests/{}/approve", id_of(&held)), &as_user(&s.owner), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", approved);
    let (_, history) = get(&s.router, &format!("/api/v1/products/{}/price-history", soap), &as_user(&s.owner)).await;
    assert_eq!(history["data"][0]["previous_amount_minor"], 1_100);
    assert_eq!(history["data"][0]["new_amount_minor"], 1_500);

    // A drop counts too, and the store's own policy wins over the business-wide one
    let (status, _) = post(&s.router, &path, &at_lagos, rule(600)).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    approval_policy(&s.router, &s.owner, Some(s.lagos), "price_change", json!({ "limit_basis_points": 9_000 })).await;
    let (status, body) = post(&s.router, &path, &at_lagos, rule(600)).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

#[sqlx::test]
async fn policies_take_one_limit_in_the_operations_unit_and_changes_are_audited(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "policy-admin@example.com").await;
    let owner = as_user(&s.owner);
    let put = |body: Value| call(&s.router, "PUT", "/api/v1/approval-policies", Some(&owner), Some(body));
    let (status, body) = put(json!({ "store_id": null, "operation": "price_change", "limit_quantity": "5" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{}", body);
    let (status, _) = put(json!({ "store_id": null, "operation": "refund", "limit_amount_minor": 10, "limit_quantity": "5" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (_, manager) = seed_staff(&pool, s.tenant_id, s.lagos, "store_manager").await;
    let (status, _) = call(&s.router, "PUT", "/api/v1/approval-policies", Some(&at_store(&manager, &s.lagos.to_string())), Some(json!({ "store_id": null, "operation": "refund", "limit_amount_minor": 50_000 }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "business-wide needs tenant.manage");

    approval_policy(&s.router, &s.owner, Some(s.lagos), "refund", json!({ "limit_amount_minor": 50_000 })).await;
    approval_policy(&s.router, &s.owner, Some(s.lagos), "refund", json!({ "limit_amount_minor": 80_000 })).await;
    let (_, policies) = get(&s.router, &format!("/api/v1/approval-policies?store_id={}", s.lagos), &as_user(&s.owner)).await;
    assert_eq!(policies["data"].as_array().unwrap().len(), 1, "a second PUT replaces, not adds");
    let (_, trail) = get(&s.router, "/api/v1/audit-events?action=approval_policy.set", &as_user(&s.owner)).await;
    assert_eq!(trail["data"][0]["before"]["limit_amount_minor"], 50_000);
    assert_eq!(trail["data"][0]["after"]["limit_amount_minor"], 80_000);
    assert_eq!(trail["data"][0]["store_id"], s.lagos.to_string());

    let (status, _) = delete(&s.router, &format!("/api/v1/approval-policies/{}", policies["data"][0]["id"].as_str().unwrap()), &as_user(&s.owner)).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn with_self_approval_off_someone_else_has_to_approve(pool: PgPool) {
    let s = shop(&pool, build_router(pool.clone()), "self-approval@example.com").await;
    let soap = create_product(&s.router, &s.owner, s.unit, "Soap", "SOAP-S").await;
    stock_up(&s.router, &s.owner, s.lagos, soap, "10").await;
    store_settings(&s.router, &s.owner, s.lagos, json!({ "allow_self_approval": false })).await;
    let (_, manager) = seed_staff(&pool, s.tenant_id, s.lagos, "store_manager").await;

    let (_, count) = post(
        &s.router,
        "/api/v1/stock/counts",
        &as_user(&s.owner),
        json!({ "store_id": s.lagos, "reason": "Spot check", "lines": [{ "product_id": soap, "stock_item_id": null, "counted_quantity": "8" }] }),
    )
    .await;
    let path = format!("/api/v1/stock/counts/{}/approve", id_of(&count));
    let (status, body) = post(&s.router, &path, &as_user(&s.owner), json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "APPROVAL_SELF_FORBIDDEN");
    let (status, body) = post(&s.router, &path, &at_store(&manager, &s.lagos.to_string()), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);

    let (_, settings) = get(&s.router, &format!("/api/v1/stores/{}/settings", s.lagos), &as_user(&s.owner)).await;
    assert_eq!(settings["data"]["allow_self_approval"], false);
    let (_, audit) = get(&s.router, &format!("/api/v1/audit-events?action=store.settings_update&store_id={}", s.lagos), &as_user(&s.owner)).await;
    assert_eq!(audit["data"][0]["after"]["allow_self_approval"], false);
    assert_eq!(audit["data"][0]["before"]["allow_self_approval"], true);
}
