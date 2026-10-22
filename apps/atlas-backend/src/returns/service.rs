use chrono::{DateTime, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::audit;
use crate::crypto;
use crate::error::{codes, ApiError};
use crate::inventory::{self, MovementType, NegativeStockPolicy, Posting, PostingLine};
use crate::payments::models::Payment;
use crate::payments::service as payments;
use crate::pricing::engine;
use crate::roles;
use crate::sales::receipts;
use crate::sales::service as sales;

use super::models::{
    CreateReturnRequest, RefundMethod, Return, ReturnDetail, ReturnLine, CONDITIONS,
};

const RETURN_COLUMNS: &str = "id, store_id, sale_id, return_number, status::text AS status, reason_code, notes,
     refund_method, refund_payment_method_id, outside_window, refund_total_minor, requested_by_user_id,
     approved_by_user_id, approved_at, rejected_by_user_id, rejection_reason, completed_at, device_id, created_at,
     refund_reference, request_hash";

#[derive(sqlx::FromRow)]
struct ReturnPolicy {
    timezone: String,
    return_window_days: i32,
    refund_approval_threshold_minor: Option<i64>,
    restockable_conditions: Vec<String>,
}

/// One original sale line with everything already returned and refunded against it.
#[derive(sqlx::FromRow)]
struct ReturnableLine {
    product_name: String,
    quantity: Decimal,
    base_quantity: Decimal,
    line_total_amount_minor: i64,
    tax_amount_minor: i64,
    is_returnable: bool,
    returned_quantity: Decimal,
    refunded_minor: i64,
    tax_refunded_minor: i64,
}

/**
 * Whether `now` is still inside the return window: until the end of the `days`-th local day after the sale completed, in the store's own timezone - "7 days" means a customer who bought on Monday afternoon has all of the following Monday, not until Monday afternoon.
 */
pub fn within_window(completed_at: DateTime<Utc>, days: i32, timezone: &str, now: DateTime<Utc>) -> bool {
    let tz: chrono_tz::Tz = timezone.parse().unwrap_or(chrono_tz::Africa::Lagos);
    let last_day = completed_at.with_timezone(&tz).date_naive() + Duration::days(i64::from(days));
    let Some(midnight) = last_day.succ_opt().and_then(|d| d.and_hms_opt(0, 0, 0)) else {
        return false;
    };
    return match tz.from_local_datetime(&midnight).earliest() {
        Some(deadline) => now < deadline.with_timezone(&Utc),
        None => false,
    };
}

pub async fn get_return(conn: &mut PgConnection, return_id: Uuid) -> Result<Return, ApiError> {
    return sqlx::query_as::<_, Return>(&format!("SELECT {} FROM atlas.returns WHERE id = $1", RETURN_COLUMNS))
        .bind(return_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::not_found("Return not found."));
}

pub async fn list_lines(conn: &mut PgConnection, return_id: Uuid) -> Result<Vec<ReturnLine>, ApiError> {
    return sqlx::query_as::<_, ReturnLine>(
        "SELECT rl.id, rl.return_id, rl.sale_line_id, sl.product_id, sl.product_name, sl.stock_item_id, sl.unit_id,
                sl.unit_name, rl.quantity, rl.base_quantity, rl.condition::text AS condition, rl.restock,
                rl.refund_amount_minor, rl.tax_refund_minor, rl.stock_movement_id
         FROM atlas.return_lines rl
         JOIN atlas.sale_lines sl ON sl.id = rl.sale_line_id
         WHERE rl.return_id = $1
         ORDER BY sl.line_number, rl.id",
    )
    .bind(return_id)
    .fetch_all(conn)
    .await
    .map_err(ApiError::from);
}

pub async fn detail(conn: &mut PgConnection, header: Return) -> Result<ReturnDetail, ApiError> {
    let lines = list_lines(conn, header.id).await?;
    let refunds = sqlx::query_as::<_, crate::payments::models::Refund>(&format!(
        "{} WHERE r.return_id = $1 ORDER BY r.created_at, r.id",
        payments::REFUND_SELECT
    ))
    .bind(header.id)
    .fetch_all(&mut *conn)
    .await?;
    return Ok(ReturnDetail { header, lines, refunds });
}

/**
 * Starts a return against a completed sale (FR-18). With the sale row locked - so two returns of the same last unit can't both pass - it checks every line against what was sold and what earlier returns already claimed, prices the refund pro-rata on what the customer actually paid, and classifies each item by condition.
 * A return needs approval when it's outside the store's return window, when its refund reaches the approval threshold, or when the requester can't authorise refunds themselves (`sale.refund`); it's then held as `pending_approval` with its quantities reserved. Otherwise it completes immediately.
 * Idempotent by the client-generated `payload.id`.
 */
pub async fn create_return(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    sale_id: Uuid,
    user_id: Uuid,
    payload: &CreateReturnRequest,
) -> Result<ReturnDetail, ApiError> {
    let request_hash = crypto::hash_lookup_token(&serde_json::to_string(payload).map_err(|e| ApiError::Internal(e.to_string()))?);
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("return:{}", payload.id))
        .execute(&mut *conn)
        .await?;
    if let Ok(existing) = get_return(conn, payload.id).await {
        if existing.request_hash != request_hash || existing.sale_id != sale_id {
            return Err(ApiError::conflict_with_code("A different return was already recorded with this id.", codes::IDEMPOTENCY_CONFLICT));
        }
        return detail(conn, existing).await;
    }

    if payload.lines.is_empty() {
        return Err(ApiError::bad_request("a return must have at least one line"));
    }
    if payload.reason_code.trim().is_empty() {
        return Err(ApiError::bad_request("a return needs a reason"));
    }

    let sale = sales::lock_sale(conn, sale_id).await?;
    if sale.status != "completed" {
        return Err(ApiError::conflict_with_code("Only a completed sale can be returned against.", codes::SALE_NOT_COMPLETED));
    }
    let policy = sqlx::query_as::<_, ReturnPolicy>(
        "SELECT timezone, return_window_days, refund_approval_threshold_minor, restockable_conditions::text[] AS restockable_conditions
         FROM atlas.stores WHERE id = $1",
    )
    .bind(sale.store_id)
    .fetch_one(&mut *conn)
    .await?;
    let completed_at = sale.completed_at.unwrap_or(sale.created_at);
    let outside_window = !within_window(completed_at, policy.return_window_days, &policy.timezone, Utc::now());

    check_refund_target(conn, payload, sale.customer_id).await?;

    let mut seen = std::collections::HashSet::new();
    let mut prepared = Vec::with_capacity(payload.lines.len());
    for input in &payload.lines {
        if !seen.insert(input.sale_line_id) {
            return Err(ApiError::bad_request("each sale line can appear only once in a return"));
        }
        if input.quantity <= Decimal::ZERO {
            return Err(ApiError::bad_request("return quantities must be positive"));
        }
        if !CONDITIONS.contains(&input.condition.as_str()) {
            return Err(ApiError::bad_request(format!("condition must be one of {}", CONDITIONS.join(", "))));
        }
        let line = returnable_line(conn, sale_id, input.sale_line_id).await?;
        if !line.is_returnable {
            return Err(ApiError::bad_request_with_code(
                format!("{} can't be returned.", line.product_name),
                codes::ITEM_NOT_RETURNABLE,
            ));
        }
        let remaining = line.quantity - line.returned_quantity;
        if input.quantity > remaining {
            return Err(ApiError::conflict_with_code(
                format!(
                    "Only {} of {} can still be returned.",
                    remaining.max(Decimal::ZERO).normalize(),
                    line.product_name
                ),
                codes::RETURN_QUANTITY_EXCEEDED,
            ));
        }
        let refund = engine::prorated_refund(line.line_total_amount_minor, line.quantity, line.returned_quantity, line.refunded_minor, input.quantity);
        let tax_refund = engine::prorated_refund(line.tax_amount_minor, line.quantity, line.returned_quantity, line.tax_refunded_minor, input.quantity);
        let base_quantity = line.base_quantity * input.quantity / line.quantity;
        let restock = policy.restockable_conditions.contains(&input.condition);
        prepared.push((input, refund, tax_refund, base_quantity, restock));
    }

    let refund_total: i64 = prepared.iter().map(|(_, refund, ..)| refund).sum();
    let pending_elsewhere: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(refund_total_minor), 0)::bigint FROM atlas.returns WHERE sale_id = $1 AND status = 'pending_approval'",
    )
    .bind(sale_id)
    .fetch_one(&mut *conn)
    .await?;
    let refundable = payments::refundable_on_sale(conn, sale_id).await? - pending_elsewhere;
    if refund_total > refundable {
        return Err(ApiError::conflict_with_code(
            format!(
                "This return is worth {} but only {} paid on the sale can still be refunded. Settle the sale's balance first.",
                refund_total,
                refundable.max(0)
            ),
            codes::REFUND_AMOUNT_EXCEEDED,
        ));
    }

    let can_refund = roles::service::has_permission(conn, user_id, tenant_id, Some(sale.store_id), "sale.refund").await?;
    let over_threshold = policy.refund_approval_threshold_minor.is_some_and(|t| refund_total >= t);
    let needs_approval = outside_window || over_threshold || !can_refund;

    sqlx::query(
        "INSERT INTO atlas.returns
            (id, tenant_id, store_id, sale_id, status, reason_code, notes, refund_method, refund_payment_method_id,
             refund_reference, outside_window, refund_total_minor, requested_by_user_id, device_id, request_hash)
         VALUES ($1, $2, $3, $4, 'pending_approval', $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(payload.id)
    .bind(tenant_id)
    .bind(sale.store_id)
    .bind(sale_id)
    .bind(payload.reason_code.trim())
    .bind(&payload.notes)
    .bind(payload.refund.method.as_str())
    .bind(payload.refund.payment_method_id)
    .bind(payload.refund.reference.as_deref().map(str::trim).filter(|r| !r.is_empty()))
    .bind(outside_window)
    .bind(refund_total)
    .bind(user_id)
    .bind(&payload.device_id)
    .bind(&request_hash)
    .execute(&mut *conn)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            ApiError::conflict_with_code("A different return was already recorded with this id.", codes::IDEMPOTENCY_CONFLICT)
        }
        other => ApiError::from(other),
    })?;
    for (input, refund, tax_refund, base_quantity, restock) in &prepared {
        sqlx::query(
            "INSERT INTO atlas.return_lines
                (id, tenant_id, return_id, sale_line_id, quantity, base_quantity, condition, restock, refund_amount_minor, tax_refund_minor)
             VALUES ($1, $2, $3, $4, $5, $6, $7::atlas.item_condition, $8, $9, $10)",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(payload.id)
        .bind(input.sale_line_id)
        .bind(input.quantity)
        .bind(base_quantity)
        .bind(&input.condition)
        .bind(restock)
        .bind(refund)
        .bind(tax_refund)
        .execute(&mut *conn)
        .await?;
    }

    audit::record_change(
        conn,
        audit::Change {
            tenant_id,
            actor_user_id: user_id,
            device_id: payload.device_id.as_deref(),
            entity_type: "return",
            entity_id: payload.id,
            action: "return.create",
            before: None,
            after: Some(serde_json::json!({
                "sale_id": sale_id,
                "refund_total_minor": refund_total,
                "outside_window": outside_window,
                "needs_approval": needs_approval,
            })),
        },
    )
    .await?;

    if !needs_approval {
        complete_return(conn, tenant_id, payload.id, user_id, None).await?;
    }
    let header = get_return(conn, payload.id).await?;
    return detail(conn, header).await;
}

/// Approves a pending return and completes it. The approver must hold `sale.refund` (checked by the caller) and can't be the person who asked for it.
pub async fn approve(conn: &mut PgConnection, tenant_id: Uuid, return_id: Uuid, approver_id: Uuid) -> Result<ReturnDetail, ApiError> {
    let header = get_return(conn, return_id).await?;
    sales::lock_sale(conn, header.sale_id).await?;
    let header = get_return(conn, return_id).await?;
    if header.status != "pending_approval" {
        return Err(ApiError::conflict_with_code("This return isn't waiting for approval.", codes::RETURN_NOT_PENDING));
    }
    if header.requested_by_user_id == approver_id {
        return Err(ApiError::forbidden_with_code(
            "A return has to be approved by someone other than the person who started it.",
            codes::APPROVAL_SELF_FORBIDDEN,
        ));
    }
    complete_return(conn, tenant_id, return_id, approver_id, Some(approver_id)).await?;
    let header = get_return(conn, return_id).await?;
    return detail(conn, header).await;
}

/// Turns a pending return down. Its quantities are released for a future return; nothing moves.
pub async fn reject(conn: &mut PgConnection, return_id: Uuid, user_id: Uuid, reason: &str) -> Result<ReturnDetail, ApiError> {
    if reason.trim().is_empty() {
        return Err(ApiError::bad_request("a rejection needs a reason"));
    }
    let header = get_return(conn, return_id).await?;
    sales::lock_sale(conn, header.sale_id).await?;
    let updated = sqlx::query(
        "UPDATE atlas.returns SET status = 'rejected', rejected_by_user_id = $2, rejection_reason = $3
         WHERE id = $1 AND status = 'pending_approval'",
    )
    .bind(return_id)
    .bind(user_id)
    .bind(reason.trim())
    .execute(&mut *conn)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::conflict_with_code("This return isn't waiting for approval.", codes::RETURN_NOT_PENDING));
    }
    let header = get_return(conn, return_id).await?;
    return detail(conn, header).await;
}

/**
 * Executes a return: numbers it, puts restockable items back through the inventory service (only those - damaged or expired goods are recorded but never re-enter saleable stock), pays the refund out by the chosen method, and adds it to the sale's refunded total.
 * Expects the sale row to be locked. Locks follow the documented order: sale row, return counter, inventory.
 */
async fn complete_return(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    return_id: Uuid,
    executor_id: Uuid,
    approver_id: Option<Uuid>,
) -> Result<(), ApiError> {
    let header = get_return(conn, return_id).await?;
    let sale = sales::get_sale(conn, header.sale_id).await?;

    // Pending returns don't hold money, so it may have been refunded elsewhere since this one was requested.
    if header.refund_total_minor > payments::refundable_on_sale(conn, sale.id).await? {
        return Err(ApiError::conflict_with_code("The sale no longer has enough paid to cover this refund.", codes::REFUND_AMOUNT_EXCEEDED));
    }

    let return_number = next_return_number(conn, tenant_id, sale.store_id).await?;

    let lines = list_lines(conn, return_id).await?;
    let restocked: Vec<&ReturnLine> = lines.iter().filter(|l| l.restock).collect();
    if !restocked.is_empty() {
        let posting_lines: Vec<PostingLine> = restocked
            .iter()
            .map(|line| PostingLine {
                product_id: line.product_id,
                stock_item_id: line.stock_item_id,
                unit_id: Some(line.unit_id),
                quantity: line.quantity,
            })
            .collect();
        let movements = inventory::service::post(
            conn,
            Posting {
                tenant_id,
                store_id: sale.store_id,
                movement_type: MovementType::SaleReturn,
                reason: &format!("Return {}", return_number),
                reference_id: Some(return_id),
                actor_user_id: executor_id,
                policy: NegativeStockPolicy::Allow,
                source: "sale_return",
                lines: &posting_lines,
            },
        )
        .await?;
        for (line, movement) in restocked.iter().zip(movements) {
            sqlx::query("UPDATE atlas.return_lines SET stock_movement_id = $2 WHERE id = $1")
                .bind(line.id)
                .bind(movement.id)
                .execute(&mut *conn)
                .await?;
        }
    }

    let shift_id = payments::open_shift_at(conn, sale.store_id).await?;
    let refund = RefundContext {
        tenant_id,
        sale_id: sale.id,
        return_id,
        reason: &header.reason_code,
        executor_id,
        approver_id,
        shift_id,
    };
    if header.refund_total_minor > 0 {
        match header.refund_method.as_str() {
            "original" => refund_to_original(conn, &refund, header.refund_total_minor).await?,
            "store_credit" => {
                let customer_id = sale.customer_id.ok_or_else(|| {
                    ApiError::bad_request_with_code("A store credit refund needs a customer on the sale.", codes::CUSTOMER_REQUIRED)
                })?;
                refund_to_store_credit(conn, &refund, customer_id, header.refund_total_minor).await?;
            }
            _ => {
                let method_id = header.refund_payment_method_id.ok_or_else(|| ApiError::bad_request("payment_method_id is required"))?;
                insert_refund(conn, &refund, None, method_id, header.refund_total_minor, header.refund_reference.as_deref()).await?;
            }
        }
    }

    sqlx::query("UPDATE atlas.sales SET amount_refunded_minor = amount_refunded_minor + $2 WHERE id = $1")
        .bind(sale.id)
        .bind(header.refund_total_minor)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "UPDATE atlas.returns
         SET status = 'completed', completed_at = now(), return_number = $2,
             approved_by_user_id = COALESCE($3, approved_by_user_id),
             approved_at = CASE WHEN $3 IS NOT NULL THEN now() ELSE approved_at END
         WHERE id = $1",
    )
    .bind(return_id)
    .bind(&return_number)
    .bind(approver_id)
    .execute(&mut *conn)
    .await?;

    audit::record(
        conn,
        tenant_id,
        executor_id,
        "return",
        return_id,
        "return.complete",
        Some(serde_json::json!({ "return_number": return_number, "refund_total_minor": header.refund_total_minor })),
    )
    .await?;
    return Ok(());
}

struct RefundContext<'a> {
    tenant_id: Uuid,
    sale_id: Uuid,
    return_id: Uuid,
    reason: &'a str,
    executor_id: Uuid,
    approver_id: Option<Uuid>,
    shift_id: Option<Uuid>,
}

async fn insert_refund(
    conn: &mut PgConnection,
    ctx: &RefundContext<'_>,
    payment_id: Option<Uuid>,
    payment_method_id: Uuid,
    amount_minor: i64,
    reference: Option<&str>,
) -> Result<Uuid, ApiError> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO atlas.refunds
            (id, tenant_id, sale_id, return_id, payment_id, payment_method_id, amount_minor, status, reason, reference,
             requested_by_user_id, approved_by_user_id, approved_at, executed_by_user_id, executed_at, shift_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'executed', $8, $9, $10, $11, CASE WHEN $11 IS NOT NULL THEN now() END, $10, now(), $12)",
    )
    .bind(id)
    .bind(ctx.tenant_id)
    .bind(ctx.sale_id)
    .bind(ctx.return_id)
    .bind(payment_id)
    .bind(payment_method_id)
    .bind(amount_minor)
    .bind(ctx.reason)
    .bind(reference)
    .bind(ctx.executor_id)
    .bind(ctx.approver_id)
    .bind(ctx.shift_id)
    .execute(&mut *conn)
    .await?;
    return Ok(id);
}

/**
 * Sends the refund back the way it came in, across the sale's payments in order: card, transfer and other non-cash first (the money goes back to where it can be traced), cash last. Each payment gives back at most what's left on it.
 */
async fn refund_to_original(conn: &mut PgConnection, ctx: &RefundContext<'_>, amount_minor: i64) -> Result<(), ApiError> {
    let paid: Vec<Payment> = payments::list_payments_for_sale(conn, ctx.sale_id)
        .await?
        .into_iter()
        .filter(|p| p.status == "succeeded" || p.status == "partially_refunded")
        .collect();
    let mut ordered: Vec<&Payment> = paid.iter().filter(|p| p.method != "cash").collect();
    ordered.extend(paid.iter().filter(|p| p.method == "cash"));

    let mut remaining = amount_minor;
    for payment in ordered {
        if remaining == 0 {
            break;
        }
        let take = remaining.min(payment.amount_minor - payment.refunded_amount_minor);
        if take <= 0 {
            continue;
        }
        let method_id = payment.payment_method_id.ok_or_else(|| ApiError::Internal("payment without a method".to_string()))?;
        insert_refund(conn, ctx, Some(payment.id), method_id, take, None).await?;
        payments::apply_refund_to_payment(conn, payment.id, take).await?;
        remaining -= take;
    }
    if remaining > 0 {
        return Err(ApiError::conflict_with_code(
            "The sale's payments don't have enough left to refund this amount.",
            codes::REFUND_AMOUNT_EXCEEDED,
        ));
    }
    return Ok(());
}

async fn refund_to_store_credit(conn: &mut PgConnection, ctx: &RefundContext<'_>, customer_id: Uuid, amount_minor: i64) -> Result<(), ApiError> {
    let method = payments::store_credit_method(conn).await?;
    let refund_id = insert_refund(conn, ctx, None, method.id, amount_minor, None).await?;
    sqlx::query(
        "INSERT INTO atlas.customer_credit_movements (id, tenant_id, customer_id, amount_minor, refund_id, sale_id, created_by_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::now_v7())
    .bind(ctx.tenant_id)
    .bind(customer_id)
    .bind(amount_minor)
    .bind(refund_id)
    .bind(ctx.sale_id)
    .bind(ctx.executor_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query("UPDATE atlas.customers SET store_credit_amount_minor = store_credit_amount_minor + $2, updated_at = now() WHERE id = $1")
        .bind(customer_id)
        .bind(amount_minor)
        .execute(&mut *conn)
        .await?;
    return Ok(());
}

/// Fails early, before anything is written, if the chosen refund route can't work for this sale.
async fn check_refund_target(conn: &mut PgConnection, payload: &CreateReturnRequest, customer_id: Option<Uuid>) -> Result<(), ApiError> {
    return match payload.refund.method {
        RefundMethod::Original => Ok(()),
        RefundMethod::StoreCredit if customer_id.is_none() => Err(ApiError::bad_request_with_code(
            "A walk-in sale can't be refunded to store credit - there's no customer to hold it.",
            codes::CUSTOMER_REQUIRED,
        )),
        RefundMethod::StoreCredit => Ok(()),
        RefundMethod::PaymentMethod => {
            let method_id = payload
                .refund
                .payment_method_id
                .ok_or_else(|| ApiError::bad_request("payment_method_id is required for a payment_method refund"))?;
            let method = payments::active_method(conn, method_id).await?;
            if method.kind == "store_credit" {
                return Err(ApiError::bad_request("use the store_credit refund method for store credit"));
            }
            if method.requires_reference && payload.refund.reference.as_deref().is_none_or(|r| r.trim().is_empty()) {
                return Err(ApiError::bad_request_with_code(
                    format!("{} refunds need a transaction reference.", method.name),
                    codes::PAYMENT_REFERENCE_REQUIRED,
                ));
            }
            Ok(())
        }
    };
}

async fn returnable_line(conn: &mut PgConnection, sale_id: Uuid, sale_line_id: Uuid) -> Result<ReturnableLine, ApiError> {
    return sqlx::query_as::<_, ReturnableLine>(
        "SELECT sl.product_name, sl.quantity, sl.base_quantity, sl.line_total_amount_minor, sl.tax_amount_minor,
                p.is_returnable,
                COALESCE(claimed.quantity, 0) AS returned_quantity,
                COALESCE(claimed.refund, 0)::bigint AS refunded_minor,
                COALESCE(claimed.tax_refund, 0)::bigint AS tax_refunded_minor
         FROM atlas.sale_lines sl
         JOIN atlas.products p ON p.id = sl.product_id
         LEFT JOIN LATERAL (
             SELECT SUM(rl.quantity) AS quantity, SUM(rl.refund_amount_minor) AS refund, SUM(rl.tax_refund_minor) AS tax_refund
             FROM atlas.return_lines rl
             JOIN atlas.returns r ON r.id = rl.return_id
             WHERE rl.sale_line_id = sl.id AND r.status IN ('pending_approval', 'completed')
         ) claimed ON true
         WHERE sl.id = $1 AND sl.sale_id = $2",
    )
    .bind(sale_line_id)
    .bind(sale_id)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| ApiError::bad_request("That line isn't on this sale."));
}

async fn next_return_number(conn: &mut PgConnection, tenant_id: Uuid, store_id: Uuid) -> Result<String, ApiError> {
    sqlx::query("INSERT INTO atlas.store_return_sequences (store_id, tenant_id) VALUES ($1, $2) ON CONFLICT (store_id) DO NOTHING")
        .bind(store_id)
        .bind(tenant_id)
        .execute(&mut *conn)
        .await?;
    let sequence: i64 = sqlx::query_scalar(
        "UPDATE atlas.store_return_sequences SET next_value = next_value + 1 WHERE store_id = $1 RETURNING next_value - 1",
    )
    .bind(store_id)
    .fetch_one(&mut *conn)
    .await?;
    let prefix = receipts::store_prefix(conn, store_id).await?;
    return Ok(format!("R-{}-{:08}", prefix, sequence));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(rfc3339: &str) -> DateTime<Utc> {
        return DateTime::parse_from_rfc3339(rfc3339).unwrap().with_timezone(&Utc);
    }

    #[test]
    fn the_window_runs_to_the_end_of_the_last_local_day() {
        // Sold 15:00 Lagos time (UTC+1) on Monday 5 October 2026.
        let sold = at("2026-10-05T14:00:00Z");
        assert!(within_window(sold, 7, "Africa/Lagos", at("2026-10-12T22:59:00Z")));
        assert!(!within_window(sold, 7, "Africa/Lagos", at("2026-10-12T23:00:00Z")));
        // A zero-day window still allows the rest of the day of sale.
        assert!(within_window(sold, 0, "Africa/Lagos", at("2026-10-05T22:00:00Z")));
        assert!(!within_window(sold, 0, "Africa/Lagos", at("2026-10-06T00:00:00Z")));
    }
}
