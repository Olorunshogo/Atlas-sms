use axum::extract::{Path, Query};
use axum::Json;

use crate::approvals;
use crate::audit;
use crate::error::ApiError;
use crate::extractors::TenantTx;
use crate::notifications::{self, NotificationKind};
use crate::response::ApiEnvelope;
use crate::roles;
use uuid::Uuid;

use super::models::{
    CreateExpenseCategoryRequest, CreateExpenseRequest, EditExpenseRequest, Expense, ExpenseCategory,
    ListExpensesQuery,
};
use super::service;

pub async fn list_categories(mut tx: TenantTx) -> Result<Json<ApiEnvelope<Vec<ExpenseCategory>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let categories = service::list_categories(&mut tx, tenant_id).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Expense categories fetched.", categories)));
}

pub async fn create_category(
    mut tx: TenantTx,
    Json(body): Json<CreateExpenseCategoryRequest>,
) -> Result<Json<ApiEnvelope<ExpenseCategory>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, None, "expense.manage").await?;
    let category = service::create_category(&mut tx, tenant_id, &body).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Expense category added.", category)));
}

pub async fn create_expense(
    mut tx: TenantTx,
    Json(body): Json<CreateExpenseRequest>,
) -> Result<Json<ApiEnvelope<Expense>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(body.store_id), "expense.manage").await?;
    let expense = service::create_expense(&mut tx, tenant_id, user_id, &body).await?;
    audit::record(&mut tx, tenant_id, user_id, "expense", expense.id, "expense.create", serde_json::to_value(&expense).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Expense recorded.", expense)));
}

pub async fn list_expenses(
    mut tx: TenantTx,
    Query(query): Query<ListExpensesQuery>,
) -> Result<Json<ApiEnvelope<Vec<Expense>>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, query.store_id, "expense.manage").await?;
    let expenses = service::list_expenses(&mut tx, tenant_id, &query).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Expenses fetched.", expenses)));
}

pub async fn edit_expense(
    Path(expense_id): Path<Uuid>,
    mut tx: TenantTx,
    Json(body): Json<EditExpenseRequest>,
) -> Result<Json<ApiEnvelope<Expense>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let existing = service::get_expense(&mut tx, expense_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.store_id), "expense.manage").await?;
    let before = audit::snapshot(&mut tx, "expenses", "id", expense_id).await?;
    let expense = service::edit_expense(&mut tx, tenant_id, expense_id, &body).await?;
    audit::record_edit(
        &mut tx,
        audit::Edit { tenant_id, actor_user_id: user_id, entity_type: "expense", entity_id: expense.id, action: "expense.edit", table: "expenses", key: "id", before },
    )
    .await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Expense updated.", expense)));
}

pub async fn submit_expense(Path(expense_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Expense>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let existing = service::get_expense(&mut tx, expense_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.store_id), "expense.manage").await?;
    let expense = service::submit_expense(&mut tx, tenant_id, expense_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "expense", expense.id, "expense.submit", serde_json::to_value(&expense).ok()).await?;

    // An expense policy sets the amount up to which no one needs to look at it (FR-29); without one, all need approval
    let auto_limit = approvals::amount_limit(&mut tx, tenant_id, expense.store_id, approvals::Operation::Expense).await?;
    if auto_limit.is_some_and(|limit| expense.amount_minor <= limit) {
        let expense = service::auto_approve_expense(&mut tx, tenant_id, expense_id).await?;
        audit::record(&mut tx, tenant_id, user_id, "expense", expense.id, "expense.auto_approve", serde_json::to_value(&expense).ok()).await?;
        tx.commit().await?;
        return Ok(Json(ApiEnvelope::ok("Expense submitted and approved - it's within the approval limit.", expense)));
    }
    notifications::service::expense(&mut tx, tenant_id, user_id, expense.id, NotificationKind::ExpenseSubmitted).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Expense submitted.", expense)));
}

pub async fn approve_expense(Path(expense_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Expense>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let existing = service::get_expense(&mut tx, expense_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.store_id), "expense.manage").await?;
    approvals::assert_may_approve(&mut tx, existing.store_id, user_id, existing.created_by_user_id).await?;
    let expense = service::approve_expense(&mut tx, tenant_id, user_id, expense_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "expense", expense.id, "expense.approve", serde_json::to_value(&expense).ok()).await?;
    notifications::service::expense(&mut tx, tenant_id, user_id, expense.id, NotificationKind::ExpenseApproved).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Expense approved.", expense)));
}

pub async fn reject_expense(Path(expense_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Expense>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let existing = service::get_expense(&mut tx, expense_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.store_id), "expense.manage").await?;
    approvals::assert_may_approve(&mut tx, existing.store_id, user_id, existing.created_by_user_id).await?;
    let expense = service::reject_expense(&mut tx, tenant_id, user_id, expense_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "expense", expense.id, "expense.reject", serde_json::to_value(&expense).ok()).await?;
    notifications::service::expense(&mut tx, tenant_id, user_id, expense.id, NotificationKind::ExpenseRejected).await;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Expense rejected.", expense)));
}

pub async fn mark_expense_paid(Path(expense_id): Path<Uuid>, mut tx: TenantTx) -> Result<Json<ApiEnvelope<Expense>>, ApiError> {
    let tenant_id = tx.tenant_id;
    let user_id = tx.user_id;
    let existing = service::get_expense(&mut tx, expense_id).await?;
    roles::service::assert_permission(&mut tx, user_id, tenant_id, Some(existing.store_id), "expense.manage").await?;
    let expense = service::mark_expense_paid(&mut tx, tenant_id, expense_id).await?;
    audit::record(&mut tx, tenant_id, user_id, "expense", expense.id, "expense.pay", serde_json::to_value(&expense).ok()).await?;
    tx.commit().await?;
    return Ok(Json(ApiEnvelope::ok("Expense marked paid.", expense)));
}
