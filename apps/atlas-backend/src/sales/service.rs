use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};
use crate::payments::service as payments;

use super::models::{CloseShiftRequest, OpenShiftRequest, Sale, SaleDetail, SaleLine, Shift};

// === Shifts

const SHIFT_COLUMNS: &str = "id, store_id, opened_by_user_id, opened_at, opening_cash_amount_minor,
     closed_by_user_id, closed_at, closing_cash_amount_minor, status::text AS status";

pub async fn open_shift(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    payload: &OpenShiftRequest,
) -> Result<Shift, ApiError> {
    return sqlx::query_as::<_, Shift>(&format!(
        "INSERT INTO atlas.shifts (id, tenant_id, store_id, opened_by_user_id, opening_cash_amount_minor)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING {}",
        SHIFT_COLUMNS
    ))
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(payload.store_id)
    .bind(user_id)
    .bind(payload.opening_cash_amount_minor)
    .fetch_one(conn)
    .await
    .map_err(|err| match err {
        // shifts_one_open_per_store_idx - the store already has an open shift.
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            ApiError::conflict_with_code("This store already has an open shift.", codes::SHIFT_ALREADY_OPEN)
        }
        other => ApiError::from(other),
    });
}

pub async fn close_shift(
    conn: &mut PgConnection,
    shift_id: Uuid,
    user_id: Uuid,
    payload: &CloseShiftRequest,
) -> Result<Shift, ApiError> {
    return sqlx::query_as::<_, Shift>(&format!(
        "UPDATE atlas.shifts
         SET status = 'closed', closed_by_user_id = $2, closed_at = now(), closing_cash_amount_minor = $3
         WHERE id = $1 AND status = 'open'
         RETURNING {}",
        SHIFT_COLUMNS
    ))
    .bind(shift_id)
    .bind(user_id)
    .bind(payload.closing_cash_amount_minor)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("Shift not found or already closed."));
}

pub async fn shift_store_id(conn: &mut PgConnection, shift_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar("SELECT store_id FROM atlas.shifts WHERE id = $1")
        .bind(shift_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::not_found("Shift not found."));
}

// === Sales

pub const SALE_COLUMNS: &str = "s.id, s.store_id, s.shift_id, s.customer_id, s.cashier_user_id, s.device_id,
     s.status::text AS status, s.receipt_number, s.currency, s.subtotal_amount_minor, s.discount_amount_minor,
     s.cart_discount_amount_minor, s.discount_reason, s.tax_amount_minor, s.total_amount_minor,
     s.rounding_adjustment_minor, s.amount_paid_minor, s.amount_refunded_minor,
     s.payment_status::text AS payment_status, s.prices_include_tax, s.price_overridden, s.client_created_at,
     s.created_at, s.completed_at, s.voided_at, s.voided_by_user_id, s.request_hash";

const LINE_COLUMNS: &str = "id, sale_id, line_number, product_id, product_name, sku, stock_item_id, unit_id, unit_name,
     quantity, base_quantity, list_unit_price_amount_minor, unit_price_amount_minor, price_rule_id, price_overridden,
     price_override_reason, line_discount_amount_minor, allocated_cart_discount_amount_minor, discount_reason,
     net_amount_minor, tax_rate_id, tax_rate_basis_points, tax_amount_minor, line_total_amount_minor";

pub async fn find_sale(conn: &mut PgConnection, sale_id: Uuid) -> Result<Option<Sale>, ApiError> {
    return sqlx::query_as::<_, Sale>(&format!("SELECT {} FROM atlas.sales s WHERE s.id = $1", SALE_COLUMNS))
        .bind(sale_id)
        .fetch_optional(conn)
        .await
        .map_err(ApiError::from);
}

pub async fn get_sale(conn: &mut PgConnection, sale_id: Uuid) -> Result<Sale, ApiError> {
    return find_sale(conn, sale_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Sale not found."));
}

/**
 * Takes the sale's row lock - the first lock in the documented order (sale row, then sequence rows, then inventory advisory locks) - so payments, voids, returns and refunds against one sale serialise instead of each passing its checks against stale totals.
 */
pub async fn lock_sale(conn: &mut PgConnection, sale_id: Uuid) -> Result<Sale, ApiError> {
    return sqlx::query_as::<_, Sale>(&format!(
        "SELECT {} FROM atlas.sales s WHERE s.id = $1 FOR UPDATE",
        SALE_COLUMNS
    ))
    .bind(sale_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::not_found("Sale not found."));
}

pub async fn sale_detail(conn: &mut PgConnection, sale: Sale) -> Result<SaleDetail, ApiError> {
    let lines = list_sale_lines(conn, sale.id).await?;
    let payments = payments::list_payments_for_sale(conn, sale.id).await?;
    let refunds = payments::list_refunds_for_sale(conn, sale.id).await?;
    return Ok(SaleDetail { sale, lines, payments, refunds });
}

pub async fn list_sale_lines(conn: &mut PgConnection, sale_id: Uuid) -> Result<Vec<SaleLine>, ApiError> {
    return sqlx::query_as::<_, SaleLine>(&format!(
        "SELECT {} FROM atlas.sale_lines WHERE sale_id = $1 ORDER BY line_number, id",
        LINE_COLUMNS
    ))
    .bind(sale_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

/**
 * Cancels a sale on the day it happened: puts its stock back (`sale_void` movements), reverses every live payment, and marks it `voided`.
 * Only while its shift is still open and before anything has been returned against it - after that, money and goods move through a return instead, so the paper trail matches what physically happened.
 */
pub async fn void_sale(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    sale_id: Uuid,
    actor_user_id: Uuid,
    reason: &str,
) -> Result<Sale, ApiError> {
    if reason.trim().is_empty() {
        return Err(ApiError::bad_request("a void needs a reason"));
    }
    let sale = lock_sale(conn, sale_id).await?;
    if sale.status != "completed" && sale.status != "awaiting_payment" {
        return Err(ApiError::conflict_with_code("Only a completed or awaiting-payment sale can be voided.", codes::SALE_NOT_VOIDABLE));
    }
    let shift_open: bool = sqlx::query_scalar("SELECT status = 'open' FROM atlas.shifts WHERE id = $1")
        .bind(sale.shift_id)
        .fetch_one(&mut *conn)
        .await?;
    if !shift_open {
        return Err(ApiError::conflict_with_code(
            "This sale's shift is closed; take the goods back as a return instead.",
            codes::SALE_NOT_VOIDABLE,
        ));
    }
    let has_returns: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM atlas.returns WHERE sale_id = $1 AND status IN ('pending_approval', 'completed'))",
    )
    .bind(sale_id)
    .fetch_one(&mut *conn)
    .await?;
    if sale.amount_refunded_minor > 0 || has_returns {
        return Err(ApiError::conflict_with_code(
            "Part of this sale has already been returned or refunded; it can no longer be voided.",
            codes::SALE_NOT_VOIDABLE,
        ));
    }

    if sale.status == "completed" {
        // Reverses what the sale actually posted, so batches split first-expiry-first-out go back to the same batches
        let sold: Vec<(Uuid, Option<Uuid>, Decimal)> = sqlx::query_as(
            "SELECT product_id, stock_item_id, quantity FROM atlas.stock_movements
             WHERE reference_id = $1 AND movement_type = 'sale' ORDER BY id",
        )
        .bind(sale_id)
        .fetch_all(&mut *conn)
        .await?;
        let posting_lines: Vec<PostingLine> = sold
            .into_iter()
            .map(|(product_id, stock_item_id, quantity)| PostingLine { product_id, stock_item_id, unit_id: None, quantity: -quantity })
            .collect();
        inventory::service::post(
            conn,
            Posting {
                tenant_id,
                store_id: sale.store_id,
                movement_type: MovementType::SaleVoid,
                reason,
                reference_id: Some(sale_id),
                actor_user_id,
                policy: NegativeStockPolicy::Allow,
                source: "sale_void",
                lines: &posting_lines,
            },
        )
        .await?;
    }

    sqlx::query(
        "UPDATE atlas.payments
         SET status = CASE WHEN status = 'pending' THEN 'failed'::atlas.payment_status ELSE 'reversed'::atlas.payment_status END,
             status_reason = $2, updated_at = now()
         WHERE sale_id = $1 AND status IN ('pending', 'succeeded')",
    )
    .bind(sale_id)
    .bind(format!("Sale voided: {}", reason.trim()))
    .execute(&mut *conn)
    .await?;

    sqlx::query(
        "UPDATE atlas.sales
         SET status = 'voided', voided_at = now(), voided_by_user_id = $2,
             amount_paid_minor = 0, payment_status = 'unpaid'
         WHERE id = $1",
    )
    .bind(sale_id)
    .bind(actor_user_id)
    .execute(&mut *conn)
    .await?;

    return get_sale(conn, sale_id).await;
}
