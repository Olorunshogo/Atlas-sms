use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExpenseCategory {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateExpenseCategoryRequest {
    pub name: String,
}

#[derive(Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "expense_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ExpenseStatus {
    Draft,
    Submitted,
    Approved,
    Rejected,
}

#[derive(Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "expense_payment_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ExpensePaymentStatus {
    Unpaid,
    Paid,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Expense {
    pub id: Uuid,
    pub store_id: Uuid,
    pub category_id: Uuid,
    pub amount_minor: i64,
    pub expense_date: NaiveDate,
    pub description: String,
    pub reference: Option<String>,
    pub status: ExpenseStatus,
    pub payment_status: ExpensePaymentStatus,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub decided_by_user_id: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateExpenseRequest {
    pub store_id: Uuid,
    pub category_id: Uuid,
    pub amount_minor: i64,
    pub expense_date: NaiveDate,
    pub description: String,
    pub reference: Option<String>,
}

/// `draft` only - edit before approval (FR-24). Every field optional; only what's given is changed.
#[derive(Debug, Deserialize)]
pub struct EditExpenseRequest {
    pub category_id: Option<Uuid>,
    pub amount_minor: Option<i64>,
    pub expense_date: Option<NaiveDate>,
    pub description: Option<String>,
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListExpensesQuery {
    pub store_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub status: Option<ExpenseStatus>,
    pub created_by_user_id: Option<Uuid>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}
