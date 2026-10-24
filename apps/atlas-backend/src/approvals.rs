/*!
 * FR-29 approval policy, read from the store: quantity thresholds above which a stock change needs `stock.approve`,
 * and whether anyone may approve their own transaction.
 */
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};
use crate::roles;

#[derive(Debug, Clone, Copy)]
pub enum Threshold {
    Adjustment,
    WriteOff,
}

/**
 * A stock change bigger than the store's threshold needs `stock.approve`; anyone else is refused with
 * `409 APPROVAL_REQUIRED` and nothing is written. A stock count is the route for a large correction by staff.
 */
pub async fn assert_within_threshold(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    store_id: Uuid,
    user_id: Uuid,
    threshold: Threshold,
    quantity: Decimal,
) -> Result<(), ApiError> {
    let column = match threshold {
        Threshold::Adjustment => "adjustment_approval_quantity",
        Threshold::WriteOff => "write_off_approval_quantity",
    };
    let limit: Option<Decimal> = sqlx::query_scalar(&format!("SELECT {column} FROM atlas.stores WHERE id = $1"))
        .bind(store_id)
        .fetch_optional(&mut *conn)
        .await?
        .flatten();
    let Some(limit) = limit else { return Ok(()) };
    if quantity.abs() <= limit {
        return Ok(());
    }
    if roles::service::has_permission(conn, user_id, tenant_id, Some(store_id), "stock.approve").await? {
        return Ok(());
    }
    return Err(ApiError::conflict_with_code(
        format!("Changes over {} need someone with stock.approve.", limit.normalize()),
        codes::APPROVAL_REQUIRED,
    ));
}

/// When the store turns self-approval off, whoever raised a transaction can't approve it.
pub async fn assert_may_approve(conn: &mut PgConnection, store_id: Uuid, approver_id: Uuid, author_id: Uuid) -> Result<(), ApiError> {
    if approver_id != author_id {
        return Ok(());
    }
    let allowed: Option<bool> = sqlx::query_scalar("SELECT allow_self_approval FROM atlas.stores WHERE id = $1")
        .bind(store_id)
        .fetch_optional(conn)
        .await?;
    if allowed.unwrap_or(true) {
        return Ok(());
    }
    return Err(ApiError::conflict_with_code(
        "This store requires someone else to approve what you raised.",
        codes::APPROVAL_SELF_FORBIDDEN,
    ));
}
