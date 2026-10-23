use std::collections::HashSet;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::ApiError;
use crate::stock::service::{self as stock_service, Actor, Entry};

use super::models::{CreateReturnRequest, ReturnReceipt, Sale, SaleReturnLine, VoidReceipt};
use super::service::fetch_sale;

const RETURN_COLUMNS: &str = "id, return_id, sale_id, sale_line_id, quantity, base_quantity, refund_amount_minor, returned_to_stock, reason, created_at";

// === Lookups

/** The store a sale was rung up at, so the caller can be permission-checked against it. */
pub async fn sale_store_id(conn: &mut PgConnection, sale_id: Uuid) -> Result<Uuid, ApiError> {
    sqlx::query_scalar("SELECT store_id FROM atlas.sales WHERE id = $1")
        .bind(sale_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::NotFound("Sale not found.".to_string()))
}

struct SaleHead {
    store_id: Uuid,
    status: String,
    subtotal: i64,
    total: i64,
}

/**
 * Locks the sale row for the rest of the transaction. Returns and voids on
 * one sale therefore run one at a time, and each sees the other's result:
 * two returns cannot both pass the "not more than was sold" check, and a
 * void cannot slip in between a return's check and its writes.
 */
async fn lock_sale(conn: &mut PgConnection, sale_id: Uuid) -> Result<SaleHead, ApiError> {
    let row: Option<(Uuid, String, i64, i64)> = sqlx::query_as(
        "SELECT store_id, status::text, subtotal_amount_minor, total_amount_minor
         FROM atlas.sales WHERE id = $1 FOR UPDATE",
    )
    .bind(sale_id)
    .fetch_optional(conn)
    .await?;
    let (store_id, status, subtotal, total) =
        row.ok_or_else(|| ApiError::NotFound("Sale not found.".to_string()))?;
    Ok(SaleHead {
        store_id,
        status,
        subtotal,
        total,
    })
}

fn ensure_completed(status: &str) -> Result<(), ApiError> {
    match status {
        "completed" => Ok(()),
        "voided" => Err(ApiError::conflict_with_code(
            "This sale has been voided.",
            "SALE_VOIDED",
        )),
        _ => Err(ApiError::conflict_with_code(
            "This sale is not completed.",
            "SALE_NOT_COMPLETED",
        )),
    }
}

// === Returns

/**
 * What has been refunded for `returned` units of a line, as a running
 * total. A line's refund is its share of the whole sale total (so a
 * sale-level discount and tax are refunded proportionally, not ignored).
 * Computing each return as the difference of two running totals means the
 * pieces always add up to exactly the line's share, with no rounding drift
 * however the line is split across returns.
 */
fn cumulative_refund(
    line_total: i64,
    returned: Decimal,
    line_quantity: Decimal,
    sale_total: i64,
    sale_subtotal: i64,
) -> Result<i64, ApiError> {
    if sale_subtotal == 0 {
        return Ok(0);
    }
    let overflow = || ApiError::bad_request("refund amount out of range");
    let numerator = Decimal::from(line_total)
        .checked_mul(returned)
        .and_then(|value| value.checked_mul(Decimal::from(sale_total)))
        .ok_or_else(overflow)?;
    let denominator = line_quantity
        .checked_mul(Decimal::from(sale_subtotal))
        .ok_or_else(overflow)?;
    numerator
        .checked_div(denominator)
        .map(|value| value.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero))
        .and_then(|value| value.to_i64())
        .ok_or_else(overflow)
}

#[derive(sqlx::FromRow)]
struct LineForReturn {
    id: Uuid,
    product_id: Uuid,
    stock_item_id: Option<Uuid>,
    quantity: Decimal,
    base_quantity: Decimal,
    line_total_amount_minor: i64,
}

/**
 * Returns some or all of selected lines. All-or-nothing: if any line is
 * invalid or over-returned, nothing is recorded and no stock moves. Refunds
 * are computed here, never taken from the request.
 */
pub async fn create_return(
    conn: &mut PgConnection,
    actor: Actor,
    sale_id: Uuid,
    payload: &CreateReturnRequest,
) -> Result<ReturnReceipt, ApiError> {
    let reason = stock_service::require_reason(&payload.reason)?;
    if payload.lines.is_empty() {
        return Err(ApiError::bad_request_with_code(
            "a return must have at least one line",
            "VALIDATION_FAILED",
        ));
    }
    let mut seen = HashSet::new();
    for line in &payload.lines {
        stock_service::require_positive(line.quantity)?;
        if !seen.insert(line.sale_line_id) {
            return Err(ApiError::bad_request_with_code(
                "each sale line may appear once per return",
                "VALIDATION_FAILED",
            ));
        }
    }

    let head = lock_sale(conn, sale_id).await?;
    ensure_completed(&head.status)?;

    let mut refunded_so_far: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(refund_amount_minor), 0)::bigint FROM atlas.sale_returns WHERE sale_id = $1",
    )
    .bind(sale_id)
    .fetch_one(&mut *conn)
    .await?;

    struct Planned {
        line: LineForReturn,
        quantity: Decimal,
        base_quantity: Decimal,
        refund: i64,
    }
    let mut planned: Vec<Planned> = Vec::with_capacity(payload.lines.len());
    for requested in &payload.lines {
        let line: LineForReturn = sqlx::query_as(
            "SELECT id, product_id, stock_item_id, quantity, base_quantity, line_total_amount_minor
             FROM atlas.sale_lines WHERE id = $1 AND sale_id = $2",
        )
        .bind(requested.sale_line_id)
        .bind(sale_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| ApiError::NotFound("Sale line not found on this sale.".to_string()))?;

        let (returned, base_returned): (Decimal, Decimal) = sqlx::query_as(
            "SELECT COALESCE(SUM(quantity), 0), COALESCE(SUM(base_quantity), 0)
             FROM atlas.sale_returns WHERE sale_line_id = $1",
        )
        .bind(line.id)
        .fetch_one(&mut *conn)
        .await?;

        let remaining = line.quantity - returned;
        if requested.quantity > remaining {
            return Err(ApiError::conflict_with_code(
                "Cannot return more than was sold and not yet returned.",
                "RETURN_EXCEEDS_SALE",
            ));
        }

        let before = cumulative_refund(
            line.line_total_amount_minor,
            returned,
            line.quantity,
            head.total,
            head.subtotal,
        )?;
        let after = cumulative_refund(
            line.line_total_amount_minor,
            returned + requested.quantity,
            line.quantity,
            head.total,
            head.subtotal,
        )?;
        // Never refund more than the customer paid for the whole sale.
        let refund = (after - before).clamp(0, (head.total - refunded_so_far).max(0));
        refunded_so_far += refund;

        // The last unit takes whatever is left, so base quantities sum exactly.
        let base_quantity = if requested.quantity == remaining {
            line.base_quantity - base_returned
        } else {
            requested
                .quantity
                .checked_mul(line.base_quantity)
                .and_then(|value| value.checked_div(line.quantity))
                .map(|value| {
                    value.round_dp_with_strategy(6, RoundingStrategy::MidpointAwayFromZero)
                })
                .ok_or_else(|| ApiError::bad_request("quantity out of range"))?
        };
        if base_quantity <= Decimal::ZERO {
            return Err(ApiError::bad_request_with_code(
                "quantity is too small to return",
                "INVALID_QUANTITY",
            ));
        }

        planned.push(Planned {
            line,
            quantity: requested.quantity,
            base_quantity,
            refund,
        });
    }

    // Same lock order as sales, so concurrent stock changes cannot deadlock.
    planned.sort_by_key(|item| (item.line.product_id, item.line.stock_item_id));

    let return_id = Uuid::now_v7();
    let stock_reason = format!("Return: {reason}");
    let mut lines = Vec::with_capacity(planned.len());
    let mut refund_total = 0_i64;
    for item in &planned {
        let row = sqlx::query_as::<_, SaleReturnLine>(&format!(
            "INSERT INTO atlas.sale_returns
                (id, tenant_id, sale_id, sale_line_id, quantity, refund_amount_minor, reason,
                 processed_by_user_id, return_id, base_quantity, returned_to_stock)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             RETURNING {RETURN_COLUMNS}"
        ))
        .bind(Uuid::now_v7())
        .bind(actor.tenant_id)
        .bind(sale_id)
        .bind(item.line.id)
        .bind(item.quantity)
        .bind(item.refund)
        .bind(reason)
        .bind(actor.user_id)
        .bind(return_id)
        .bind(item.base_quantity)
        .bind(payload.return_to_stock)
        .fetch_one(&mut *conn)
        .await?;
        refund_total += item.refund;
        lines.push(row);

        if payload.return_to_stock {
            stock_service::credit(
                conn,
                actor,
                &Entry {
                    store_id: head.store_id,
                    product_id: item.line.product_id,
                    stock_item_id: item.line.stock_item_id,
                    movement_type: "sale_return",
                    quantity: item.base_quantity,
                    reason: &stock_reason,
                    reference_id: Some(return_id),
                },
            )
            .await?;
        }
    }

    Ok(ReturnReceipt {
        return_id,
        sale_id,
        refund_amount_minor: refund_total,
        lines,
    })
}

pub async fn list_returns(
    conn: &mut PgConnection,
    sale_id: Uuid,
) -> Result<Vec<SaleReturnLine>, ApiError> {
    sale_store_id(conn, sale_id).await?;
    sqlx::query_as::<_, SaleReturnLine>(&format!(
        "SELECT {RETURN_COLUMNS} FROM atlas.sale_returns WHERE sale_id = $1 ORDER BY created_at, id"
    ))
    .bind(sale_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from)
}

// === Voids

/**
 * Cancels a whole completed sale: marks it voided and puts exactly what
 * left stock back, line by line. Refused once any return exists - the
 * remaining quantity should be returned, not voided - so stock can never
 * be credited twice for the same goods. A voided sale keeps its lines and
 * its original ledger rows; the void adds reversing rows beside them.
 */
pub async fn void_sale(
    conn: &mut PgConnection,
    actor: Actor,
    sale_id: Uuid,
    reason: &str,
) -> Result<VoidReceipt, ApiError> {
    let reason = stock_service::require_reason(reason)?;

    let head = lock_sale(conn, sale_id).await?;
    ensure_completed(&head.status)?;

    let has_returns: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM atlas.sale_returns WHERE sale_id = $1 LIMIT 1")
            .bind(sale_id)
            .fetch_optional(&mut *conn)
            .await?;
    if has_returns.is_some() {
        return Err(ApiError::conflict_with_code(
            "This sale already has returns; return the remaining items instead of voiding.",
            "SALE_HAS_RETURNS",
        ));
    }

    sqlx::query(
        "UPDATE atlas.sales
         SET status = 'voided', voided_at = now(), voided_by_user_id = $2, void_reason = $3
         WHERE id = $1",
    )
    .bind(sale_id)
    .bind(actor.user_id)
    .bind(reason)
    .execute(&mut *conn)
    .await?;

    let lines: Vec<(Uuid, Option<Uuid>, Decimal)> = sqlx::query_as(
        "SELECT product_id, stock_item_id, base_quantity FROM atlas.sale_lines
         WHERE sale_id = $1 ORDER BY product_id, stock_item_id",
    )
    .bind(sale_id)
    .fetch_all(&mut *conn)
    .await?;
    let stock_reason = format!("Void: {reason}");
    for (product_id, stock_item_id, base_quantity) in lines {
        stock_service::credit(
            conn,
            actor,
            &Entry {
                store_id: head.store_id,
                product_id,
                stock_item_id,
                movement_type: "sale_return",
                quantity: base_quantity,
                reason: &stock_reason,
                reference_id: Some(sale_id),
            },
        )
        .await?;
    }

    let sale: Sale = fetch_sale(conn, sale_id)
        .await?
        .ok_or_else(|| ApiError::Internal("sale vanished after void".to_string()))?;
    Ok(VoidReceipt {
        refund_amount_minor: head.total,
        sale,
    })
}
