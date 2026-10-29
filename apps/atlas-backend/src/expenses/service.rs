use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};

use super::models::{
    CreateExpenseCategoryRequest, CreateExpenseRequest, EditExpenseRequest, Expense, ExpenseCategory,
    ListExpensesQuery,
};

const EXPENSE_COLUMNS: &str = "id, store_id, category_id, amount_minor, expense_date, description, reference,
     status, payment_status, created_by_user_id, created_at, submitted_at, decided_by_user_id, decided_at";

pub async fn list_categories(conn: &mut PgConnection, tenant_id: Uuid) -> Result<Vec<ExpenseCategory>, ApiError> {
    return sqlx::query_as::<_, ExpenseCategory>(
        "SELECT id, name FROM atlas.expense_categories WHERE tenant_id = $1 ORDER BY name",
    )
    .bind(tenant_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn create_category(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payload: &CreateExpenseCategoryRequest,
) -> Result<ExpenseCategory, ApiError> {
    return sqlx::query_as::<_, ExpenseCategory>(
        "INSERT INTO atlas.expense_categories (id, tenant_id, name) VALUES ($1, $2, $3) RETURNING id, name",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(&payload.name)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn create_expense(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    payload: &CreateExpenseRequest,
) -> Result<Expense, ApiError> {
    return sqlx::query_as::<_, Expense>(&format!(
        "INSERT INTO atlas.expenses
            (id, tenant_id, store_id, category_id, amount_minor, expense_date, description, reference, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING {EXPENSE_COLUMNS}"
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.store_id)
    .bind(payload.category_id)
    .bind(payload.amount_minor)
    .bind(payload.expense_date)
    .bind(&payload.description)
    .bind(&payload.reference)
    .bind(user_id)
    .fetch_one(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn get_expense(conn: &mut PgConnection, expense_id: Uuid) -> Result<Expense, ApiError> {
    return sqlx::query_as::<_, Expense>(&format!(
        "SELECT {EXPENSE_COLUMNS} FROM atlas.expenses WHERE id = $1"
    ))
    .bind(expense_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Expense not found."));
}

pub async fn list_expenses(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    query: &ListExpensesQuery,
) -> Result<Vec<Expense>, ApiError> {
    return sqlx::query_as::<_, Expense>(&format!(
        "SELECT {EXPENSE_COLUMNS} FROM atlas.expenses
         WHERE tenant_id = $1
           AND ($2::uuid IS NULL OR store_id = $2)
           AND ($3::uuid IS NULL OR category_id = $3)
           AND ($4::atlas.expense_status IS NULL OR status = $4)
           AND ($5::uuid IS NULL OR created_by_user_id = $5)
           AND ($6::date IS NULL OR expense_date >= $6)
           AND ($7::date IS NULL OR expense_date <= $7)
         ORDER BY expense_date DESC, created_at DESC"
    ))
    .bind(tenant_id)
    .bind(query.store_id)
    .bind(query.category_id)
    .bind(query.status)
    .bind(query.created_by_user_id)
    .bind(query.from)
    .bind(query.to)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn edit_expense(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    expense_id: Uuid,
    payload: &EditExpenseRequest,
) -> Result<Expense, ApiError> {
    return sqlx::query_as::<_, Expense>(&format!(
        "UPDATE atlas.expenses
         SET category_id = COALESCE($1, category_id),
             amount_minor = COALESCE($2, amount_minor),
             expense_date = COALESCE($3, expense_date),
             description = COALESCE($4, description),
             reference = COALESCE($5, reference)
         WHERE id = $6 AND tenant_id = $7 AND status = 'draft'
         RETURNING {EXPENSE_COLUMNS}"
    ))
    .bind(payload.category_id)
    .bind(payload.amount_minor)
    .bind(payload.expense_date)
    .bind(&payload.description)
    .bind(&payload.reference)
    .bind(expense_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code("Only a draft expense can be edited.", codes::EXPENSE_NOT_DRAFT));
}

pub async fn submit_expense(conn: &mut PgConnection, tenant_id: Uuid, expense_id: Uuid) -> Result<Expense, ApiError> {
    return sqlx::query_as::<_, Expense>(&format!(
        "UPDATE atlas.expenses SET status = 'submitted', submitted_at = now()
         WHERE id = $1 AND tenant_id = $2 AND status = 'draft'
         RETURNING {EXPENSE_COLUMNS}"
    ))
    .bind(expense_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code("Only a draft expense can be submitted.", codes::EXPENSE_NOT_DRAFT));
}

pub async fn approve_expense(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    expense_id: Uuid,
) -> Result<Expense, ApiError> {
    return sqlx::query_as::<_, Expense>(&format!(
        "UPDATE atlas.expenses SET status = 'approved', decided_by_user_id = $1, decided_at = now()
         WHERE id = $2 AND tenant_id = $3 AND status = 'submitted'
         RETURNING {EXPENSE_COLUMNS}"
    ))
    .bind(user_id)
    .bind(expense_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code("Only a submitted expense can be approved.", codes::EXPENSE_NOT_SUBMITTED));
}

/// Within the expense policy's limit: approved with no decider, so the trail shows the policy decided it.
pub async fn auto_approve_expense(conn: &mut PgConnection, tenant_id: Uuid, expense_id: Uuid) -> Result<Expense, ApiError> {
    return sqlx::query_as::<_, Expense>(&format!(
        "UPDATE atlas.expenses SET status = 'approved', decided_by_user_id = NULL, decided_at = now()
         WHERE id = $1 AND tenant_id = $2 AND status = 'submitted'
         RETURNING {EXPENSE_COLUMNS}"
    ))
    .bind(expense_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code("Only a submitted expense can be approved.", codes::EXPENSE_NOT_SUBMITTED));
}

pub async fn reject_expense(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    expense_id: Uuid,
) -> Result<Expense, ApiError> {
    return sqlx::query_as::<_, Expense>(&format!(
        "UPDATE atlas.expenses SET status = 'rejected', decided_by_user_id = $1, decided_at = now()
         WHERE id = $2 AND tenant_id = $3 AND status = 'submitted'
         RETURNING {EXPENSE_COLUMNS}"
    ))
    .bind(user_id)
    .bind(expense_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code("Only a submitted expense can be rejected.", codes::EXPENSE_NOT_SUBMITTED));
}

pub async fn mark_expense_paid(conn: &mut PgConnection, tenant_id: Uuid, expense_id: Uuid) -> Result<Expense, ApiError> {
    return sqlx::query_as::<_, Expense>(&format!(
        "UPDATE atlas.expenses SET payment_status = 'paid'
         WHERE id = $1 AND tenant_id = $2 AND status = 'approved'
         RETURNING {EXPENSE_COLUMNS}"
    ))
    .bind(expense_id)
    .bind(tenant_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::conflict_with_code("Only an approved expense can be marked paid.", codes::EXPENSE_NOT_APPROVED));
}
