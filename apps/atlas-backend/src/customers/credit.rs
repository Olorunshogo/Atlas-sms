/*!
 * Store credit. Every change to a customer's balance writes a `customer_credit_movements` row in the same
 * transaction, so the balance can always be explained row by row: refunds add to it, checkout spends it, and refunding
 * or reversing a credit payment gives it back to whoever spent it.
 */
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};

pub struct Entry {
    pub tenant_id: Uuid,
    pub customer_id: Uuid,
    /// Positive adds credit, negative spends it.
    pub amount_minor: i64,
    pub sale_id: Option<Uuid>,
    pub refund_id: Option<Uuid>,
    pub payment_id: Option<Uuid>,
    pub user_id: Uuid,
}

async fn post(conn: &mut PgConnection, entry: &Entry) -> Result<(), ApiError> {
    // Guarded in the UPDATE itself, so two tills spending the same balance at once can't both succeed
    let updated: Option<i64> = sqlx::query_scalar(
        "UPDATE atlas.customers SET store_credit_amount_minor = store_credit_amount_minor + $2, updated_at = now()
         WHERE id = $1 AND store_credit_amount_minor + $2 >= 0
         RETURNING store_credit_amount_minor",
    )
    .bind(entry.customer_id)
    .bind(entry.amount_minor)
    .fetch_optional(&mut *conn)
    .await?;
    if updated.is_none() {
        let balance: i64 = sqlx::query_scalar("SELECT store_credit_amount_minor FROM atlas.customers WHERE id = $1")
            .bind(entry.customer_id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or_else(|| ApiError::not_found("Customer not found."))?;
        return Err(ApiError::conflict_with_code(
            format!("The customer has only {} in store credit.", balance),
            codes::INSUFFICIENT_STORE_CREDIT,
        ));
    }
    sqlx::query(
        "INSERT INTO atlas.customer_credit_movements (id, tenant_id, customer_id, amount_minor, refund_id, sale_id, payment_id, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::now_v7())
    .bind(entry.tenant_id)
    .bind(entry.customer_id)
    .bind(entry.amount_minor)
    .bind(entry.refund_id)
    .bind(entry.sale_id)
    .bind(entry.payment_id)
    .bind(entry.user_id)
    .execute(conn)
    .await?;
    return Ok(());
}

pub async fn add(conn: &mut PgConnection, entry: Entry) -> Result<(), ApiError> {
    return post(conn, &entry).await;
}

/// Takes a store credit payment out of the customer's balance; `409 INSUFFICIENT_STORE_CREDIT` if it doesn't cover it.
pub async fn spend(conn: &mut PgConnection, tenant_id: Uuid, customer_id: Uuid, amount_minor: i64, sale_id: Uuid, payment_id: Uuid, user_id: Uuid) -> Result<(), ApiError> {
    return post(
        conn,
        &Entry { tenant_id, customer_id, amount_minor: -amount_minor, sale_id: Some(sale_id), refund_id: None, payment_id: Some(payment_id), user_id },
    )
    .await;
}

/// Puts back part or all of a store credit payment - refunded or reversed - for the customer who spent it.
pub async fn give_back(conn: &mut PgConnection, tenant_id: Uuid, payment_id: Uuid, amount_minor: i64, refund_id: Option<Uuid>, user_id: Uuid) -> Result<(), ApiError> {
    let spent: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT customer_id, sale_id FROM atlas.customer_credit_movements WHERE payment_id = $1 AND amount_minor < 0 LIMIT 1",
    )
    .bind(payment_id)
    .fetch_optional(&mut *conn)
    .await?;
    let (customer_id, sale_id) = spent.ok_or_else(|| ApiError::Internal("store credit payment without its ledger row".to_string()))?;
    return post(conn, &Entry { tenant_id, customer_id, amount_minor, sale_id, refund_id, payment_id: Some(payment_id), user_id }).await;
}
