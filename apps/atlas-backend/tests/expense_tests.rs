//! FR-24 store expenses: categories, the draft -> submitted -> approved/rejected workflow, payment status, filters, and who may touch them.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn category(router: &axum::Router, token: &str, name: &str) -> String {
    let (status, body) = post(router, "/api/v1/expense-categories", &as_user(token), json!({ "name": name })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    return id_of(&body);
}

fn expense(store_id: Uuid, category_id: &str, amount_minor: i64, date: &str) -> Value {
    return json!({
        "store_id": store_id,
        "category_id": category_id,
        "amount_minor": amount_minor,
        "expense_date": date,
        "description": "Generator diesel",
    });
}

#[sqlx::test]
async fn an_expense_runs_draft_to_paid_and_only_moves_forward(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "expense-flow-owner@example.com").await;
    let store = create_store(&router, &owner, "expense-flow").await;
    let fuel = category(&router, &owner, "Fuel").await;
    let caller = as_user(&owner);

    let (status, body) = post(&router, "/api/v1/expenses", &caller, expense(store, &fuel, 25_000, "2026-10-20")).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "draft");
    assert_eq!(body["data"]["payment_status"], "unpaid");
    let id = id_of(&body);

    // Paying before approval is refused
    let (status, body) = post(&router, &format!("/api/v1/expenses/{}/pay", id), &caller, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", body);
    assert_eq!(body["code"], "EXPENSE_NOT_APPROVED");

    let (status, body) = patch(&router, &format!("/api/v1/expenses/{}", id), &caller, json!({ "amount_minor": 30_000 })).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["amount_minor"], 30_000);
    assert_eq!(body["data"]["description"], "Generator diesel");

    let (status, body) = post(&router, &format!("/api/v1/expenses/{}/submit", id), &caller, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "submitted");

    let (status, body) = patch(&router, &format!("/api/v1/expenses/{}", id), &caller, json!({ "amount_minor": 1 })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", body);
    assert_eq!(body["code"], "EXPENSE_NOT_DRAFT");

    let (status, body) = post(&router, &format!("/api/v1/expenses/{}/approve", id), &caller, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "approved");
    assert!(body["data"]["decided_at"].is_string());

    let (status, body) = post(&router, &format!("/api/v1/expenses/{}/reject", id), &caller, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", body);
    assert_eq!(body["code"], "EXPENSE_NOT_SUBMITTED");

    let (status, body) = post(&router, &format!("/api/v1/expenses/{}/pay", id), &caller, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["payment_status"], "paid");
}

#[sqlx::test]
async fn a_rejected_expense_cannot_be_approved_or_paid(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "expense-reject-owner@example.com").await;
    let store = create_store(&router, &owner, "expense-reject").await;
    let rent = category(&router, &owner, "Rent").await;
    let caller = as_user(&owner);

    let (_, body) = post(&router, "/api/v1/expenses", &caller, expense(store, &rent, 500_000, "2026-10-01")).await;
    let id = id_of(&body);
    post(&router, &format!("/api/v1/expenses/{}/submit", id), &caller, json!({})).await;

    let (status, body) = post(&router, &format!("/api/v1/expenses/{}/reject", id), &caller, json!({})).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
    assert_eq!(body["data"]["status"], "rejected");

    let (status, _) = post(&router, &format!("/api/v1/expenses/{}/approve", id), &caller, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = post(&router, &format!("/api/v1/expenses/{}/pay", id), &caller, json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test]
async fn expenses_filter_by_store_category_status_and_date(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "expense-filter-owner@example.com").await;
    let lagos = create_store(&router, &owner, "expense-lagos").await;
    let abuja = create_store(&router, &owner, "expense-abuja").await;
    let fuel = category(&router, &owner, "Fuel").await;
    let rent = category(&router, &owner, "Rent").await;
    let caller = as_user(&owner);

    post(&router, "/api/v1/expenses", &caller, expense(lagos, &fuel, 1_000, "2026-09-15")).await;
    let (_, body) = post(&router, "/api/v1/expenses", &caller, expense(lagos, &rent, 2_000, "2026-10-05")).await;
    post(&router, &format!("/api/v1/expenses/{}/submit", id_of(&body)), &caller, json!({})).await;
    post(&router, "/api/v1/expenses", &caller, expense(abuja, &fuel, 3_000, "2026-10-10")).await;

    let count = |body: &Value| body["data"].as_array().unwrap().len();

    let (_, body) = get(&router, &format!("/api/v1/expenses?store_id={}", lagos), &caller).await;
    assert_eq!(count(&body), 2);
    let (_, body) = get(&router, &format!("/api/v1/expenses?category_id={}", fuel), &caller).await;
    assert_eq!(count(&body), 2);
    let (_, body) = get(&router, "/api/v1/expenses?status=submitted", &caller).await;
    assert_eq!(count(&body), 1);
    let (_, body) = get(&router, "/api/v1/expenses?from=2026-10-01&to=2026-10-31", &caller).await;
    assert_eq!(count(&body), 2);
    assert_eq!(body["data"][0]["expense_date"], "2026-10-10");

    let (_, body) = get(&router, "/api/v1/expense-categories", &caller).await;
    assert_eq!(count(&body), 2);
}

#[sqlx::test]
async fn a_cashier_cannot_record_expenses_but_a_store_manager_can(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, tenant_id) = signup_owner(&router, &pool, "expense-perm-owner@example.com").await;
    let store = create_store(&router, &owner, "expense-perm").await;
    let misc = category(&router, &owner, "Misc").await;
    let (_, cashier) = seed_staff(&pool, tenant_id, store, "cashier").await;
    let (_, manager) = seed_staff(&pool, tenant_id, store, "store_manager").await;
    let store_str = store.to_string();

    let (status, _) = post(&router, "/api/v1/expenses", &at_store(&cashier, &store_str), expense(store, &misc, 100, "2026-10-20")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body) = post(&router, "/api/v1/expenses", &at_store(&manager, &store_str), expense(store, &misc, 100, "2026-10-20")).await;
    assert_eq!(status, StatusCode::OK, "{}", body);
}

#[sqlx::test]
async fn a_duplicate_category_name_is_refused(pool: PgPool) {
    let router = build_router(pool.clone());
    let (owner, _) = signup_owner(&router, &pool, "expense-dupe-owner@example.com").await;
    category(&router, &owner, "Fuel").await;

    let (status, _) = post(&router, "/api/v1/expense-categories", &as_user(&owner), json!({ "name": "Fuel" })).await;
    assert_eq!(status, StatusCode::CONFLICT);
}
