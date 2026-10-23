/*!
 * Payments after checkout (FR-16): adding a payment to a sale that still
 * owes money, confirming or failing a pending one, reversing one, and
 * keeping the sale's paid amount and payment status in step with its
 * payment rows. Every path locks the sale row first, so two payments
 * landing at once can't both pass the "doesn't exceed the balance" check.
 */
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{codes, ApiError};
use crate::inventory::NegativeStockPolicy;
use crate::payments::models::Payment;
use crate::payments::service::{self as payments, NewPayment};
use crate::pricing::resolver;

use super::checkout;
use super::models::{PaymentInput, PaymentInputStatus, Sale};
use super::service;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentOutcome {
    Succeeded,
    Failed,
}

/**
 * Records a further payment against a sale that is awaiting payment, or completed under pay-later with a balance left.
 * Idempotent by the payment's client id: replaying the same payment returns it rather than charging twice.
 */
pub async fn add_payment(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    sale_id: Uuid,
    actor_user_id: Uuid,
    input: &PaymentInput,
) -> Result<Payment, ApiError> {
    let sale = service::lock_sale(conn, sale_id).await?;

    if let Some(id) = input.id {
        let existing: Option<(Uuid, Uuid, i64)> =
            sqlx::query_as("SELECT sale_id, payment_method_id, amount_minor FROM atlas.payments WHERE id = $1")
                .bind(id)
                .fetch_optional(&mut *conn)
                .await?;
        if let Some((existing_sale, existing_method, existing_amount)) = existing {
            if existing_sale == sale_id && existing_method == input.payment_method_id && existing_amount == input.amount_minor {
                return payments::get_payment(conn, id).await;
            }
            return Err(ApiError::conflict_with_code("A different payment was already recorded with this id.", codes::IDEMPOTENCY_CONFLICT));
        }
    }

    if sale.status != "awaiting_payment" && sale.status != "completed" {
        return Err(ApiError::conflict_with_code("Payments can only be added to an open or pay-later sale.", codes::SALE_NOT_COMPLETED));
    }
    let method = payments::active_method(conn, input.payment_method_id).await?;
    if method.kind == "store_credit" {
        return Err(ApiError::bad_request_with_code("Store credit can't be used as a tender yet.", codes::PAYMENT_METHOD_INVALID));
    }
    if input.amount_minor <= 0 {
        return Err(ApiError::bad_request("payment amounts must be positive"));
    }
    if method.requires_reference && input.reference.as_deref().is_none_or(|r| r.trim().is_empty()) {
        return Err(ApiError::bad_request_with_code(
            format!("{} payments need a transaction reference.", method.name),
            codes::PAYMENT_REFERENCE_REQUIRED,
        ));
    }
    if method.kind == "cash" && input.status == PaymentInputStatus::Pending {
        return Err(ApiError::bad_request("a cash payment can't be pending"));
    }
    let change = match input.tendered_amount_minor {
        Some(tendered) if tendered < input.amount_minor => {
            return Err(ApiError::bad_request("tendered amount is less than the amount applied"));
        }
        Some(tendered) if method.kind == "cash" => tendered - input.amount_minor,
        Some(tendered) if tendered != input.amount_minor => {
            return Err(ApiError::bad_request("only cash payments can be tendered above the amount applied"));
        }
        _ => 0,
    };

    let pending: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_minor), 0)::bigint FROM atlas.payments WHERE sale_id = $1 AND status = 'pending'",
    )
    .bind(sale_id)
    .fetch_one(&mut *conn)
    .await?;
    let remaining = sale.total_amount_minor + sale.rounding_adjustment_minor - sale.amount_paid_minor - pending;
    if input.amount_minor > remaining {
        return Err(ApiError::bad_request_with_code(
            format!("Only {} is still owed on this sale.", remaining.max(0)),
            codes::PAYMENT_OVERPAID,
        ));
    }

    let shift_id = current_shift(conn, &sale).await?;
    let payment = payments::insert_payment(
        conn,
        NewPayment {
            id: input.id.unwrap_or_else(Uuid::now_v7),
            tenant_id,
            sale_id,
            shift_id,
            method: &method,
            amount_minor: input.amount_minor,
            tendered_amount_minor: input.tendered_amount_minor,
            change_amount_minor: change,
            currency: &sale.currency,
            status: match input.status {
                PaymentInputStatus::Succeeded => "succeeded",
                PaymentInputStatus::Pending => "pending",
            },
            reference: input.reference.as_deref().map(str::trim).filter(|r| !r.is_empty()),
            received_by_user_id: actor_user_id,
        },
    )
    .await?;
    recompute(conn, tenant_id, sale_id, actor_user_id).await?;
    return Ok(payment);
}

/// Confirms (`succeeded`) or fails a pending payment - a card terminal's answer arriving after the sale was rung up.
pub async fn resolve_pending(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payment_id: Uuid,
    actor_user_id: Uuid,
    outcome: PaymentOutcome,
    reference: Option<&str>,
    reason: Option<&str>,
) -> Result<Payment, ApiError> {
    let sale_id = payment_sale_id(conn, payment_id).await?;
    service::lock_sale(conn, sale_id).await?;
    let to = match outcome {
        PaymentOutcome::Succeeded => "succeeded",
        PaymentOutcome::Failed => "failed",
    };
    transition(conn, payment_id, &["pending"], to, reference, reason).await?;
    recompute(conn, tenant_id, sale_id, actor_user_id).await?;
    return payments::get_payment(conn, payment_id).await;
}

/**
 * Reverses a successful payment that should never have counted (the wrong terminal, a transfer that bounced).
 * A payment with money already refunded against it can't be reversed - that would return the same money twice.
 */
pub async fn reverse(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    payment_id: Uuid,
    actor_user_id: Uuid,
    reason: &str,
) -> Result<Payment, ApiError> {
    if reason.trim().is_empty() {
        return Err(ApiError::bad_request("a reversal needs a reason"));
    }
    let sale_id = payment_sale_id(conn, payment_id).await?;
    service::lock_sale(conn, sale_id).await?;
    let refunded: i64 = sqlx::query_scalar("SELECT refunded_amount_minor FROM atlas.payments WHERE id = $1")
        .bind(payment_id)
        .fetch_one(&mut *conn)
        .await?;
    if refunded > 0 {
        return Err(ApiError::conflict_with_code(
            "Part of this payment has been refunded; it can't also be reversed.",
            codes::INVALID_PAYMENT_TRANSITION,
        ));
    }
    transition(conn, payment_id, &["succeeded"], "reversed", None, Some(reason)).await?;
    recompute(conn, tenant_id, sale_id, actor_user_id).await?;
    return payments::get_payment(conn, payment_id).await;
}

/**
 * The payment state machine, in one guarded UPDATE: `pending -> succeeded | failed`, `succeeded -> reversed`. Refund states are set by the refund path, never here.
 * Anything else (failing a payment that already succeeded, confirming a reversed one) is `INVALID_PAYMENT_TRANSITION`.
 */
async fn transition(
    conn: &mut PgConnection,
    payment_id: Uuid,
    from: &[&str],
    to: &str,
    reference: Option<&str>,
    reason: Option<&str>,
) -> Result<(), ApiError> {
    let from: Vec<String> = from.iter().map(|s| s.to_string()).collect();
    let updated = sqlx::query(
        "UPDATE atlas.payments
         SET status = $2::atlas.payment_status,
             reference = COALESCE($3, reference),
             status_reason = COALESCE($4, status_reason),
             processed_at = CASE WHEN $2 = 'succeeded' THEN now() ELSE processed_at END,
             updated_at = now()
         WHERE id = $1 AND status::text = ANY($5)",
    )
    .bind(payment_id)
    .bind(to)
    .bind(reference.map(str::trim).filter(|r| !r.is_empty()))
    .bind(reason)
    .bind(&from)
    .execute(&mut *conn)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err) if db_err.constraint() == Some("payments_reference_unique_idx") => {
            ApiError::conflict_with_code("A live payment with this reference is already recorded.", codes::DUPLICATE_PAYMENT_REFERENCE)
        }
        other => ApiError::from(other),
    })?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::conflict_with_code(
            format!("This payment can't move to {} from its current status.", to),
            codes::INVALID_PAYMENT_TRANSITION,
        ));
    }
    return Ok(());
}

/**
 * Brings the sale's `amount_paid_minor` and `payment_status` back in line with its payment rows, and completes a sale that was waiting on payment once it's covered (taking its stock at that point).
 * Refunded payments still count as paid here; money returned is tracked on `amount_refunded_minor`.
 */
pub async fn recompute(conn: &mut PgConnection, tenant_id: Uuid, sale_id: Uuid, actor_user_id: Uuid) -> Result<Sale, ApiError> {
    let sale = service::lock_sale(conn, sale_id).await?;
    let paid: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_minor), 0)::bigint FROM atlas.payments
         WHERE sale_id = $1 AND status IN ('succeeded', 'partially_refunded', 'refunded')",
    )
    .bind(sale_id)
    .fetch_one(&mut *conn)
    .await?;
    let due = sale.total_amount_minor + sale.rounding_adjustment_minor;
    let payment_status = if paid >= due {
        "paid"
    } else if paid > 0 {
        "partially_paid"
    } else {
        "unpaid"
    };
    sqlx::query("UPDATE atlas.sales SET amount_paid_minor = $2, payment_status = $3::atlas.sale_payment_status WHERE id = $1")
        .bind(sale_id)
        .bind(paid)
        .bind(payment_status)
        .execute(&mut *conn)
        .await?;

    if sale.status == "awaiting_payment" && paid >= due {
        let store = resolver::load_store_pricing(conn, sale.store_id).await?;
        let policy = if store.allow_negative_stock { NegativeStockPolicy::Allow } else { NegativeStockPolicy::Reject };
        checkout::complete_sale(conn, tenant_id, sale_id, actor_user_id, policy).await?;
    }
    return service::get_sale(conn, sale_id).await;
}

async fn payment_sale_id(conn: &mut PgConnection, payment_id: Uuid) -> Result<Uuid, ApiError> {
    return sqlx::query_scalar("SELECT sale_id FROM atlas.payments WHERE id = $1")
        .bind(payment_id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| ApiError::not_found("Payment not found."));
}

/// A later payment lands in whatever shift is open at the sale's store now, falling back to the sale's own shift - cash taken today belongs in today's drawer count.
async fn current_shift(conn: &mut PgConnection, sale: &Sale) -> Result<Uuid, ApiError> {
    let open: Option<Uuid> = sqlx::query_scalar("SELECT id FROM atlas.shifts WHERE store_id = $1 AND status = 'open'")
        .bind(sale.store_id)
        .fetch_optional(conn)
        .await?;
    return Ok(open.unwrap_or(sale.shift_id));
}
